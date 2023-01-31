// Copyright 2021-2022 Zenlink.
// Licensed under Apache 2.0.

use super::*;

pub trait ValidateCurrency<CurrencyId> {
	fn validate_pooled_currency(a: &[CurrencyId]) -> bool;
	fn validate_pool_lp_currency(a: CurrencyId) -> bool;
}

pub trait StablePoolLpCurrencyIdGenerate<CurrencyId, PoolId> {
	fn generate_by_pool_id(pool_id: PoolId) -> CurrencyId;
}

pub trait StableAmmApi<PoolId, CurrencyId, AccountId, Balance> {
	fn stable_amm_calculate_currency_amount(
		pool_id: PoolId,
		amounts: &[Balance],
		deposit: bool,
	) -> Result<Balance, DispatchError>;

	fn stable_amm_calculate_swap_amount(
		pool_id: PoolId,
		i: usize,
		j: usize,
		in_balance: Balance,
	) -> Option<Balance>;

	fn stable_amm_calculate_remove_liquidity(
		pool_id: PoolId,
		amount: Balance,
	) -> Option<Vec<Balance>>;

	fn stable_amm_calculate_remove_liquidity_one_currency(
		pool_id: PoolId,
		amount: Balance,
		index: u32,
	) -> Option<Balance>;

	fn currency_index(pool_id: PoolId, currency: CurrencyId) -> Option<u32>;

	fn add_liquidity(
		who: &AccountId,
		pool_id: PoolId,
		amounts: &[Balance],
		min_mint_amount: Balance,
		to: &AccountId,
	) -> Result<Balance, sp_runtime::DispatchError>;

	fn swap(
		who: &AccountId,
		poo_id: PoolId,
		from_index: u32,
		to_index: u32,
		in_amount: Balance,
		min_out_amount: Balance,
		to: &AccountId,
	) -> Result<Balance, sp_runtime::DispatchError>;

	fn remove_liquidity(
		who: &AccountId,
		poo_id: PoolId,
		lp_amount: Balance,
		min_amounts: &[Balance],
		to: &AccountId,
	) -> DispatchResult;

	fn remove_liquidity_one_currency(
		who: &AccountId,
		poo_id: PoolId,
		lp_amount: Balance,
		index: u32,
		min_amount: Balance,
		to: &AccountId,
	) -> Result<Balance, DispatchError>;

	fn remove_liquidity_imbalance(
		who: &AccountId,
		pool_id: PoolId,
		amounts: &[Balance],
		max_burn_amount: Balance,
		to: &AccountId,
	) -> DispatchResult;

	fn swap_pool_from_base(
		who: &AccountId,
		pool_id: PoolId,
		base_pool_id: PoolId,
		in_index: u32,
		out_index: u32,
		dx: Balance,
		min_dy: Balance,
		to: &AccountId,
	) -> Result<Balance, DispatchError>;

	fn swap_pool_to_base(
		who: &AccountId,
		pool_id: PoolId,
		base_pool_id: PoolId,
		in_index: u32,
		out_index: u32,
		dx: Balance,
		min_dy: Balance,
		to: &AccountId,
	) -> Result<Balance, DispatchError>;
}

impl<T: Config> StableAmmApi<T::PoolId, T::CurrencyId, T::AccountId, Balance> for Pallet<T> {
	fn stable_amm_calculate_currency_amount(
		pool_id: T::PoolId,
		amounts: &[Balance],
		deposit: bool,
	) -> Result<Balance, DispatchError> {
		Self::calculate_currency_amount(pool_id, amounts.to_vec(), deposit)
	}

	fn stable_amm_calculate_remove_liquidity(
		pool_id: T::PoolId,
		amount: Balance,
	) -> Option<Vec<Balance>> {
		if let Some(pool) = Self::pools(pool_id) {
			return match pool {
				Pool::Base(bp) => Self::calculate_base_remove_liquidity(&bp, amount),
				Pool::Meta(mp) => Self::calculate_base_remove_liquidity(&mp.info, amount),
			}
		}
		None
	}

	fn stable_amm_calculate_swap_amount(
		pool_id: T::PoolId,
		i: usize,
		j: usize,
		in_balance: Balance,
	) -> Option<Balance> {
		if let Some(pool) = Self::pools(pool_id) {
			return match pool {
				Pool::Base(bp) => Self::calculate_base_swap_amount(&bp, i, j, in_balance),
				Pool::Meta(mp) => {
					let virtual_price = Self::calculate_meta_virtual_price(&mp)?;
					let res =
						Self::calculate_meta_swap_amount(&mp, i, j, in_balance, virtual_price)?;
					Some(res.0)
				},
			}
		}
		None
	}

	fn stable_amm_calculate_remove_liquidity_one_currency(
		pool_id: T::PoolId,
		amount: Balance,
		index: u32,
	) -> Option<Balance> {
		if let Some(pool) = Self::pools(pool_id) {
			if let Some(res) = match pool {
				Pool::Base(bp) =>
					Self::calculate_base_remove_liquidity_one_token(&bp, amount, index),
				Pool::Meta(mp) => {
					let total_supply = T::MultiCurrency::total_issuance(mp.info.lp_currency_id);
					Self::calculate_meta_remove_liquidity_one_currency(
						&mp,
						amount,
						index as usize,
						total_supply,
					)
				},
			} {
				return Some(res.0)
			}
		}
		None
	}

	fn currency_index(pool_id: T::PoolId, currency: T::CurrencyId) -> Option<u32> {
		Self::get_currency_index(pool_id, currency)
	}

	fn add_liquidity(
		who: &T::AccountId,
		pool_id: T::PoolId,
		amounts: &[Balance],
		min_mint_amount: Balance,
		to: &T::AccountId,
	) -> Result<Balance, sp_runtime::DispatchError> {
		Self::inner_add_liquidity(who, pool_id, amounts, min_mint_amount, to)
	}

	fn swap(
		who: &T::AccountId,
		poo_id: T::PoolId,
		from_index: u32,
		to_index: u32,
		in_amount: Balance,
		min_out_amount: Balance,
		to: &T::AccountId,
	) -> Result<Balance, sp_runtime::DispatchError> {
		Self::inner_swap(
			who,
			poo_id,
			from_index as usize,
			to_index as usize,
			in_amount,
			min_out_amount,
			to,
		)
	}

	fn remove_liquidity(
		who: &T::AccountId,
		poo_id: T::PoolId,
		lp_amount: Balance,
		min_amounts: &[Balance],
		to: &T::AccountId,
	) -> DispatchResult {
		Self::inner_remove_liquidity(poo_id, who, lp_amount, min_amounts, to)
	}

	fn remove_liquidity_one_currency(
		who: &T::AccountId,
		poo_id: T::PoolId,
		lp_amount: Balance,
		index: u32,
		min_amount: Balance,
		to: &T::AccountId,
	) -> Result<Balance, DispatchError> {
		Self::inner_remove_liquidity_one_currency(poo_id, who, lp_amount, index, min_amount, to)
	}

	fn remove_liquidity_imbalance(
		who: &T::AccountId,
		pool_id: T::PoolId,
		amounts: &[Balance],
		max_burn_amount: Balance,
		to: &T::AccountId,
	) -> DispatchResult {
		Self::inner_remove_liquidity_imbalance(who, pool_id, amounts, max_burn_amount, to)
	}

	fn swap_pool_from_base(
		who: &T::AccountId,
		pool_id: T::PoolId,
		base_pool_id: T::PoolId,
		in_index: u32,
		out_index: u32,
		dx: Balance,
		min_dy: Balance,
		to: &T::AccountId,
	) -> Result<Balance, DispatchError> {
		Self::inner_swap_pool_from_base(
			who,
			pool_id,
			base_pool_id,
			in_index,
			out_index,
			dx,
			min_dy,
			to,
		)
	}

	fn swap_pool_to_base(
		who: &T::AccountId,
		pool_id: T::PoolId,
		base_pool_id: T::PoolId,
		in_index: u32,
		out_index: u32,
		dx: Balance,
		min_dy: Balance,
		to: &T::AccountId,
	) -> Result<Balance, DispatchError> {
		Self::inner_swap_pool_to_base(
			who,
			pool_id,
			base_pool_id,
			in_index,
			out_index,
			dx,
			min_dy,
			to,
		)
	}
}
