// Copyright 2021-2022 Zenlink.
// Licensed under Apache 2.0.

//! RPC interface for the zenlink dex module.
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
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::sync::Arc;

use zenlink_protocol::{AssetBalance, PairInfo};
use zenlink_protocol_runtime_api::ZenlinkProtocolApi as ZenlinkProtocolRuntimeApi;

#[rpc(client, server)]
pub trait ZenlinkProtocolApi<BlockHash, AccountId, AssetId> {
	#[method(name = "zenlinkProtocol_getBalance")]
	fn get_balance(
		&self,
		asset_id: AssetId,
		account: AccountId,
		at: Option<BlockHash>,
	) -> RpcResult<NumberOrHex>;

	#[method(name = "zenlinkProtocol_getSovereignsInfo")]
	fn get_sovereigns_info(
		&self,
		asset_id: AssetId,
		at: Option<BlockHash>,
	) -> RpcResult<Vec<(u32, AccountId, NumberOrHex)>>;

	#[method(name = "zenlinkProtocol_getPairByAssetId")]
	fn get_pair_by_asset_id(
		&self,
		asset_0: AssetId,
		asset_1: AssetId,
		at: Option<BlockHash>,
	) -> RpcResult<Option<PairInfo<AccountId, NumberOrHex, AssetId>>>;

	#[method(name = "zenlinkProtocol_getAmountInPrice")]
	fn get_amount_in_price(
		&self,
		supply: AssetBalance,
		path: Vec<AssetId>,
		at: Option<BlockHash>,
	) -> RpcResult<NumberOrHex>;

	#[method(name = "zenlinkProtocol_getAmountOutPrice")]
	fn get_amount_out_price(
		&self,
		supply: AssetBalance,
		path: Vec<AssetId>,
		at: Option<BlockHash>,
	) -> RpcResult<NumberOrHex>;

	#[method(name = "zenlinkProtocol_getEstimateLptoken")]
	fn get_estimate_lptoken(
		&self,
		asset_0: AssetId,
		asset_1: AssetId,
		amount_0_desired: AssetBalance,
		amount_1_desired: AssetBalance,
		amount_0_min: AssetBalance,
		amount_1_min: AssetBalance,
		at: Option<BlockHash>,
	) -> RpcResult<NumberOrHex>;
}

pub struct ZenlinkProtocol<C, M> {
	client: Arc<C>,
	_marker: std::marker::PhantomData<M>,
}

impl<C, M> ZenlinkProtocol<C, M> {
	pub fn new(client: Arc<C>) -> Self {
		Self { client, _marker: Default::default() }
	}
}

impl<C, Block, AccountId, AssetId>
	ZenlinkProtocolApiServer<<Block as BlockT>::Hash, AccountId, AssetId> for ZenlinkProtocol<C, Block>
where
	Block: BlockT,
	AccountId: Codec,
	AssetId: Codec,
	C: Send + Sync + 'static,
	C: ProvideRuntimeApi<Block>,
	C: HeaderBackend<Block>,
	C::Api: ZenlinkProtocolRuntimeApi<Block, AccountId, AssetId>,
{
	//buy amount asset price
	fn get_amount_in_price(
		&self,
		supply: AssetBalance,
		path: Vec<AssetId>,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<NumberOrHex> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		api.get_amount_in_price(&at, supply, path)
			.map(|price| price.into())
			.map_err(runtime_error_into_rpc_err)
	}

	//sell amount asset price
	fn get_amount_out_price(
		&self,
		supply: AssetBalance,
		path: Vec<AssetId>,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<NumberOrHex> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		api.get_amount_out_price(&at, supply, path)
			.map(|price| price.into())
			.map_err(runtime_error_into_rpc_err)
	}

	fn get_estimate_lptoken(
		&self,
		asset_0: AssetId,
		asset_1: AssetId,
		amount_0_desired: AssetBalance,
		amount_1_desired: AssetBalance,
		amount_0_min: AssetBalance,
		amount_1_min: AssetBalance,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<NumberOrHex> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		api.get_estimate_lptoken(
			&at,
			asset_0,
			asset_1,
			amount_0_desired,
			amount_1_desired,
			amount_0_min,
			amount_1_min,
		)
		.map(|price| price.into())
		.map_err(runtime_error_into_rpc_err)
	}

	fn get_balance(
		&self,
		asset_id: AssetId,
		account: AccountId,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<NumberOrHex> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		api.get_balance(&at, asset_id, account)
			.map(|asset_balance| asset_balance.into())
			.map_err(runtime_error_into_rpc_err)
	}

	fn get_sovereigns_info(
		&self,
		asset_id: AssetId,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<Vec<(u32, AccountId, NumberOrHex)>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		api.get_sovereigns_info(&at, asset_id)
			.map(|infos| {
				infos
					.into_iter()
					.map(|(para_id, account, asset_balance)| {
						(para_id, account, asset_balance.into())
					})
					.collect::<Vec<_>>()
			})
			.map_err(runtime_error_into_rpc_err)
	}

	fn get_pair_by_asset_id(
		&self,
		asset_0: AssetId,
		asset_1: AssetId,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<Option<PairInfo<AccountId, NumberOrHex, AssetId>>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		api.get_pair_by_asset_id(&at, asset_0, asset_1)
			.map(|pairs| {
				pairs.map(|pair| PairInfo {
					asset_0: pair.asset_0,
					asset_1: pair.asset_1,
					account: pair.account,
					total_liquidity: pair.total_liquidity.into(),
					holding_liquidity: pair.holding_liquidity.into(),
					reserve_0: pair.reserve_0.into(),
					reserve_1: pair.reserve_1.into(),
					lp_asset_id: pair.lp_asset_id,
					status: pair.status,
				})
			})
			.map_err(runtime_error_into_rpc_err)
	}
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

/// Converts a runtime trap into an RPC error.
fn runtime_error_into_rpc_err(err: impl std::fmt::Display) -> JsonRpseeError {
	CallError::Custom(ErrorObject::owned(
		Error::RuntimeError.into(),
		"error in zenlink pallet",
		Some(err.to_string()),
	))
	.into()
}
