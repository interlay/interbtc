//! A collection of node-specific RPC methods.
//! Substrate provides the `sc-rpc` crate, which defines the core RPC layer
//! used by Substrate nodes. This file extends those RPC definitions with
//! capabilities that are specific to this project's runtime configuration.

#![warn(missing_docs)]

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

use zenlink_protocol_runtime_api::ZenlinkProtocolApi as ZenlinkProtocolRuntimeApi;
use zenlink_stable_amm_runtime_api::StableAmmApi as ZenlinkStableAmmRuntimeApi;

pub use jsonrpsee;

/// A type representing all RPC extensions.
pub type RpcExtension = jsonrpsee::RpcModule<()>;

/// Full client dependencies.
pub struct FullDeps<C, P> {
    /// The client instance to use.
    pub client: Arc<C>,
    /// Transaction pool instance.
    pub pool: Arc<P>,
    /// Whether to deny unsafe calls
    pub deny_unsafe: DenyUnsafe,
    /// Manual seal command sink
    pub command_sink: Option<futures::channel::mpsc::Sender<EngineCommand<Hash>>>,
}

/// Instantiate all full RPC extensions.
pub fn create_full<C, P>(deps: FullDeps<C, P>) -> Result<RpcExtension, Box<dyn std::error::Error + Send + Sync>>
where
    C: ProvideRuntimeApi<Block>,
    C: HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockChainError> + 'static,
    C: Send + Sync + 'static,
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
    C::Api: ZenlinkProtocolRuntimeApi<Block, AccountId, CurrencyId>,
    C::Api: ZenlinkStableAmmRuntimeApi<Block, CurrencyId, Balance, AccountId, StablePoolId>,
    C::Api: BlockBuilder<Block>,
    P: TransactionPool + 'static,
{
    use btc_relay_rpc::{BtcRelay, BtcRelayApiServer};
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
    use zenlink_protocol_rpc::{ZenlinkProtocol, ZenlinkProtocolApiServer};
    use zenlink_stable_amm_rpc::{StableAmm as ZenlinkStableAmm, StableAmmApiServer as ZenlinkStableAmmApiServer};

    let mut module = RpcExtension::new(());
    let FullDeps {
        client,
        pool,
        deny_unsafe,
        command_sink,
    } = deps;

    if let Some(command_sink) = command_sink {
        module.merge(
            // We provide the rpc handler with the sending end of the channel to allow the rpc
            // send EngineCommands to the background block authorship task.
            ManualSeal::new(command_sink).into_rpc(),
        )?;
    }

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

    module.merge(ZenlinkProtocol::new(client.clone()).into_rpc())?;

    module.merge(ZenlinkStableAmm::new(client).into_rpc())?;

    Ok(module)
}
