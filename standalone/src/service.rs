use futures::channel::mpsc;
use interbtc_runtime::{primitives::Block, RuntimeApi};
use sc_client_api::RemoteBackend;
use sc_consensus_manual_seal::{
    rpc::{ManualSeal, ManualSealApi},
    ManualSealParams,
};
use sc_executor::NativeElseWasmExecutor;
use sc_service::{error::Error as ServiceError, Configuration, RpcHandlers, TFullBackend, TFullClient, TaskManager};
use sc_telemetry::{Telemetry, TelemetryWorker};
use std::sync::Arc;

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
        (
            sc_finality_grandpa::GrandpaBlockImport<FullBackend, Block, FullClient, FullSelectChain>,
            sc_finality_grandpa::LinkHalf<Block, FullClient, FullSelectChain>,
            Option<Telemetry>,
        ),
    >,
    ServiceError,
> {
    if config.keystore_remote.is_some() {
        return Err(ServiceError::Other(format!("Remote Keystores are not supported.")));
    }

    let telemetry = config
        .telemetry_endpoints
        .clone()
        .filter(|x| !x.is_empty())
        .map(|endpoints| -> Result<_, sc_telemetry::Error> {
            let worker = TelemetryWorker::new(16)?;
            let telemetry = worker.handle().new_telemetry(endpoints);
            Ok((worker, telemetry))
        })
        .transpose()?;

    let executor = NativeElseWasmExecutor::<Executor>::new(
        config.wasm_method,
        config.default_heap_pages,
        config.max_runtime_instances,
    );

    let (client, backend, keystore_container, task_manager) = sc_service::new_full_parts::<Block, RuntimeApi, _>(
        &config,
        telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
        executor,
    )?;
    let client = Arc::new(client);

    let telemetry = telemetry.map(|(worker, telemetry)| {
        task_manager.spawn_handle().spawn("telemetry", worker.run());
        telemetry
    });

    let select_chain = sc_consensus::LongestChain::new(backend.clone());

    let transaction_pool = sc_transaction_pool::BasicPool::new_full(
        config.transaction_pool.clone(),
        // Authority is irrelevant in SEAL block production
        true.into(),
        config.prometheus_registry(),
        task_manager.spawn_essential_handle(),
        client.clone(),
    );

    let (grandpa_block_import, grandpa_link) = sc_finality_grandpa::block_import(
        client.clone(),
        &(client.clone() as Arc<_>),
        select_chain.clone(),
        telemetry.as_ref().map(|x| x.handle()),
    )?;

    let import_queue = sc_consensus_manual_seal::import_queue(
        Box::new(client.clone()),
        &task_manager.spawn_essential_handle(),
        config.prometheus_registry(),
    );

    Ok(sc_service::PartialComponents {
        client,
        backend,
        task_manager,
        import_queue,
        keystore_container,
        select_chain,
        transaction_pool,
        other: (grandpa_block_import, grandpa_link, telemetry),
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
        other: (_grandpa_block_import, _grandpa_link, mut telemetry),
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
    let (command_sink, commands_stream) = mpsc::channel(10);

    let rpc_sink = command_sink.clone();

    let rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        config,
        client: client.clone(),
        backend: backend.clone(),
        task_manager: &mut task_manager,
        keystore: keystore_container.sync_keystore(),
        on_demand: None,
        transaction_pool: transaction_pool.clone(),
        rpc_extensions_builder: Box::new(move |_, _| {
            let mut io = jsonrpc_core::IoHandler::default();
            io.extend_with(ManualSealApi::to_delegate(ManualSeal::new(rpc_sink.clone())));
            Ok(io)
        }),
        remote_blockchain: None,
        network,
        system_rpc_tx,
        telemetry: telemetry.as_mut(),
    })?;

    // Background authorship future.
    let authorship_future = sc_consensus_manual_seal::run_manual_seal(ManualSealParams {
        block_import: client.clone(),
        env,
        client,
        pool: transaction_pool.clone(),
        commands_stream,
        select_chain,
        consensus_data_provider: None,
        create_inherent_data_providers: move |_, ()| async move {
            Ok(sp_timestamp::InherentDataProvider::from_system_time())
        },
    });
    // spawn the authorship task as an essential task.
    task_manager
        .spawn_essential_handle()
        .spawn("manual-seal", authorship_future);

    network_starter.start_network();
    Ok((task_manager, rpc_handlers))
}

/// Builds a new service for a light client.
pub fn new_light(mut config: Configuration) -> Result<(TaskManager, RpcHandlers), ServiceError> {
    let telemetry = config
        .telemetry_endpoints
        .clone()
        .filter(|x| !x.is_empty())
        .map(|endpoints| -> Result<_, sc_telemetry::Error> {
            let worker = TelemetryWorker::new(16)?;
            let telemetry = worker.handle().new_telemetry(endpoints);
            Ok((worker, telemetry))
        })
        .transpose()?;

    let executor = NativeElseWasmExecutor::<Executor>::new(
        config.wasm_method,
        config.default_heap_pages,
        config.max_runtime_instances,
    );

    let (client, backend, keystore_container, mut task_manager, on_demand) =
        sc_service::new_light_parts::<Block, RuntimeApi, _>(
            &config,
            telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
            executor,
        )?;

    let mut telemetry = telemetry.map(|(worker, telemetry)| {
        task_manager.spawn_handle().spawn("telemetry", worker.run());
        telemetry
    });

    config
        .network
        .extra_sets
        .push(sc_finality_grandpa::grandpa_peers_set_config());

    let transaction_pool = Arc::new(sc_transaction_pool::BasicPool::new_light(
        config.transaction_pool.clone(),
        config.prometheus_registry(),
        task_manager.spawn_essential_handle(),
        client.clone(),
        on_demand.clone(),
    ));

    let import_queue = sc_consensus_manual_seal::import_queue(
        Box::new(client.clone()),
        &task_manager.spawn_essential_handle(),
        config.prometheus_registry(),
    );

    let (network, system_rpc_tx, network_starter) = sc_service::build_network(sc_service::BuildNetworkParams {
        config: &config,
        client: client.clone(),
        transaction_pool: transaction_pool.clone(),
        spawn_handle: task_manager.spawn_handle(),
        import_queue,
        on_demand: Some(on_demand.clone()),
        block_announce_validator_builder: None,
        warp_sync: None,
    })?;

    if config.offchain_worker.enabled {
        sc_service::build_offchain_workers(&config, task_manager.spawn_handle(), client.clone(), network.clone());
    }

    let rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        remote_blockchain: Some(backend.remote_blockchain()),
        transaction_pool,
        task_manager: &mut task_manager,
        on_demand: Some(on_demand),
        rpc_extensions_builder: Box::new(|_, _| Ok(())),
        config,
        client,
        keystore: keystore_container.sync_keystore(),
        backend,
        network,
        system_rpc_tx,
        telemetry: telemetry.as_mut(),
    })?;

    network_starter.start_network();

    Ok((task_manager, rpc_handlers))
}
