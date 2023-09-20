use crate::service::{FullBackend, FullClient};
pub use fc_consensus::FrontierBlockImport;
use fc_rpc::{EthTask, OverrideHandle};
pub use fc_rpc_core::types::{FeeHistoryCache, FeeHistoryCacheLimit, FilterPool};
use fp_rpc::EthereumRuntimeRPCApi;
use futures::{future, prelude::*};
use primitives::Block;
use sc_client_api::{BlockchainEvents, StateBackendFor};
use sc_executor::NativeExecutionDispatch;
use sc_network_sync::SyncingService;
use sc_service::{error::Error as ServiceError, Configuration, TaskManager};
use sc_transaction_pool::{ChainApi, Pool};
use sp_api::{ConstructRuntimeApi, ProvideRuntimeApi};
use sp_runtime::traits::{BlakeTwo256, Block as BlockT};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

/// Frontier DB backend type.
pub type FrontierBackend = fc_db::Backend<Block>;

pub fn db_config_dir(config: &Configuration) -> PathBuf {
    config.base_path.config_dir(config.chain_spec.id())
}

pub fn open_frontier_backend<C>(
    client: Arc<C>,
    config: &Configuration,
    eth_config: &EthConfiguration,
    overrides: Arc<OverrideHandle<Block>>,
) -> Result<FrontierBackend, String>
where
    C: sc_client_api::HeaderBackend<Block>,
{
    let frontier_backend = match eth_config.frontier_backend_type {
        BackendType::KeyValue => FrontierBackend::KeyValue(fc_db::kv::Backend::open(
            Arc::clone(&client),
            &config.database,
            &db_config_dir(config),
        )?),
        BackendType::Sql => {
            let db_path = db_config_dir(config).join("sql");
            std::fs::create_dir_all(&db_path).expect("failed creating sql db directory");
            let backend = futures::executor::block_on(fc_db::sql::Backend::new(
                fc_db::sql::BackendConfig::Sqlite(fc_db::sql::SqliteBackendConfig {
                    path: Path::new("sqlite:///")
                        .join(db_path)
                        .join("frontier.db3")
                        .to_str()
                        .unwrap(),
                    create_if_missing: true,
                    thread_count: eth_config.frontier_sql_backend_thread_count,
                    cache_size: eth_config.frontier_sql_backend_cache_size,
                }),
                eth_config.frontier_sql_backend_pool_size,
                std::num::NonZeroU32::new(eth_config.frontier_sql_backend_num_ops_timeout),
                overrides,
            ))
            .unwrap_or_else(|err| panic!("failed creating sql backend: {:?}", err));
            FrontierBackend::Sql(backend)
        }
    };
    Ok(frontier_backend)
}

/// Available frontier backend types.
#[derive(Debug, Copy, Clone, Default, clap::ValueEnum)]
pub enum BackendType {
    /// Either RocksDb or ParityDb as per inherited from the global backend settings.
    #[default]
    KeyValue,
    /// Sql database with custom log indexing.
    Sql,
}

/// The ethereum-compatibility configuration used to run a node.
#[derive(Clone, Debug, clap::Parser)]
pub struct EthConfiguration {
    /// Maximum number of logs in a query.
    #[arg(long, default_value = "10000")]
    pub max_past_logs: u32,

    /// Maximum fee history cache size.
    #[arg(long, default_value = "2048")]
    pub fee_history_limit: u64,

    #[arg(long)]
    pub enable_dev_signer: bool,

    /// The dynamic-fee pallet target gas price set by block author
    #[arg(long, default_value = "1")]
    pub target_gas_price: u64,

    /// Maximum allowed gas limit will be `block.gas_limit * execute_gas_limit_multiplier`
    /// when using eth_call/eth_estimateGas.
    #[arg(long, default_value = "10")]
    pub execute_gas_limit_multiplier: u64,

    /// Size in bytes of the LRU cache for block data.
    #[arg(long, default_value = "50")]
    pub eth_log_block_cache: usize,

    /// Size in bytes of the LRU cache for transactions statuses data.
    #[arg(long, default_value = "50")]
    pub eth_statuses_cache: usize,

    /// Sets the frontier backend type (KeyValue or Sql)
    #[arg(long, value_enum, ignore_case = true, default_value_t = BackendType::default())]
    pub frontier_backend_type: BackendType,

    // Sets the SQL backend's pool size.
    #[arg(long, default_value = "100")]
    pub frontier_sql_backend_pool_size: u32,

    /// Sets the SQL backend's query timeout in number of VM ops.
    #[arg(long, default_value = "10000000")]
    pub frontier_sql_backend_num_ops_timeout: u32,

    /// Sets the SQL backend's auxiliary thread limit.
    #[arg(long, default_value = "4")]
    pub frontier_sql_backend_thread_count: u32,

    /// Sets the SQL backend's query timeout in number of VM ops.
    /// Default value is 200MB.
    #[arg(long, default_value = "209715200")]
    pub frontier_sql_backend_cache_size: u64,
}

pub struct FrontierPartialComponents {
    pub filter_pool: Option<FilterPool>,
    pub fee_history_cache: FeeHistoryCache,
    pub fee_history_cache_limit: FeeHistoryCacheLimit,
}

pub fn new_frontier_partial(config: &EthConfiguration) -> Result<FrontierPartialComponents, ServiceError> {
    Ok(FrontierPartialComponents {
        filter_pool: Some(Arc::new(Mutex::new(BTreeMap::new()))),
        fee_history_cache: Arc::new(Mutex::new(BTreeMap::new())),
        fee_history_cache_limit: config.fee_history_limit,
    })
}

/// A set of APIs that ethereum-compatible runtimes must implement.
pub trait EthCompatRuntimeApiCollection:
    sp_api::ApiExt<Block> + fp_rpc::ConvertTransactionRuntimeApi<Block> + fp_rpc::EthereumRuntimeRPCApi<Block>
where
    <Self as sp_api::ApiExt<Block>>::StateBackend: sp_api::StateBackend<BlakeTwo256>,
{
}

impl<Api> EthCompatRuntimeApiCollection for Api
where
    Api: sp_api::ApiExt<Block> + fp_rpc::ConvertTransactionRuntimeApi<Block> + fp_rpc::EthereumRuntimeRPCApi<Block>,
    <Self as sp_api::ApiExt<Block>>::StateBackend: sp_api::StateBackend<BlakeTwo256>,
{
}

pub async fn spawn_frontier_tasks<RuntimeApi, Executor>(
    task_manager: &TaskManager,
    client: Arc<FullClient<RuntimeApi, Executor>>,
    backend: Arc<FullBackend>,
    frontier_backend: FrontierBackend,
    filter_pool: Option<FilterPool>,
    overrides: Arc<OverrideHandle<Block>>,
    fee_history_cache: FeeHistoryCache,
    fee_history_cache_limit: FeeHistoryCacheLimit,
    sync: Arc<SyncingService<Block>>,
    pubsub_notification_sinks: Arc<
        fc_mapping_sync::EthereumBlockNotificationSinks<fc_mapping_sync::EthereumBlockNotification<Block>>,
    >,
) where
    RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>>,
    RuntimeApi: Send + Sync + 'static,
    RuntimeApi::RuntimeApi: EthCompatRuntimeApiCollection<StateBackend = StateBackendFor<FullBackend, Block>>,
    Executor: NativeExecutionDispatch + 'static,
{
    // Spawn main mapping sync worker background task.
    match frontier_backend {
        fc_db::Backend::KeyValue(b) => {
            task_manager.spawn_essential_handle().spawn(
                "frontier-mapping-sync-worker",
                Some("frontier"),
                fc_mapping_sync::kv::MappingSyncWorker::new(
                    client.import_notification_stream(),
                    Duration::new(6, 0),
                    client.clone(),
                    backend,
                    overrides.clone(),
                    Arc::new(b),
                    3,
                    0, // TODO: update when deployed
                    fc_mapping_sync::SyncStrategy::Normal,
                    sync,
                    pubsub_notification_sinks,
                )
                .for_each(|()| future::ready(())),
            );
        }
        fc_db::Backend::Sql(b) => {
            task_manager.spawn_essential_handle().spawn_blocking(
                "frontier-mapping-sync-worker",
                Some("frontier"),
                fc_mapping_sync::sql::SyncWorker::run(
                    client.clone(),
                    backend,
                    Arc::new(b),
                    client.import_notification_stream(),
                    fc_mapping_sync::sql::SyncWorkerConfig {
                        read_notification_timeout: Duration::from_secs(10),
                        check_indexed_blocks_interval: Duration::from_secs(60),
                    },
                    fc_mapping_sync::SyncStrategy::Parachain,
                    sync,
                    pubsub_notification_sinks,
                ),
            );
        }
    }

    // Spawn Frontier EthFilterApi maintenance task.
    if let Some(filter_pool) = filter_pool {
        // Each filter is allowed to stay in the pool for 100 blocks.
        const FILTER_RETAIN_THRESHOLD: u64 = 100;
        task_manager.spawn_essential_handle().spawn(
            "frontier-filter-pool",
            Some("frontier"),
            EthTask::filter_pool_task(client.clone(), filter_pool, FILTER_RETAIN_THRESHOLD),
        );
    }

    // Spawn Frontier FeeHistory cache maintenance task.
    task_manager.spawn_essential_handle().spawn(
        "frontier-fee-history",
        Some("frontier"),
        EthTask::fee_history_task(client, overrides, fee_history_cache, fee_history_cache_limit),
    );
}

pub fn new_eth_deps<C, P, A: ChainApi, CT>(
    client: Arc<C>,
    transaction_pool: Arc<P>,
    graph: Arc<Pool<A>>,
    config: &mut Configuration,
    eth_config: &EthConfiguration,
    network: Arc<sc_network::NetworkService<Block, <Block as BlockT>::Hash>>,
    sync_service: Arc<SyncingService<Block>>,
    frontier_backend: FrontierBackend,
    overrides: Arc<OverrideHandle<Block>>,
    task_manager: &TaskManager,
    filter_pool: Option<FilterPool>,
    fee_history_cache: FeeHistoryCache,
    fee_history_cache_limit: FeeHistoryCacheLimit,
) -> interbtc_rpc::EthDeps<C, P, A, CT, Block>
where
    C: ProvideRuntimeApi<Block>,
    C::Api: EthereumRuntimeRPCApi<Block>,
    C: sc_client_api::HeaderBackend<Block> + sc_client_api::StorageProvider<Block, FullBackend> + 'static,
    CT: fp_rpc::ConvertTransaction<<Block as BlockT>::Extrinsic> + Default + Send + Sync + 'static,
{
    // use ethereum style for subscription ids
    config.rpc_id_provider = Some(Box::new(fc_rpc::EthereumSubIdProvider));
    interbtc_rpc::EthDeps {
        client: client.clone(),
        pool: transaction_pool,
        graph,
        converter: Some(CT::default()),
        is_authority: config.role.is_authority(),
        enable_dev_signer: eth_config.enable_dev_signer,
        network,
        sync: sync_service.clone(),
        frontier_backend: match frontier_backend {
            fc_db::Backend::KeyValue(b) => Arc::new(b),
            fc_db::Backend::Sql(b) => Arc::new(b),
        },
        overrides: overrides.clone(),
        block_data_cache: Arc::new(fc_rpc::EthBlockDataCacheTask::new(
            task_manager.spawn_handle(),
            overrides.clone(),
            eth_config.eth_log_block_cache,
            eth_config.eth_statuses_cache,
            config.prometheus_registry().cloned(),
        )),
        filter_pool,
        max_past_logs: eth_config.max_past_logs,
        fee_history_cache,
        fee_history_cache_limit,
        execute_gas_limit_multiplier: eth_config.execute_gas_limit_multiplier,
        forced_parent_hashes: None,
    }
}
