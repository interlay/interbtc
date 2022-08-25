//! RPC interface for the Reward Module.

use codec::Codec;
use jsonrpsee::{
    core::{async_trait, Error as JsonRpseeError, RpcResult},
    proc_macros::rpc,
    types::error::{CallError, ErrorCode, ErrorObject},
};
use module_oracle_rpc_runtime_api::BalanceWrapper;
use sp_api::{ApiError, ProvideRuntimeApi};
use sp_blockchain::HeaderBackend;
use sp_runtime::{
    generic::BlockId,
    traits::{Block as BlockT, MaybeDisplay, MaybeFromStr},
    DispatchError,
};
use std::sync::Arc;

pub use module_reward_rpc_runtime_api::RewardApi as RewardRuntimeApi;

#[rpc(client, server)]
pub trait RewardApi<BlockHash, RewardId, CurrencyId, Balance>
where
    Balance: Codec + MaybeDisplay + MaybeFromStr,
    RewardId: Codec,
    CurrencyId: Codec,
{
    #[method(name = "reward_computeReward")]
    fn compute_reward(
        &self,
        account_id: RewardId,
        currency_id: CurrencyId,
        at: Option<BlockHash>,
    ) -> RpcResult<BalanceWrapper<Balance>>;
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

// fn handle_response<T>(result: Result<T, ApiError>, msg: String) -> RpcResult<T> {
//     result.map_err(|err| internal_err(format!("Runtime error: {:?}: {:?}", msg, err)))
// }
fn handle_response<T, E: std::fmt::Debug>(result: Result<Result<T, DispatchError>, E>, msg: String) -> RpcResult<T> {
    result
        .map_err(|err| internal_err(format!("Runtime error: {:?}: {:?}", msg, err)))?
        .map_err(|err| internal_err(format!("Execution error: {:?}: {:?}", msg, err)))
}

#[async_trait]
impl<C, Block, RewardId, CurrencyId, Balance> RewardApiServer<<Block as BlockT>::Hash, RewardId, CurrencyId, Balance>
    for Reward<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: RewardRuntimeApi<Block, RewardId, CurrencyId, Balance>,
    RewardId: Codec,
    CurrencyId: Codec,
    Balance: Codec + MaybeDisplay + MaybeFromStr,
{
    fn compute_reward(
        &self,
        account_id: RewardId,
        currency_id: CurrencyId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<BalanceWrapper<Balance>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        handle_response(
            api.compute_reward(&at, account_id, currency_id),
            "Unable to obtain the current reward".into(),
        )
    }
}
