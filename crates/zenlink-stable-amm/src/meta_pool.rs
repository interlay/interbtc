// Copyright 2021-2022 Zenlink.
// Licensed under Apache 2.0.

use super::*;
use crate::traits::StableAmmApi;

impl<T: Config> Pallet<T> {
	pub(crate) fn meta_pool_add_liquidity(
		who: &T::AccountId,
		pool_id: T::PoolId,
		meta_pool: &mut MetaPool<
			T::PoolId,
			T::CurrencyId,
			T::AccountId,
			BoundedVec<u8, T::PoolCurrencySymbolLimit>,
		>,
		amounts: &[Balance],
		min_mint_amount: Balance,
		to: &T::AccountId,
	) -> Result<Balance, DispatchError> {
		let n_currencies = meta_pool.info.currency_ids.len();
		ensure!(n_currencies == amounts.len(), Error::<T>::MismatchParameter);
		let mut fees = vec![Zero::zero(); n_currencies];
		let mut d0 = Balance::default();
		let lp_total_supply = T::MultiCurrency::total_issuance(meta_pool.info.lp_currency_id);
		let amp = Self::get_a_precise(&meta_pool.info).ok_or(Error::<T>::Arithmetic)?;
		let base_virtual_price =
			Self::meta_pool_update_virtual_price(meta_pool).ok_or(Error::<T>::Arithmetic)?;

		if !lp_total_supply.is_zero() {
			let normalized_balances = Self::meta_pool_xp(
				&meta_pool.info.balances,
				&meta_pool.info.token_multipliers,
				base_virtual_price,
			)
			.ok_or(Error::<T>::Arithmetic)?;

			d0 = Self::get_d(&normalized_balances, amp).ok_or(Error::<T>::Arithmetic)?;
		}

		let mut new_balances = meta_pool.info.balances.clone();
		for (i, currency) in meta_pool.info.currency_ids.iter().enumerate() {
			ensure!(
				!lp_total_supply.is_zero() || amounts[i] > Zero::zero(),
				Error::<T>::RequireAllCurrencies
			);
			if !amounts[i].is_zero() {
				new_balances[i] = new_balances[i]
					.checked_add(Self::do_transfer_in(
						*currency,
						who,
						&meta_pool.info.account,
						amounts[i],
					)?)
					.ok_or(Error::<T>::Arithmetic)?;
			}
		}

		let normalized_balances = Self::meta_pool_xp(
			&new_balances,
			&meta_pool.info.token_multipliers,
			base_virtual_price,
		)
		.ok_or(Error::<T>::Arithmetic)?;

		let d1 = Self::get_d(&normalized_balances, amp).ok_or(Error::<T>::Arithmetic)?;
		ensure!(d1 > d0, Error::<T>::CheckDFailed);

		// updated to reflect fees and calculate the user's LP tokens
		let d2: Balance;
		let mint_amount: Balance;

		if !lp_total_supply.is_zero() {
			let fee_per_token =
				Self::calculate_fee_per_token(&meta_pool.info).ok_or(Error::<T>::Arithmetic)?;
			for i in 0..meta_pool.info.currency_ids.len() {
				let ideal_balance = U256::from(d1)
					.checked_mul(U256::from(meta_pool.info.balances[i]))
					.and_then(|n| n.checked_div(U256::from(d0)))
					.ok_or(Error::<T>::Arithmetic)?;

				fees[i] = U256::from(fee_per_token)
					.checked_mul(Self::distance(ideal_balance, U256::from(new_balances[i])))
					.and_then(|n| n.checked_div(U256::from(FEE_DENOMINATOR)))
					.and_then(|n| TryInto::<Balance>::try_into(n).ok())
					.ok_or(Error::<T>::Arithmetic)?;

				meta_pool.info.balances[i] = U256::from(fees[i])
					.checked_mul(U256::from(meta_pool.info.admin_fee))
					.and_then(|n| n.checked_div(U256::from(FEE_DENOMINATOR)))
					.and_then(|n| U256::from(new_balances[i]).checked_sub(n))
					.and_then(|n| TryInto::<Balance>::try_into(n).ok())
					.ok_or(Error::<T>::Arithmetic)?;

				new_balances[i] =
					new_balances[i].checked_sub(fees[i]).ok_or(Error::<T>::Arithmetic)?;
			}

			d2 = Self::get_d(
				&Self::meta_pool_xp(
					&new_balances,
					&meta_pool.info.token_multipliers,
					base_virtual_price,
				)
				.ok_or(Error::<T>::Arithmetic)?,
				amp,
			)
			.ok_or(Error::<T>::Arithmetic)?;

			mint_amount = U256::from(d2)
				.checked_sub(U256::from(d0))
				.and_then(|n| n.checked_mul(U256::from(lp_total_supply)))
				.and_then(|n| n.checked_div(U256::from(d0)))
				.and_then(|n| TryInto::<Balance>::try_into(n).ok())
				.ok_or(Error::<T>::Arithmetic)?;
		} else {
			meta_pool.info.balances = new_balances;
			mint_amount = d1;
		}

		ensure!(min_mint_amount <= mint_amount, Error::<T>::AmountSlippage);
		T::MultiCurrency::deposit(meta_pool.info.lp_currency_id, to, mint_amount)?;

		Self::deposit_event(Event::AddLiquidity {
			pool_id,
			who: who.clone(),
			to: to.clone(),
			supply_amounts: amounts.to_vec(),
			fees: fees.to_vec(),
			new_d: d1,
			mint_amount,
		});

		Ok(mint_amount)
	}

	pub(crate) fn meta_pool_swap(
		who: &T::AccountId,
		pool_id: T::PoolId,
		meta_pool: &mut MetaPool<
			T::PoolId,
			T::CurrencyId,
			T::AccountId,
			BoundedVec<u8, T::PoolCurrencySymbolLimit>,
		>,
		i: usize,
		j: usize,
		in_amount: Balance,
		out_min_amount: Balance,
		to: &T::AccountId,
	) -> Result<Balance, DispatchError> {
		let n_currencies = meta_pool.info.currency_ids.len();
		ensure!(i < n_currencies && j < n_currencies, Error::<T>::CurrencyIndexOutRange);

		let in_amount = Self::do_transfer_in(
			meta_pool.info.currency_ids[i],
			who,
			&meta_pool.info.account,
			in_amount,
		)?;

		let virtual_price =
			Self::meta_pool_update_virtual_price(meta_pool).ok_or(Error::<T>::Arithmetic)?;
		let (dy, dy_fee) =
			Self::calculate_meta_swap_amount(meta_pool, i, j, in_amount, virtual_price)
				.ok_or(Error::<T>::Arithmetic)?;

		ensure!(dy >= out_min_amount, Error::<T>::AmountSlippage);

		let admin_fee = U256::from(dy_fee)
			.checked_mul(U256::from(meta_pool.info.admin_fee))
			.and_then(|n| n.checked_div(U256::from(FEE_DENOMINATOR)))
			.and_then(|n| n.checked_div(U256::from(meta_pool.info.token_multipliers[j])))
			.and_then(|n| TryInto::<Balance>::try_into(n).ok())
			.ok_or(Error::<T>::Arithmetic)?;

		//update pool balance
		meta_pool.info.balances[i] = meta_pool.info.balances[i]
			.checked_add(in_amount)
			.ok_or(Error::<T>::Arithmetic)?;
		meta_pool.info.balances[j] = meta_pool.info.balances[j]
			.checked_sub(dy)
			.and_then(|n| n.checked_sub(admin_fee))
			.ok_or(Error::<T>::Arithmetic)?;

		T::MultiCurrency::transfer(meta_pool.info.currency_ids[j], &meta_pool.info.account, to, dy)
			.map_err(|_| Error::<T>::InsufficientReserve)?;

		Self::deposit_event(Event::CurrencyExchange {
			pool_id,
			who: who.clone(),
			to: to.clone(),
			in_index: i as u32,
			in_amount,
			out_index: j as u32,
			out_amount: dy,
		});
		Ok(dy)
	}

	pub(crate) fn meta_pool_remove_liquidity_one_currency(
		pool_id: T::PoolId,
		meta_pool: &mut MetaPool<
			T::PoolId,
			T::CurrencyId,
			T::AccountId,
			BoundedVec<u8, T::PoolCurrencySymbolLimit>,
		>,
		who: &T::AccountId,
		lp_amount: Balance,
		index: u32,
		min_amount: Balance,
		to: &T::AccountId,
	) -> Result<Balance, DispatchError> {
		let total_supply = T::MultiCurrency::total_issuance(meta_pool.info.lp_currency_id);
		ensure!(
			T::MultiCurrency::free_balance(meta_pool.info.lp_currency_id, who) >= lp_amount &&
				lp_amount <= total_supply,
			Error::<T>::InsufficientSupply
		);
		ensure!(
			index < meta_pool.info.currency_ids.len() as u32,
			Error::<T>::CurrencyIndexOutRange
		);

		Self::meta_pool_update_virtual_price(meta_pool).ok_or(Error::<T>::Arithmetic)?;

		let (dy, dy_fee) = Self::calculate_meta_remove_liquidity_one_currency(
			meta_pool,
			lp_amount,
			index as usize,
			total_supply,
		)
		.ok_or(Error::<T>::Arithmetic)?;

		ensure!(dy >= min_amount, Error::<T>::AmountSlippage);
		let fee_denominator = U256::from(FEE_DENOMINATOR);

		meta_pool.info.balances[index as usize] = U256::from(dy_fee)
			.checked_mul(U256::from(meta_pool.info.admin_fee))
			.and_then(|n| n.checked_div(fee_denominator))
			.and_then(|n| n.checked_add(U256::from(dy)))
			.and_then(|n| TryInto::<Balance>::try_into(n).ok())
			.and_then(|n| meta_pool.info.balances[index as usize].checked_sub(n))
			.ok_or(Error::<T>::Arithmetic)?;

		T::MultiCurrency::withdraw(meta_pool.info.lp_currency_id, who, lp_amount)?;
		T::MultiCurrency::transfer(
			meta_pool.info.currency_ids[index as usize],
			&meta_pool.info.account,
			to,
			dy,
		)?;

		Self::deposit_event(Event::RemoveLiquidityOneCurrency {
			pool_id,
			who: who.clone(),
			to: to.clone(),
			out_index: index,
			burn_amount: lp_amount,
			out_amount: dy,
		});
		Ok(dy)
	}

	pub(crate) fn meta_pool_remove_liquidity_imbalance(
		who: &T::AccountId,
		pool_id: T::PoolId,
		meta_pool: &mut MetaPool<
			T::PoolId,
			T::CurrencyId,
			T::AccountId,
			BoundedVec<u8, T::PoolCurrencySymbolLimit>,
		>,
		amounts: &[Balance],
		max_burn_amount: Balance,
		to: &T::AccountId,
	) -> DispatchResult {
		let total_supply = T::MultiCurrency::total_issuance(meta_pool.info.lp_currency_id);

		ensure!(total_supply > Zero::zero(), Error::<T>::InsufficientLpReserve);
		ensure!(amounts.len() == meta_pool.info.currency_ids.len(), Error::<T>::MismatchParameter);

		let base_virtual_price =
			Self::meta_pool_update_virtual_price(meta_pool).ok_or(Error::<T>::Arithmetic)?;

		let (mut burn_amount, fees, d1) = Self::calculate_meta_remove_liquidity_imbalance(
			meta_pool,
			amounts,
			total_supply,
			base_virtual_price,
		)
		.ok_or(Error::<T>::Arithmetic)?;

		ensure!(burn_amount > Zero::zero(), Error::<T>::AmountSlippage);

		burn_amount = burn_amount.checked_add(One::one()).ok_or(Error::<T>::Arithmetic)?;

		ensure!(burn_amount <= max_burn_amount, Error::<T>::AmountSlippage);
		T::MultiCurrency::withdraw(meta_pool.info.lp_currency_id, who, burn_amount)?;

		for (i, balance) in amounts.iter().enumerate() {
			if *balance > Zero::zero() {
				T::MultiCurrency::transfer(
					meta_pool.info.currency_ids[i],
					&meta_pool.info.account,
					to,
					*balance,
				)?;
			}
		}

		Self::deposit_event(Event::RemoveLiquidityImbalance {
			pool_id,
			who: who.clone(),
			to: to.clone(),
			amounts: amounts.to_vec(),
			fees,
			new_d: d1,
			new_total_supply: total_supply - burn_amount,
		});

		Ok(())
	}

	pub(crate) fn meta_pool_swap_underlying(
		meta_pool: &mut MetaPool<
			T::PoolId,
			T::CurrencyId,
			T::AccountId,
			BoundedVec<u8, T::PoolCurrencySymbolLimit>,
		>,
		pool_id: T::PoolId,
		who: &T::AccountId,
		to: &T::AccountId,
		in_amount: Balance,
		min_out_amount: Balance,
		currency_index_from: usize,
		currency_index_to: usize,
	) -> Result<Balance, DispatchError> {
		let base_lp_currency_index = meta_pool
			.info
			.currency_ids
			.len()
			.checked_sub(One::one())
			.ok_or(Error::<T>::Arithmetic)?;

		let base_virtual_price =
			Self::meta_pool_update_virtual_price(meta_pool).ok_or(Error::<T>::Arithmetic)?;

		let max_range = base_lp_currency_index + meta_pool.base_currencies.len();
		ensure!(
			currency_index_from != currency_index_to &&
				currency_index_from < max_range &&
				currency_index_to < max_range,
			Error::<T>::MismatchParameter
		);

		let currency_from: T::CurrencyId;
		let currency_to: T::CurrencyId;

		let meta_index_from: usize;
		let meta_index_to: usize;

		if currency_index_from < base_lp_currency_index {
			currency_from = meta_pool.info.currency_ids[currency_index_from];
			meta_index_from = currency_index_from;
		} else {
			currency_from = meta_pool.base_currencies[currency_index_from - base_lp_currency_index];
			meta_index_from = base_lp_currency_index;
		}

		if currency_index_to < base_lp_currency_index {
			currency_to = meta_pool.info.currency_ids[currency_index_to];
			meta_index_to = currency_index_to;
		} else {
			currency_to = meta_pool.base_currencies[currency_index_to - base_lp_currency_index];
			meta_index_to = base_lp_currency_index;
		}

		let mut dx = Self::do_transfer_in(currency_from, who, &meta_pool.info.account, in_amount)?;
		let mut dy: Balance;
		if currency_index_from < base_lp_currency_index ||
			currency_index_to < base_lp_currency_index
		{
			let old_balances = meta_pool.info.balances.clone();

			let xp = Self::meta_pool_xp(
				&old_balances,
				&meta_pool.info.token_multipliers,
				base_virtual_price,
			)
			.ok_or(Error::<T>::Arithmetic)?;
			let x: Balance;
			if currency_index_from < base_lp_currency_index {
				x = dx
					.checked_mul(meta_pool.info.token_multipliers[currency_index_from])
					.and_then(|n| n.checked_add(xp[currency_index_from]))
					.ok_or(Error::<T>::Arithmetic)?;
			} else {
				let mut base_amounts = vec![Balance::default(); meta_pool.base_currencies.len()];
				base_amounts[currency_index_from - base_lp_currency_index] = dx;
				dx = Self::inner_add_liquidity(
					&meta_pool.info.account,
					meta_pool.base_pool_id,
					&base_amounts,
					0,
					&meta_pool.info.account,
				)?;

				x = U256::from(dx)
					.checked_mul(U256::from(base_virtual_price))
					.and_then(|n| n.checked_div(U256::from(BASE_VIRTUAL_PRICE_PRECISION)))
					.and_then(|n| n.checked_add(U256::from(xp[base_lp_currency_index])))
					.and_then(|n| TryInto::<Balance>::try_into(n).ok())
					.ok_or(Error::<T>::Arithmetic)?;
			}

			let y = Self::get_y(&meta_pool.info, meta_index_from, meta_index_to, x, &xp)
				.ok_or(Error::<T>::Arithmetic)?;

			dy = xp[meta_index_to]
				.checked_sub(y)
				.and_then(|n| n.checked_sub(One::one()))
				.ok_or(Error::<T>::Arithmetic)?;

			if currency_index_to >= base_lp_currency_index {
				dy = U256::from(dy)
					.checked_mul(U256::from(BASE_VIRTUAL_PRICE_PRECISION))
					.and_then(|n| n.checked_div(U256::from(base_virtual_price)))
					.and_then(|n| TryInto::<Balance>::try_into(n).ok())
					.ok_or(Error::<T>::Arithmetic)?;
			}
			let dy_fee = U256::from(dy)
				.checked_mul(U256::from(meta_pool.info.fee))
				.and_then(|n| n.checked_div(U256::from(FEE_DENOMINATOR)))
				.and_then(|n| TryInto::<Balance>::try_into(n).ok())
				.ok_or(Error::<T>::Arithmetic)?;

			dy = dy
				.checked_sub(dy_fee)
				.and_then(|n| n.checked_div(meta_pool.info.token_multipliers[meta_index_to]))
				.ok_or(Error::<T>::Arithmetic)?;

			let mut dy_admin_fee = U256::from(dy_fee)
				.checked_mul(U256::from(meta_pool.info.admin_fee))
				.and_then(|n| n.checked_div(U256::from(FEE_DENOMINATOR)))
				.and_then(|n| TryInto::<Balance>::try_into(n).ok())
				.ok_or(Error::<T>::Arithmetic)?;

			dy_admin_fee = dy_admin_fee
				.checked_div(meta_pool.info.token_multipliers[meta_index_to])
				.ok_or(Error::<T>::Arithmetic)?;

			meta_pool.info.balances[meta_index_from] =
				old_balances[meta_index_from].checked_add(dx).ok_or(Error::<T>::Arithmetic)?;
			meta_pool.info.balances[meta_index_to] = old_balances[meta_index_to]
				.checked_sub(dy)
				.and_then(|n| n.checked_sub(dy_admin_fee))
				.ok_or(Error::<T>::Arithmetic)?;

			if currency_index_to >= base_lp_currency_index {
				let old_balance =
					T::MultiCurrency::free_balance(currency_to, &meta_pool.info.account);
				Self::inner_remove_liquidity_one_currency(
					meta_pool.base_pool_id,
					&meta_pool.info.account,
					dy,
					(currency_index_to - base_lp_currency_index) as u32,
					0,
					&meta_pool.info.account,
				)?;
				dy = T::MultiCurrency::free_balance(currency_to, &meta_pool.info.account)
					.checked_sub(old_balance)
					.ok_or(Error::<T>::Arithmetic)?;
			}

			ensure!(dy >= min_out_amount, Error::<T>::AmountSlippage);
		} else {
			// swap in base pool
			dy = T::MultiCurrency::free_balance(currency_to, &meta_pool.info.account);
			Self::inner_swap(
				&meta_pool.info.account,
				meta_pool.base_pool_id,
				currency_index_from - base_lp_currency_index,
				currency_index_to - base_lp_currency_index,
				dx,
				min_out_amount,
				to,
			)?;
			dy = T::MultiCurrency::free_balance(currency_to, &meta_pool.info.account)
				.checked_sub(dy)
				.ok_or(Error::<T>::Arithmetic)?;
		}

		T::MultiCurrency::transfer(currency_to, &meta_pool.info.account, to, dy)?;

		Self::deposit_event(Event::CurrencyExchangeUnderlying {
			pool_id,
			account: who.clone(),
			in_amount,
			out_amount: dy,
			currency_index_from: currency_index_from as u32,
			currency_index_to: currency_index_to as u32,
			to: to.clone(),
		});
		Ok(dy)
	}

	pub(crate) fn meta_pool_update_virtual_price(
		meta_pool: &mut MetaPool<
			T::PoolId,
			T::CurrencyId,
			T::AccountId,
			BoundedVec<u8, T::PoolCurrencySymbolLimit>,
		>,
	) -> Option<Balance> {
		let now = T::TimeProvider::now().as_secs();
		if now > (meta_pool.base_cache_last_updated + BASE_CACHE_EXPIRE_TIME) {
			let base_pool = Self::pools(meta_pool.base_pool_id)?;
			let base_virtual_price = Self::get_pool_virtual_price(&base_pool)?;

			meta_pool.base_virtual_price = base_virtual_price;
			meta_pool.base_cache_last_updated = now;

			Some(base_virtual_price)
		} else {
			Some(meta_pool.base_virtual_price)
		}
	}

	pub(crate) fn calculate_meta_virtual_price(
		meta_pool: &MetaPool<
			T::PoolId,
			T::CurrencyId,
			T::AccountId,
			BoundedVec<u8, T::PoolCurrencySymbolLimit>,
		>,
	) -> Option<Balance> {
		let base_virtual_price = Self::meta_pool_base_virtual_price(meta_pool)?;
		let normalized_balances = Self::meta_pool_xp(
			&meta_pool.info.balances,
			&meta_pool.info.token_multipliers,
			base_virtual_price,
		)?;
		let amp = Self::get_a_precise(&meta_pool.info)?;
		let d = Self::get_d(&normalized_balances, amp)?;
		let supply = T::MultiCurrency::total_issuance(meta_pool.info.lp_currency_id);
		if !supply.is_zero() {
			return U256::from(d)
				.checked_mul(U256::from(BASE_VIRTUAL_PRICE_PRECISION))
				.and_then(|n| n.checked_div(U256::from(supply)))
				.and_then(|n| TryInto::<Balance>::try_into(n).ok())
		}
		None
	}

	fn meta_pool_xp(
		balances: &[Balance],
		rates: &[Balance],
		base_virtual_price: Balance,
	) -> Option<Vec<Balance>> {
		let mut xp = Self::xp(balances, rates)?;
		let base_lp_token_index = balances.len().checked_sub(1)?;
		xp[base_lp_token_index] = U256::from(xp[base_lp_token_index])
			.checked_mul(U256::from(base_virtual_price))
			.and_then(|n| n.checked_div(U256::from(BASE_VIRTUAL_PRICE_PRECISION)))
			.and_then(|n| TryInto::<Balance>::try_into(n).ok())?;

		Some(xp)
	}

	fn meta_pool_base_virtual_price(
		meta_pool: &MetaPool<
			T::PoolId,
			T::CurrencyId,
			T::AccountId,
			BoundedVec<u8, T::PoolCurrencySymbolLimit>,
		>,
	) -> Option<Balance> {
		let now = T::TimeProvider::now().as_secs();
		if now > (meta_pool.base_cache_last_updated + BASE_CACHE_EXPIRE_TIME) {
			let pool = Self::pools(meta_pool.base_pool_id)?;
			return match pool {
				Pool::Base(bp) => Self::calculate_base_virtual_price(&bp),
				Pool::Meta(mp) => Self::calculate_meta_virtual_price(&mp),
			}
		}
		Some(meta_pool.base_virtual_price)
	}

	pub(crate) fn calculate_meta_remove_liquidity_one_currency(
		meta_pool: &MetaPool<
			T::PoolId,
			T::CurrencyId,
			T::AccountId,
			BoundedVec<u8, T::PoolCurrencySymbolLimit>,
		>,
		token_amount: Balance,
		index: usize,
		total_supply: Balance,
	) -> Option<(Balance, Balance)> {
		if index >= meta_pool.info.currency_ids.len() {
			return None
		}

		let base_virtual_price = Self::meta_pool_base_virtual_price(meta_pool)?;

		let mut xp = Self::meta_pool_xp(
			&meta_pool.info.balances,
			&meta_pool.info.token_multipliers,
			base_virtual_price,
		)?;
		let a_precise = Self::get_a_precise(&meta_pool.info)?;
		let d0 = Self::get_d(&xp, a_precise)?;
		let fee_per_token = Self::calculate_fee_per_token(&meta_pool.info)?;

		let d1 = U256::from(d0)
			.checked_sub(
				U256::from(token_amount)
					.checked_mul(U256::from(d0))?
					.checked_div(U256::from(total_supply))?,
			)
			.and_then(|n| TryInto::<Balance>::try_into(n).ok())?;

		let mut new_y = Self::get_yd(&meta_pool.info, a_precise, index as u32, &xp, d1)?;
		let mut xp_reduced = vec![Zero::zero(); xp.len()];

		for (i, xpi) in xp.iter().enumerate() {
			let u256_xpi = U256::from(*xpi);
			let dx_expected: U256 = if i == index {
				u256_xpi
					.checked_mul(U256::from(d1))?
					.checked_div(U256::from(d0))?
					.checked_sub(U256::from(new_y))?
			} else {
				u256_xpi
					.checked_mul(U256::from(d1))?
					.checked_div(U256::from(d0))
					.and_then(|n| u256_xpi.checked_sub(n))?
			};

			xp_reduced[i] = dx_expected
				.checked_mul(U256::from(fee_per_token))?
				.checked_div(U256::from(FEE_DENOMINATOR))
				.and_then(|n| u256_xpi.checked_sub(n))
				.and_then(|n| TryInto::<Balance>::try_into(n).ok())?;
		}

		let mut dy = xp_reduced[index].checked_sub(Self::get_yd(
			&meta_pool.info,
			a_precise,
			index as u32,
			&xp_reduced,
			d1,
		)?)?;

		if index == (xp.len() - 1) {
			dy = U256::from(dy)
				.checked_mul(U256::from(BASE_VIRTUAL_PRICE_PRECISION))?
				.checked_div(U256::from(base_virtual_price))
				.and_then(|n| TryInto::<Balance>::try_into(n).ok())?;

			new_y = U256::from(new_y)
				.checked_mul(U256::from(BASE_VIRTUAL_PRICE_PRECISION))?
				.checked_div(U256::from(base_virtual_price))
				.and_then(|n| TryInto::<Balance>::try_into(n).ok())?;

			xp[index] = U256::from(xp[index])
				.checked_mul(U256::from(BASE_VIRTUAL_PRICE_PRECISION))?
				.checked_div(U256::from(base_virtual_price))
				.and_then(|n| TryInto::<Balance>::try_into(n).ok())?;
		}

		dy = dy
			.checked_sub(One::one())?
			.checked_div(meta_pool.info.token_multipliers[index])?;

		let swap_fee = xp[index]
			.checked_sub(new_y)?
			.checked_div(meta_pool.info.token_multipliers[index])?
			.checked_sub(dy)?;

		Some((dy, swap_fee))
	}

	pub(crate) fn calculate_meta_currency_amount(
		meta_pool: &MetaPool<
			T::PoolId,
			T::CurrencyId,
			T::AccountId,
			BoundedVec<u8, T::PoolCurrencySymbolLimit>,
		>,
		amounts: Vec<Balance>,
		deposit: bool,
	) -> Result<Balance, DispatchError> {
		let base_virtual_price =
			Self::meta_pool_base_virtual_price(meta_pool).ok_or(Error::<T>::Arithmetic)?;
		let mut new_balances = meta_pool.info.balances.clone();
		let token_multipliers = meta_pool.info.token_multipliers.clone();

		let xp = Self::meta_pool_xp(&new_balances, &token_multipliers, base_virtual_price)
			.ok_or(Error::<T>::Arithmetic)?;
		let a = Self::get_a_precise(&meta_pool.info).ok_or(Error::<T>::Arithmetic)?;
		let d0 = Self::get_d(&xp, a).ok_or(Error::<T>::Arithmetic)?;

		for (i, balance) in amounts.iter().enumerate() {
			if deposit {
				new_balances[i] =
					new_balances[i].checked_add(*balance).ok_or(Error::<T>::Arithmetic)?;
			} else {
				new_balances[i] =
					new_balances[i].checked_sub(*balance).ok_or(Error::<T>::Arithmetic)?;
			}
		}

		let xp1 = Self::meta_pool_xp(&new_balances, &token_multipliers, base_virtual_price)
			.ok_or(Error::<T>::Arithmetic)?;
		let d1 = Self::get_d(&xp1, a).ok_or(Error::<T>::Arithmetic)?;

		let total_supply = T::MultiCurrency::total_issuance(meta_pool.info.lp_currency_id);

		if total_supply.is_zero() {
			return Ok(d1) // first depositor take it all
		}

		let diff: Balance = if deposit {
			d1.checked_sub(d0).ok_or(Error::<T>::Arithmetic)?
		} else {
			d0.checked_sub(d1).ok_or(Error::<T>::Arithmetic)?
		};

		let amount = U256::from(diff)
			.checked_mul(U256::from(total_supply))
			.and_then(|n| n.checked_div(U256::from(d0)))
			.and_then(|n| TryInto::<Balance>::try_into(n).ok())
			.ok_or(Error::<T>::Arithmetic)?;

		Ok(amount)
	}

	pub(crate) fn calculate_meta_remove_liquidity_imbalance(
		meta_pool: &mut MetaPool<
			T::PoolId,
			T::CurrencyId,
			T::AccountId,
			BoundedVec<u8, T::PoolCurrencySymbolLimit>,
		>,
		amounts: &[Balance],
		total_supply: Balance,
		base_virtual_price: Balance,
	) -> Option<(Balance, Vec<Balance>, Balance)> {
		let currencies_len = meta_pool.info.currency_ids.len();
		let fee_per_token = U256::from(Self::calculate_fee_per_token(&meta_pool.info)?);
		let amp = Self::get_a_precise(&meta_pool.info)?;
		let mut fees = vec![Balance::default(); currencies_len];
		let mut new_balances = meta_pool.info.balances.clone();
		let fee_denominator = U256::from(FEE_DENOMINATOR);

		let xp = Self::meta_pool_xp(
			&new_balances,
			&meta_pool.info.token_multipliers,
			base_virtual_price,
		)?;
		let d0 = U256::from(Self::get_d(&xp, amp)?);

		for (i, x) in amounts.iter().enumerate() {
			new_balances[i] = new_balances[i].checked_sub(*x)?;
		}

		let new_xp = Self::meta_pool_xp(
			&new_balances,
			&meta_pool.info.token_multipliers,
			base_virtual_price,
		)?;
		let d1 = U256::from(Self::get_d(&new_xp, amp)?);

		for (i, balance) in meta_pool.info.balances.iter_mut().enumerate() {
			let ideal_balance = d1.checked_mul(U256::from(*balance))?.checked_div(d0)?;
			let diff = Self::distance(U256::from(new_balances[i]), ideal_balance);
			fees[i] = fee_per_token
				.checked_mul(diff)?
				.checked_div(fee_denominator)
				.and_then(|n| TryInto::<Balance>::try_into(n).ok())?;

			*balance = U256::from(new_balances[i])
				.checked_sub(
					U256::from(fees[i])
						.checked_mul(U256::from(meta_pool.info.admin_fee))?
						.checked_div(fee_denominator)?,
				)
				.and_then(|n| TryInto::<Balance>::try_into(n).ok())?;

			new_balances[i] = new_balances[i].checked_sub(fees[i])?;
		}

		let d1 = Self::get_d(
			&Self::meta_pool_xp(
				&new_balances,
				&meta_pool.info.token_multipliers,
				base_virtual_price,
			)?,
			amp,
		)?;

		let burn_amount = d0
			.checked_sub(U256::from(d1))?
			.checked_mul(U256::from(total_supply))?
			.checked_div(d0)
			.and_then(|n| TryInto::<Balance>::try_into(n).ok())?;

		Some((burn_amount, fees, d1))
	}

	pub(crate) fn calculate_meta_swap_amount(
		meta_pool: &MetaPool<
			T::PoolId,
			T::CurrencyId,
			T::AccountId,
			BoundedVec<u8, T::PoolCurrencySymbolLimit>,
		>,
		i: usize,
		j: usize,
		in_amount: Balance,
		base_virtual_price: Balance,
	) -> Option<(Balance, Balance)> {
		let n_currencies = meta_pool.info.currency_ids.len();
		if i == j || i >= n_currencies || j >= n_currencies {
			return None
		}
		let xp = Self::meta_pool_xp(
			&meta_pool.info.balances,
			&meta_pool.info.token_multipliers,
			base_virtual_price,
		)?;
		let base_lp_currency_index = xp.len().checked_sub(One::one())?;
		let mut x = in_amount.checked_mul(meta_pool.info.token_multipliers[i])?;

		if i == base_lp_currency_index {
			x = U256::from(x)
				.checked_mul(U256::from(base_virtual_price))?
				.checked_div(U256::from(BASE_VIRTUAL_PRICE_PRECISION))
				.and_then(|n| TryInto::<Balance>::try_into(n).ok())?;
		}

		x = x.checked_add(xp[i])?;

		let y = Self::get_y(&meta_pool.info, i, j, x, &xp)?;

		let mut dy = xp[j].checked_sub(y)?.checked_sub(One::one())?;

		if j == base_lp_currency_index {
			dy = U256::from(dy)
				.checked_mul(U256::from(BASE_VIRTUAL_PRICE_PRECISION))?
				.checked_div(U256::from(base_virtual_price))
				.and_then(|n| TryInto::<Balance>::try_into(n).ok())?;
		}

		let fee = U256::from(dy)
			.checked_mul(U256::from(meta_pool.info.fee))?
			.checked_div(U256::from(FEE_DENOMINATOR))
			.and_then(|n| TryInto::<Balance>::try_into(n).ok())?;

		dy = dy.checked_sub(fee)?;

		dy = dy.checked_div(meta_pool.info.token_multipliers[j])?;

		Some((dy, fee))
	}

	pub fn calculate_meta_swap_underlying(
		pool_id: T::PoolId,
		in_amount: Balance,
		currency_index_from: usize,
		currency_index_to: usize,
	) -> Result<Balance, DispatchError> {
		let pool = Self::pools(pool_id).ok_or(Error::<T>::InvalidPoolId)?;
		let meta_pool = match pool {
			Pool::Base(_) => None,
			Pool::Meta(mp) => Some(mp),
		}
		.ok_or(Error::<T>::InvalidPoolId)?;

		let base_lp_currency_index = meta_pool
			.info
			.currency_ids
			.len()
			.checked_sub(One::one())
			.ok_or(Error::<T>::Arithmetic)?;

		let base_virtual_price =
			Self::calculate_meta_virtual_price(&meta_pool).ok_or(Error::<T>::Arithmetic)?;

		let base_pool_currency_len = meta_pool.base_currencies.len();

		let max_range = base_lp_currency_index + base_pool_currency_len;

		let mut currency_index_from = currency_index_from;
		let currency_index_to = currency_index_to;

		ensure!(
			currency_index_from != currency_index_to &&
				currency_index_from < max_range &&
				currency_index_to < max_range,
			Error::<T>::MismatchParameter
		);

		let xp = Self::meta_pool_xp(
			&meta_pool.info.balances,
			&meta_pool.info.token_multipliers,
			base_virtual_price,
		)
		.ok_or(Error::<T>::Arithmetic)?;

		let mut x: Balance;
		if currency_index_from < base_lp_currency_index {
			// from currency in meta pool
			x = in_amount
				.checked_mul(meta_pool.info.token_multipliers[currency_index_from])
				.and_then(|n| n.checked_add(xp[currency_index_from]))
				.ok_or(Error::<T>::Arithmetic)?;
		} else {
			currency_index_from -= base_lp_currency_index;
			if currency_index_to < base_lp_currency_index {
				// from currency in base pool and to currency in meta pool
				let mut base_inputs = vec![Zero::zero(); base_pool_currency_len];
				base_inputs[currency_index_from] = in_amount;
				x = U256::from(Self::calculate_currency_amount(
					meta_pool.base_pool_id,
					base_inputs,
					true,
				)?)
				.checked_mul(U256::from(base_virtual_price))
				.and_then(|n| n.checked_div(U256::from(BASE_VIRTUAL_PRICE_PRECISION)))
				.and_then(|n| TryInto::<Balance>::try_into(n).ok())
				.ok_or(Error::<T>::Arithmetic)?;

				// when adding to the base pool,you pay approx 50% of the swap fee
				let base_pool =
					Self::pools(meta_pool.base_pool_id).ok_or(Error::<T>::InvalidBasePool)?;
				let base_pool_fee = base_pool.get_fee();

				let x_u256 = U256::from(x);
				x = x_u256
					.checked_mul(U256::from(base_pool_fee))
					.and_then(|n| n.checked_div(U256::from(FEE_DENOMINATOR * 2)))
					.and_then(|n| x_u256.checked_sub(n))
					.and_then(|n| n.checked_add(U256::from(xp[base_lp_currency_index])))
					.and_then(|n| TryInto::<Balance>::try_into(n).ok())
					.ok_or(Error::<T>::Arithmetic)?;
			} else {
				// from currency in base pool and to currency in base pool
				return Self::stable_amm_calculate_swap_amount(
					meta_pool.base_pool_id,
					currency_index_from,
					currency_index_to - base_lp_currency_index,
					in_amount,
				)
				.ok_or_else(|| Error::<T>::Arithmetic.into())
			}
			currency_index_from = base_lp_currency_index;
		}

		let mut meta_index_to = base_lp_currency_index;
		if currency_index_to < base_lp_currency_index {
			meta_index_to = currency_index_to;
		}

		let y = Self::get_y(&meta_pool.info, currency_index_from, meta_index_to, x, &xp)
			.ok_or(Error::<T>::Arithmetic)?;

		let mut dy = xp[meta_index_to]
			.checked_sub(y)
			.and_then(|n| n.checked_sub(One::one()))
			.ok_or(Error::<T>::Arithmetic)?;

		let dy_fee = U256::from(dy)
			.checked_mul(U256::from(meta_pool.info.fee))
			.and_then(|n| n.checked_div(U256::from(FEE_DENOMINATOR)))
			.and_then(|n| TryInto::<Balance>::try_into(n).ok())
			.ok_or(Error::<T>::Arithmetic)?;

		dy = dy.checked_sub(dy_fee).ok_or(Error::<T>::Arithmetic)?;

		if currency_index_to < base_lp_currency_index {
			dy = dy
				.checked_div(meta_pool.info.token_multipliers[meta_index_to])
				.ok_or(Error::<T>::Arithmetic)?;
		} else {
			let amount = U256::from(dy)
				.checked_mul(U256::from(BASE_VIRTUAL_PRICE_PRECISION))
				.and_then(|n| n.checked_div(U256::from(base_virtual_price)))
				.and_then(|n| TryInto::<Balance>::try_into(n).ok())
				.ok_or(Error::<T>::Arithmetic)?;

			dy = Self::stable_amm_calculate_remove_liquidity_one_currency(
				meta_pool.base_pool_id,
				amount,
				(currency_index_to - base_lp_currency_index) as u32,
			)
			.ok_or(Error::<T>::Arithmetic)?;
		}

		Ok(dy)
	}
}
