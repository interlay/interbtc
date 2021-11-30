use futures::{channel::mpsc, stream, StreamExt};
use interbtc_runtime::{primitives::Block, RuntimeApi};
use sc_consensus_manual_seal::{
    rpc::{ManualSeal, ManualSealApi},
    EngineCommand, ManualSealParams,
};
use sc_executor::NativeElseWasmExecutor;
use sc_service::{
    error::Error as ServiceError, Configuration, RpcHandlers, TFullBackend, TFullClient, TaskManager, TransactionPool,
};
use sp_consensus::SlotData;
use std::sync::Arc;

use interbtc_runtime::Hash;

// Native executor instance.
pub struct Executor;

impl sc_executor::NativeExecutionDispatch for Executor {
    type ExtendHostFunctions = frame_benchmarking::benchmarking::HostFunctions;

    fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
        interbtc_runtime::api::dispatch(method, data)
    }

    fn native_version() -> sc_executor::NativeVersion {
        interbtc_runtime::native_version()
    }
}

pub type FullClient = TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>;
pub type FullBackend = TFullBackend<Block>;

type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;

pub fn new_partial(
    config: &Configuration,
) -> Result<
    sc_service::PartialComponents<
        FullClient,
        FullBackend,
        FullSelectChain,
        sc_consensus::DefaultImportQueue<Block, FullClient>,
        sc_transaction_pool::FullPool<Block, FullClient>,
        (),
    >,
    ServiceError,
> {
    if config.keystore_remote.is_some() {
        return Err(ServiceError::Other(format!("Remote Keystores are not supported.")));
    }

    let executor = NativeElseWasmExecutor::<Executor>::new(
        config.wasm_method,
        config.default_heap_pages,
        config.max_runtime_instances,
    );

    let (client, backend, keystore_container, task_manager) =
        sc_service::new_full_parts::<Block, RuntimeApi, _>(&config, None, executor)?;
    let client = Arc::new(client);

    let select_chain = sc_consensus::LongestChain::new(backend.clone());

    let import_queue = sc_consensus_manual_seal::import_queue(
        Box::new(client.clone()),
        &task_manager.spawn_essential_handle(),
        config.prometheus_registry(),
    );

    let transaction_pool = sc_transaction_pool::BasicPool::new_full(
        config.transaction_pool.clone(),
        // Authority is irrelevant in SEAL block production
        true.into(),
        config.prometheus_registry(),
        task_manager.spawn_essential_handle(),
        client.clone(),
    );

    Ok(sc_service::PartialComponents {
        client,
        backend,
        task_manager,
        import_queue,
        keystore_container,
        select_chain,
        transaction_pool,
        other: (),
    })
}

/// Builds a new service for a full client.
pub fn new_full(config: Configuration) -> Result<(TaskManager, RpcHandlers), ServiceError> {
    let sc_service::PartialComponents {
        client,
        backend,
        mut task_manager,
        import_queue,
        keystore_container,
        select_chain,
        transaction_pool,
        other: _,
    } = new_partial(&config)?;

    let (network, system_rpc_tx, network_starter) = sc_service::build_network(sc_service::BuildNetworkParams {
        config: &config,
        client: client.clone(),
        transaction_pool: transaction_pool.clone(),
        spawn_handle: task_manager.spawn_handle(),
        import_queue,
        on_demand: None,
        block_announce_validator_builder: None,
        warp_sync: None,
    })?;

    if config.offchain_worker.enabled {
        sc_service::build_offchain_workers(&config, task_manager.spawn_handle(), client.clone(), network.clone());
    }

    // Proposer object for block authorship.
    let env = sc_basic_authorship::ProposerFactory::new(
        task_manager.spawn_handle(),
        client.clone(),
        transaction_pool.clone(),
        config.prometheus_registry(),
        None,
    );

    // Channel for the rpc handler to communicate with the authorship task.
    let (command_sink, commands_stream) = mpsc::channel::<EngineCommand<Hash>>(10);

    let rpc_sink = command_sink.clone();

    let pool_import_stream = transaction_pool
        .clone()
        .pool()
        .validated_pool()
        .import_notification_stream();

    let pool_stream = pool_import_stream.map(|_| EngineCommand::SealNewBlock {
        create_empty: true,
        finalize: true,
        parent_hash: None,
        sender: None,
    });

    let combined_stream = stream::select(pool_stream, commands_stream);

    let rpc_extensions_builder = {
        let client = client.clone();
        let pool = transaction_pool.clone();

        Box::new(move |deny_unsafe, _| {
            let deps = interbtc_rpc::FullDeps {
                client: client.clone(),
                pool: pool.clone(),
                deny_unsafe,
            };

            let mut io = interbtc_rpc::create_full(deps);
            io.extend_with(ManualSealApi::to_delegate(ManualSeal::new(rpc_sink.clone())));

            Ok(io)
        })
    };

    let rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        config,
        client: client.clone(),
        backend: backend.clone(),
        task_manager: &mut task_manager,
        keystore: keystore_container.sync_keystore(),
        on_demand: None,
        transaction_pool: transaction_pool.clone(),
        rpc_extensions_builder,
        remote_blockchain: None,
        network,
        system_rpc_tx,
        telemetry: None,
    })?;

    let slot_duration = sc_consensus_aura::slot_duration(&*client)?.slot_duration();

    // Background authorship future.
    let authorship_future = sc_consensus_manual_seal::run_manual_seal(ManualSealParams {
        block_import: client.clone(),
        env,
        client: client.clone(),
        pool: transaction_pool.clone(),
        commands_stream: combined_stream,
        select_chain,
        consensus_data_provider: None,
        create_inherent_data_providers: move |_, ()| async move {
            let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

            let slot = sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_duration(
                *timestamp,
                slot_duration,
            );

            Ok((timestamp, slot))
        },
    });

    // spawn the authorship task as an essential task.
    task_manager
        .spawn_essential_handle()
        .spawn("manual-seal", authorship_future);

    network_starter.start_network();
    Ok((task_manager, rpc_handlers))
}
