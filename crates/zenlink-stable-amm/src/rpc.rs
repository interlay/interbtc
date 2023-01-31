// Copyright 2021-2022 Zenlink.
// Licensed under Apache 2.0.

#![allow(clippy::type_complexity)]

use super::*;

impl<T: Config> Pallet<T> {
	pub fn get_virtual_price(pool_id: T::PoolId) -> Balance {
		if let Some(pool) = Self::pools(pool_id) {
			return Self::get_pool_virtual_price(&pool).unwrap_or_default()
		};
		Balance::default()
	}

	pub fn get_a(pool_id: T::PoolId) -> Balance {
		if let Some(general_pool) = Self::pools(pool_id) {
			let pool = match general_pool {
				Pool::Base(bp) => bp,
				Pool::Meta(mp) => mp.info,
			};
			return Self::get_a_precise(&pool).unwrap_or_default() / A_PRECISION
		};
		Balance::default()
	}

	pub fn get_a_precise_by_id(pool_id: T::PoolId) -> Balance {
		if let Some(general_pool) = Self::pools(pool_id) {
			let pool = match general_pool {
				Pool::Base(bp) => bp,
				Pool::Meta(mp) => mp.info,
			};
			return Self::get_a_precise(&pool).unwrap_or_default()
		};
		Balance::default()
	}

	pub fn get_currencies(pool_id: T::PoolId) -> Vec<T::CurrencyId> {
		if let Some(pool) = Self::pools(pool_id) {
			return pool.get_currency_ids()
		};
		Vec::new()
	}

	pub fn get_currency_index(pool_id: T::PoolId, currency_id: T::CurrencyId) -> Option<u32> {
		if let Some(pool) = Self::pools(pool_id) {
			for (i, c) in pool.get_currency_ids().iter().enumerate() {
				if *c == currency_id {
					return Some(i as u32)
				}
			}
		};
		None
	}

	pub fn get_currency(pool_id: T::PoolId, index: u32) -> Option<T::CurrencyId> {
		if let Some(pool) = Self::pools(pool_id) {
			let currency_ids = pool.get_currency_ids();
			if currency_ids.len() < index as usize {
				return Some(currency_ids[index as usize])
			}
		};
		None
	}

	pub fn get_lp_currency(pool_id: T::PoolId) -> Option<T::CurrencyId> {
		if let Some(pool) = Self::pools(pool_id) {
			return Some(pool.get_lp_currency())
		};
		None
	}

	pub fn get_currency_precision_multipliers(pool_id: T::PoolId) -> Vec<Balance> {
		if let Some(pool) = Self::pools(pool_id) {
			return pool.get_token_multipliers()
		};
		Vec::new()
	}

	pub fn get_currency_balances(pool_id: T::PoolId) -> Vec<Balance> {
		if let Some(pool) = Self::pools(pool_id) {
			return pool.get_balances()
		};
		Vec::new()
	}

	pub fn get_number_of_currencies(pool_id: T::PoolId) -> u32 {
		if let Some(pool) = Self::pools(pool_id) {
			return pool.get_currency_ids().len() as u32
		};
		0
	}

	pub fn get_admin_balances(pool_id: T::PoolId) -> Vec<Balance> {
		let mut balances = Vec::new();
		if let Some(pool) = Self::pools(pool_id) {
			for (i, _) in pool.get_currency_ids().iter().enumerate() {
				balances.push(Self::get_admin_balance(pool_id, i).unwrap_or_default());
			}
		};
		balances
	}
}
