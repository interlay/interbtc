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

use sp_runtime::{traits::Zero, DispatchResult};

use crate::*;

impl<T: Config> Pallet<T> {
    pub fn reward_account_id() -> T::AccountId {
        T::PalletId::get().into_sub_account_truncating(REWARD_SUB_ACCOUNT)
    }

    #[cfg_attr(test, mutate)]
    fn reward_scale() -> u128 {
        10_u128.pow(12)
    }

    #[cfg_attr(test, mutate)]
    fn calculate_reward_delta_index(
        delta_block: BlockNumberFor<T>,
        reward_speed: BalanceOf<T>,
        total_share: BalanceOf<T>,
    ) -> Result<u128, sp_runtime::DispatchError> {
        if total_share.is_zero() {
            return Ok(0);
        }
        let delta_block: BalanceOf<T> = delta_block.saturated_into();
        let delta_index = delta_block
            .get_big_uint()
            .checked_mul(&reward_speed.get_big_uint())
            .and_then(|r| r.checked_mul(&Self::reward_scale().get_big_uint()))
            .and_then(|r| r.checked_div(&total_share.get_big_uint()))
            .and_then(|r| r.to_u128())
            .ok_or(ArithmeticError::Overflow)?;
        Ok(delta_index)
    }

    #[cfg_attr(test, mutate)]
    fn calculate_reward_delta(
        share: BalanceOf<T>,
        reward_delta_index: u128,
    ) -> Result<u128, sp_runtime::DispatchError> {
        let reward_delta = share
            .get_big_uint()
            .checked_mul(&reward_delta_index.get_big_uint())
            .and_then(|r| r.checked_div(&Self::reward_scale().get_big_uint()))
            .and_then(|r| r.to_u128())
            .ok_or(ArithmeticError::Overflow)?;
        Ok(reward_delta)
    }

    #[cfg_attr(test, mutate)]
    pub(crate) fn update_reward_supply_index(asset_id: CurrencyId<T>) -> DispatchResult {
        let current_block_number = <frame_system::Pallet<T>>::block_number();
        RewardSupplyState::<T>::try_mutate(asset_id, |supply_state| -> DispatchResult {
            let delta_block = current_block_number.saturating_sub(supply_state.block);
            if delta_block.is_zero() {
                return Ok(());
            }
            let supply_speed = RewardSupplySpeed::<T>::get(asset_id);
            if !supply_speed.is_zero() {
                let total_supply = Self::total_supply(asset_id)?;
                let delta_index = Self::calculate_reward_delta_index(delta_block, supply_speed, total_supply.amount())?;
                supply_state.index = supply_state
                    .index
                    .checked_add(delta_index)
                    .ok_or(ArithmeticError::Overflow)?;
            }
            supply_state.block = current_block_number;

            Ok(())
        })
    }

    #[cfg_attr(test, mutate)]
    pub(crate) fn update_reward_borrow_index(asset_id: CurrencyId<T>) -> DispatchResult {
        let current_block_number = <frame_system::Pallet<T>>::block_number();
        RewardBorrowState::<T>::try_mutate(asset_id, |borrow_state| -> DispatchResult {
            let delta_block = current_block_number.saturating_sub(borrow_state.block);
            if delta_block.is_zero() {
                return Ok(());
            }
            let borrow_speed = RewardBorrowSpeed::<T>::get(asset_id);
            if !borrow_speed.is_zero() {
                let current_borrow_amount = Self::total_borrows(asset_id);
                let current_borrow_index = BorrowIndex::<T>::get(asset_id);
                let base_borrow_amount = current_borrow_amount.checked_div(&current_borrow_index)?.amount();
                let delta_index = Self::calculate_reward_delta_index(delta_block, borrow_speed, base_borrow_amount)?;
                borrow_state.index = borrow_state
                    .index
                    .checked_add(delta_index)
                    .ok_or(ArithmeticError::Overflow)?;
            }
            borrow_state.block = current_block_number;

            Ok(())
        })
    }

    #[cfg_attr(test, mutate)]
    pub(crate) fn distribute_supplier_reward(asset_id: CurrencyId<T>, supplier: &T::AccountId) -> DispatchResult {
        RewardSupplierIndex::<T>::try_mutate(asset_id, supplier, |supplier_index| -> DispatchResult {
            let supply_state = RewardSupplyState::<T>::get(asset_id);
            let delta_index = supply_state
                .index
                .checked_sub(*supplier_index)
                .ok_or(ArithmeticError::Underflow)?;
            *supplier_index = supply_state.index;

            let lend_token_id = Self::lend_token_id(asset_id)?;
            RewardAccrued::<T>::try_mutate(supplier, |total_reward| -> DispatchResult {
                // Frozen balance is not counted towards the total, so emitted rewards
                // may be lower than intended.
                // No balance freezing is done currently, so this is correct.
                let total_balance = Self::balance(lend_token_id, supplier);
                let reward_delta = Self::calculate_reward_delta(total_balance.amount(), delta_index)?;
                *total_reward = total_reward
                    .checked_add(reward_delta)
                    .ok_or(ArithmeticError::Overflow)?;
                Self::deposit_event(Event::<T>::DistributedSupplierReward {
                    underlying_currency_id: asset_id,
                    supplier: supplier.clone(),
                    reward_delta,
                    supply_reward_index: supply_state.index,
                });

                Ok(())
            })
        })
    }

    #[cfg_attr(test, mutate)]
    pub(crate) fn distribute_borrower_reward(asset_id: CurrencyId<T>, borrower: &T::AccountId) -> DispatchResult {
        RewardBorrowerIndex::<T>::try_mutate(asset_id, borrower, |borrower_index| -> DispatchResult {
            let borrow_state = RewardBorrowState::<T>::get(asset_id);
            let delta_index = borrow_state
                .index
                .checked_sub(*borrower_index)
                .ok_or(ArithmeticError::Underflow)?;
            *borrower_index = borrow_state.index;

            RewardAccrued::<T>::try_mutate(borrower, |total_reward| -> DispatchResult {
                let current_borrow_amount = Self::current_borrow_balance(borrower, asset_id)?;
                let current_borrow_index = BorrowIndex::<T>::get(asset_id);
                let base_borrow_amount = current_borrow_amount.checked_div(&current_borrow_index)?.amount();
                let reward_delta = Self::calculate_reward_delta(base_borrow_amount, delta_index)?;
                *total_reward = total_reward
                    .checked_add(reward_delta)
                    .ok_or(ArithmeticError::Overflow)?;
                Self::deposit_event(Event::<T>::DistributedBorrowerReward {
                    underlying_currency_id: asset_id,
                    borrower: borrower.clone(),
                    reward_delta,
                    borrow_reward_index: borrow_state.index,
                });

                Ok(())
            })
        })
    }

    #[cfg_attr(test, mutate)]
    pub(crate) fn collect_market_reward(asset_id: CurrencyId<T>, user: &T::AccountId) -> DispatchResult {
        Self::update_reward_supply_index(asset_id)?;
        Self::distribute_supplier_reward(asset_id, user)?;

        Self::update_reward_borrow_index(asset_id)?;
        Self::distribute_borrower_reward(asset_id, user)?;

        Ok(())
    }

    #[cfg_attr(test, mutate)]
    pub(crate) fn pay_reward(user: &T::AccountId) -> DispatchResult {
        let pool_account = Self::reward_account_id();
        let reward_asset = T::RewardAssetId::get();
        let total_reward = RewardAccrued::<T>::get(user);
        if total_reward > 0 {
            let amount: Amount<T> = Amount::new(total_reward, reward_asset);
            amount.transfer(&pool_account, user)?;
            RewardAccrued::<T>::remove(user);
        }
        Self::deposit_event(Event::<T>::RewardPaid {
            receiver: user.clone(),
            amount: total_reward,
        });
        Ok(())
    }
}
