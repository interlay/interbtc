//! RPC interface for the Exchange Rate Oracle.

use codec::Codec;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::sync::Arc;

pub use self::gen_client::Client as ExchangeRateOracleClient;
pub use module_exchange_rate_oracle_rpc_runtime_api::ExchangeRateOracleApi as ExchangeRateOracleRuntimeApi;

#[rpc]
pub trait ExchangeRateOracleApi<BlockHash, PolkaBTC, DOT> {
    #[rpc(name = "exchangeRateOracle_btcToDots")]
    fn btc_to_dots(&self, amount: PolkaBTC, at: Option<BlockHash>) -> Result<DOT>;
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

impl<C, Block, PolkaBTC, DOT> ExchangeRateOracleApi<<Block as BlockT>::Hash, PolkaBTC, DOT>
    for ExchangeRateOracle<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: ExchangeRateOracleRuntimeApi<Block, PolkaBTC, DOT>,
    PolkaBTC: Codec,
    DOT: Codec,
{
    fn btc_to_dots(&self, amount: PolkaBTC, at: Option<<Block as BlockT>::Hash>) -> Result<DOT> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        api.btc_to_dots(&at, amount).map_or_else(
            |e| {
                Err(RpcError {
                    code: ErrorCode::ServerError(Error::RuntimeError.into()),
                    message: "Unable to convert PolkaBTC to dots.".into(),
                    data: Some(format!("{:?}", e).into()),
                })
            },
            |result| {
                result.map_err(|e| RpcError {
                    code: ErrorCode::ServerError(Error::RuntimeError.into()),
                    message: "Unable to convert PolkaBTC to dots.".into(),
                    data: Some(format!("{:?}", e).into()),
                })
            },
        )
    }
}
