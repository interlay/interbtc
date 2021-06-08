#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod btc_relay {
    use bitcoin::types::Transaction;
    use btc_relay::BtcAddress;
    use frame_support::dispatch::DispatchError;
    use sp_core::H256;
    use sp_std::{convert::TryInto, vec::Vec};

    pub fn verify_and_validate_op_return_transaction<T: crate::Config, V: TryInto<i64>>(
        merkle_proof: Vec<u8>,
        transaction: Transaction,
        recipient_btc_address: BtcAddress,
        expected_btc: V,
        op_return_id: H256,
    ) -> Result<(), DispatchError> {
        <btc_relay::Pallet<T>>::verify_and_validate_op_return_transaction(
            merkle_proof,
            transaction,
            recipient_btc_address,
            expected_btc,
            op_return_id,
        )
    }

    pub fn get_best_block_height<T: crate::Config>() -> u32 {
        <btc_relay::Pallet<T>>::get_best_block_height()
    }

    pub fn parse_transaction<T: btc_relay::Config>(raw_tx: &[u8]) -> Result<Transaction, DispatchError> {
        <btc_relay::Pallet<T>>::parse_transaction(raw_tx)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod vault_registry {
    use crate::{Collateral, Wrapped};
    use btc_relay::BtcAddress;
    use frame_support::dispatch::{DispatchError, DispatchResult};
    use vault_registry::types::CurrencySource;

    pub fn transfer_funds<T: crate::Config>(
        from: CurrencySource<T>,
        to: CurrencySource<T>,
        amount: Collateral<T>,
    ) -> DispatchResult {
        <vault_registry::Pallet<T>>::transfer_funds(from, to, amount)
    }
    pub fn replace_tokens<T: crate::Config>(
        old_vault_id: T::AccountId,
        new_vault_id: T::AccountId,
        tokens: Wrapped<T>,
        collateral: Collateral<T>,
    ) -> DispatchResult {
        <vault_registry::Pallet<T>>::replace_tokens(&old_vault_id, &new_vault_id, tokens, collateral)
    }

    pub fn cancel_replace_tokens<T: crate::Config>(
        old_vault_id: &T::AccountId,
        new_vault_id: &T::AccountId,
        tokens: Wrapped<T>,
    ) -> DispatchResult {
        <vault_registry::Pallet<T>>::cancel_replace_tokens(old_vault_id, new_vault_id, tokens)
    }

    pub fn is_vault_liquidated<T: crate::Config>(vault_id: &T::AccountId) -> Result<bool, DispatchError> {
        <vault_registry::Pallet<T>>::is_vault_liquidated(vault_id)
    }

    pub fn try_increase_to_be_redeemed_tokens<T: crate::Config>(
        vault_id: &T::AccountId,
        tokens: Wrapped<T>,
    ) -> DispatchResult {
        <vault_registry::Pallet<T>>::try_increase_to_be_redeemed_tokens(vault_id, tokens)
    }

    pub fn ensure_not_banned<T: crate::Config>(vault: &T::AccountId) -> DispatchResult {
        <vault_registry::Pallet<T>>::_ensure_not_banned(vault)
    }

    pub fn insert_vault_deposit_address<T: crate::Config>(
        vault_id: &T::AccountId,
        btc_address: BtcAddress,
    ) -> DispatchResult {
        <vault_registry::Pallet<T>>::insert_vault_deposit_address(vault_id, btc_address)
    }

    pub fn try_increase_to_be_issued_tokens<T: crate::Config>(
        vault_id: &T::AccountId,
        amount: Wrapped<T>,
    ) -> Result<(), DispatchError> {
        <vault_registry::Pallet<T>>::try_increase_to_be_issued_tokens(vault_id, amount)
    }

    pub fn requestable_to_be_replaced_tokens<T: crate::Config>(
        vault_id: &T::AccountId,
    ) -> Result<Wrapped<T>, DispatchError> {
        <vault_registry::Pallet<T>>::requestable_to_be_replaced_tokens(vault_id)
    }

    pub fn try_increase_to_be_replaced_tokens<T: crate::Config>(
        vault_id: &T::AccountId,
        amount: Wrapped<T>,
        griefing_collateral: Collateral<T>,
    ) -> Result<(Wrapped<T>, Collateral<T>), DispatchError> {
        <vault_registry::Pallet<T>>::try_increase_to_be_replaced_tokens(vault_id, amount, griefing_collateral)
    }

    pub fn decrease_to_be_replaced_tokens<T: crate::Config>(
        vault_id: &T::AccountId,
        tokens: Wrapped<T>,
    ) -> Result<(Wrapped<T>, Collateral<T>), DispatchError> {
        <vault_registry::Pallet<T>>::decrease_to_be_replaced_tokens(vault_id, tokens)
    }

    pub fn try_deposit_collateral<T: crate::Config>(
        vault_id: &T::AccountId,
        amount: Collateral<T>,
    ) -> Result<(), DispatchError> {
        <vault_registry::Pallet<T>>::try_deposit_collateral(vault_id, amount)
    }

    pub fn force_withdraw_collateral<T: crate::Config>(
        vault_id: &T::AccountId,
        amount: Collateral<T>,
    ) -> Result<(), DispatchError> {
        <vault_registry::Pallet<T>>::force_withdraw_collateral(vault_id, amount)
    }

    pub fn is_allowed_to_withdraw_collateral<T: crate::Config>(
        vault_id: &T::AccountId,
        amount: Collateral<T>,
    ) -> Result<bool, DispatchError> {
        <vault_registry::Pallet<T>>::is_allowed_to_withdraw_collateral(vault_id, amount)
    }

    pub fn calculate_collateral<T: crate::Config>(
        collateral: Collateral<T>,
        numerator: Wrapped<T>,
        denominator: Wrapped<T>,
    ) -> Result<Collateral<T>, DispatchError> {
        <vault_registry::Pallet<T>>::calculate_collateral(collateral, numerator, denominator)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod collateral {
    use crate::Collateral;
    use frame_support::dispatch::DispatchResult;

    type CollateralPallet<T> = currency::Pallet<T, currency::Collateral>;

    pub fn release_collateral<T: crate::Config>(sender: &T::AccountId, amount: Collateral<T>) -> DispatchResult {
        CollateralPallet::<T>::release(sender, amount)
    }

    pub fn lock_collateral<T: crate::Config>(sender: T::AccountId, amount: Collateral<T>) -> DispatchResult {
        CollateralPallet::<T>::lock(&sender, amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    use frame_support::dispatch::{DispatchError, DispatchResult};
    use sp_core::H256;

    pub fn get_secure_id<T: crate::Config>(id: &T::AccountId) -> H256 {
        <security::Pallet<T>>::get_secure_id(id)
    }

    pub fn ensure_parachain_status_not_shutdown<T: crate::Config>() -> DispatchResult {
        <security::Pallet<T>>::ensure_parachain_status_not_shutdown()
    }

    pub fn active_block_number<T: crate::Config>() -> T::BlockNumber {
        <security::Pallet<T>>::active_block_number()
    }

    pub fn has_expired<T: crate::Config>(
        opentime: T::BlockNumber,
        period: T::BlockNumber,
    ) -> Result<bool, DispatchError> {
        <security::Pallet<T>>::has_expired(opentime, period)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod oracle {
    use crate::types::{Collateral, Wrapped};
    use frame_support::dispatch::DispatchError;

    pub fn wrapped_to_collateral<T: crate::Config>(amount: Wrapped<T>) -> Result<Collateral<T>, DispatchError> {
        <exchange_rate_oracle::Pallet<T>>::wrapped_to_collateral(amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod fee {
    use crate::types::Collateral;
    use frame_support::dispatch::DispatchError;

    pub fn get_replace_griefing_collateral<T: crate::Config>(
        amount: Collateral<T>,
    ) -> Result<Collateral<T>, DispatchError> {
        <fee::Pallet<T>>::get_replace_griefing_collateral(amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod nomination {
    use sp_runtime::DispatchError;

    pub fn is_nominatable<T: crate::Config>(vault_id: &T::AccountId) -> Result<bool, DispatchError> {
        <nomination::Pallet<T>>::is_nominatable(vault_id)
    }
}
