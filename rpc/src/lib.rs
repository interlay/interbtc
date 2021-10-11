//! A collection of node-specific RPC methods.
//! Substrate provides the `sc-rpc` crate, which defines the core RPC layer
//! used by Substrate nodes. This file extends those RPC definitions with
//! capabilities that are specific to this project's runtime configuration.

#![warn(missing_docs)]

use primitives::{
    issue::IssueRequest, redeem::RedeemRequest, refund::RefundRequest, replace::ReplaceRequest, AccountId, Balance,
    Block, BlockNumber, CurrencyId, H256Le, Nonce, VaultId,
};
pub use sc_rpc_api::DenyUnsafe;
use sc_transaction_pool_api::TransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_arithmetic::FixedU128;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use sp_core::H256;
use std::sync::Arc;

pub use jsonrpc_core;

/// Full client dependencies.
pub struct FullDeps<C, P> {
    /// The client instance to use.
    pub client: Arc<C>,
    /// Transaction pool instance.
    pub pool: Arc<P>,
    /// Whether to deny unsafe calls
    pub deny_unsafe: DenyUnsafe,
}

/// Instantiate all full RPC extensions.
pub fn create_full<C, P>(deps: FullDeps<C, P>) -> jsonrpc_core::IoHandler<sc_rpc::Metadata>
where
    C: ProvideRuntimeApi<Block>,
    C: HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockChainError> + 'static,
    C: Send + Sync + 'static,
    C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>,
    C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
    C::Api: module_btc_relay_rpc::BtcRelayRuntimeApi<Block, H256Le>,
    C::Api: module_oracle_rpc::OracleRuntimeApi<Block, Balance, CurrencyId>,
    C::Api: module_relay_rpc::RelayRuntimeApi<Block, VaultId<AccountId, CurrencyId>>,
    C::Api: module_vault_registry_rpc::VaultRegistryRuntimeApi<
        Block,
        VaultId<AccountId, CurrencyId>,
        Balance,
        FixedU128,
        CurrencyId,
        AccountId,
    >,
    C::Api: module_issue_rpc::IssueRuntimeApi<
        Block,
        AccountId,
        H256,
        IssueRequest<AccountId, BlockNumber, Balance, CurrencyId>,
    >,
    C::Api: module_redeem_rpc::RedeemRuntimeApi<
        Block,
        AccountId,
        H256,
        RedeemRequest<AccountId, BlockNumber, Balance, CurrencyId>,
    >,
    C::Api: module_refund_rpc::RefundRuntimeApi<Block, AccountId, H256, RefundRequest<AccountId, Balance, CurrencyId>>,
    C::Api: module_replace_rpc::ReplaceRuntimeApi<
        Block,
        AccountId,
        H256,
        ReplaceRequest<AccountId, BlockNumber, Balance, CurrencyId>,
    >,
    C::Api: BlockBuilder<Block>,
    P: TransactionPool + 'static,
{
    use module_btc_relay_rpc::{BtcRelay, BtcRelayApi};
    use module_issue_rpc::{Issue, IssueApi};
    use module_oracle_rpc::{Oracle, OracleApi};
    use module_redeem_rpc::{Redeem, RedeemApi};
    use module_refund_rpc::{Refund, RefundApi};
    use module_relay_rpc::{Relay, RelayApi};
    use module_replace_rpc::{Replace, ReplaceApi};
    use module_vault_registry_rpc::{VaultRegistry, VaultRegistryApi};
    use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApi};
    use substrate_frame_rpc_system::{FullSystem, SystemApi};

    let mut io = jsonrpc_core::IoHandler::default();
    let FullDeps {
        client,
        pool,
        deny_unsafe,
    } = deps;

    io.extend_with(SystemApi::to_delegate(FullSystem::new(
        client.clone(),
        pool,
        deny_unsafe,
    )));

    io.extend_with(TransactionPaymentApi::to_delegate(TransactionPayment::new(
        client.clone(),
    )));

    io.extend_with(BtcRelayApi::to_delegate(BtcRelay::new(client.clone())));

    io.extend_with(OracleApi::to_delegate(Oracle::new(client.clone())));

    io.extend_with(RelayApi::to_delegate(Relay::new(client.clone())));

    io.extend_with(VaultRegistryApi::to_delegate(VaultRegistry::new(client.clone())));

    io.extend_with(IssueApi::to_delegate(Issue::new(client.clone())));

    io.extend_with(RedeemApi::to_delegate(Redeem::new(client.clone())));

    io.extend_with(RefundApi::to_delegate(Refund::new(client.clone())));

    io.extend_with(ReplaceApi::to_delegate(Replace::new(client)));

    io
}
