use interbtc_runtime::{primitives::Block, RuntimeApi};
use sc_client_api::{ExecutorProvider, RemoteBackend};
use sc_consensus_aura::{ImportQueueParams, SlotProportion, StartAuraParams};
use sc_consensus_manual_seal::{InstantSealParams, ManualSealParams, consensus::babe::{BabeConsensusDataProvider, SlotTimestampProvider}, rpc::{ManualSeal, ManualSealApi}, run_manual_seal};
use sc_executor::NativeElseWasmExecutor;
use sc_finality_grandpa::SharedVoterState;
use sc_keystore::LocalKeystore;
use sc_service::{error::Error as ServiceError, Configuration, RpcHandlers, TFullBackend, TFullClient, TaskManager};
use sc_telemetry::{Telemetry, TelemetryWorker};
use sp_consensus::SlotData;
use sp_consensus_aura::sr25519::AuthorityPair as AuraPair;
use sc_consensus_babe::{self, AuthorityId};
use std::{sync::Arc, time::Duration};
use futures::channel::mpsc;


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

    let slot_duration = sc_consensus_aura::slot_duration(&*client)?.slot_duration();

    let import_queue = sc_consensus_aura::import_queue::<AuraPair, _, _, _, _, _, _>(ImportQueueParams {
        block_import: grandpa_block_import.clone(),
        justification_import: Some(Box::new(grandpa_block_import.clone())),
        client: client.clone(),
        create_inherent_data_providers: move |_, ()| async move {
            let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

            let slot = sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_duration(
                *timestamp,
                slot_duration,
            );

            Ok((timestamp, slot))
        },
        spawner: &task_manager.spawn_essential_handle(),
        can_author_with: sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone()),
        registry: config.prometheus_registry(),
        check_for_equivocation: Default::default(),
        telemetry: telemetry.as_ref().map(|x| x.handle()),
    })?;

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

fn remote_keystore(_url: &String) -> Result<Arc<LocalKeystore>, &'static str> {
    // FIXME: here would the concrete keystore be built,
    //        must return a concrete type (NOT `LocalKeystore`) that
    //        implements `CryptoStore` and `SyncCryptoStore`
    Err("Remote Keystore not supported.")
}

/// Builds a new service for a full client.
pub fn new_full(mut config: Configuration) -> Result<(TaskManager, RpcHandlers), ServiceError> {
    let sc_service::PartialComponents {
        client,
        backend,
        mut task_manager,
        import_queue,
        mut keystore_container,
        select_chain,
        transaction_pool,
        other: (grandpa_block_import, grandpa_link, mut telemetry),
    } = new_partial(&config)?;

	let slot_duration = sc_consensus_babe::Config::get_or_compute(&*client)?;
    let (block_import, babe_link) = sc_consensus_babe::block_import(
		slot_duration.clone(),
		grandpa_block_import,
		client.clone(),
	)?;

	let consensus_data_provider = BabeConsensusDataProvider::new(
		client.clone(),
		keystore_container.sync_keystore(),
		babe_link.epoch_changes().clone(),
		vec![(AuthorityId::from(Alice.public()), 1000)],
	)
	.expect("failed to create ConsensusDataProvider");

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

	let cloned_client = client.clone();
	let create_inherent_data_providers = Box::new(move |_, _| {
		let client = cloned_client.clone();
		async move {
			let timestamp =
				SlotTimestampProvider::new(client.clone()).map_err(|err| format!("{:?}", err))?;
			let babe =
				sp_consensus_babe::inherents::InherentDataProvider::new(timestamp.slot().into());
			Ok((timestamp, babe))
		}
	});

	// Background authorship future.
	let authorship_future = run_manual_seal(ManualSealParams {
		block_import,
		env,
		client: client.clone(),
		pool: transaction_pool.clone(),
		commands_stream,
		select_chain,
		consensus_data_provider: Some(Box::new(consensus_data_provider)),
		create_inherent_data_providers,
	});

	// spawn the authorship task as an essential task.
	task_manager.spawn_essential_handle().spawn("manual-seal", authorship_future);

	let rpc_handler = rpc_handlers.io_handler();

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

    let select_chain = sc_consensus::LongestChain::new(backend.clone());

    let transaction_pool = Arc::new(sc_transaction_pool::BasicPool::new_light(
        config.transaction_pool.clone(),
        config.prometheus_registry(),
        task_manager.spawn_essential_handle(),
        client.clone(),
        on_demand.clone(),
    ));

    let (grandpa_block_import, _) = sc_finality_grandpa::block_import(
        client.clone(),
        &(client.clone() as Arc<_>),
        select_chain.clone(),
        telemetry.as_ref().map(|x| x.handle()),
    )?;

    let slot_duration = sc_consensus_aura::slot_duration(&*client)?.slot_duration();

    let import_queue = sc_consensus_aura::import_queue::<AuraPair, _, _, _, _, _, _>(ImportQueueParams {
        block_import: grandpa_block_import.clone(),
        justification_import: Some(Box::new(grandpa_block_import.clone())),
        client: client.clone(),
        create_inherent_data_providers: move |_, ()| async move {
            let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

            let slot = sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_duration(
                *timestamp,
                slot_duration,
            );

            Ok((timestamp, slot))
        },
        spawner: &task_manager.spawn_essential_handle(),
        can_author_with: sp_consensus::NeverCanAuthor,
        registry: config.prometheus_registry(),
        check_for_equivocation: Default::default(),
        telemetry: telemetry.as_ref().map(|x| x.handle()),
    })?;

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
