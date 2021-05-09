#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod btc_relay {
    use btc_relay::BtcAddress;
    use frame_support::dispatch::DispatchError;
    use primitive_types::H256;
    use sp_std::vec::Vec;

    pub fn verify_and_validate_transaction<T: btc_relay::Config>(
        raw_merkle_proof: Vec<u8>,
        raw_tx: Vec<u8>,
        recipient_btc_address: BtcAddress,
        minimum_btc: Option<i64>,
        op_return_id: Option<H256>,
        confirmations: Option<u32>,
    ) -> Result<(BtcAddress, i64), DispatchError> {
        <btc_relay::Pallet<T>>::_verify_and_validate_transaction(
            raw_merkle_proof,
            raw_tx,
            recipient_btc_address,
            minimum_btc,
            op_return_id,
            confirmations,
        )
    }

    pub fn get_best_block_height<T: btc_relay::Config>() -> u32 {
        <btc_relay::Pallet<T>>::get_best_block_height()
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod vault_registry {
    use crate::{Backing, Issuing};
    use btc_relay::BtcAddress;
    use frame_support::dispatch::{DispatchError, DispatchResult};
    use vault_registry::types::CurrencySource;

    pub fn slash_collateral<T: vault_registry::Config>(
        from: CurrencySource<T>,
        to: CurrencySource<T>,
        amount: Backing<T>,
    ) -> DispatchResult {
        <vault_registry::Pallet<T>>::slash_collateral(from, to, amount)
    }
    pub fn replace_tokens<T: vault_registry::Config>(
        old_vault_id: T::AccountId,
        new_vault_id: T::AccountId,
        tokens: Issuing<T>,
        collateral: Backing<T>,
    ) -> DispatchResult {
        <vault_registry::Pallet<T>>::replace_tokens(&old_vault_id, &new_vault_id, tokens, collateral)
    }

    pub fn get_auctionable_tokens<T: vault_registry::Config>(
        vault_id: &T::AccountId,
    ) -> Result<Issuing<T>, DispatchError> {
        <vault_registry::Pallet<T>>::get_auctionable_tokens(vault_id)
    }

    pub fn cancel_replace_tokens<T: vault_registry::Config>(
        old_vault_id: &T::AccountId,
        new_vault_id: &T::AccountId,
        tokens: Issuing<T>,
    ) -> DispatchResult {
        <vault_registry::Pallet<T>>::cancel_replace_tokens(old_vault_id, new_vault_id, tokens)
    }

    pub fn is_vault_liquidated<T: vault_registry::Config>(vault_id: &T::AccountId) -> Result<bool, DispatchError> {
        <vault_registry::Pallet<T>>::is_vault_liquidated(vault_id)
    }

    pub fn try_increase_to_be_redeemed_tokens<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        tokens: Issuing<T>,
    ) -> DispatchResult {
        <vault_registry::Pallet<T>>::try_increase_to_be_redeemed_tokens(vault_id, tokens)
    }

    pub fn is_vault_below_auction_threshold<T: vault_registry::Config>(
        vault_id: T::AccountId,
    ) -> Result<bool, DispatchError> {
        <vault_registry::Pallet<T>>::is_vault_below_auction_threshold(&vault_id)
    }

    pub fn ensure_not_banned<T: vault_registry::Config>(vault: &T::AccountId) -> DispatchResult {
        <vault_registry::Pallet<T>>::_ensure_not_banned(vault)
    }

    pub fn insert_vault_deposit_address<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        btc_address: BtcAddress,
    ) -> DispatchResult {
        <vault_registry::Pallet<T>>::insert_vault_deposit_address(vault_id, btc_address)
    }

    pub fn try_increase_to_be_issued_tokens<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        amount: Issuing<T>,
    ) -> Result<(), DispatchError> {
        <vault_registry::Pallet<T>>::try_increase_to_be_issued_tokens(vault_id, amount)
    }

    pub fn requestable_to_be_replaced_tokens<T: vault_registry::Config>(
        vault_id: &T::AccountId,
    ) -> Result<Issuing<T>, DispatchError> {
        <vault_registry::Pallet<T>>::requestable_to_be_replaced_tokens(vault_id)
    }

    pub fn try_increase_to_be_replaced_tokens<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        amount: Issuing<T>,
        griefing_collateral: Backing<T>,
    ) -> Result<(Issuing<T>, Backing<T>), DispatchError> {
        <vault_registry::Pallet<T>>::try_increase_to_be_replaced_tokens(vault_id, amount, griefing_collateral)
    }

    pub fn decrease_to_be_replaced_tokens<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        tokens: Issuing<T>,
    ) -> Result<(Issuing<T>, Backing<T>), DispatchError> {
        <vault_registry::Pallet<T>>::decrease_to_be_replaced_tokens(vault_id, tokens)
    }

    pub fn lock_additional_collateral<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        amount: Backing<T>,
    ) -> Result<(), DispatchError> {
        <vault_registry::Pallet<T>>::_lock_additional_collateral(vault_id, amount)
    }

    pub fn force_withdraw_collateral<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        amount: Backing<T>,
    ) -> Result<(), DispatchError> {
        <vault_registry::Pallet<T>>::force_withdraw_collateral(vault_id, amount)
    }

    pub fn is_allowed_to_withdraw_collateral<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        amount: Backing<T>,
    ) -> Result<bool, DispatchError> {
        <vault_registry::Pallet<T>>::is_allowed_to_withdraw_collateral(vault_id, amount)
    }

    pub fn calculate_collateral<T: vault_registry::Config>(
        collateral: Backing<T>,
        numerator: Issuing<T>,
        denominator: Issuing<T>,
    ) -> Result<Backing<T>, DispatchError> {
        <vault_registry::Pallet<T>>::calculate_collateral(collateral, numerator, denominator)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod collateral {
    use crate::Backing;
    use frame_support::dispatch::DispatchResult;

    type CollateralPallet<T> = currency::Pallet<T, currency::Collateral>;

    pub fn release_collateral<T: currency::Config<currency::Collateral>>(
        sender: &T::AccountId,
        amount: Backing<T>,
    ) -> DispatchResult {
        CollateralPallet::<T>::release(sender, amount)
    }

    pub fn lock_collateral<T: currency::Config<currency::Collateral>>(
        sender: T::AccountId,
        amount: Backing<T>,
    ) -> DispatchResult {
        CollateralPallet::<T>::lock(&sender, amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    use frame_support::dispatch::{DispatchError, DispatchResult};
    use primitive_types::H256;

    pub fn get_secure_id<T: security::Config>(id: &T::AccountId) -> H256 {
        <security::Pallet<T>>::get_secure_id(id)
    }

    pub fn ensure_parachain_status_not_shutdown<T: security::Config>() -> DispatchResult {
        <security::Pallet<T>>::ensure_parachain_status_not_shutdown()
    }

    pub fn active_block_number<T: security::Config>() -> T::BlockNumber {
        <security::Pallet<T>>::active_block_number()
    }

    pub fn has_expired<T: security::Config>(
        opentime: T::BlockNumber,
        period: T::BlockNumber,
    ) -> Result<bool, DispatchError> {
        <security::Pallet<T>>::has_expired(opentime, period)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod oracle {
    use crate::types::{Backing, Issuing};
    use frame_support::dispatch::DispatchError;

    pub fn issuing_to_backing<T: exchange_rate_oracle::Config>(
        amount: Issuing<T>,
    ) -> Result<Backing<T>, DispatchError> {
        <exchange_rate_oracle::Pallet<T>>::issuing_to_backing(amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod fee {
    use crate::types::Backing;
    use frame_support::dispatch::DispatchError;

    pub fn get_replace_griefing_collateral<T: fee::Config>(amount: Backing<T>) -> Result<Backing<T>, DispatchError> {
        <fee::Pallet<T>>::get_replace_griefing_collateral(amount)
    }

    pub fn get_auction_redeem_fee<T: fee::Config>(amount: Backing<T>) -> Result<Backing<T>, DispatchError> {
        <fee::Pallet<T>>::get_auction_redeem_fee(amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod nomination {
    use sp_runtime::DispatchError;

    pub fn is_operator<T: nomination::Config>(operator_id: &T::AccountId) -> Result<bool, DispatchError> {
        <nomination::Module<T>>::is_operator(operator_id)
    }
}
