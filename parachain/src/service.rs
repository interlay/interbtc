use cumulus_client_cli::CollatorOptions;
use cumulus_client_consensus_aura::{AuraConsensus, BuildAuraConsensusParams, SlotProportion};
use cumulus_client_consensus_common::ParachainConsensus;
use cumulus_client_network::BlockAnnounceValidator;
use cumulus_client_service::{
    prepare_node_config, start_collator, start_full_node, StartCollatorParams, StartFullNodeParams,
};
use cumulus_primitives_core::ParaId;
use cumulus_primitives_parachain_inherent::{MockValidationDataInherentDataProvider, MockXcmConfig};
use cumulus_relay_chain_inprocess_interface::build_inprocess_relay_chain;
use cumulus_relay_chain_interface::{RelayChainError, RelayChainInterface, RelayChainResult};
use cumulus_relay_chain_rpc_interface::{create_client_and_start_worker, RelayChainRpcInterface};
use polkadot_service::CollatorPair;

use futures::StreamExt;
use jsonrpsee::RpcModule;
use primitives::*;
use sc_client_api::HeaderBackend;
use sc_consensus::LongestChain;
use sc_executor::NativeElseWasmExecutor;
use sc_network::{NetworkBlock, NetworkService};
use sc_service::{Configuration, PartialComponents, RpcHandlers, TFullBackend, TFullClient, TaskManager};
use sc_telemetry::{Telemetry, TelemetryHandle, TelemetryWorker, TelemetryWorkerHandle};
use sp_api::ConstructRuntimeApi;
use sp_consensus_aura::sr25519::{AuthorityId as AuraId, AuthorityPair as AuraPair};
use sp_keystore::SyncCryptoStorePtr;
use sp_runtime::traits::BlakeTwo256;
use std::{sync::Arc, time::Duration};
use substrate_prometheus_endpoint::Registry;

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

// Native testnet executor instance.
new_runtime_executor!(TestnetKintsugiRuntimeExecutor, testnet_kintsugi_runtime);

// Native testnet executor instance.
new_runtime_executor!(TestnetInterlayRuntimeExecutor, testnet_interlay_runtime);

pub trait RuntimeApiCollection:
    sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
    + sp_api::Metadata<Block>
    + sp_session::SessionKeys<Block>
    + sp_api::ApiExt<Block, StateBackend = sc_client_api::StateBackendFor<TFullBackend<Block>, Block>>
    + sp_offchain::OffchainWorkerApi<Block>
    + sp_block_builder::BlockBuilder<Block>
    + substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>
    + pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>
    + cumulus_primitives_core::CollectCollationInfo<Block>
    + module_btc_relay_rpc_runtime_api::BtcRelayApi<Block, H256Le>
    + module_oracle_rpc_runtime_api::OracleApi<Block, Balance, CurrencyId>
    + module_vault_registry_rpc_runtime_api::VaultRegistryApi<
        Block,
        VaultId<AccountId, CurrencyId>,
        Balance,
        UnsignedFixedPoint,
        CurrencyId,
        AccountId,
    > + module_escrow_rpc_runtime_api::EscrowApi<Block, AccountId, BlockNumber, Balance>
    + module_issue_rpc_runtime_api::IssueApi<
        Block,
        AccountId,
        H256,
        issue::IssueRequest<AccountId, BlockNumber, Balance, CurrencyId>,
    > + module_redeem_rpc_runtime_api::RedeemApi<
        Block,
        AccountId,
        H256,
        redeem::RedeemRequest<AccountId, BlockNumber, Balance, CurrencyId>,
    > + module_replace_rpc_runtime_api::ReplaceApi<
        Block,
        AccountId,
        H256,
        replace::ReplaceRequest<AccountId, BlockNumber, Balance, CurrencyId>,
    > + module_reward_rpc_runtime_api::RewardApi<Block, AccountId, VaultId<AccountId, CurrencyId>, CurrencyId, Balance>
where
    <Self as sp_api::ApiExt<Block>>::StateBackend: sp_api::StateBackend<BlakeTwo256>,
{
}

impl<Api> RuntimeApiCollection for Api
where
    Api: sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
        + sp_api::Metadata<Block>
        + sp_session::SessionKeys<Block>
        + sp_api::ApiExt<Block, StateBackend = sc_client_api::StateBackendFor<TFullBackend<Block>, Block>>
        + sp_offchain::OffchainWorkerApi<Block>
        + sp_block_builder::BlockBuilder<Block>
        + substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>
        + pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>
        + cumulus_primitives_core::CollectCollationInfo<Block>
        + module_btc_relay_rpc_runtime_api::BtcRelayApi<Block, H256Le>
        + module_oracle_rpc_runtime_api::OracleApi<Block, Balance, CurrencyId>
        + module_vault_registry_rpc_runtime_api::VaultRegistryApi<
            Block,
            VaultId<AccountId, CurrencyId>,
            Balance,
            UnsignedFixedPoint,
            CurrencyId,
            AccountId,
        > + module_escrow_rpc_runtime_api::EscrowApi<Block, AccountId, BlockNumber, Balance>
        + module_issue_rpc_runtime_api::IssueApi<
            Block,
            AccountId,
            H256,
            issue::IssueRequest<AccountId, BlockNumber, Balance, CurrencyId>,
        > + module_redeem_rpc_runtime_api::RedeemApi<
            Block,
            AccountId,
            H256,
            redeem::RedeemRequest<AccountId, BlockNumber, Balance, CurrencyId>,
        > + module_replace_rpc_runtime_api::ReplaceApi<
            Block,
            AccountId,
            H256,
            replace::ReplaceRequest<AccountId, BlockNumber, Balance, CurrencyId>,
        > + module_reward_rpc_runtime_api::RewardApi<Block, AccountId, VaultId<AccountId, CurrencyId>, CurrencyId, Balance>,
    <Self as sp_api::ApiExt<Block>>::StateBackend: sp_api::StateBackend<BlakeTwo256>,
{
}

type FullBackend = TFullBackend<Block>;

type FullClient<RuntimeApi, ExecutorDispatch> =
    sc_service::TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<ExecutorDispatch>>;

type MaybeFullSelectChain = Option<LongestChain<FullBackend, Block>>;

/// Starts a `ServiceBuilder` for a full service.
///
/// Use this macro if you don't actually need the full service, but just the builder in order to
/// be able to perform chain operations.
pub fn new_partial<RuntimeApi, Executor>(
    config: &Configuration,
    instant_seal: bool,
) -> Result<
    PartialComponents<
        TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
        TFullBackend<Block>,
        MaybeFullSelectChain,
        sc_consensus::DefaultImportQueue<Block, TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
        sc_transaction_pool::FullPool<Block, TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
        (Option<Telemetry>, Option<TelemetryWorkerHandle>),
    >,
    sc_service::Error,
>
where
    RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>> + Send + Sync + 'static,
    RuntimeApi::RuntimeApi: RuntimeApiCollection<StateBackend = sc_client_api::StateBackendFor<FullBackend, Block>>,
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

    let executor = NativeElseWasmExecutor::<Executor>::new(
        config.wasm_method,
        config.default_heap_pages,
        config.max_runtime_instances,
        config.runtime_cache_size,
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

    let client_clone = client.clone();
    let import_queue = if instant_seal {
        // instance sealing
        sc_consensus_manual_seal::import_queue(
            Box::new(client.clone()),
            &task_manager.spawn_essential_handle(),
            registry,
        )
    } else {
        cumulus_client_consensus_aura::import_queue::<AuraPair, _, _, _, _, _>(
            cumulus_client_consensus_aura::ImportQueueParams {
                block_import: client.clone(),
                client: client.clone(),
                create_inherent_data_providers: move |parent: sp_core::H256, _| {
                    let client_clone = client_clone.clone();
                    async move {
                        let slot_ms = match client_clone.clone().runtime_version_at(&BlockId::Hash(parent.clone())) {
                            Ok(x) if x.spec_name.starts_with("kintsugi") => 6000,
                            _ => 12000,
                        };
                        let slot_duration = sp_consensus_aura::SlotDuration::from_millis(slot_ms);

                        let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

                        let slot = sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                            *timestamp,
                            slot_duration,
                        );

                        Ok((slot, timestamp))
                    }
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
        other: (telemetry, telemetry_worker_handle),
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
    match collator_options.relay_chain_rpc_url {
        Some(relay_chain_url) => {
            let client = create_client_and_start_worker(relay_chain_url, task_manager).await?;
            Ok((Arc::new(RelayChainRpcInterface::new(client)) as Arc<_>, None))
        }
        None => build_inprocess_relay_chain(
            polkadot_config,
            parachain_config,
            telemetry_worker_handle,
            task_manager,
            None,
        ),
    }
}

/// Start a node with the given parachain `Configuration` and relay chain `Configuration`.
///
/// This is the actual implementation that is abstract over the executor and the runtime api.
#[sc_tracing::logging::prefix_logs_with("Parachain")]
async fn start_node_impl<RB, RuntimeApi, Executor, BIC>(
    parachain_config: Configuration,
    polkadot_config: Configuration,
    collator_options: CollatorOptions,
    id: ParaId,
    _rpc_ext_builder: RB,
    build_consensus: BIC,
) -> sc_service::error::Result<(TaskManager, Arc<FullClient<RuntimeApi, Executor>>)>
where
    RB: Fn(Arc<FullClient<RuntimeApi, Executor>>) -> Result<RpcModule<()>, sc_service::Error> + Send + 'static,
    RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>> + Send + Sync + 'static,
    RuntimeApi::RuntimeApi: RuntimeApiCollection<StateBackend = sc_client_api::StateBackendFor<FullBackend, Block>>,
    RuntimeApi::RuntimeApi: sp_consensus_aura::AuraApi<Block, AuraId>,
    Executor: sc_executor::NativeExecutionDispatch + 'static,
    BIC: FnOnce(
        Arc<FullClient<RuntimeApi, Executor>>,
        Option<&Registry>,
        Option<TelemetryHandle>,
        &TaskManager,
        Arc<dyn RelayChainInterface>,
        Arc<sc_transaction_pool::FullPool<Block, FullClient<RuntimeApi, Executor>>>,
        Arc<NetworkService<Block, Hash>>,
        SyncCryptoStorePtr,
        bool,
    ) -> Result<Box<dyn ParachainConsensus<Block>>, sc_service::Error>,
{
    let parachain_config = prepare_node_config(parachain_config);

    let params = new_partial(&parachain_config, false)?;
    let (mut telemetry, telemetry_worker_handle) = params.other;

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
    .map_err(|e| match e {
        RelayChainError::ServiceError(polkadot_service::Error::Sub(x)) => x,
        s => format!("{}", s).into(),
    })?;

    let block_announce_validator = BlockAnnounceValidator::new(relay_chain_interface.clone(), id);

    let force_authoring = parachain_config.force_authoring;
    let validator = parachain_config.role.is_authority();
    let prometheus_registry = parachain_config.prometheus_registry().cloned();
    let transaction_pool = params.transaction_pool.clone();
    let import_queue = cumulus_client_service::SharedImportQueue::new(params.import_queue);
    let (network, system_rpc_tx, tx_handler_controller, start_network) =
        sc_service::build_network(sc_service::BuildNetworkParams {
            config: &parachain_config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue: import_queue.clone(),
            block_announce_validator_builder: Some(Box::new(|_| Box::new(block_announce_validator))),
            warp_sync: None,
        })?;

    let rpc_builder = {
        let client = client.clone();
        let transaction_pool = transaction_pool.clone();

        move |deny_unsafe, _| {
            let deps = interbtc_rpc::FullDeps {
                client: client.clone(),
                pool: transaction_pool.clone(),
                deny_unsafe,
                command_sink: None,
            };

            interbtc_rpc::create_full(deps).map_err(Into::into)
        }
    };

    if parachain_config.offchain_worker.enabled {
        sc_service::build_offchain_workers(
            &parachain_config,
            task_manager.spawn_handle(),
            client.clone(),
            network.clone(),
        );
    };

    sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        rpc_builder: Box::new(rpc_builder),
        client: client.clone(),
        transaction_pool: transaction_pool.clone(),
        task_manager: &mut task_manager,
        config: parachain_config,
        keystore: params.keystore_container.sync_keystore(),
        backend: backend.clone(),
        network: network.clone(),
        system_rpc_tx,
        tx_handler_controller,
        telemetry: telemetry.as_mut(),
    })?;

    let announce_block = {
        let network = network.clone();
        Arc::new(move |hash, data| network.announce_block(hash, data))
    };

    let relay_chain_slot_duration = Duration::from_secs(6);

    if validator {
        let parachain_consensus = build_consensus(
            client.clone(),
            prometheus_registry.as_ref(),
            telemetry.as_ref().map(|t| t.handle()),
            &task_manager,
            relay_chain_interface.clone(),
            transaction_pool,
            network,
            params.keystore_container.sync_keystore(),
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
            import_queue,
            collator_key: collator_key.expect("Command line arguments do not allow this. qed"),
            relay_chain_slot_duration,
        };

        start_collator(params).await?;
    } else {
        let params = StartFullNodeParams {
            client: client.clone(),
            announce_block,
            task_manager: &mut task_manager,
            para_id: id,
            relay_chain_interface,
            import_queue,
            relay_chain_slot_duration,
            collator_options,
        };

        start_full_node(params)?;
    }

    start_network.start_network();

    Ok((task_manager, client))
}

/// Start a normal parachain node.
pub async fn start_node<RuntimeApi, Executor>(
    parachain_config: Configuration,
    polkadot_config: Configuration,
    collator_options: CollatorOptions,
    id: ParaId,
) -> sc_service::error::Result<(TaskManager, Arc<FullClient<RuntimeApi, Executor>>)>
where
    RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>> + Send + Sync + 'static,
    RuntimeApi::RuntimeApi: RuntimeApiCollection<StateBackend = sc_client_api::StateBackendFor<FullBackend, Block>>,
    RuntimeApi::RuntimeApi: sp_consensus_aura::AuraApi<Block, AuraId>,
    Executor: sc_executor::NativeExecutionDispatch + 'static,
{
    let slot_ms = if parachain_config.chain_spec.id() == "kusama" {
        6000
    } else {
        12000
    };
    start_node_impl(
        parachain_config,
        polkadot_config,
        collator_options,
        id,
        |_| Ok(RpcModule::new(())),
        |client,
         prometheus_registry,
         telemetry,
         task_manager,
         relay_chain_interface,
         transaction_pool,
         sync_oracle,
         keystore,
         force_authoring| {
            let slot_duration = sp_consensus_aura::SlotDuration::from_millis(slot_ms);

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
                block_import: client.clone(),
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

pub async fn start_instant<RuntimeApi, Executor>(
    config: Configuration,
) -> sc_service::error::Result<(TaskManager, RpcHandlers)>
where
    RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>> + Send + Sync + 'static,
    RuntimeApi::RuntimeApi: RuntimeApiCollection<StateBackend = sc_client_api::StateBackendFor<FullBackend, Block>>,
    RuntimeApi::RuntimeApi: sp_consensus_aura::AuraApi<Block, AuraId>,
    Executor: sc_executor::NativeExecutionDispatch + 'static,
{
    let sc_service::PartialComponents {
        client,
        backend,
        mut task_manager,
        import_queue,
        keystore_container,
        select_chain: maybe_select_chain,
        transaction_pool,
        other: (mut telemetry, _),
    } = new_partial::<RuntimeApi, Executor>(&config, true)?;

    let (network, system_rpc_tx, tx_handler_controller, network_starter) =
        sc_service::build_network(sc_service::BuildNetworkParams {
            config: &config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue,
            block_announce_validator_builder: None,
            warp_sync: None,
        })?;

    if config.offchain_worker.enabled {
        sc_service::build_offchain_workers(&config, task_manager.spawn_handle(), client.clone(), network.clone());
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

    let rpc_builder = {
        let client = client.clone();
        let transaction_pool = transaction_pool.clone();

        move |deny_unsafe, _| {
            let deps = interbtc_rpc::FullDeps {
                client: client.clone(),
                pool: transaction_pool.clone(),
                deny_unsafe,
                command_sink: command_sink.clone(),
            };

            interbtc_rpc::create_full(deps).map_err(Into::into)
        }
    };

    let rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        rpc_builder: Box::new(rpc_builder),
        client: client.clone(),
        transaction_pool,
        task_manager: &mut task_manager,
        config,
        keystore: keystore_container.sync_keystore(),
        backend,
        network,
        system_rpc_tx,
        tx_handler_controller,
        telemetry: telemetry.as_mut(),
    })?;

    network_starter.start_network();

    Ok((task_manager, rpc_handlers))
}
