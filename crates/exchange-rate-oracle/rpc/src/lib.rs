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
pub trait ExchangeRateOracleApi<BlockHash, Wrapped, Collateral, CurrencyId>
where
    Wrapped: Codec + MaybeDisplay + MaybeFromStr,
    Collateral: Codec + MaybeDisplay + MaybeFromStr,
    CurrencyId: Codec,
{
    #[rpc(name = "exchangeRateOracle_wrappedToCollateral")]
    fn wrapped_to_collateral(
        &self,
        amount: BalanceWrapper<Wrapped>,
        currency_id: CurrencyId,
        at: Option<BlockHash>,
    ) -> JsonRpcResult<BalanceWrapper<Collateral>>;

    #[rpc(name = "exchangeRateOracle_collateralToWrapped")]
    fn collateral_to_wrapped(
        &self,
        amount: BalanceWrapper<Collateral>,
        currency_id: CurrencyId,
        at: Option<BlockHash>,
    ) -> JsonRpcResult<BalanceWrapper<Wrapped>>;
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

impl<C, Block, Wrapped, Collateral, CurrencyId>
    ExchangeRateOracleApi<<Block as BlockT>::Hash, Wrapped, Collateral, CurrencyId> for ExchangeRateOracle<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: ExchangeRateOracleRuntimeApi<Block, Wrapped, Collateral, CurrencyId>,
    Wrapped: Codec + MaybeDisplay + MaybeFromStr,
    Collateral: Codec + MaybeDisplay + MaybeFromStr,
    CurrencyId: Codec,
{
    fn wrapped_to_collateral(
        &self,
        amount: BalanceWrapper<Wrapped>,
        currency_id: CurrencyId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> JsonRpcResult<BalanceWrapper<Collateral>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        handle_response(
            api.wrapped_to_collateral(&at, amount, currency_id),
            "Unable to convert Wrapped to Collateral.".into(),
        )
    }

    fn collateral_to_wrapped(
        &self,
        amount: BalanceWrapper<Collateral>,
        currency_id: CurrencyId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> JsonRpcResult<BalanceWrapper<Wrapped>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        handle_response(
            api.collateral_to_wrapped(&at, amount, currency_id),
            "Unable to convert Collateral to Wrapped.".into(),
        )
    }
}
