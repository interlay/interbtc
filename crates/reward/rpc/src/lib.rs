//! RPC interface for the Reward Module.

use codec::Codec;
use jsonrpsee::{
    core::{async_trait, Error as JsonRpseeError, RpcResult},
    proc_macros::rpc,
    types::error::{CallError, ErrorCode, ErrorObject},
};
use oracle_rpc_runtime_api::BalanceWrapper;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{
    generic::BlockId,
    traits::{Block as BlockT, MaybeDisplay, MaybeFromStr},
    DispatchError,
};
use std::sync::Arc;

pub use reward_rpc_runtime_api::RewardApi as RewardRuntimeApi;

#[rpc(client, server)]
pub trait RewardApi<BlockHash, AccountId, VaultId, CurrencyId, Balance, BlockNumber, UnsignedFixedPoint>
where
    Balance: Codec + MaybeDisplay + MaybeFromStr,
    AccountId: Codec,
    VaultId: Codec,
    CurrencyId: Codec,
    BlockNumber: Codec,
    UnsignedFixedPoint: Codec,
{
    #[method(name = "reward_computeEscrowReward")]
    fn compute_escrow_reward(
        &self,
        account_id: AccountId,
        currency_id: CurrencyId,
        at: Option<BlockHash>,
    ) -> RpcResult<BalanceWrapper<Balance>>;

    #[method(name = "reward_computeFarmingReward")]
    fn compute_farming_reward(
        &self,
        account_id: AccountId,
        pool_currency_id: CurrencyId,
        reward_currency_id: CurrencyId,
        at: Option<BlockHash>,
    ) -> RpcResult<BalanceWrapper<Balance>>;

    // TODO: change this to support querying by nominator_id
    #[method(name = "reward_computeVaultReward")]
    fn compute_vault_reward(
        &self,
        vault_id: VaultId,
        currency_id: CurrencyId,
        at: Option<BlockHash>,
    ) -> RpcResult<BalanceWrapper<Balance>>;

    #[method(name = "reward_estimateEscrowRewardRate")]
    fn estimate_escrow_reward_rate(
        &self,
        account_id: AccountId,
        amount: Option<Balance>,
        lock_time: Option<BlockNumber>,
        at: Option<BlockHash>,
    ) -> RpcResult<UnsignedFixedPoint>;

    #[method(name = "reward_estimateFarmingReward")]
    fn estimate_farming_reward(
        &self,
        account_id: AccountId,
        pool_currency_id: CurrencyId,
        reward_currency_id: CurrencyId,
        at: Option<BlockHash>,
    ) -> RpcResult<BalanceWrapper<Balance>>;

    #[method(name = "reward_estimateVaultRewardRate")]
    fn estimate_vault_reward_rate(&self, vault_id: VaultId, at: Option<BlockHash>) -> RpcResult<UnsignedFixedPoint>;
}

fn internal_err<T: ToString>(message: T) -> JsonRpseeError {
    JsonRpseeError::Call(CallError::Custom(ErrorObject::owned(
        ErrorCode::InternalError.code(),
        message.to_string(),
        None::<()>,
    )))
}

/// A struct that implements the [`RewardApi`].
pub struct Reward<C, B> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<B>,
}

impl<C, B> Reward<C, B> {
    /// Create new `Reward` with the given reference to the client.
    pub fn new(client: Arc<C>) -> Self {
        Reward {
            client,
            _marker: Default::default(),
        }
    }
}

fn handle_response<T, E: std::fmt::Debug>(result: Result<Result<T, DispatchError>, E>, msg: String) -> RpcResult<T> {
    result
        .map_err(|err| internal_err(format!("Runtime error: {:?}: {:?}", msg, err)))?
        .map_err(|err| internal_err(format!("Execution error: {:?}: {:?}", msg, err)))
}

#[async_trait]
impl<C, Block, AccountId, VaultId, CurrencyId, Balance, BlockNumber, UnsignedFixedPoint>
    RewardApiServer<<Block as BlockT>::Hash, AccountId, VaultId, CurrencyId, Balance, BlockNumber, UnsignedFixedPoint>
    for Reward<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: RewardRuntimeApi<Block, AccountId, VaultId, CurrencyId, Balance, BlockNumber, UnsignedFixedPoint>,
    AccountId: Codec,
    VaultId: Codec,
    CurrencyId: Codec,
    Balance: Codec + MaybeDisplay + MaybeFromStr,
    BlockNumber: Codec,
    UnsignedFixedPoint: Codec,
{
    fn compute_escrow_reward(
        &self,
        account_id: AccountId,
        currency_id: CurrencyId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<BalanceWrapper<Balance>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        handle_response(
            api.compute_escrow_reward(&at, account_id, currency_id),
            "Unable to compute the current reward".into(),
        )
    }

    fn compute_farming_reward(
        &self,
        account_id: AccountId,
        pool_currency_id: CurrencyId,
        reward_currency_id: CurrencyId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<BalanceWrapper<Balance>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        handle_response(
            api.compute_farming_reward(&at, account_id, pool_currency_id, reward_currency_id),
            "Unable to compute the current reward".into(),
        )
    }

    fn compute_vault_reward(
        &self,
        vault_id: VaultId,
        currency_id: CurrencyId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<BalanceWrapper<Balance>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        handle_response(
            api.compute_vault_reward(&at, vault_id, currency_id),
            "Unable to compute the current reward".into(),
        )
    }

    fn estimate_escrow_reward_rate(
        &self,
        account_id: AccountId,
        amount: Option<Balance>,
        lock_time: Option<BlockNumber>,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<UnsignedFixedPoint> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        handle_response(
            api.estimate_escrow_reward_rate(&at, account_id, amount, lock_time),
            "Unable to estimate the current reward".into(),
        )
    }

    fn estimate_farming_reward(
        &self,
        account_id: AccountId,
        pool_currency_id: CurrencyId,
        reward_currency_id: CurrencyId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<BalanceWrapper<Balance>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        handle_response(
            api.estimate_farming_reward(&at, account_id, pool_currency_id, reward_currency_id),
            "Unable to estimate the current reward".into(),
        )
    }

    fn estimate_vault_reward_rate(
        &self,
        vault_id: VaultId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<UnsignedFixedPoint> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        handle_response(
            api.estimate_vault_reward_rate(&at, vault_id),
            "Unable to estimate the current reward".into(),
        )
    }
}
