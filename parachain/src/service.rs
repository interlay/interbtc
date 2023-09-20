use cumulus_client_cli::CollatorOptions;
use cumulus_client_consensus_aura::{AuraConsensus, BuildAuraConsensusParams, SlotProportion};
use cumulus_client_consensus_common::{ParachainBlockImport as TParachainBlockImport, ParachainConsensus};
use cumulus_client_network::RequireSecondedInBlockAnnounce;
use cumulus_client_service::{
    prepare_node_config, start_collator, start_full_node, StartCollatorParams, StartFullNodeParams,
};
use cumulus_primitives_core::ParaId;
use cumulus_primitives_parachain_inherent::{MockValidationDataInherentDataProvider, MockXcmConfig};
use cumulus_relay_chain_inprocess_interface::build_inprocess_relay_chain;
use cumulus_relay_chain_interface::{RelayChainInterface, RelayChainResult};
use cumulus_relay_chain_minimal_node::build_minimal_relay_chain_node;
use futures::StreamExt;
use polkadot_service::CollatorPair;
use primitives::*;
use sc_client_api::{Backend, HeaderBackend, StateBackendFor};
use sc_consensus::{ImportQueue, LongestChain};
use sc_executor::{HeapAllocStrategy, NativeElseWasmExecutor, WasmExecutor, DEFAULT_HEAP_ALLOC_STRATEGY};
use sc_network::NetworkBlock;
use sc_network_sync::SyncingService;
use sc_service::{Configuration, PartialComponents, RpcHandlers, TFullBackend, TFullClient, TaskManager};
use sc_telemetry::{Telemetry, TelemetryHandle, TelemetryWorker, TelemetryWorkerHandle};
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use sp_api::ConstructRuntimeApi;
use sp_consensus_aura::{
    sr25519::{AuthorityId as AuraId, AuthorityPair as AuraPair},
    SlotDuration,
};
use sp_core::H256;
use sp_keystore::KeystorePtr;
use sp_runtime::traits::{BlakeTwo256, Block as BlockT};
use std::{sync::Arc, time::Duration};
use substrate_prometheus_endpoint::Registry;

// Frontier imports
use crate::eth::{
    new_eth_deps, new_frontier_partial, open_frontier_backend, spawn_frontier_tasks, EthCompatRuntimeApiCollection,
    EthConfiguration, FrontierBackend, FrontierPartialComponents,
};

macro_rules! new_runtime_executor {
    ($name:ident,$runtime:ident) => {
        pub struct $name;

        impl sc_executor::NativeExecutionDispatch for $name {
            type ExtendHostFunctions = frame_benchmarking::benchmarking::HostFunctions;

            fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
                $runtime::api::dispatch(method, data)
            }

            fn native_version() -> sc_executor::NativeVersion {
                $runtime::native_version()
            }
        }
    };
}

// Native interlay executor instance.
new_runtime_executor!(InterlayRuntimeExecutor, interlay_runtime);

// Native kintsugi executor instance.
new_runtime_executor!(KintsugiRuntimeExecutor, kintsugi_runtime);

/// A set of APIs that every runtime must implement.
pub trait BaseRuntimeApiCollection:
    sp_api::ApiExt<Block>
    + sp_api::Metadata<Block>
    + sp_block_builder::BlockBuilder<Block>
    + sp_offchain::OffchainWorkerApi<Block>
    + sp_session::SessionKeys<Block>
    + sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
where
    <Self as sp_api::ApiExt<Block>>::StateBackend: sp_api::StateBackend<BlakeTwo256>,
{
}

impl<Api> BaseRuntimeApiCollection for Api
where
    Api: sp_api::ApiExt<Block>
        + sp_api::Metadata<Block>
        + sp_block_builder::BlockBuilder<Block>
        + sp_offchain::OffchainWorkerApi<Block>
        + sp_session::SessionKeys<Block>
        + sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>,
    <Self as sp_api::ApiExt<Block>>::StateBackend: sp_api::StateBackend<BlakeTwo256>,
{
}
pub trait RuntimeApiCollection:
    BaseRuntimeApiCollection
    + EthCompatRuntimeApiCollection
    + substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>
    + pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>
    + cumulus_primitives_core::CollectCollationInfo<Block>
    + btc_relay_rpc_runtime_api::BtcRelayApi<Block, H256Le>
    + oracle_rpc_runtime_api::OracleApi<Block, Balance, CurrencyId>
    + vault_registry_rpc_runtime_api::VaultRegistryApi<
        Block,
        VaultId<AccountId, CurrencyId>,
        Balance,
        UnsignedFixedPoint,
        CurrencyId,
        AccountId,
    > + escrow_rpc_runtime_api::EscrowApi<Block, AccountId, BlockNumber, Balance>
    + issue_rpc_runtime_api::IssueApi<
        Block,
        AccountId,
        H256,
        issue::IssueRequest<AccountId, BlockNumber, Balance, CurrencyId>,
    > + redeem_rpc_runtime_api::RedeemApi<
        Block,
        AccountId,
        H256,
        redeem::RedeemRequest<AccountId, BlockNumber, Balance, CurrencyId>,
    > + replace_rpc_runtime_api::ReplaceApi<
        Block,
        AccountId,
        H256,
        replace::ReplaceRequest<AccountId, BlockNumber, Balance, CurrencyId>,
    > + reward_rpc_runtime_api::RewardApi<
        Block,
        AccountId,
        VaultId<AccountId, CurrencyId>,
        CurrencyId,
        Balance,
        BlockNumber,
        UnsignedFixedPoint,
    > + loans_rpc_runtime_api::LoansApi<Block, AccountId, Balance>
    + dex_general_rpc_runtime_api::DexGeneralApi<Block, AccountId, CurrencyId>
    + dex_stable_rpc_runtime_api::DexStableApi<Block, CurrencyId, Balance, AccountId, StablePoolId>
where
    <Self as sp_api::ApiExt<Block>>::StateBackend: sp_api::StateBackend<BlakeTwo256>,
{
}

impl<Api> RuntimeApiCollection for Api
where
    Api: BaseRuntimeApiCollection
        + EthCompatRuntimeApiCollection
        + substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>
        + pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>
        + cumulus_primitives_core::CollectCollationInfo<Block>
        + btc_relay_rpc_runtime_api::BtcRelayApi<Block, H256Le>
        + oracle_rpc_runtime_api::OracleApi<Block, Balance, CurrencyId>
        + vault_registry_rpc_runtime_api::VaultRegistryApi<
            Block,
            VaultId<AccountId, CurrencyId>,
            Balance,
            UnsignedFixedPoint,
            CurrencyId,
            AccountId,
        > + escrow_rpc_runtime_api::EscrowApi<Block, AccountId, BlockNumber, Balance>
        + issue_rpc_runtime_api::IssueApi<
            Block,
            AccountId,
            H256,
            issue::IssueRequest<AccountId, BlockNumber, Balance, CurrencyId>,
        > + redeem_rpc_runtime_api::RedeemApi<
            Block,
            AccountId,
            H256,
            redeem::RedeemRequest<AccountId, BlockNumber, Balance, CurrencyId>,
        > + replace_rpc_runtime_api::ReplaceApi<
            Block,
            AccountId,
            H256,
            replace::ReplaceRequest<AccountId, BlockNumber, Balance, CurrencyId>,
        > + reward_rpc_runtime_api::RewardApi<
            Block,
            AccountId,
            VaultId<AccountId, CurrencyId>,
            CurrencyId,
            Balance,
            BlockNumber,
            UnsignedFixedPoint,
        > + loans_rpc_runtime_api::LoansApi<Block, AccountId, Balance>
        + dex_general_rpc_runtime_api::DexGeneralApi<Block, AccountId, CurrencyId>
        + dex_stable_rpc_runtime_api::DexStableApi<Block, CurrencyId, Balance, AccountId, StablePoolId>,
    <Self as sp_api::ApiExt<Block>>::StateBackend: sp_api::StateBackend<BlakeTwo256>,
{
}

pub(crate) type FullBackend = TFullBackend<Block>;

pub(crate) type FullClient<RuntimeApi, ExecutorDispatch> =
    TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<ExecutorDispatch>>;

type MaybeFullSelectChain = Option<LongestChain<FullBackend, Block>>;

type ParachainBlockImport<RuntimeApi, ExecutorDispatch> =
    TParachainBlockImport<Block, Arc<FullClient<RuntimeApi, ExecutorDispatch>>, FullBackend>;

// 0x9af9a64e6e4da8e3073901c3ff0cc4c3aad9563786d89daf6ad820b6e14a0b8b
const KINTSUGI_GENESIS_HASH: H256 = H256([
    154, 249, 166, 78, 110, 77, 168, 227, 7, 57, 1, 195, 255, 12, 196, 195, 170, 217, 86, 55, 134, 216, 157, 175, 106,
    216, 32, 182, 225, 74, 11, 139,
]);

fn import_slot_duration<C>(client: &C) -> SlotDuration
where
    C: sc_client_api::backend::AuxStore
        + sp_api::ProvideRuntimeApi<Block>
        + sc_client_api::UsageProvider<Block>
        + sp_api::CallApiAt<Block>,
    C::Api: sp_consensus_aura::AuraApi<Block, AuraId>,
{
    if client.usage_info().chain.genesis_hash == KINTSUGI_GENESIS_HASH
        && client.usage_info().chain.best_number < 1983993
    {
        // the kintsugi runtime was misconfigured at genesis to use a slot duration of 6s
        // which stalled collators when we upgraded to polkadot-v0.9.16 and subsequently
        // broke mainnet when we introduced the aura timestamp hook, collators should only
        // switch when syncing after the (failed) 1.20.0 upgrade
        SlotDuration::from_millis(6000)
    } else {
        // this is pallet_timestamp::MinimumPeriod * 2 at the current height
        // on kintsugi we increased MinimumPeriod from 3_000 to 6_000 at 16_593
        // but the interlay runtime has always used 6_000
        sc_consensus_aura::slot_duration(&*client).unwrap()
    }
}

/// Starts a `ServiceBuilder` for a full service.
///
/// Use this macro if you don't actually need the full service, but just the builder in order to
/// be able to perform chain operations.
pub fn new_partial<RuntimeApi, Executor>(
    config: &Configuration,
    eth_config: &EthConfiguration,
    instant_seal: bool,
) -> Result<
    PartialComponents<
        FullClient<RuntimeApi, Executor>,
        FullBackend,
        MaybeFullSelectChain,
        sc_consensus::DefaultImportQueue<Block, FullClient<RuntimeApi, Executor>>,
        sc_transaction_pool::FullPool<Block, FullClient<RuntimeApi, Executor>>,
        (
            ParachainBlockImport<RuntimeApi, Executor>,
            Option<Telemetry>,
            Option<TelemetryWorkerHandle>,
            FrontierBackend,
            Arc<fc_rpc::OverrideHandle<Block>>,
        ),
    >,
    sc_service::Error,
>
where
    RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>> + Send + Sync + 'static,
    RuntimeApi::RuntimeApi: BaseRuntimeApiCollection<StateBackend = StateBackendFor<FullBackend, Block>>,
    RuntimeApi::RuntimeApi: EthCompatRuntimeApiCollection<StateBackend = StateBackendFor<FullBackend, Block>>,
    RuntimeApi::RuntimeApi: sp_consensus_aura::AuraApi<Block, AuraId>,
    Executor: sc_executor::NativeExecutionDispatch + 'static,
{
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

    let heap_pages = config
        .default_heap_pages
        .map_or(DEFAULT_HEAP_ALLOC_STRATEGY, |h| HeapAllocStrategy::Static {
            extra_pages: h as _,
        });

    let executor = NativeElseWasmExecutor::<Executor>::new_with_wasm_executor(
        WasmExecutor::builder()
            .with_execution_method(config.wasm_method)
            .with_onchain_heap_alloc_strategy(heap_pages)
            .with_offchain_heap_alloc_strategy(heap_pages)
            .with_max_runtime_instances(config.max_runtime_instances)
            .with_runtime_cache_size(config.runtime_cache_size)
            .build(),
    );

    let (client, backend, keystore_container, task_manager) = sc_service::new_full_parts::<Block, RuntimeApi, _>(
        &config,
        telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
        executor,
    )?;
    let client = Arc::new(client);

    let telemetry_worker_handle = telemetry.as_ref().map(|(worker, _)| worker.handle());

    let telemetry = telemetry.map(|(worker, telemetry)| {
        task_manager.spawn_handle().spawn("telemetry", None, worker.run());
        telemetry
    });

    let registry = config.prometheus_registry();

    let transaction_pool = sc_transaction_pool::BasicPool::new_full(
        config.transaction_pool.clone(),
        config.role.is_authority().into(),
        registry,
        task_manager.spawn_essential_handle(),
        client.clone(),
    );

    let select_chain = if instant_seal {
        Some(LongestChain::new(backend.clone()))
    } else {
        None
    };

    let overrides = interbtc_rpc::overrides_handle(client.clone());
    let frontier_backend = open_frontier_backend(client.clone(), config, eth_config, overrides.clone())?;
    let parachain_block_import = ParachainBlockImport::new(client.clone(), backend.clone());

    let import_queue = if instant_seal {
        // instant sealing
        sc_consensus_manual_seal::import_queue(
            Box::new(client.clone()),
            &task_manager.spawn_essential_handle(),
            registry,
        )
    } else {
        let slot_duration = import_slot_duration(&*client);

        cumulus_client_consensus_aura::import_queue::<AuraPair, _, _, _, _, _>(
            cumulus_client_consensus_aura::ImportQueueParams {
                block_import: parachain_block_import.clone(),
                client: client.clone(),
                create_inherent_data_providers: move |_parent: sp_core::H256, _| async move {
                    let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

                    let slot = sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                        *timestamp,
                        slot_duration,
                    );

                    Ok((slot, timestamp))
                },
                registry,
                spawner: &task_manager.spawn_essential_handle(),
                telemetry: telemetry.as_ref().map(|telemetry| telemetry.handle()),
            },
        )?
    };

    let params = PartialComponents {
        backend,
        client: client.clone(),
        import_queue,
        keystore_container,
        task_manager,
        transaction_pool,
        select_chain,
        other: (
            parachain_block_import,
            telemetry,
            telemetry_worker_handle,
            frontier_backend,
            overrides,
        ),
    };

    Ok(params)
}

async fn build_relay_chain_interface(
    polkadot_config: Configuration,
    parachain_config: &Configuration,
    telemetry_worker_handle: Option<TelemetryWorkerHandle>,
    task_manager: &mut TaskManager,
    collator_options: CollatorOptions,
) -> RelayChainResult<(Arc<(dyn RelayChainInterface + 'static)>, Option<CollatorPair>)> {
    if !collator_options.relay_chain_rpc_urls.is_empty() {
        build_minimal_relay_chain_node(polkadot_config, task_manager, collator_options.relay_chain_rpc_urls).await
    } else {
        build_inprocess_relay_chain(
            polkadot_config,
            parachain_config,
            telemetry_worker_handle,
            task_manager,
            None,
        )
    }
}

/// Start a node with the given parachain `Configuration` and relay chain `Configuration`.
///
/// This is the actual implementation that is abstract over the executor and the runtime api.
#[sc_tracing::logging::prefix_logs_with("Parachain")]
async fn start_node_impl<RuntimeApi, Executor, CT, BIC>(
    parachain_config: Configuration,
    polkadot_config: Configuration,
    eth_config: EthConfiguration,
    collator_options: CollatorOptions,
    id: ParaId,
    build_consensus: BIC,
) -> sc_service::error::Result<(TaskManager, Arc<FullClient<RuntimeApi, Executor>>)>
where
    RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>> + Send + Sync + 'static,
    RuntimeApi::RuntimeApi: RuntimeApiCollection<StateBackend = sc_client_api::StateBackendFor<FullBackend, Block>>,
    RuntimeApi::RuntimeApi: sp_consensus_aura::AuraApi<Block, AuraId>,
    Executor: sc_executor::NativeExecutionDispatch + 'static,
    CT: fp_rpc::ConvertTransaction<<Block as BlockT>::Extrinsic> + Clone + Default + Send + Sync + 'static,
    BIC: FnOnce(
        Arc<FullClient<RuntimeApi, Executor>>,
        ParachainBlockImport<RuntimeApi, Executor>,
        Option<&Registry>,
        Option<TelemetryHandle>,
        &TaskManager,
        Arc<dyn RelayChainInterface>,
        Arc<sc_transaction_pool::FullPool<Block, FullClient<RuntimeApi, Executor>>>,
        Arc<SyncingService<Block>>,
        KeystorePtr,
        bool,
    ) -> Result<Box<dyn ParachainConsensus<Block>>, sc_service::Error>,
{
    let mut parachain_config = prepare_node_config(parachain_config);

    let params = new_partial(&parachain_config, &eth_config, false)?;
    let (parachain_block_import, mut telemetry, telemetry_worker_handle, frontier_backend, overrides) = params.other;
    let net_config = sc_network::config::FullNetworkConfiguration::new(&parachain_config.network);

    let client = params.client.clone();
    let backend = params.backend.clone();
    let mut task_manager = params.task_manager;

    let (relay_chain_interface, collator_key) = build_relay_chain_interface(
        polkadot_config,
        &parachain_config,
        telemetry_worker_handle,
        &mut task_manager,
        collator_options.clone(),
    )
    .await
    .map_err(|e| sc_service::Error::Application(Box::new(e) as Box<_>))?;

    let block_announce_validator = RequireSecondedInBlockAnnounce::new(relay_chain_interface.clone(), id);

    let force_authoring = parachain_config.force_authoring;
    let validator = parachain_config.role.is_authority();
    let prometheus_registry = parachain_config.prometheus_registry().cloned();
    let transaction_pool = params.transaction_pool.clone();
    let import_queue_service = params.import_queue.service();
    let (network, system_rpc_tx, tx_handler_controller, start_network, sync_service) =
        sc_service::build_network(sc_service::BuildNetworkParams {
            config: &parachain_config,
            net_config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue: params.import_queue,
            block_announce_validator_builder: Some(Box::new(|_| Box::new(block_announce_validator))),
            warp_sync_params: None,
        })?;

    let FrontierPartialComponents {
        filter_pool,
        fee_history_cache,
        fee_history_cache_limit,
    } = new_frontier_partial(&eth_config)?;

    let pubsub_notification_sinks: fc_mapping_sync::EthereumBlockNotificationSinks<
        fc_mapping_sync::EthereumBlockNotification<Block>,
    > = Default::default();
    let pubsub_notification_sinks = Arc::new(pubsub_notification_sinks);

    let eth_rpc_params = new_eth_deps::<_, _, _, CT>(
        client.clone(),
        transaction_pool.clone(),
        transaction_pool.pool().clone(),
        &mut parachain_config,
        &eth_config,
        network.clone(),
        sync_service.clone(),
        frontier_backend.clone(),
        overrides.clone(),
        &task_manager,
        filter_pool.clone(),
        fee_history_cache.clone(),
        fee_history_cache_limit,
    );

    let rpc_builder = {
        let client = client.clone();
        let transaction_pool = transaction_pool.clone();
        let pubsub_notification_sinks = pubsub_notification_sinks.clone();

        move |deny_unsafe, subscription_task_executor| {
            let deps = interbtc_rpc::FullDeps {
                client: client.clone(),
                pool: transaction_pool.clone(),
                deny_unsafe,
                command_sink: None,
                eth: eth_rpc_params.clone(),
            };

            interbtc_rpc::create_full(deps, subscription_task_executor, pubsub_notification_sinks.clone())
                .map_err(Into::into)
        }
    };

    if parachain_config.offchain_worker.enabled {
        use futures::FutureExt;

        task_manager.spawn_handle().spawn(
            "offchain-workers-runner",
            "offchain-work",
            sc_offchain::OffchainWorkers::new(sc_offchain::OffchainWorkerOptions {
                runtime_api_provider: client.clone(),
                keystore: Some(params.keystore_container.keystore()),
                offchain_db: backend.offchain_storage(),
                transaction_pool: Some(OffchainTransactionPoolFactory::new(transaction_pool.clone())),
                network_provider: network.clone(),
                is_validator: parachain_config.role.is_authority(),
                enable_http_requests: false,
                custom_extensions: move |_| vec![],
            })
            .run(client.clone(), task_manager.spawn_handle())
            .boxed(),
        );
    };

    sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        rpc_builder: Box::new(rpc_builder),
        client: client.clone(),
        transaction_pool: transaction_pool.clone(),
        task_manager: &mut task_manager,
        config: parachain_config,
        keystore: params.keystore_container.keystore(),
        backend: backend.clone(),
        network: network.clone(),
        system_rpc_tx,
        tx_handler_controller,
        telemetry: telemetry.as_mut(),
        sync_service: sync_service.clone(),
    })?;

    spawn_frontier_tasks(
        &task_manager,
        client.clone(),
        backend.clone(),
        frontier_backend,
        filter_pool,
        overrides,
        fee_history_cache,
        fee_history_cache_limit,
        sync_service.clone(),
        pubsub_notification_sinks,
    )
    .await;

    let announce_block = {
        let sync_service = sync_service.clone();
        Arc::new(move |hash, data| sync_service.announce_block(hash, data))
    };

    let relay_chain_slot_duration = Duration::from_secs(6);

    let overseer_handle = relay_chain_interface
        .overseer_handle()
        .map_err(|e| sc_service::Error::Application(Box::new(e)))?;

    if validator {
        let parachain_consensus = build_consensus(
            client.clone(),
            parachain_block_import,
            prometheus_registry.as_ref(),
            telemetry.as_ref().map(|t| t.handle()),
            &task_manager,
            relay_chain_interface.clone(),
            transaction_pool,
            sync_service.clone(),
            params.keystore_container.keystore(),
            force_authoring,
        )?;

        let spawner = task_manager.spawn_handle();

        let params = StartCollatorParams {
            para_id: id,
            block_status: client.clone(),
            announce_block,
            client: client.clone(),
            task_manager: &mut task_manager,
            relay_chain_interface,
            spawner,
            parachain_consensus,
            import_queue: import_queue_service,
            collator_key: collator_key.expect("Command line arguments do not allow this. qed"),
            relay_chain_slot_duration,
            recovery_handle: Box::new(overseer_handle),
            sync_service: sync_service.clone(),
        };

        start_collator(params).await?;
    } else {
        let params = StartFullNodeParams {
            client: client.clone(),
            announce_block,
            task_manager: &mut task_manager,
            para_id: id,
            relay_chain_interface,
            import_queue: import_queue_service,
            relay_chain_slot_duration,
            recovery_handle: Box::new(overseer_handle),
            sync_service: sync_service.clone(),
        };

        start_full_node(params)?;
    }

    start_network.start_network();

    Ok((task_manager, client))
}

/// Start a normal parachain node.
pub async fn start_node<RuntimeApi, Executor, CT>(
    parachain_config: Configuration,
    polkadot_config: Configuration,
    eth_config: EthConfiguration,
    collator_options: CollatorOptions,
    id: ParaId,
) -> sc_service::error::Result<(TaskManager, Arc<FullClient<RuntimeApi, Executor>>)>
where
    RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>> + Send + Sync + 'static,
    RuntimeApi::RuntimeApi: RuntimeApiCollection<StateBackend = sc_client_api::StateBackendFor<FullBackend, Block>>,
    RuntimeApi::RuntimeApi: sp_consensus_aura::AuraApi<Block, AuraId>,
    Executor: sc_executor::NativeExecutionDispatch + 'static,
    CT: fp_rpc::ConvertTransaction<<Block as BlockT>::Extrinsic> + Clone + Default + Send + Sync + 'static,
{
    start_node_impl::<_, _, CT, _>(
        parachain_config,
        polkadot_config,
        eth_config,
        collator_options,
        id,
        |client,
         block_import,
         prometheus_registry,
         telemetry,
         task_manager,
         relay_chain_interface,
         transaction_pool,
         sync_oracle,
         keystore,
         force_authoring| {
            let slot_duration = import_slot_duration(&*client);

            let proposer_factory = sc_basic_authorship::ProposerFactory::with_proof_recording(
                task_manager.spawn_handle(),
                client.clone(),
                transaction_pool,
                prometheus_registry,
                telemetry.clone(),
            );

            Ok(AuraConsensus::build::<
                sp_consensus_aura::sr25519::AuthorityPair,
                _,
                _,
                _,
                _,
                _,
                _,
            >(BuildAuraConsensusParams {
                proposer_factory,
                create_inherent_data_providers: move |_, (relay_parent, validation_data)| {
                    let relay_chain_interface = relay_chain_interface.clone();
                    async move {
                        let parachain_inherent =
                            cumulus_primitives_parachain_inherent::ParachainInherentData::create_at(
                                relay_parent,
                                &relay_chain_interface,
                                &validation_data,
                                id,
                            )
                            .await;

                        let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

                        let slot = sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                            *timestamp,
                            slot_duration,
                        );

                        let parachain_inherent = parachain_inherent.ok_or_else(|| {
                            Box::<dyn std::error::Error + Send + Sync>::from("Failed to create parachain inherent")
                        })?;
                        Ok((slot, timestamp, parachain_inherent))
                    }
                },
                block_import,
                para_client: client,
                backoff_authoring_blocks: Option::<()>::None,
                sync_oracle,
                keystore,
                force_authoring,
                slot_duration,
                // We got around 500ms for proposing
                block_proposal_slot_portion: SlotProportion::new(1f32 / 24f32),
                // And a maximum of 750ms if slots are skipped
                max_block_proposal_slot_portion: Some(SlotProportion::new(1f32 / 16f32)),
                telemetry,
            }))
        },
    )
    .await
}

pub async fn start_instant<RuntimeApi, Executor, CT>(
    mut config: Configuration,
    eth_config: EthConfiguration,
) -> sc_service::error::Result<(TaskManager, RpcHandlers)>
where
    RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>> + Send + Sync + 'static,
    RuntimeApi::RuntimeApi: RuntimeApiCollection<StateBackend = sc_client_api::StateBackendFor<FullBackend, Block>>,
    RuntimeApi::RuntimeApi: sp_consensus_aura::AuraApi<Block, AuraId>,
    Executor: sc_executor::NativeExecutionDispatch + 'static,
    CT: fp_rpc::ConvertTransaction<<Block as BlockT>::Extrinsic> + Clone + Default + Send + Sync + 'static,
{
    let sc_service::PartialComponents {
        client,
        backend,
        mut task_manager,
        import_queue,
        keystore_container,
        select_chain: maybe_select_chain,
        transaction_pool,
        other: (_, mut telemetry, _telemetry_worker_handle, frontier_backend, overrides),
    } = new_partial::<RuntimeApi, Executor>(&config, &eth_config, true)?;
    let net_config = sc_network::config::FullNetworkConfiguration::new(&config.network);

    let (network, system_rpc_tx, tx_handler_controller, network_starter, sync_service) =
        sc_service::build_network(sc_service::BuildNetworkParams {
            config: &config,
            net_config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue,
            block_announce_validator_builder: None,
            warp_sync_params: None,
        })?;

    if config.offchain_worker.enabled {
        use futures::FutureExt;

        task_manager.spawn_handle().spawn(
            "offchain-workers-runner",
            "offchain-work",
            sc_offchain::OffchainWorkers::new(sc_offchain::OffchainWorkerOptions {
                runtime_api_provider: client.clone(),
                keystore: None,
                offchain_db: backend.offchain_storage(),
                transaction_pool: Some(OffchainTransactionPoolFactory::new(transaction_pool.clone())),
                network_provider: network.clone(),
                is_validator: config.role.is_authority(),
                enable_http_requests: false,
                custom_extensions: move |_| vec![],
            })
            .run(client.clone(), task_manager.spawn_handle())
            .boxed(),
        );
    };

    let prometheus_registry = config.prometheus_registry().cloned();

    let role = config.role.clone();

    let select_chain = maybe_select_chain.expect("`new_partial` will return some `select_chain`; qed");

    let command_sink = if role.is_authority() {
        let proposer_factory = sc_basic_authorship::ProposerFactory::new(
            task_manager.spawn_handle(),
            client.clone(),
            transaction_pool.clone(),
            prometheus_registry.as_ref(),
            telemetry.as_ref().map(|x| x.handle()),
        );

        // Channel for the rpc handler to communicate with the authorship task.
        let (command_sink, commands_stream) = futures::channel::mpsc::channel(1024);

        let pool = transaction_pool.pool().clone();
        let import_stream = pool.validated_pool().import_notification_stream().map(|_| {
            sc_consensus_manual_seal::rpc::EngineCommand::SealNewBlock {
                create_empty: true,
                finalize: true,
                parent_hash: None,
                sender: None,
            }
        });

        let client_for_cidp = client.clone();

        let authorship_future = sc_consensus_manual_seal::run_manual_seal(sc_consensus_manual_seal::ManualSealParams {
            block_import: client.clone(),
            env: proposer_factory,
            client: client.clone(),
            pool: transaction_pool.clone(),
            commands_stream: futures::stream_select!(commands_stream, import_stream),
            select_chain,
            consensus_data_provider: None,
            create_inherent_data_providers: move |block: Hash, _| {
                let current_para_block = client_for_cidp
                    .number(block)
                    .expect("Header lookup should succeed")
                    .expect("Header passed in as parent should be present in backend.");
                let client_for_xcm = client_for_cidp.clone();
                async move {
                    let mocked_parachain = MockValidationDataInherentDataProvider {
                        current_para_block,
                        relay_offset: 1000,
                        relay_blocks_per_para_block: 2,
                        para_blocks_per_relay_epoch: 0,
                        relay_randomness_config: (),
                        xcm_config: MockXcmConfig::new(&*client_for_xcm, block, Default::default(), Default::default()),
                        raw_downward_messages: vec![],
                        raw_horizontal_messages: vec![],
                    };
                    Ok((sp_timestamp::InherentDataProvider::from_system_time(), mocked_parachain))
                }
            },
        });
        // we spawn the future on a background thread managed by service.
        task_manager.spawn_essential_handle().spawn_blocking(
            "instant-seal",
            Some("block-authoring"),
            authorship_future,
        );
        Some(command_sink)
    } else {
        None
    };

    let FrontierPartialComponents {
        filter_pool,
        fee_history_cache,
        fee_history_cache_limit,
    } = new_frontier_partial(&eth_config)?;

    let pubsub_notification_sinks: fc_mapping_sync::EthereumBlockNotificationSinks<
        fc_mapping_sync::EthereumBlockNotification<Block>,
    > = Default::default();
    let pubsub_notification_sinks = Arc::new(pubsub_notification_sinks);

    let eth_rpc_params = new_eth_deps::<_, _, _, CT>(
        client.clone(),
        transaction_pool.clone(),
        transaction_pool.pool().clone(),
        &mut config,
        &eth_config,
        network.clone(),
        sync_service.clone(),
        frontier_backend.clone(),
        overrides.clone(),
        &task_manager,
        filter_pool.clone(),
        fee_history_cache.clone(),
        fee_history_cache_limit,
    );

    let rpc_builder = {
        let client = client.clone();
        let transaction_pool = transaction_pool.clone();
        let pubsub_notification_sinks = pubsub_notification_sinks.clone();

        move |deny_unsafe, subscription_task_executor| {
            let deps = interbtc_rpc::FullDeps {
                client: client.clone(),
                pool: transaction_pool.clone(),
                deny_unsafe,
                command_sink: command_sink.clone(),
                eth: eth_rpc_params.clone(),
            };

            interbtc_rpc::create_full(deps, subscription_task_executor, pubsub_notification_sinks.clone())
                .map_err(Into::into)
        }
    };

    let rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        rpc_builder: Box::new(rpc_builder),
        client: client.clone(),
        transaction_pool,
        task_manager: &mut task_manager,
        config,
        keystore: keystore_container.keystore(),
        backend: backend.clone(),
        network,
        system_rpc_tx,
        tx_handler_controller,
        telemetry: telemetry.as_mut(),
        sync_service: sync_service.clone(),
    })?;

    spawn_frontier_tasks(
        &task_manager,
        client.clone(),
        backend,
        frontier_backend,
        filter_pool,
        overrides,
        fee_history_cache,
        fee_history_cache_limit,
        sync_service.clone(),
        pubsub_notification_sinks,
    )
    .await;

    network_starter.start_network();

    Ok((task_manager, rpc_handlers))
}
