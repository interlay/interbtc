//! RPC interface for the swap router pallet.
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

use codec::Codec;
use jsonrpsee::{
    core::{Error as JsonRpseeError, RpcResult},
    proc_macros::rpc,
    types::error::{CallError, ErrorObject},
};

use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_rpc::number::NumberOrHex;
use sp_runtime::{
    generic::BlockId,
    traits::{Block as BlockT, MaybeDisplay},
};
use std::sync::Arc;

pub use dex_swap_router_rpc_runtime_api::{DexSwapRouterApi as DexSwapRouterRuntimeApi, Route};

#[rpc(client, server)]
pub trait DexSwapRouterApi<BlockHash, Balance, CurrencyId, PoolId> {
    #[method(name = "dexSwapRouter_findBestTradeExactIn")]
    fn find_best_trade_exact_in(
        &self,
        input_amount: Balance,
        input_currency: CurrencyId,
        output_currency: CurrencyId,
        at: Option<BlockHash>,
    ) -> RpcResult<Option<(NumberOrHex, Vec<Route<PoolId, CurrencyId>>)>>;
}

pub struct DexSwapRouter<C, M> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<M>,
}

impl<C, M> DexSwapRouter<C, M> {
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            _marker: Default::default(),
        }
    }
}

impl<C, Block, Balance, CurrencyId, PoolId> DexSwapRouterApiServer<<Block as BlockT>::Hash, Balance, CurrencyId, PoolId>
    for DexSwapRouter<C, Block>
where
    Block: BlockT,
    Balance: Codec + TryInto<NumberOrHex> + std::fmt::Debug + MaybeDisplay + Copy,
    CurrencyId: Codec + std::cmp::PartialEq,
    PoolId: Codec,
    C: Send + Sync + 'static,
    C: ProvideRuntimeApi<Block>,
    C: HeaderBackend<Block>,
    C::Api: DexSwapRouterRuntimeApi<Block, Balance, CurrencyId, PoolId>,
{
    fn find_best_trade_exact_in(
        &self,
        input_amount: Balance,
        input_currency: CurrencyId,
        output_currency: CurrencyId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<Option<(NumberOrHex, Vec<Route<PoolId, CurrencyId>>)>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        if let Some((output_amount, route)) = api
            .find_best_trade_exact_in(&at, input_amount, input_currency, output_currency)
            .map_err(runtime_error_into_rpc_err)?
        {
            Ok(Some((try_into_rpc_balance(output_amount)?, route)))
        } else {
            Ok(None)
        }
    }
}

fn try_into_rpc_balance<Balance: Codec + TryInto<NumberOrHex> + MaybeDisplay + Copy + std::fmt::Debug>(
    value: Balance,
) -> RpcResult<NumberOrHex> {
    value.try_into().map_err(|_| {
        CallError::Custom(ErrorObject::owned(
            Error::RuntimeError.into(),
            "error in swap router pallet",
            Some("convert into rpc balance".to_string()),
        ))
        .into()
    })
}

/// Error type of this RPC api.
pub enum Error {
    /// The call to runtime failed.
    RuntimeError,
}

impl From<Error> for i32 {
    fn from(e: Error) -> i32 {
        match e {
            Error::RuntimeError => 1,
        }
    }
}

fn runtime_error_into_rpc_err(err: impl std::fmt::Display) -> JsonRpseeError {
    CallError::Custom(ErrorObject::owned(
        Error::RuntimeError.into(),
        "error in swap router pallet",
        Some(err.to_string()),
    ))
    .into()
}
