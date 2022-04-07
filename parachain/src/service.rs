use cumulus_client_cli::CollatorOptions;
use cumulus_client_consensus_aura::{AuraConsensus, BuildAuraConsensusParams, SlotProportion};
use cumulus_client_consensus_common::ParachainConsensus;
use cumulus_client_network::BlockAnnounceValidator;
use cumulus_client_service::{
    prepare_node_config, start_collator, start_full_node, StartCollatorParams, StartFullNodeParams,
};
use cumulus_primitives_core::ParaId;
use cumulus_relay_chain_inprocess_interface::build_inprocess_relay_chain;
use cumulus_relay_chain_interface::{RelayChainError, RelayChainInterface, RelayChainResult};
use cumulus_relay_chain_rpc_interface::RelayChainRPCInterface;
use polkadot_service::CollatorPair;

use primitives::*;
use sc_client_api::ExecutorProvider;
use sc_executor::NativeElseWasmExecutor;
use sc_network::NetworkService;
use sc_service::{Configuration, PartialComponents, Role, TFullBackend, TFullClient, TaskManager};
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
new_runtime_executor!(TestnetRuntimeExecutor, testnet_runtime);

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
    + module_relay_rpc_runtime_api::RelayApi<Block, VaultId<AccountId, CurrencyId>>
    + module_vault_registry_rpc_runtime_api::VaultRegistryApi<
        Block,
        VaultId<AccountId, CurrencyId>,
        Balance,
        UnsignedFixedPoint,
        CurrencyId,
        AccountId,
    > + module_issue_rpc_runtime_api::IssueApi<
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
    > + module_refund_rpc_runtime_api::RefundApi<
        Block,
        AccountId,
        H256,
        refund::RefundRequest<AccountId, Balance, CurrencyId>,
    >
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
        + module_relay_rpc_runtime_api::RelayApi<Block, VaultId<AccountId, CurrencyId>>
        + module_vault_registry_rpc_runtime_api::VaultRegistryApi<
            Block,
            VaultId<AccountId, CurrencyId>,
            Balance,
            UnsignedFixedPoint,
            CurrencyId,
            AccountId,
        > + module_issue_rpc_runtime_api::IssueApi<
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
        > + module_refund_rpc_runtime_api::RefundApi<
            Block,
            AccountId,
            H256,
            refund::RefundRequest<AccountId, Balance, CurrencyId>,
        >,
    <Self as sp_api::ApiExt<Block>>::StateBackend: sp_api::StateBackend<BlakeTwo256>,
{
}

type FullBackend = TFullBackend<Block>;

type FullClient<RuntimeApi, ExecutorDispatch> =
    sc_service::TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<ExecutorDispatch>>;

/// Starts a `ServiceBuilder` for a full service.
///
/// Use this macro if you don't actually need the full service, but just the builder in order to
/// be able to perform chain operations.
pub fn new_partial<RuntimeApi, Executor>(
    config: &Configuration,
) -> Result<
    PartialComponents<
        TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
        TFullBackend<Block>,
        (),
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

    let client_clone = client.clone();
    let import_queue = {
        cumulus_client_consensus_aura::import_queue::<AuraPair, _, _, _, _, _, _>(
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

                        let time = sp_timestamp::InherentDataProvider::from_system_time();

                        let slot = sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                            *time,
                            slot_duration,
                        );

                        Ok((time, slot))
                    }
                },
                registry,
                can_author_with: sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone()),
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
        select_chain: (),
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
        Some(relay_chain_url) => Ok((
            Arc::new(RelayChainRPCInterface::new(relay_chain_url).await?) as Arc<_>,
            None,
        )),
        None => build_inprocess_relay_chain(polkadot_config, parachain_config, telemetry_worker_handle, task_manager),
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
    RB: Fn(
            Arc<FullClient<RuntimeApi, Executor>>,
        ) -> Result<jsonrpc_core::IoHandler<sc_rpc::Metadata>, sc_service::Error>
        + Send
        + 'static,
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
    if matches!(parachain_config.role, Role::Light) {
        return Err("Light client not supported!".into());
    }

    let parachain_config = prepare_node_config(parachain_config);

    let params = new_partial(&parachain_config)?;
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
    let (network, system_rpc_tx, start_network) = sc_service::build_network(sc_service::BuildNetworkParams {
        config: &parachain_config,
        client: client.clone(),
        transaction_pool: transaction_pool.clone(),
        spawn_handle: task_manager.spawn_handle(),
        import_queue: import_queue.clone(),
        block_announce_validator_builder: Some(Box::new(|_| Box::new(block_announce_validator))),
        warp_sync: None,
    })?;

    let rpc_extensions_builder = {
        let client = client.clone();
        let transaction_pool = transaction_pool.clone();

        Box::new(move |deny_unsafe, _| {
            let deps = interbtc_rpc::FullDeps {
                client: client.clone(),
                pool: transaction_pool.clone(),
                deny_unsafe,
            };

            Ok(interbtc_rpc::create_full(deps))
        })
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
        rpc_extensions_builder,
        client: client.clone(),
        transaction_pool: transaction_pool.clone(),
        task_manager: &mut task_manager,
        config: parachain_config,
        keystore: params.keystore_container.sync_keystore(),
        backend: backend.clone(),
        network: network.clone(),
        system_rpc_tx,
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
        |_| Ok(Default::default()),
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

                        let time = sp_timestamp::InherentDataProvider::from_system_time();

                        let slot = sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                            *time,
                            slot_duration,
                        );

                        let parachain_inherent = parachain_inherent.ok_or_else(|| {
                            Box::<dyn std::error::Error + Send + Sync>::from("Failed to create parachain inherent")
                        })?;
                        Ok((time, slot, parachain_inherent))
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
