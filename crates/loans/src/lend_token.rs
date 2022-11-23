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

use crate::{AssetIdOf, BalanceOf, *};

#[cfg(test)]
use frame_support::traits::tokens::{DepositConsequence, WithdrawConsequence};

#[cfg(test)]
impl<T: Config> Pallet<T> {
    /// The total amount of issuance in the system.
    pub fn total_issuance(lend_token_id: AssetIdOf<T>) -> BalanceOf<T> {
        if let Ok(underlying_id) = Self::underlying_id(lend_token_id) {
            if let Ok(supply) = Self::total_supply(underlying_id) {
                return supply;
            }
        }
        Balance::default()
    }

    /// The minimum balance any single account may have.
    pub fn minimum_balance(_lend_token_id: AssetIdOf<T>) -> BalanceOf<T> {
        Zero::zero()
    }

    /// Get the maximum amount that `who` can withdraw/transfer successfully.
    /// For lend_token, We don't care if keep_alive is enabled
    pub fn reducible_balance(lend_token_id: AssetIdOf<T>, who: &T::AccountId, _keep_alive: bool) -> BalanceOf<T> {
        Self::reducible_asset(lend_token_id, who).unwrap_or_default()
    }

    /// Returns `true` if the balance of `who` may be increased by `amount`.
    pub fn can_deposit(
        lend_token_id: AssetIdOf<T>,
        who: &T::AccountId,
        amount: BalanceOf<T>,
        _mint: bool,
    ) -> DepositConsequence {
        let underlying_id = match Self::underlying_id(lend_token_id) {
            Ok(asset_id) => asset_id,
            Err(_) => return DepositConsequence::UnknownAsset,
        };

        if let Err(res) = Self::ensure_active_market(underlying_id).map_err(|_| DepositConsequence::UnknownAsset) {
            return res;
        }

        if let Ok(total_supply) = Self::total_supply(underlying_id) {
            if total_supply.checked_add(amount).is_none() {
                return DepositConsequence::Overflow;
            }
        } else {
            return DepositConsequence::UnknownAsset;
        }

        if Self::balance(lend_token_id, who) + amount < Self::minimum_balance(lend_token_id) {
            return DepositConsequence::BelowMinimum;
        }

        DepositConsequence::Success
    }

    /// Returns `Failed` if the balance of `who` may not be decreased by `amount`, otherwise
    /// the consequence.
    pub fn can_withdraw(
        lend_token_id: AssetIdOf<T>,
        who: &T::AccountId,
        amount: BalanceOf<T>,
    ) -> WithdrawConsequence<BalanceOf<T>> {
        let underlying_id = match Self::underlying_id(lend_token_id) {
            Ok(asset_id) => asset_id,
            Err(_) => return WithdrawConsequence::UnknownAsset,
        };

        if let Err(res) = Self::ensure_active_market(underlying_id).map_err(|_| WithdrawConsequence::UnknownAsset) {
            return res;
        }

        let sub_result = Self::balance(lend_token_id, who).checked_sub(amount);
        if sub_result.is_none() {
            return WithdrawConsequence::NoFunds;
        }

        let rest = sub_result.expect("Cannot be none; qed");
        if rest < Self::minimum_balance(lend_token_id) {
            return WithdrawConsequence::ReducedToZero(rest);
        }

        WithdrawConsequence::Success
    }

    /// Returns `Err` if the reducible lend_token of `who` is insufficient
    ///
    /// For lend_token, We don't care if keep_alive is enabled
    #[transactional]
    pub fn transfer(
        lend_token_id: AssetIdOf<T>,
        source: &T::AccountId,
        dest: &T::AccountId,
        amount: BalanceOf<T>,
        _keep_alive: bool,
    ) -> Result<BalanceOf<T>, DispatchError> {
        <orml_tokens::Pallet<T> as MultiCurrency<T::AccountId>>::transfer(lend_token_id, source, dest, amount)
            .map_err(|_| Error::<T>::InsufficientCollateral)?;
        Ok(amount)
    }

    fn reducible_asset(lend_token_id: AssetIdOf<T>, who: &T::AccountId) -> Result<BalanceOf<T>, DispatchError> {
        let voucher_balance = Self::account_deposits(lend_token_id, &who);

        let underlying_id = Self::underlying_id(lend_token_id)?;
        let market = Self::ensure_active_market(underlying_id)?;
        let collateral_value = Self::collateral_asset_value(who, underlying_id)?;

        // liquidity of all assets
        let (liquidity, _) = Self::get_account_liquidity(who)?;

        if liquidity >= collateral_value {
            return Ok(voucher_balance);
        }

        // Formula
        // reducible_underlying_amount = liquidity / collateral_factor / price
        let reducible_supply_value = liquidity
            .checked_div(&market.collateral_factor.into())
            .ok_or(ArithmeticError::Overflow)?;
        let reducible_supply_amount =
            Amount::<T>::from_unsigned_fixed_point(reducible_supply_value, T::ReferenceAssetId::get())?;
        let reducible_underlying_amount = reducible_supply_amount.convert_to(underlying_id)?.amount();

        let exchange_rate = Self::exchange_rate(underlying_id);
        let amount = Self::calc_collateral_amount(reducible_underlying_amount, exchange_rate)?;
        Ok(amount)
    }
}
