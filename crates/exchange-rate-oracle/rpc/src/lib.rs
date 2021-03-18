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
pub trait ExchangeRateOracleApi<BlockHash, PolkaBTC, DOT>
where
    PolkaBTC: Codec + MaybeDisplay + MaybeFromStr,
    DOT: Codec + MaybeDisplay + MaybeFromStr,
{
    #[rpc(name = "exchangeRateOracle_btcToDots")]
    fn btc_to_dots(
        &self,
        amount: BalanceWrapper<PolkaBTC>,
        at: Option<BlockHash>,
    ) -> JsonRpcResult<BalanceWrapper<DOT>>;

    #[rpc(name = "exchangeRateOracle_dotsToBtc")]
    fn dots_to_btc(
        &self,
        amount: BalanceWrapper<DOT>,
        at: Option<BlockHash>,
    ) -> JsonRpcResult<BalanceWrapper<PolkaBTC>>;
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

impl<C, Block, PolkaBTC, DOT> ExchangeRateOracleApi<<Block as BlockT>::Hash, PolkaBTC, DOT>
    for ExchangeRateOracle<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: ExchangeRateOracleRuntimeApi<Block, PolkaBTC, DOT>,
    PolkaBTC: Codec + MaybeDisplay + MaybeFromStr,
    DOT: Codec + MaybeDisplay + MaybeFromStr,
{
    fn btc_to_dots(
        &self,
        amount: BalanceWrapper<PolkaBTC>,
        at: Option<<Block as BlockT>::Hash>,
    ) -> JsonRpcResult<BalanceWrapper<DOT>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        handle_response(
            api.btc_to_dots(&at, amount),
            "Unable to convert PolkaBTC to DOT.".into(),
        )
    }

    fn dots_to_btc(
        &self,
        amount: BalanceWrapper<DOT>,
        at: Option<<Block as BlockT>::Hash>,
    ) -> JsonRpcResult<BalanceWrapper<PolkaBTC>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        handle_response(
            api.dots_to_btc(&at, amount),
            "Unable to convert DOT to PolkaBTC.".into(),
        )
    }
}
