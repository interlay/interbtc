// Copyright 2021-2022 Zenlink.
// Licensed under Apache 2.0.

//! Runtime API definition for stable amm.

#![cfg_attr(not(feature = "std"), no_std)]
// The `too_many_arguments` warning originates from `decl_runtime_apis` macro.
#![allow(clippy::too_many_arguments)]
// The `unnecessary_mut_passed` warning originates from `decl_runtime_apis` macro.
#![allow(clippy::unnecessary_mut_passed)]
use codec::Codec;
use sp_std::vec::Vec;

sp_api::decl_runtime_apis! {
	pub trait StableAmmApi<CurrencyId, Balance, AccountId, PoolId> where
		Balance: Codec,
		CurrencyId: Codec,
		AccountId: Codec,
		PoolId: Codec,
	{
		fn get_virtual_price(pool_id: PoolId)->Balance;

		fn get_a(pool_id: PoolId)->Balance;

		fn get_a_precise(pool_id: PoolId)->Balance;

		fn get_currencies(pool_id: PoolId)->Vec<CurrencyId>;

		fn get_currency(pool_id: PoolId, index: u32)->Option<CurrencyId>;

		fn get_lp_currency(pool_id: PoolId)->Option<CurrencyId>;

		fn get_currency_precision_multipliers(pool_id: PoolId)->Vec<Balance>;

		fn get_currency_balances(pool_id: PoolId)->Vec<Balance>;

		fn get_number_of_currencies(pool_id: PoolId)->u32;

		fn get_admin_balances(pool_id: PoolId)->Vec<Balance>;

		fn calculate_currency_amount(pool_id: PoolId, amounts:Vec<Balance>, deposit: bool)->Balance;

		fn calculate_swap(pool_id: PoolId, in_index: u32, out_index: u32, in_amount: Balance)->Balance;

		fn calculate_remove_liquidity(pool_id: PoolId, amount: Balance)->Vec<Balance>;

		fn calculate_remove_liquidity_one_currency(pool_id: PoolId, amount:Balance, index: u32)->Balance;
	}
}
