//! RPC interface for the Oracle.

pub use self::gen_client::Client as OracleClient;
use codec::Codec;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result as JsonRpcResult};
use jsonrpc_derive::rpc;
pub use module_oracle_rpc_runtime_api::{BalanceWrapper, OracleApi as OracleRuntimeApi};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{
    generic::BlockId,
    traits::{Block as BlockT, MaybeDisplay, MaybeFromStr},
    DispatchError,
};
use std::sync::Arc;

#[rpc]
pub trait OracleApi<BlockHash, Balance, CurrencyId>
where
    Balance: Codec + MaybeDisplay + MaybeFromStr,
    CurrencyId: Codec,
{
    #[rpc(name = "oracle_wrappedToCollateral")]
    fn wrapped_to_collateral(
        &self,
        amount: BalanceWrapper<Balance>,
        currency_id: CurrencyId,
        at: Option<BlockHash>,
    ) -> JsonRpcResult<BalanceWrapper<Balance>>;

    #[rpc(name = "oracle_collateralToWrapped")]
    fn collateral_to_wrapped(
        &self,
        amount: BalanceWrapper<Balance>,
        currency_id: CurrencyId,
        at: Option<BlockHash>,
    ) -> JsonRpcResult<BalanceWrapper<Balance>>;
}

/// A struct that implements the [`OracleApi`].
pub struct Oracle<C, B> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<B>,
}

impl<C, B> Oracle<C, B> {
    /// Create new `Oracle` with the given reference to the client.
    pub fn new(client: Arc<C>) -> Self {
        Oracle {
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

impl<C, Block, Balance, CurrencyId> OracleApi<<Block as BlockT>::Hash, Balance, CurrencyId> for Oracle<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: OracleRuntimeApi<Block, Balance, CurrencyId>,
    Balance: Codec + MaybeDisplay + MaybeFromStr,
    CurrencyId: Codec,
{
    fn wrapped_to_collateral(
        &self,
        amount: BalanceWrapper<Balance>,
        currency_id: CurrencyId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> JsonRpcResult<BalanceWrapper<Balance>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        handle_response(
            api.wrapped_to_collateral(&at, amount, currency_id),
            "Unable to convert Wrapped to Collateral.".into(),
        )
    }

    fn collateral_to_wrapped(
        &self,
        amount: BalanceWrapper<Balance>,
        currency_id: CurrencyId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> JsonRpcResult<BalanceWrapper<Balance>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        handle_response(
            api.collateral_to_wrapped(&at, amount, currency_id),
            "Unable to convert Collateral to Wrapped.".into(),
        )
    }
}
