//! RPC interface for the Exchange Rate Oracle.

pub use self::gen_client::Client as ExchangeRateOracleClient;
use codec::Codec;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result as JsonRpcResult};
use jsonrpc_derive::rpc;
pub use module_exchange_rate_oracle_rpc_runtime_api::{
    BalanceWrapper, ExchangeRateOracleApi as ExchangeRateOracleRuntimeApi,
};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{
    generic::BlockId,
    traits::{Block as BlockT, MaybeDisplay, MaybeFromStr},
    DispatchError,
};
use std::sync::Arc;

#[rpc]
pub trait ExchangeRateOracleApi<BlockHash, Issuing, Backing>
where
    Issuing: Codec + MaybeDisplay + MaybeFromStr,
    Backing: Codec + MaybeDisplay + MaybeFromStr,
{
    #[rpc(name = "exchangeRateOracle_issuingToBacking")]
    fn issuing_to_backing(
        &self,
        amount: BalanceWrapper<Issuing>,
        at: Option<BlockHash>,
    ) -> JsonRpcResult<BalanceWrapper<Backing>>;

    #[rpc(name = "exchangeRateOracle_backingToIssuing")]
    fn backing_to_issuing(
        &self,
        amount: BalanceWrapper<Backing>,
        at: Option<BlockHash>,
    ) -> JsonRpcResult<BalanceWrapper<Issuing>>;
}

/// A struct that implements the [`ExchangeRateOracleApi`].
pub struct ExchangeRateOracle<C, B> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<B>,
}

impl<C, B> ExchangeRateOracle<C, B> {
    /// Create new `ExchangeRateOracle` with the given reference to the client.
    pub fn new(client: Arc<C>) -> Self {
        ExchangeRateOracle {
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

impl<C, Block, Issuing, Backing> ExchangeRateOracleApi<<Block as BlockT>::Hash, Issuing, Backing>
    for ExchangeRateOracle<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: ExchangeRateOracleRuntimeApi<Block, Issuing, Backing>,
    Issuing: Codec + MaybeDisplay + MaybeFromStr,
    Backing: Codec + MaybeDisplay + MaybeFromStr,
{
    fn issuing_to_backing(
        &self,
        amount: BalanceWrapper<Issuing>,
        at: Option<<Block as BlockT>::Hash>,
    ) -> JsonRpcResult<BalanceWrapper<Backing>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        handle_response(
            api.issuing_to_backing(&at, amount),
            "Unable to convert Issuing to Backing.".into(),
        )
    }

    fn backing_to_issuing(
        &self,
        amount: BalanceWrapper<Backing>,
        at: Option<<Block as BlockT>::Hash>,
    ) -> JsonRpcResult<BalanceWrapper<Issuing>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        handle_response(
            api.backing_to_issuing(&at, amount),
            "Unable to convert Backing to Issuing.".into(),
        )
    }
}
