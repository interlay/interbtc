// Copyright 2021-2022 Zenlink.
// Licensed under Apache 2.0.

use super::*;

impl<T: Config> Pallet<T> {
	pub(crate) fn inner_create_base_pool(
		currency_ids: &[T::CurrencyId],
		currency_decimals: Vec<u32>,
		a: Number,
		fee: Number,
		admin_fee: Number,
		admin_fee_receiver: &T::AccountId,
		lp_currency_symbol: Vec<u8>,
	) -> Result<
		(
			BasePool<T::CurrencyId, T::AccountId, BoundedVec<u8, T::PoolCurrencySymbolLimit>>,
			T::PoolId,
		),
		DispatchError,
	> {
		ensure!(
			T::EnsurePoolAsset::validate_pooled_currency(currency_ids),
			Error::<T>::InvalidPooledCurrency
		);

		ensure!(currency_ids.len() == currency_decimals.len(), Error::<T>::MismatchParameter);
		ensure!(a < MAX_A, Error::<T>::ExceedMaxA);
		ensure!(fee <= MAX_SWAP_FEE, Error::<T>::ExceedMaxFee);
		ensure!(admin_fee <= MAX_ADMIN_FEE, Error::<T>::ExceedMaxAdminFee);

		let mut rate = Vec::new();

		for (i, _) in currency_ids.iter().enumerate() {
			ensure!(
				currency_decimals[i] <= POOL_TOKEN_COMMON_DECIMALS,
				Error::<T>::InvalidCurrencyDecimal
			);
			let r = checked_pow(
				Balance::from(10u32),
				(POOL_TOKEN_COMMON_DECIMALS - currency_decimals[i]) as usize,
			)
			.ok_or(Error::<T>::Arithmetic)?;
			rate.push(r)
		}

		let pool_id = Self::next_pool_id();
		let lp_currency_id = T::LpGenerate::generate_by_pool_id(pool_id);

		ensure!(Self::lp_currencies(lp_currency_id).is_none(), Error::<T>::LpCurrencyAlreadyUsed);

		let account = T::PalletId::get().into_sub_account_truncating(pool_id);
		frame_system::Pallet::<T>::inc_providers(&account);
		let a_with_precision = a.checked_mul(A_PRECISION).ok_or(Error::<T>::Arithmetic)?;

		let symbol: BoundedVec<u8, T::PoolCurrencySymbolLimit> =
			lp_currency_symbol.try_into().map_err(|_| Error::<T>::BadPoolCurrencySymbol)?;

		Ok((
			BasePool {
				currency_ids: currency_ids.to_vec(),
				lp_currency_id,
				token_multipliers: rate,
				balances: vec![Zero::zero(); currency_ids.len()],
				fee,
				admin_fee,
				initial_a: a_with_precision,
				future_a: a_with_precision,
				initial_a_time: Zero::zero(),
				future_a_time: Zero::zero(),
				account,
				admin_fee_receiver: admin_fee_receiver.clone(),
				lp_currency_symbol: symbol,
				lp_currency_decimal: POOL_LP_CURRENCY_ID_DECIMAL,
			},
			pool_id,
		))
	}

	pub(crate) fn base_pool_add_liquidity(
		who: &T::AccountId,
		pool_id: T::PoolId,
		pool: &mut BasePool<
			T::CurrencyId,
			T::AccountId,
			BoundedVec<u8, T::PoolCurrencySymbolLimit>,
		>,
		amounts: &[Balance],
		min_mint_amount: Balance,
		to: &T::AccountId,
	) -> Result<Balance, DispatchError> {
		let n_currencies = pool.currency_ids.len();
		ensure!(n_currencies == amounts.len(), Error::<T>::MismatchParameter);
		let mut fees = Vec::new();
		let fee_per_token = Self::calculate_fee_per_token(pool).ok_or(Error::<T>::Arithmetic)?;

		let lp_total_supply = T::MultiCurrency::total_issuance(pool.lp_currency_id);

		let mut d0 = Balance::default();
		let amp = Self::get_a_precise(pool).ok_or(Error::<T>::Arithmetic)?;
		if lp_total_supply > Zero::zero() {
			d0 = Self::get_d(
				&Self::xp(&pool.balances, &pool.token_multipliers).ok_or(Error::<T>::Arithmetic)?,
				amp,
			)
			.ok_or(Error::<T>::Arithmetic)?;
		}

		let mut new_balances = pool.balances.clone();

		for i in 0..n_currencies {
			if lp_total_supply == Zero::zero() {
				ensure!(!amounts[i].is_zero(), Error::<T>::RequireAllCurrencies);
			}
			new_balances[i] = new_balances[i]
				.checked_add(Self::do_transfer_in(
					pool.currency_ids[i],
					who,
					&pool.account,
					amounts[i],
				)?)
				.ok_or(Error::<T>::Arithmetic)?;
		}

		let mut d1 = Self::get_d(
			&Self::xp(&new_balances, &pool.token_multipliers).ok_or(Error::<T>::Arithmetic)?,
			amp,
		)
		.ok_or(Error::<T>::Arithmetic)?;

		ensure!(d1 > d0, Error::<T>::CheckDFailed);

		let mint_amount: Balance;
		if lp_total_supply.is_zero() {
			pool.balances = new_balances;
			mint_amount = d1;
		} else {
			(mint_amount, fees) = Self::calculate_base_mint_amount(
				pool,
				&mut new_balances,
				d0,
				&mut d1,
				fee_per_token,
				amp,
				lp_total_supply,
			)
			.ok_or(Error::<T>::Arithmetic)?;
		}

		ensure!(min_mint_amount <= mint_amount, Error::<T>::AmountSlippage);

		T::MultiCurrency::deposit(pool.lp_currency_id, to, mint_amount)?;

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

	pub(crate) fn base_pool_swap(
		who: &T::AccountId,
		pool_id: T::PoolId,
		pool: &mut BasePool<
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
		let n_currencies = pool.currency_ids.len();
		ensure!(i < n_currencies && j < n_currencies, Error::<T>::CurrencyIndexOutRange);

		let in_amount = Self::do_transfer_in(pool.currency_ids[i], who, &pool.account, in_amount)?;

		let normalized_balances =
			Self::xp(&pool.balances, &pool.token_multipliers).ok_or(Error::<T>::Arithmetic)?;

		let x = in_amount
			.checked_mul(pool.token_multipliers[i])
			.and_then(|n| n.checked_add(normalized_balances[i]))
			.ok_or(Error::<T>::Arithmetic)?;

		let y = Self::get_y(pool, i, j, x, &normalized_balances).ok_or(Error::<T>::Arithmetic)?;

		let mut dy = normalized_balances[j]
			.checked_sub(y)
			.and_then(|n| n.checked_sub(One::one()))
			.ok_or(Error::<T>::Arithmetic)?;

		let dy_fee = U256::from(dy)
			.checked_mul(U256::from(pool.fee))
			.and_then(|n| n.checked_div(U256::from(FEE_DENOMINATOR)))
			.and_then(|n| TryInto::<Balance>::try_into(n).ok())
			.ok_or(Error::<T>::Arithmetic)?;

		dy = dy
			.checked_sub(dy_fee)
			.and_then(|n| n.checked_div(pool.token_multipliers[j]))
			.ok_or(Error::<T>::Arithmetic)?;

		ensure!(dy >= out_min_amount, Error::<T>::AmountSlippage);

		let admin_fee = U256::from(dy_fee)
			.checked_mul(U256::from(pool.admin_fee))
			.and_then(|n| n.checked_div(U256::from(FEE_DENOMINATOR)))
			.and_then(|n| n.checked_div(U256::from(pool.token_multipliers[j])))
			.and_then(|n| TryInto::<Balance>::try_into(n).ok())
			.ok_or(Error::<T>::Arithmetic)?;

		//update pool balance
		pool.balances[i] = pool.balances[i].checked_add(in_amount).ok_or(Error::<T>::Arithmetic)?;
		pool.balances[j] = pool.balances[j]
			.checked_sub(dy)
			.and_then(|n| n.checked_sub(admin_fee))
			.ok_or(Error::<T>::Arithmetic)?;

		T::MultiCurrency::transfer(pool.currency_ids[j], &pool.account, to, dy)
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

	pub(crate) fn base_pool_remove_liquidity_one_currency(
		pool_id: T::PoolId,
		pool: &mut BasePool<
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
		let total_supply = T::MultiCurrency::total_issuance(pool.lp_currency_id);
		ensure!(total_supply > Zero::zero(), Error::<T>::InsufficientLpReserve);
		ensure!(
			T::MultiCurrency::free_balance(pool.lp_currency_id, who) >= lp_amount &&
				lp_amount <= total_supply,
			Error::<T>::InsufficientSupply
		);
		ensure!(index < pool.currency_ids.len() as u32, Error::<T>::CurrencyIndexOutRange);

		let (dy, dy_fee) = Self::calculate_base_remove_liquidity_one_token(pool, lp_amount, index)
			.ok_or(Error::<T>::Arithmetic)?;

		ensure!(dy >= min_amount, Error::<T>::AmountSlippage);
		let fee_denominator = U256::from(FEE_DENOMINATOR);

		pool.balances[index as usize] = U256::from(dy_fee)
			.checked_mul(U256::from(pool.admin_fee))
			.and_then(|n| n.checked_div(fee_denominator))
			.and_then(|n| n.checked_add(U256::from(dy)))
			.and_then(|n| TryInto::<Balance>::try_into(n).ok())
			.and_then(|n| pool.balances[index as usize].checked_sub(n))
			.ok_or(Error::<T>::Arithmetic)?;

		T::MultiCurrency::withdraw(pool.lp_currency_id, who, lp_amount)?;
		T::MultiCurrency::transfer(pool.currency_ids[index as usize], &pool.account, to, dy)?;

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

	pub(crate) fn base_pool_remove_liquidity_imbalance(
		who: &T::AccountId,
		pool_id: T::PoolId,
		pool: &mut BasePool<
			T::CurrencyId,
			T::AccountId,
			BoundedVec<u8, T::PoolCurrencySymbolLimit>,
		>,
		amounts: &[Balance],
		max_burn_amount: Balance,
		to: &T::AccountId,
	) -> DispatchResult {
		let total_supply = T::MultiCurrency::total_issuance(pool.lp_currency_id);

		ensure!(total_supply > Zero::zero(), Error::<T>::InsufficientLpReserve);
		ensure!(amounts.len() == pool.currency_ids.len(), Error::<T>::MismatchParameter);

		let (mut burn_amount, fees, d1) =
			Self::calculate_base_remove_liquidity_imbalance(pool, amounts, total_supply)
				.ok_or(Error::<T>::Arithmetic)?;
		ensure!(burn_amount > Zero::zero(), Error::<T>::AmountSlippage);

		burn_amount = burn_amount.checked_add(One::one()).ok_or(Error::<T>::Arithmetic)?;

		ensure!(burn_amount <= max_burn_amount, Error::<T>::AmountSlippage);

		T::MultiCurrency::withdraw(pool.lp_currency_id, who, burn_amount)?;

		for (i, balance) in amounts.iter().enumerate() {
			if *balance > Zero::zero() {
				T::MultiCurrency::transfer(pool.currency_ids[i], &pool.account, to, *balance)?;
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

	pub(crate) fn calculate_base_virtual_price(
		pool: &BasePool<T::CurrencyId, T::AccountId, BoundedVec<u8, T::PoolCurrencySymbolLimit>>,
	) -> Option<Balance> {
		let d = Self::get_d(
			&Self::xp(&pool.balances, &pool.token_multipliers)?,
			Self::get_a_precise(pool)?,
		)?;

		let total_supply = T::MultiCurrency::total_issuance(pool.lp_currency_id);

		if total_supply > Zero::zero() {
			return U256::from(10)
				.checked_pow(U256::from(POOL_TOKEN_COMMON_DECIMALS))
				.and_then(|n| n.checked_mul(U256::from(d)))
				.and_then(|n| n.checked_div(U256::from(total_supply)))
				.and_then(|n| TryInto::<Balance>::try_into(n).ok())
		}
		None
	}

	pub(crate) fn calculate_base_remove_liquidity_imbalance(
		pool: &mut BasePool<
			T::CurrencyId,
			T::AccountId,
			BoundedVec<u8, T::PoolCurrencySymbolLimit>,
		>,
		amounts: &[Balance],
		total_supply: Balance,
	) -> Option<(Balance, Vec<Balance>, Balance)> {
		let currencies_len = pool.currency_ids.len();
		let fee_per_token = U256::from(Self::calculate_fee_per_token(pool)?);
		let amp = Self::get_a_precise(pool)?;

		let mut new_balances = pool.balances.clone();
		let d0 = U256::from(Self::get_d(&Self::xp(&pool.balances, &pool.token_multipliers)?, amp)?);

		for (i, x) in amounts.iter().enumerate() {
			new_balances[i] = new_balances[i].checked_sub(*x)?;
		}

		let d1 = U256::from(Self::get_d(&Self::xp(&new_balances, &pool.token_multipliers)?, amp)?);
		let mut fees = vec![Balance::default(); currencies_len];
		let fee_denominator = U256::from(FEE_DENOMINATOR);

		for (i, balance) in pool.balances.iter_mut().enumerate() {
			let ideal_balance = d1.checked_mul(U256::from(*balance))?.checked_div(d0)?;
			let diff = Self::distance(U256::from(new_balances[i]), ideal_balance);
			fees[i] = fee_per_token
				.checked_mul(diff)?
				.checked_div(fee_denominator)
				.and_then(|n| TryInto::<Balance>::try_into(n).ok())?;

			*balance = U256::from(new_balances[i])
				.checked_sub(
					U256::from(fees[i])
						.checked_mul(U256::from(pool.admin_fee))?
						.checked_div(fee_denominator)?,
				)
				.and_then(|n| TryInto::<Balance>::try_into(n).ok())?;

			new_balances[i] = new_balances[i].checked_sub(fees[i])?;
		}

		let d1 = Self::get_d(&Self::xp(&new_balances, &pool.token_multipliers)?, amp)?;
		let burn_amount = d0
			.checked_sub(U256::from(d1))?
			.checked_mul(U256::from(total_supply))?
			.checked_div(d0)
			.and_then(|n| TryInto::<Balance>::try_into(n).ok())?;

		Some((burn_amount, fees, d1))
	}

	pub(crate) fn calculate_base_remove_liquidity_one_token(
		pool: &BasePool<T::CurrencyId, T::AccountId, BoundedVec<u8, T::PoolCurrencySymbolLimit>>,
		token_amount: Balance,
		index: u32,
	) -> Option<(Balance, Balance)> {
		if index >= pool.currency_ids.len() as u32 {
			return None
		}
		let total_supply = T::MultiCurrency::total_issuance(pool.lp_currency_id);

		let amp = Self::get_a_precise(pool)?;
		let xp = Self::xp(&pool.balances, &pool.token_multipliers)?;
		let d0 = Self::get_d(&xp, amp)?;

		let d1 = U256::from(d0)
			.checked_sub(
				U256::from(token_amount)
					.checked_mul(U256::from(d0))?
					.checked_div(U256::from(total_supply))?,
			)
			.and_then(|n| TryInto::<Balance>::try_into(n).ok())?;

		let new_y = Self::get_yd(pool, amp, index, &xp, d1)?;

		let fee_per_token = U256::from(Self::calculate_fee_per_token(pool)?);
		let fee_denominator = U256::from(FEE_DENOMINATOR);

		let mut xp_reduced = vec![Zero::zero(); xp.len()];
		for (i, x) in xp.iter().enumerate() {
			let expected_dx = if i as u32 == index {
				U256::from(*x)
					.checked_mul(U256::from(d1))?
					.checked_div(U256::from(d0))?
					.checked_sub(U256::from(new_y))?
			} else {
				U256::from(*x).checked_sub(
					U256::from(*x).checked_mul(U256::from(d1))?.checked_div(U256::from(d0))?,
				)?
			};
			xp_reduced[i] = xp[i].checked_sub(
				fee_per_token
					.checked_mul(expected_dx)?
					.checked_div(fee_denominator)
					.and_then(|n| TryInto::<Balance>::try_into(n).ok())?,
			)?;
		}

		let mut dy = xp_reduced[index as usize].checked_sub(Self::get_yd(
			pool,
			amp,
			index,
			&xp_reduced,
			d1,
		)?)?;
		dy = dy
			.checked_sub(One::one())?
			.checked_div(pool.token_multipliers[index as usize])?;

		let fee = xp[index as usize]
			.checked_sub(new_y)?
			.checked_div(pool.token_multipliers[index as usize])?
			.checked_sub(dy)?;

		Some((dy, fee))
	}

	pub(crate) fn calculate_base_swap_amount(
		pool: &BasePool<T::CurrencyId, T::AccountId, BoundedVec<u8, T::PoolCurrencySymbolLimit>>,
		i: usize,
		j: usize,
		in_balance: Balance,
	) -> Option<Balance> {
		let n_currencies = pool.currency_ids.len();
		if i == j || i >= n_currencies || j >= n_currencies {
			return None
		}

		let normalized_balances = Self::xp(&pool.balances, &pool.token_multipliers)?;
		let new_in_balance = normalized_balances[i]
			.checked_add(in_balance.checked_mul(pool.token_multipliers[i])?)?;

		let out_balance = Self::get_y(pool, i, j, new_in_balance, &normalized_balances)?;
		let mut out_amount = normalized_balances[j]
			.checked_sub(out_balance)?
			.checked_sub(One::one())?
			.checked_div(pool.token_multipliers[j])?;

		let fee = U256::from(out_amount)
			.checked_mul(U256::from(pool.fee))?
			.checked_div(U256::from(FEE_DENOMINATOR))
			.and_then(|n| TryInto::<Balance>::try_into(n).ok())?;

		out_amount = out_amount.checked_sub(fee).and_then(|n| n.checked_sub(One::one()))?;

		Some(out_amount)
	}

	pub(crate) fn calculate_base_mint_amount(
		pool: &mut BasePool<
			T::CurrencyId,
			T::AccountId,
			BoundedVec<u8, T::PoolCurrencySymbolLimit>,
		>,
		new_balances: &mut [Balance],
		d0: Balance,
		d1: &mut Balance,
		fee: Balance,
		amp: Balance,
		total_supply: Balance,
	) -> Option<(Balance, Vec<Balance>)> {
		let mut diff: U256;
		let n_currencies = pool.currency_ids.len();
		let fee_denominator = U256::from(FEE_DENOMINATOR);
		let mut fees = vec![Zero::zero(); n_currencies];

		for i in 0..n_currencies {
			diff = Self::distance(
				U256::from(*d1)
					.checked_mul(U256::from(pool.balances[i]))
					.and_then(|n| n.checked_div(U256::from(d0)))?,
				U256::from(new_balances[i]),
			);

			fees[i] = U256::from(fee)
				.checked_mul(diff)
				.and_then(|n| n.checked_div(fee_denominator))
				.and_then(|n| TryInto::<Balance>::try_into(n).ok())?;

			pool.balances[i] = new_balances[i].checked_sub(
				U256::from(fees[i])
					.checked_mul(U256::from(pool.admin_fee))
					.and_then(|n| n.checked_div(fee_denominator))
					.and_then(|n| TryInto::<Balance>::try_into(n).ok())?,
			)?;

			new_balances[i] = new_balances[i].checked_sub(fees[i])?;
		}
		*d1 = Self::get_d(&Self::xp(new_balances, &pool.token_multipliers)?, amp)?;

		let mint_amount = U256::from(total_supply)
			.checked_mul(U256::from(*d1).checked_sub(U256::from(d0))?)?
			.checked_div(U256::from(d0))
			.and_then(|n| TryInto::<Balance>::try_into(n).ok())?;

		Some((mint_amount, fees))
	}

	pub(crate) fn calculate_base_remove_liquidity(
		pool: &BasePool<T::CurrencyId, T::AccountId, BoundedVec<u8, T::PoolCurrencySymbolLimit>>,
		amount: Balance,
	) -> Option<Vec<Balance>> {
		let lp_total_supply = T::MultiCurrency::total_issuance(pool.lp_currency_id);
		if lp_total_supply < amount {
			return None
		}
		let mut amounts = Vec::new();
		for b in pool.balances.iter() {
			amounts.push(
				U256::from(*b)
					.checked_mul(U256::from(amount))?
					.checked_div(U256::from(lp_total_supply))
					.and_then(|n| TryInto::<Balance>::try_into(n).ok())?,
			);
		}
		Some(amounts)
	}

	pub(crate) fn calculate_base_currency_amount(
		pool: &BasePool<T::CurrencyId, T::AccountId, BoundedVec<u8, T::PoolCurrencySymbolLimit>>,
		amounts: Vec<Balance>,
		deposit: bool,
	) -> Result<Balance, DispatchError> {
		ensure!(pool.currency_ids.len() == amounts.len(), Error::<T>::MismatchParameter);
		let amp = Self::get_a_precise(pool).ok_or(Error::<T>::Arithmetic)?;

		let d0 = Self::xp(&pool.balances, &pool.token_multipliers)
			.and_then(|xp| Self::get_d(&xp, amp))
			.ok_or(Error::<T>::Arithmetic)?;

		let mut new_balances = pool.balances.clone();
		for (i, balance) in amounts.iter().enumerate() {
			if deposit {
				new_balances[i] =
					new_balances[i].checked_add(*balance).ok_or(Error::<T>::Arithmetic)?;
			} else {
				new_balances[i] =
					new_balances[i].checked_sub(*balance).ok_or(Error::<T>::Arithmetic)?;
			}
		}

		let d1 = Self::xp(&new_balances, &pool.token_multipliers)
			.and_then(|xp| Self::get_d(&xp, amp))
			.ok_or(Error::<T>::Arithmetic)?;

		let total_supply = T::MultiCurrency::total_issuance(pool.lp_currency_id);

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
}
