//! RPC interface for the Sla Module.

pub use self::gen_client::Client as SlaClient;
use codec::Codec;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result as JsonRpcResult};
use jsonrpc_derive::rpc;
use module_exchange_rate_oracle_rpc_runtime_api::BalanceWrapper;
pub use module_sla_rpc_runtime_api::SlaApi as SlaRuntimeApi;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{
    generic::BlockId,
    traits::{Block as BlockT, MaybeDisplay, MaybeFromStr},
    DispatchError,
};
use std::sync::Arc;

#[rpc]
pub trait SlaApi<AccountId, Backing, BlockHash>
where
    Backing: Codec + MaybeDisplay + MaybeFromStr,
{
    #[rpc(name = "sla_calculateSlashedAmount")]
    fn calculate_slashed_amount(
        &self,
        vault_id: AccountId,
        stake: BalanceWrapper<Backing>,
        at: Option<BlockHash>,
    ) -> JsonRpcResult<BalanceWrapper<Backing>>;
}

/// A struct that implements the [`SlaApi`].
pub struct Sla<C, B> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<B>,
}

impl<C, B> Sla<C, B> {
    /// Create new `Sla` with the given reference to the client.
    pub fn new(client: Arc<C>) -> Self {
        Sla {
            client,
            _marker: Default::default(),
        }
    }
}

pub enum Error {
    RuntimeError,
}

impl From<Error> for i64 {
    fn from(e: Error) -> i64 {
        match e {
            Error::RuntimeError => 1,
        }
    }
}

fn handle_response<T, E: std::fmt::Debug>(
    result: Result<Result<T, DispatchError>, E>,
    msg: String,
) -> JsonRpcResult<T> {
    result.map_or_else(
        |e| {
            Err(RpcError {
                code: ErrorCode::ServerError(Error::RuntimeError.into()),
                message: msg.clone(),
                data: Some(format!("{:?}", e).into()),
            })
        },
        |result| {
            result.map_err(|e| RpcError {
                code: ErrorCode::ServerError(Error::RuntimeError.into()),
                message: msg.clone(),
                data: Some(format!("{:?}", e).into()),
            })
        },
    )
}

impl<C, Block, AccountId, Backing> SlaApi<AccountId, Backing, <Block as BlockT>::Hash> for Sla<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: SlaRuntimeApi<Block, AccountId, Backing>,
    AccountId: Codec,
    Backing: Codec + MaybeDisplay + MaybeFromStr,
{
    fn calculate_slashed_amount(
        &self,
        vault_id: AccountId,
        stake: BalanceWrapper<Backing>,
        at: Option<<Block as BlockT>::Hash>,
    ) -> JsonRpcResult<BalanceWrapper<Backing>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        handle_response(
            api.calculate_slashed_amount(&at, vault_id, stake),
            "Unable to calculate slashed amount.".into(),
        )
    }
}
