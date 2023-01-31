// Copyright 2021-2022 Zenlink.
// Licensed under Apache 2.0.

use super::*;

impl<T: Config> Pallet<T> {
	pub(crate) fn xp(balances: &[Balance], rates: &[Balance]) -> Option<Vec<Balance>> {
		let mut normalized_res = Vec::new();
		for (i, _) in balances.iter().enumerate() {
			normalized_res.push(balances[i].checked_mul(rates[i])?)
		}
		Some(normalized_res)
	}

	pub(crate) fn get_d(balances: &[Balance], amp: Balance) -> Option<Balance> {
		let n_currencies = Balance::from(balances.len() as u64);
		let sum = Self::sum_of(balances)?;
		if sum == Balance::default() {
			return Some(Balance::default())
		}
		let mut d_prev: U256;
		let mut d = U256::from(sum);
		let ann = U256::from(amp.checked_mul(n_currencies)?);
		let a_precision = U256::from(A_PRECISION);

		for _i in 0..MAX_ITERATION {
			let mut d_p = d;
			for b in balances.iter() {
				d_p = d_p
					.checked_mul(d)?
					.checked_div(U256::from(*b).checked_mul(U256::from(n_currencies))?)?;
			}
			d_prev = d;

			let numerator = ann
				.checked_mul(U256::from(sum))
				.and_then(|n| n.checked_div(a_precision))
				.and_then(|n| n.checked_add(d_p.checked_mul(U256::from(n_currencies))?))
				.and_then(|n| n.checked_mul(d))?;

			let denominator = ann
				.checked_sub(a_precision)
				.and_then(|n| n.checked_mul(d))
				.and_then(|n| n.checked_div(a_precision))
				.and_then(|n| {
					n.checked_add(
						U256::from(n_currencies).checked_add(U256::from(1u32))?.checked_mul(d_p)?,
					)
				})?;

			d = numerator.checked_div(denominator)?;

			if Self::distance::<U256>(d, d_prev) <= U256::from(1u32) {
				return TryInto::<Balance>::try_into(d).ok()
			}
		}
		None
	}

	pub(crate) fn get_y(
		pool: &BasePool<T::CurrencyId, T::AccountId, BoundedVec<u8, T::PoolCurrencySymbolLimit>>,
		in_index: usize,
		out_index: usize,
		in_balance: Balance,
		normalized_balances: &[Balance],
	) -> Option<Balance> {
		let pool_currencies_len = pool.currency_ids.len();
		let n_currencies = U256::from(pool_currencies_len as u64);
		let amp = Self::get_a_precise(pool)?;
		let ann = n_currencies.checked_mul(U256::from(amp))?;
		let d = U256::from(Self::get_d(normalized_balances, amp)?);
		let mut c = d;
		let mut sum = U256::default();

		for (i, normalized_balance) in
			normalized_balances.iter().enumerate().take(pool_currencies_len)
		{
			if i == out_index {
				continue
			}
			let x: Balance = if i == in_index { in_balance } else { *normalized_balance };

			sum = sum.checked_add(U256::from(x))?;

			c = c.checked_mul(d)?.checked_div(U256::from(x).checked_mul(n_currencies)?)?;
		}
		let a_percision = U256::from(A_PRECISION);
		c = c
			.checked_mul(d)?
			.checked_mul(a_percision)?
			.checked_div(ann.checked_mul(n_currencies)?)?;

		let b = sum.checked_add(d.checked_mul(a_percision)?.checked_div(ann)?)?;

		let mut last_y: U256;
		let mut y = d;
		for _i in 0..MAX_ITERATION {
			last_y = y;
			y = y
				.checked_mul(y)?
				.checked_add(c)?
				.checked_div(U256::from(2u32).checked_mul(y)?.checked_add(b)?.checked_sub(d)?)?;
			if Self::distance(last_y, y) <= U256::from(1) {
				return TryInto::<Balance>::try_into(y).ok()
			}
		}

		None
	}

	pub(crate) fn get_yd(
		pool: &BasePool<T::CurrencyId, T::AccountId, BoundedVec<u8, T::PoolCurrencySymbolLimit>>,
		a: Balance,
		index: u32,
		xp: &[Balance],
		d: Balance,
	) -> Option<Balance> {
		let currencies_len = pool.currency_ids.len();
		if index >= currencies_len as u32 {
			return None
		}

		let n_currencies = U256::from(currencies_len as u64);
		let ann = U256::from(a) * n_currencies;
		let mut c = U256::from(d);
		let mut s = U256::default();
		let _x: U256;
		let mut y_prev: U256;

		for (i, x) in xp.iter().enumerate() {
			if i as u32 == index {
				continue
			}
			s = s.checked_add(U256::from(*x))?;
			c = c
				.checked_mul(U256::from(d))?
				.checked_div(U256::from(*x).checked_mul(n_currencies)?)?;
		}

		let a_precision = U256::from(A_PRECISION);
		c = c
			.checked_mul(U256::from(d))?
			.checked_mul(a_precision)?
			.checked_div(ann.checked_mul(n_currencies)?)?;
		let b = s.checked_add(U256::from(d).checked_mul(a_precision)?.checked_div(ann)?)?;
		let mut y = U256::from(d);

		for _i in 0..MAX_ITERATION {
			y_prev = y;
			y = y.checked_mul(y)?.checked_add(c)?.checked_div(
				U256::from(2u32).checked_mul(y)?.checked_add(b)?.checked_sub(U256::from(d))?,
			)?;

			if Self::distance(y, y_prev) <= U256::from(1) {
				return TryInto::<Balance>::try_into(y).ok()
			}
		}

		None
	}

	pub(crate) fn sum_of(balances: &[Balance]) -> Option<Balance> {
		let mut sum = Balance::default();
		for b in balances.iter() {
			sum = sum.checked_add(*b)?
		}
		Some(sum)
	}

	pub(crate) fn do_transfer_in(
		currency_id: T::CurrencyId,
		from: &T::AccountId,
		to: &T::AccountId,
		amount: Balance,
	) -> Result<Balance, Error<T>> {
		let to_prior_balance = T::MultiCurrency::free_balance(currency_id, to);
		T::MultiCurrency::transfer(currency_id, from, to, amount)
			.map_err(|_| Error::<T>::InsufficientReserve)?;
		let to_new_balance = T::MultiCurrency::free_balance(currency_id, to);

		to_new_balance.checked_sub(to_prior_balance).ok_or(Error::<T>::Arithmetic)
	}

	pub(crate) fn get_a_precise(
		pool: &BasePool<T::CurrencyId, T::AccountId, BoundedVec<u8, T::PoolCurrencySymbolLimit>>,
	) -> Option<Number> {
		let now = T::TimeProvider::now().as_secs() as Number;

		if now >= pool.future_a_time {
			return Some(pool.future_a)
		}

		let future_a = U256::from(pool.future_a);
		let initial_a = U256::from(pool.initial_a);
		let now = U256::from(now);
		let future_a_time = U256::from(pool.future_a_time);
		let initial_a_time = U256::from(pool.initial_a_time);

		if pool.future_a > pool.initial_a {
			return initial_a
				.checked_add(
					future_a
						.checked_sub(initial_a)?
						.checked_mul(now.checked_sub(initial_a_time)?)?
						.checked_div(future_a_time.checked_sub(initial_a_time)?)?,
				)
				.and_then(|n| TryInto::<Balance>::try_into(n).ok())
		}

		initial_a
			.checked_sub(
				initial_a
					.checked_sub(future_a)?
					.checked_mul(now.checked_sub(initial_a_time)?)?
					.checked_div(future_a_time.checked_sub(initial_a_time)?)?,
			)
			.and_then(|n| TryInto::<Balance>::try_into(n).ok())
	}

	pub(crate) fn get_pool_virtual_price(
		pool: &Pool<
			T::PoolId,
			T::CurrencyId,
			T::AccountId,
			BoundedVec<u8, T::PoolCurrencySymbolLimit>,
		>,
	) -> Option<Balance> {
		match pool {
			Pool::Base(bp) => Self::calculate_base_virtual_price(bp),
			Pool::Meta(mp) => Self::calculate_meta_virtual_price(mp),
		}
	}

	pub(crate) fn calculate_fee_per_token(
		pool: &BasePool<T::CurrencyId, T::AccountId, BoundedVec<u8, T::PoolCurrencySymbolLimit>>,
	) -> Option<Balance> {
		let n_pooled_currency = Balance::from(pool.currency_ids.len() as u64);

		pool.fee.checked_mul(n_pooled_currency)?.checked_div(
			Balance::from(4u32).checked_mul(n_pooled_currency.checked_sub(One::one())?)?,
		)
	}

	pub(crate) fn distance<Number: PartialOrd + Sub<Output = Number>>(
		x: Number,
		y: Number,
	) -> Number {
		if x > y {
			x - y
		} else {
			y - x
		}
	}
}
