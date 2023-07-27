//! A collection of node-specific RPC methods.
//! Substrate provides the `sc-rpc` crate, which defines the core RPC layer
//! used by Substrate nodes. This file extends those RPC definitions with
//! capabilities that are specific to this project's runtime configuration.

use primitives::{
    issue::IssueRequest, redeem::RedeemRequest, replace::ReplaceRequest, AccountId, Balance, Block, BlockNumber,
    CurrencyId, H256Le, Hash, Nonce, StablePoolId, VaultId,
};
use sc_consensus_manual_seal::rpc::{EngineCommand, ManualSeal, ManualSealApiServer};
pub use sc_rpc_api::DenyUnsafe;
use sc_transaction_pool_api::TransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_arithmetic::FixedU128;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use sp_core::H256;
use std::sync::Arc;

// Frontier imports
use sc_client_api::{backend::Backend, client::BlockchainEvents, StorageProvider};
use sc_rpc::SubscriptionTaskExecutor;
use sc_transaction_pool::ChainApi;
use sp_api::CallApiAt;
use sp_runtime::traits::Block as BlockT;

pub mod eth;
pub use self::eth::{create_eth, overrides_handle, EthDeps};

/// A type representing all RPC extensions.
pub type RpcExtension = jsonrpsee::RpcModule<()>;

/// Full client dependencies.
pub struct FullDeps<C, P, A: ChainApi, CT> {
    /// The client instance to use.
    pub client: Arc<C>,
    /// Transaction pool instance.
    pub pool: Arc<P>,
    /// Whether to deny unsafe calls
    pub deny_unsafe: DenyUnsafe,
    /// Manual seal command sink
    pub command_sink: Option<futures::channel::mpsc::Sender<EngineCommand<Hash>>>,
    /// Ethereum-compatibility specific dependencies.
    pub eth: EthDeps<C, P, A, CT, Block>,
}

pub struct DefaultEthConfig<C, BE>(std::marker::PhantomData<(C, BE)>);

impl<C, BE> fc_rpc::EthConfig<Block, C> for DefaultEthConfig<C, BE>
where
    C: sc_client_api::StorageProvider<Block, BE> + Sync + Send + 'static,
    BE: Backend<Block> + 'static,
{
    type EstimateGasAdapter = ();
    type RuntimeStorageOverride = fc_rpc::frontier_backend_client::SystemAccountId20StorageOverride<Block, C, BE>;
}

/// Instantiate all full RPC extensions.
pub fn create_full<C, P, BE, A, CT>(
    deps: FullDeps<C, P, A, CT>,
    subscription_task_executor: SubscriptionTaskExecutor,
    pubsub_notification_sinks: Arc<
        fc_mapping_sync::EthereumBlockNotificationSinks<fc_mapping_sync::EthereumBlockNotification<Block>>,
    >,
) -> Result<RpcExtension, Box<dyn std::error::Error + Send + Sync>>
where
    C: CallApiAt<Block> + ProvideRuntimeApi<Block>,
    C: BlockchainEvents<Block>,
    C: HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockChainError> + StorageProvider<Block, BE>,
    C: Send + Sync + 'static,
    C::Api: BlockBuilder<Block>,
    C::Api: fp_rpc::ConvertTransactionRuntimeApi<Block>,
    C::Api: fp_rpc::EthereumRuntimeRPCApi<Block>,
    C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>,
    C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
    C::Api: btc_relay_rpc::BtcRelayRuntimeApi<Block, H256Le>,
    C::Api: oracle_rpc::OracleRuntimeApi<Block, Balance, CurrencyId>,
    C::Api: vault_registry_rpc::VaultRegistryRuntimeApi<
        Block,
        VaultId<AccountId, CurrencyId>,
        Balance,
        FixedU128,
        CurrencyId,
        AccountId,
    >,
    C::Api:
        issue_rpc::IssueRuntimeApi<Block, AccountId, H256, IssueRequest<AccountId, BlockNumber, Balance, CurrencyId>>,
    C::Api: redeem_rpc::RedeemRuntimeApi<
        Block,
        AccountId,
        H256,
        RedeemRequest<AccountId, BlockNumber, Balance, CurrencyId>,
    >,
    C::Api: replace_rpc::ReplaceRuntimeApi<
        Block,
        AccountId,
        H256,
        ReplaceRequest<AccountId, BlockNumber, Balance, CurrencyId>,
    >,
    C::Api: escrow_rpc::EscrowRuntimeApi<Block, AccountId, BlockNumber, Balance>,
    C::Api: reward_rpc::RewardRuntimeApi<
        Block,
        AccountId,
        VaultId<AccountId, CurrencyId>,
        CurrencyId,
        Balance,
        BlockNumber,
        FixedU128,
    >,
    C::Api: loans_rpc::LoansRuntimeApi<Block, AccountId, Balance>,
    C::Api: dex_general_rpc::DexGeneralRuntimeApi<Block, AccountId, CurrencyId>,
    C::Api: dex_stable_rpc::DexStableRuntimeApi<Block, CurrencyId, Balance, AccountId, StablePoolId>,
    P: TransactionPool<Block = Block> + 'static,
    BE: Backend<Block> + 'static,
    A: ChainApi<Block = Block> + 'static,
    CT: fp_rpc::ConvertTransaction<<Block as BlockT>::Extrinsic> + Send + Sync + 'static,
{
    use btc_relay_rpc::{BtcRelay, BtcRelayApiServer};
    use dex_general_rpc::{DexGeneral, DexGeneralApiServer};
    use dex_stable_rpc::{DexStable, DexStableApiServer};
    use escrow_rpc::{Escrow, EscrowApiServer};
    use issue_rpc::{Issue, IssueApiServer};
    use loans_rpc::{Loans, LoansApiServer};
    use oracle_rpc::{Oracle, OracleApiServer};
    use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApiServer};
    use redeem_rpc::{Redeem, RedeemApiServer};
    use replace_rpc::{Replace, ReplaceApiServer};
    use reward_rpc::{Reward, RewardApiServer};
    use substrate_frame_rpc_system::{System, SystemApiServer};
    use vault_registry_rpc::{VaultRegistry, VaultRegistryApiServer};

    let mut module = RpcExtension::new(());
    let FullDeps {
        client,
        pool,
        deny_unsafe,
        command_sink,
        eth,
    } = deps;

    if let Some(command_sink) = command_sink {
        module.merge(
            // We provide the rpc handler with the sending end of the channel to allow the rpc
            // send EngineCommands to the background block authorship task.
            ManualSeal::new(command_sink).into_rpc(),
        )?;
    }

    // Ethereum compatibility RPCs
    let mut module = create_eth::<_, _, _, _, _, _, DefaultEthConfig<C, BE>>(
        module,
        eth,
        subscription_task_executor,
        pubsub_notification_sinks,
    )?;

    module.merge(System::new(client.clone(), pool, deny_unsafe).into_rpc())?;

    module.merge(TransactionPayment::new(client.clone()).into_rpc())?;

    module.merge(BtcRelay::new(client.clone()).into_rpc())?;

    module.merge(Oracle::new(client.clone()).into_rpc())?;

    module.merge(VaultRegistry::new(client.clone()).into_rpc())?;

    module.merge(Escrow::new(client.clone()).into_rpc())?;

    module.merge(Reward::new(client.clone()).into_rpc())?;

    module.merge(Issue::new(client.clone()).into_rpc())?;

    module.merge(Redeem::new(client.clone()).into_rpc())?;

    module.merge(Replace::new(client.clone()).into_rpc())?;

    module.merge(Loans::new(client.clone()).into_rpc())?;

    module.merge(DexGeneral::new(client.clone()).into_rpc())?;

    module.merge(DexStable::new(client).into_rpc())?;

    Ok(module)
}
