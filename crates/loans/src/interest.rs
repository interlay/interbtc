// Copyright 2022 Interlay.
// This file is part of Interlay.

// Copyright 2021 Parallel Finance Developer.
// This file is part of Parallel Finance.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use primitives::{Timestamp, SECONDS_PER_YEAR};
use sp_runtime::{traits::Zero, DispatchResult};

use crate::*;

impl<T: Config> Pallet<T> {
    /// Accrue interest and update corresponding storage
    #[cfg_attr(any(test, feature = "integration-tests"), visibility::make(pub))]
    pub(crate) fn accrue_interest(asset_id: CurrencyId<T>) -> DispatchResult {
        let now = T::UnixTime::now().as_secs();
        let last_accrued_interest_time = Self::last_accrued_interest_time(asset_id);
        if last_accrued_interest_time.is_zero() {
            // For the initialization
            Self::update_last_accrued_interest_time(asset_id, now)?;
            return Ok(());
        }
        if now <= last_accrued_interest_time {
            return Ok(());
        }

        let (borrow_rate, supply_rate, exchange_rate, util, total_borrows_new, total_reserves_new, borrow_index_new) =
            Self::get_market_status(asset_id)?;

        Self::update_last_accrued_interest_time(asset_id, now)?;
        TotalBorrows::<T>::insert(asset_id, total_borrows_new);
        TotalReserves::<T>::insert(asset_id, total_reserves_new);
        BorrowIndex::<T>::insert(asset_id, borrow_index_new);

        //save redundant storage right now.
        UtilizationRatio::<T>::insert(asset_id, util);
        BorrowRate::<T>::insert(asset_id, borrow_rate);
        SupplyRate::<T>::insert(asset_id, supply_rate);
        ExchangeRate::<T>::insert(asset_id, exchange_rate);
        Self::on_exchange_rate_change(&asset_id);

        Self::deposit_event(Event::<T>::InterestAccrued {
            underlying_currency_id: asset_id,
            total_borrows: total_borrows_new,
            total_reserves: total_reserves_new,
            borrow_index: borrow_index_new,
            utilization_ratio: util,
            borrow_rate,
            supply_rate,
            exchange_rate,
        });

        Ok(())
    }

    pub fn get_market_status(
        asset_id: CurrencyId<T>,
    ) -> Result<(Rate, Rate, Rate, Ratio, BalanceOf<T>, BalanceOf<T>, FixedU128), DispatchError> {
        let market = Self::market(asset_id)?;
        let total_supply = Self::total_supply(asset_id)?;
        let total_cash = Self::get_total_cash(asset_id);
        let mut total_borrows = Self::total_borrows(asset_id);
        let mut total_reserves = Self::total_reserves(asset_id);
        let borrow_index = Self::borrow_index(asset_id);
        let mut borrow_index_new = borrow_index;

        let util = Self::calc_utilization_ratio(&total_cash, &total_borrows, &total_reserves)?;
        let borrow_rate = market
            .rate_model
            .get_borrow_rate(util)
            .ok_or(ArithmeticError::Overflow)?;
        let supply_rate = InterestRateModel::get_supply_rate(borrow_rate, util, market.reserve_factor);

        let now = T::UnixTime::now().as_secs();
        let last_accrued_interest_time = Self::last_accrued_interest_time(asset_id);
        if now > last_accrued_interest_time {
            let delta_time = now
                .checked_sub(last_accrued_interest_time)
                .ok_or(ArithmeticError::Underflow)?;
            borrow_index_new = Self::accrue_index(borrow_rate, borrow_index, delta_time)?;
            let total_borrows_old = total_borrows.clone();
            total_borrows =
                Self::borrow_balance_from_old_and_new_index(&borrow_index, &borrow_index_new, total_borrows)?;
            let interest_accummulated = total_borrows.checked_sub(&total_borrows_old)?;
            total_reserves = interest_accummulated
                .map(|x| market.reserve_factor.mul_floor(x))
                .checked_add(&total_reserves)?;
        }

        let exchange_rate = Self::calculate_exchange_rate(&total_supply, &total_cash, &total_borrows, &total_reserves)?;

        Ok((
            borrow_rate,
            supply_rate,
            exchange_rate,
            util,
            total_borrows.amount(),
            total_reserves.amount(),
            borrow_index_new,
        ))
    }

    /// Update the exchange rate according to the totalCash, totalBorrows and totalSupply.
    /// This function does not accrue interest before calculating the exchange rate.
    /// exchangeRate = (totalCash + totalBorrows - totalReserves) / totalSupply
    pub fn exchange_rate_stored(asset_id: CurrencyId<T>) -> Result<Rate, DispatchError> {
        let total_supply = Self::total_supply(asset_id)?;
        let total_cash = Self::get_total_cash(asset_id);
        let total_borrows = Self::total_borrows(asset_id);
        let total_reserves = Self::total_reserves(asset_id);

        Self::calculate_exchange_rate(&total_supply, &total_cash, &total_borrows, &total_reserves)
    }

    /// Calculate the borrowing utilization ratio of the specified market
    ///
    /// utilizationRatio = totalBorrows / (totalCash + totalBorrows − totalReserves)
    pub(crate) fn calc_utilization_ratio(
        cash: &Amount<T>,
        borrows: &Amount<T>,
        reserves: &Amount<T>,
    ) -> Result<Ratio, DispatchError> {
        // utilization ratio is 0 when there are no borrows
        if borrows.is_zero() {
            return Ok(Ratio::zero());
        }
        let total = cash.checked_add(&borrows)?.checked_sub(&reserves)?;

        Ok(Ratio::from_rational(borrows.amount(), total.amount()))
    }

    /// The exchange rate should be greater than the `MinExchangeRate` storage value and less than
    /// the `MaxExchangeRate` storage value.
    /// This ensures the exchange rate cannot be attacked by a deposit so big that
    /// subsequent deposits to receive zero lendTokens (because of rounding down). See this
    /// PR for more details: https://github.com/parallel-finance/parallel/pull/1552/files
    pub(crate) fn ensure_valid_exchange_rate(exchange_rate: Rate) -> DispatchResult {
        ensure!(
            exchange_rate >= Self::min_exchange_rate() && exchange_rate < Self::max_exchange_rate(),
            Error::<T>::InvalidExchangeRate
        );

        Ok(())
    }

    pub(crate) fn update_last_accrued_interest_time(asset_id: CurrencyId<T>, time: Timestamp) -> DispatchResult {
        LastAccruedInterestTime::<T>::try_mutate(asset_id, |last_time| -> DispatchResult {
            *last_time = time;
            Ok(())
        })
    }

    fn accrue_index(borrow_rate: Rate, index: Rate, delta_time: Timestamp) -> Result<Rate, DispatchError> {
        // TODO: Replace simple interest formula with compound interest formula
        // Currently:        new_index = old_index * (1 + annual_borrow_rate * fraction_of_a_year)
        // Compound formula: new_index = old_index * (1 + annual_borrow_rate / SECONDS_PER_YEAR) ^ (delta_time *
        // SECONDS_PER_YEAR)
        let fractional_part = borrow_rate
            .checked_mul(&FixedU128::saturating_from_integer(delta_time))
            .ok_or(ArithmeticError::Overflow)?
            .checked_div(&FixedU128::saturating_from_integer(SECONDS_PER_YEAR))
            .ok_or(ArithmeticError::Underflow)?;
        let accummulated = Rate::one()
            .checked_add(&fractional_part)
            .ok_or(ArithmeticError::Overflow)?;
        Ok(index.checked_mul(&accummulated).ok_or(ArithmeticError::Overflow)?)
    }

    fn calculate_exchange_rate(
        total_supply: &Amount<T>,
        total_cash: &Amount<T>,
        total_borrows: &Amount<T>,
        total_reserves: &Amount<T>,
    ) -> Result<Rate, DispatchError> {
        if total_supply.is_zero() {
            return Ok(Self::min_exchange_rate());
        }

        let cash_plus_borrows_minus_reserves = total_cash.checked_add(total_borrows)?.checked_sub(total_reserves)?;
        let exchange_rate =
            Rate::checked_from_rational(cash_plus_borrows_minus_reserves.amount(), total_supply.amount())
                .ok_or(ArithmeticError::Underflow)?;
        Self::ensure_valid_exchange_rate(exchange_rate)?;

        Ok(exchange_rate)
    }
}
