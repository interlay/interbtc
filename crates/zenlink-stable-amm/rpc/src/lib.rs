// Copyright 2021-2022 Zenlink.
// Licensed under Apache 2.0.

//! RPC interface for the stable amm pallet.
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

use zenlink_stable_amm_runtime_api::StableAmmApi as StableAmmRuntimeApi;

#[rpc(client, server)]
pub trait StableAmmApi<BlockHash, CurrencyId, Balance, AccountId, PoolId> {
	#[method(name = "zenlinkStableAmm_getVirtualPrice")]
	fn get_virtual_price(&self, pool_id: PoolId, at: Option<BlockHash>) -> RpcResult<NumberOrHex>;

	#[method(name = "zenlinkStableAmm_getA")]
	fn get_a(&self, pool_id: PoolId, at: Option<BlockHash>) -> RpcResult<NumberOrHex>;

	#[method(name = "zenlinkStableAmm_getAPrecise")]
	fn get_a_precise(&self, pool_id: PoolId, at: Option<BlockHash>) -> RpcResult<NumberOrHex>;

	#[method(name = "zenlinkStableAmm_getCurrencies")]
	fn get_currencies(&self, pool_id: PoolId, at: Option<BlockHash>) -> RpcResult<Vec<CurrencyId>>;

	#[method(name = "zenlinkStableAmm_getCurrency")]
	fn get_currency(
		&self,
		pool_id: PoolId,
		index: u32,
		at: Option<BlockHash>,
	) -> RpcResult<CurrencyId>;

	#[method(name = "zenlinkStableAmm_getLpCurrency")]
	fn get_lp_currency(&self, pool_id: PoolId, at: Option<BlockHash>) -> RpcResult<CurrencyId>;

	#[method(name = "zenlinkStableAmm_getCurrencyIndex")]
	fn get_currency_index(
		&self,
		pool_id: PoolId,
		currency: CurrencyId,
		at: Option<BlockHash>,
	) -> RpcResult<u32>;

	#[method(name = "zenlinkStableAmm_getCurrencyPrecisionMultipliers")]
	fn get_currency_precision_multipliers(
		&self,
		pool_id: PoolId,
		at: Option<BlockHash>,
	) -> RpcResult<Vec<NumberOrHex>>;

	#[method(name = "zenlinkStableAmm_getCurrencyBalances")]
	fn get_currency_balances(
		&self,
		pool_id: PoolId,
		at: Option<BlockHash>,
	) -> RpcResult<Vec<NumberOrHex>>;

	#[method(name = "zenlinkStableAmm_getNumberOfCurrencies")]
	fn get_number_of_currencies(&self, pool_id: PoolId, at: Option<BlockHash>) -> RpcResult<u32>;

	#[method(name = "zenlinkStableAmm_getAdminBalances")]
	fn get_admin_balances(
		&self,
		pool_id: PoolId,
		at: Option<BlockHash>,
	) -> RpcResult<Vec<NumberOrHex>>;

	#[method(name = "zenlinkStableAmm_getAdminBalance")]
	fn get_admin_balance(
		&self,
		pool_id: PoolId,
		index: u32,
		at: Option<BlockHash>,
	) -> RpcResult<NumberOrHex>;

	#[method(name = "zenlinkStableAmm_calculateCurrencyAmount")]
	fn calculate_currency_amount(
		&self,
		pool_id: PoolId,
		amounts: Vec<Balance>,
		deposit: bool,
		at: Option<BlockHash>,
	) -> RpcResult<NumberOrHex>;

	#[method(name = "zenlinkStableAmm_calculateSwap")]
	fn calculate_swap(
		&self,
		pool_id: PoolId,
		in_index: u32,
		out_index: u32,
		in_amount: Balance,
		at: Option<BlockHash>,
	) -> RpcResult<NumberOrHex>;

	#[method(name = "zenlinkStableAmm_calculateRemoveLiquidity")]
	fn calculate_remove_liquidity(
		&self,
		pool_id: PoolId,
		amount: Balance,
		at: Option<BlockHash>,
	) -> RpcResult<Vec<NumberOrHex>>;

	#[method(name = "stableAmm_calculateRemoveLiquidityOneCurrency")]
	fn calculate_remove_liquidity_one_currency(
		&self,
		pool_id: PoolId,
		amount: Balance,
		index: u32,
		at: Option<BlockHash>,
	) -> RpcResult<NumberOrHex>;
}

pub struct StableAmm<C, M> {
	client: Arc<C>,
	_marker: std::marker::PhantomData<M>,
}

impl<C, M> StableAmm<C, M> {
	pub fn new(client: Arc<C>) -> Self {
		Self { client, _marker: Default::default() }
	}
}

impl<C, Block, CurrencyId, Balance, AccountId, PoolId>
	StableAmmApiServer<<Block as BlockT>::Hash, CurrencyId, Balance, AccountId, PoolId>
	for StableAmm<C, Block>
where
	Block: BlockT,
	CurrencyId: Codec + std::cmp::PartialEq,
	Balance: Codec + TryInto<NumberOrHex> + std::fmt::Debug + MaybeDisplay + Copy,
	AccountId: Codec,
	PoolId: Codec,
	C: Send + Sync + 'static,
	C: ProvideRuntimeApi<Block>,
	C: HeaderBackend<Block>,
	C::Api: StableAmmRuntimeApi<Block, CurrencyId, Balance, AccountId, PoolId>,
{
	fn get_virtual_price(
		&self,
		pool_id: PoolId,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<NumberOrHex> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		let price = api.get_virtual_price(&at, pool_id).map_err(runtime_error_into_rpc_err)?;

		try_into_rpc_balance(price)
	}

	fn get_a(
		&self,
		pool_id: PoolId,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<NumberOrHex> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		let price = api.get_a(&at, pool_id).map_err(runtime_error_into_rpc_err)?;

		try_into_rpc_balance(price)
	}

	fn get_a_precise(
		&self,
		pool_id: PoolId,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<NumberOrHex> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		let price = api.get_a_precise(&at, pool_id).map_err(runtime_error_into_rpc_err)?;

		try_into_rpc_balance(price)
	}

	fn get_currencies(
		&self,
		pool_id: PoolId,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<Vec<CurrencyId>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		api.get_currencies(&at, pool_id).map_err(runtime_error_into_rpc_err)
	}

	fn get_currency(
		&self,
		pool_id: PoolId,
		index: u32,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<CurrencyId> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		api.get_currency(&at, pool_id, index).map_or_else(
			|e| Err(runtime_error_into_rpc_err(e)),
			|v| v.ok_or(runtime_error_into_rpc_err("not found")),
		)
	}

	fn get_lp_currency(
		&self,
		pool_id: PoolId,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<CurrencyId> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		api.get_lp_currency(&at, pool_id).map_or_else(
			|e| Err(runtime_error_into_rpc_err(e)),
			|v| v.ok_or(runtime_error_into_rpc_err("not found")),
		)
	}

	fn get_currency_index(
		&self,
		pool_id: PoolId,
		currency: CurrencyId,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<u32> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		let currencies = api.get_currencies(&at, pool_id).map_err(runtime_error_into_rpc_err)?;

		for (i, c) in currencies.iter().enumerate() {
			if *c == currency {
				return Ok(i as u32)
			}
		}
		Err(runtime_error_into_rpc_err("invalid index"))
	}

	fn get_currency_precision_multipliers(
		&self,
		pool_id: PoolId,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<Vec<NumberOrHex>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		api.get_currency_precision_multipliers(&at, pool_id)
			.map_err(runtime_error_into_rpc_err)?
			.iter()
			.map(|b| try_into_rpc_balance(*b))
			.collect()
	}

	fn get_currency_balances(
		&self,
		pool_id: PoolId,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<Vec<NumberOrHex>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		api.get_currency_balances(&at, pool_id)
			.map_err(runtime_error_into_rpc_err)?
			.iter()
			.map(|b| try_into_rpc_balance(*b))
			.collect()
	}

	fn get_number_of_currencies(
		&self,
		pool_id: PoolId,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<u32> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		api.get_number_of_currencies(&at, pool_id).map_err(runtime_error_into_rpc_err)
	}

	fn get_admin_balances(
		&self,
		pool_id: PoolId,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<Vec<NumberOrHex>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		api.get_admin_balances(&at, pool_id)
			.map_err(runtime_error_into_rpc_err)?
			.iter()
			.map(|b| try_into_rpc_balance(*b))
			.collect()
	}

	fn get_admin_balance(
		&self,
		pool_id: PoolId,
		index: u32,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<NumberOrHex> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		let balances = api.get_admin_balances(&at, pool_id).map_err(runtime_error_into_rpc_err)?;

		for (i, balance) in balances.iter().enumerate() {
			if i as u32 == index {
				return try_into_rpc_balance(*balance)
			}
		}

		Err(runtime_error_into_rpc_err("invalid index"))
	}

	fn calculate_currency_amount(
		&self,
		pool_id: PoolId,
		amounts: Vec<Balance>,
		deposit: bool,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<NumberOrHex> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		let amount = api
			.calculate_currency_amount(&at, pool_id, amounts, deposit)
			.map_err(runtime_error_into_rpc_err)?;

		try_into_rpc_balance(amount)
	}

	fn calculate_swap(
		&self,
		pool_id: PoolId,
		in_index: u32,
		out_index: u32,
		in_amount: Balance,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<NumberOrHex> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		let amount = api
			.calculate_swap(&at, pool_id, in_index, out_index, in_amount)
			.map_err(runtime_error_into_rpc_err)?;

		try_into_rpc_balance(amount)
	}

	fn calculate_remove_liquidity(
		&self,
		pool_id: PoolId,
		amount: Balance,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<Vec<NumberOrHex>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		api.calculate_remove_liquidity(&at, pool_id, amount)
			.map_err(runtime_error_into_rpc_err)?
			.iter()
			.map(|b| try_into_rpc_balance(*b))
			.collect()
	}

	fn calculate_remove_liquidity_one_currency(
		&self,
		pool_id: PoolId,
		amount: Balance,
		index: u32,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<NumberOrHex> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		let amount = api
			.calculate_remove_liquidity_one_currency(&at, pool_id, amount, index)
			.map_err(runtime_error_into_rpc_err)?;

		try_into_rpc_balance(amount)
	}
}

fn try_into_rpc_balance<
	Balance: Codec + TryInto<NumberOrHex> + MaybeDisplay + Copy + std::fmt::Debug,
>(
	value: Balance,
) -> RpcResult<NumberOrHex> {
	value.try_into().map_err(|_| {
		CallError::Custom(ErrorObject::owned(
			Error::RuntimeError.into(),
			"error in stable amm pallet",
			Some("transfer into rpc balance".to_string()),
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
		"error in stable pallet",
		Some(err.to_string()),
	))
	.into()
}
