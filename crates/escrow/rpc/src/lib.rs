//! RPC interface for the Escrow Module.

use codec::Codec;
use jsonrpsee::{
    core::{async_trait, Error as JsonRpseeError, RpcResult},
    proc_macros::rpc,
    types::error::{CallError, ErrorCode, ErrorObject},
};
use oracle_rpc_runtime_api::BalanceWrapper;
use sp_api::{ApiError, ProvideRuntimeApi};
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::{Block as BlockT, MaybeDisplay, MaybeFromStr};
use std::sync::Arc;

pub use escrow_rpc_runtime_api::EscrowApi as EscrowRuntimeApi;

#[rpc(client, server)]
pub trait EscrowApi<BlockHash, AccountId, BlockNumber, Balance>
where
    Balance: Codec + MaybeDisplay + MaybeFromStr,
    BlockNumber: Codec,
    AccountId: Codec,
{
    #[method(name = "escrow_balanceAt")]
    fn balance_at(
        &self,
        account_id: AccountId,
        height: Option<BlockNumber>,
        at: Option<BlockHash>,
    ) -> RpcResult<BalanceWrapper<Balance>>;

    #[method(name = "escrow_totalSupply")]
    fn total_supply(&self, height: Option<BlockNumber>, at: Option<BlockHash>) -> RpcResult<BalanceWrapper<Balance>>;
}

fn internal_err<T: ToString>(message: T) -> JsonRpseeError {
    JsonRpseeError::Call(CallError::Custom(ErrorObject::owned(
        ErrorCode::InternalError.code(),
        message.to_string(),
        None::<()>,
    )))
}

/// A struct that implements the [`EscrowApi`].
pub struct Escrow<C, B> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<B>,
}

impl<C, B> Escrow<C, B> {
    /// Create new `Escrow` with the given reference to the client.
    pub fn new(client: Arc<C>) -> Self {
        Escrow {
            client,
            _marker: Default::default(),
        }
    }
}

fn handle_response<T>(result: Result<T, ApiError>, msg: String) -> RpcResult<T> {
    result.map_err(|err| internal_err(format!("Runtime error: {:?}: {:?}", msg, err)))
}

#[async_trait]
impl<C, Block, AccountId, BlockNumber, Balance>
    EscrowApiServer<<Block as BlockT>::Hash, AccountId, BlockNumber, Balance> for Escrow<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: EscrowRuntimeApi<Block, AccountId, BlockNumber, Balance>,
    AccountId: Codec,
    BlockNumber: Codec,
    Balance: Codec + MaybeDisplay + MaybeFromStr,
{
    fn balance_at(
        &self,
        account_id: AccountId,
        height: Option<BlockNumber>,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<BalanceWrapper<Balance>> {
        let api = self.client.runtime_api();
        let at = at.unwrap_or_else(|| self.client.info().best_hash);

        handle_response(
            api.balance_at(at, account_id, height),
            "Unable to obtain the escrow balance".into(),
        )
    }

    fn total_supply(
        &self,
        height: Option<BlockNumber>,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<BalanceWrapper<Balance>> {
        let api = self.client.runtime_api();
        let at = at.unwrap_or_else(|| self.client.info().best_hash);

        handle_response(
            api.total_supply(at, height),
            "Unable to obtain the escrow total supply".into(),
        )
    }
}
