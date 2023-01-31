// Copyright 2021-2022 Zenlink.
// Licensed under Apache 2.0.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::unnecessary_mut_passed)]

use codec::Codec;
use sp_std::vec::Vec;
use zenlink_protocol::{AssetBalance, PairInfo};

sp_api::decl_runtime_apis! {
	 pub trait ZenlinkProtocolApi<AccountId, AssetId>
	 where
		AccountId: Codec,
		AssetBalance: Codec,
		AssetId: Codec
	 {

		fn get_balance(asset_id: AssetId, owner: AccountId) -> AssetBalance;

		fn get_sovereigns_info(asset_id: AssetId) -> Vec<(u32, AccountId, AssetBalance)>;

		fn get_pair_by_asset_id(
			asset_0: AssetId,
			asset_1: AssetId
		) -> Option<PairInfo<AccountId, AssetBalance, AssetId>>;

		//buy amount asset price
		fn get_amount_in_price(supply: AssetBalance, path: Vec<AssetId>) -> AssetBalance;

		//sell amount asset price
		fn get_amount_out_price(supply: AssetBalance, path: Vec<AssetId>) -> AssetBalance;

		fn get_estimate_lptoken(
			asset_0: AssetId,
			asset_1: AssetId,
			amount_0_desired: AssetBalance,
			amount_1_desired: AssetBalance,
			amount_0_min: AssetBalance,
			amount_1_min: AssetBalance,
		) -> AssetBalance;
	 }
}
