#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod btc_relay {
    use btc_relay::BtcAddress;
    use frame_support::dispatch::DispatchError;
    use sp_core::H256;
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
    use crate::types::{Collateral, Wrapped};
    use frame_support::dispatch::{DispatchError, DispatchResult};
    use vault_registry::types::{CurrencySource, Vault};

    pub fn get_backing_collateral<T: vault_registry::Config>(
        vault_id: &T::AccountId,
    ) -> Result<Collateral<T>, DispatchError> {
        <vault_registry::Pallet<T>>::get_backing_collateral(vault_id)
    }

    pub fn transfer_funds<T: vault_registry::Config>(
        from: CurrencySource<T>,
        to: CurrencySource<T>,
        amount: Collateral<T>,
    ) -> DispatchResult {
        <vault_registry::Pallet<T>>::transfer_funds(from, to, amount)
    }

    pub fn transfer_funds_saturated<T: vault_registry::Config>(
        from: CurrencySource<T>,
        to: CurrencySource<T>,
        amount: Collateral<T>,
    ) -> Result<Collateral<T>, DispatchError> {
        <vault_registry::Pallet<T>>::transfer_funds_saturated(from, to, amount)
    }

    pub fn get_vault_from_id<T: vault_registry::Config>(
        vault_id: &T::AccountId,
    ) -> Result<
        Vault<T::AccountId, T::BlockNumber, Wrapped<T>, Collateral<T>, <T as vault_registry::Config>::SignedFixedPoint>,
        DispatchError,
    > {
        <vault_registry::Pallet<T>>::get_vault_from_id(vault_id)
    }

    pub fn try_increase_to_be_redeemed_tokens<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        amount: Wrapped<T>,
    ) -> DispatchResult {
        <vault_registry::Pallet<T>>::try_increase_to_be_redeemed_tokens(vault_id, amount)
    }

    pub fn redeem_tokens<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        tokens: Wrapped<T>,
        premium: Collateral<T>,
        redeemer_id: &T::AccountId,
    ) -> DispatchResult {
        <vault_registry::Pallet<T>>::redeem_tokens(vault_id, tokens, premium, redeemer_id)
    }

    pub fn decrease_tokens<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        user_id: &T::AccountId,
        tokens: Wrapped<T>,
    ) -> DispatchResult {
        <vault_registry::Pallet<T>>::decrease_tokens(vault_id, user_id, tokens)
    }

    pub fn redeem_tokens_liquidation<T: vault_registry::Config>(
        redeemer_id: &T::AccountId,
        amount: Wrapped<T>,
    ) -> DispatchResult {
        <vault_registry::Pallet<T>>::redeem_tokens_liquidation(redeemer_id, amount)
    }

    pub fn ban_vault<T: vault_registry::Config>(vault_id: T::AccountId) -> DispatchResult {
        <vault_registry::Pallet<T>>::ban_vault(vault_id)
    }

    pub fn ensure_not_banned<T: vault_registry::Config>(vault: &T::AccountId) -> DispatchResult {
        <vault_registry::Pallet<T>>::_ensure_not_banned(vault)
    }

    pub fn is_vault_below_premium_threshold<T: vault_registry::Config>(
        vault_id: &T::AccountId,
    ) -> Result<bool, DispatchError> {
        <vault_registry::Pallet<T>>::is_vault_below_premium_threshold(vault_id)
    }

    pub fn is_vault_below_secure_threshold<T: vault_registry::Config>(
        vault_id: &T::AccountId,
    ) -> Result<bool, DispatchError> {
        <vault_registry::Pallet<T>>::is_vault_below_secure_threshold(vault_id)
    }

    pub fn decrease_to_be_redeemed_tokens<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        tokens: Wrapped<T>,
    ) -> DispatchResult {
        <vault_registry::Pallet<T>>::decrease_to_be_redeemed_tokens(vault_id, tokens)
    }

    pub fn calculate_collateral<T: vault_registry::Config>(
        collateral: Collateral<T>,
        numerator: Wrapped<T>,
        denominator: Wrapped<T>,
    ) -> Result<Collateral<T>, DispatchError> {
        <vault_registry::Pallet<T>>::calculate_collateral(collateral, numerator, denominator)
    }

    pub fn try_increase_to_be_issued_tokens<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        amount: Wrapped<T>,
    ) -> Result<(), DispatchError> {
        <vault_registry::Pallet<T>>::try_increase_to_be_issued_tokens(vault_id, amount)
    }

    pub fn issue_tokens<T: vault_registry::Config>(vault_id: &T::AccountId, amount: Wrapped<T>) -> DispatchResult {
        <vault_registry::Pallet<T>>::issue_tokens(vault_id, amount)
    }

    pub fn decrease_to_be_replaced_tokens<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        tokens: Wrapped<T>,
    ) -> Result<(Wrapped<T>, Collateral<T>), DispatchError> {
        <vault_registry::Pallet<T>>::decrease_to_be_replaced_tokens(vault_id, tokens)
    }

    pub fn calculate_slashed_amount<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        stake: Collateral<T>,
        reimburse: bool,
    ) -> Result<Collateral<T>, DispatchError> {
        <vault_registry::Pallet<T>>::calculate_slashed_amount(vault_id, stake, reimburse)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod sla {
    use crate::types::{Collateral, Wrapped};
    use frame_support::dispatch::DispatchError;
    pub use sla::types::VaultEvent;

    pub fn event_update_vault_sla<T: sla::Config>(
        vault_id: &T::AccountId,
        event: VaultEvent<Wrapped<T>, Collateral<T>>,
    ) -> Result<(), DispatchError> {
        <sla::Pallet<T>>::event_update_vault_sla(vault_id, event)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod treasury {
    use crate::types::Wrapped;
    use frame_support::dispatch::DispatchResult;

    type TreasuryPallet<T> = currency::Pallet<T, currency::Wrapped>;

    pub fn get_balance<T: currency::Config<currency::Wrapped>>(account: T::AccountId) -> Wrapped<T> {
        TreasuryPallet::<T>::get_free_balance(&account)
    }

    pub fn lock<T: currency::Config<currency::Wrapped>>(redeemer: T::AccountId, amount: Wrapped<T>) -> DispatchResult {
        TreasuryPallet::<T>::lock(&redeemer, amount)
    }

    pub fn unlock<T: currency::Config<currency::Wrapped>>(account: T::AccountId, amount: Wrapped<T>) -> DispatchResult {
        TreasuryPallet::<T>::unlock(account, amount)
    }

    pub fn burn<T: currency::Config<currency::Wrapped>>(redeemer: T::AccountId, amount: Wrapped<T>) -> DispatchResult {
        TreasuryPallet::<T>::burn(&redeemer, amount)
    }

    pub fn unlock_and_transfer<T: currency::Config<currency::Wrapped>>(
        source: &T::AccountId,
        destination: &T::AccountId,
        amount: Wrapped<T>,
    ) -> DispatchResult {
        TreasuryPallet::<T>::unlock_and_transfer(source, destination, amount)
    }

    pub fn mint<T: currency::Config<currency::Wrapped>>(requester: T::AccountId, amount: Wrapped<T>) {
        TreasuryPallet::<T>::mint(requester, amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    use frame_support::dispatch::DispatchError;
    use sp_core::H256;

    pub fn get_secure_id<T: security::Config>(id: &T::AccountId) -> H256 {
        <security::Pallet<T>>::get_secure_id(id)
    }

    pub fn ensure_parachain_status_not_shutdown<T: security::Config>() -> Result<(), DispatchError> {
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
    use crate::types::{Collateral, Wrapped};
    use exchange_rate_oracle::BtcTxFeesPerByte;
    use frame_support::dispatch::DispatchError;

    pub fn satoshi_per_bytes<T: exchange_rate_oracle::Config>() -> BtcTxFeesPerByte {
        <exchange_rate_oracle::Pallet<T>>::satoshi_per_bytes()
    }

    pub fn wrapped_to_collateral<T: exchange_rate_oracle::Config>(
        amount: Wrapped<T>,
    ) -> Result<Collateral<T>, DispatchError> {
        <exchange_rate_oracle::Pallet<T>>::wrapped_to_collateral(amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod fee {
    use crate::types::{Collateral, Wrapped};
    use frame_support::dispatch::{DispatchError, DispatchResult};

    pub fn fee_pool_account_id<T: fee::Config>() -> T::AccountId {
        <fee::Pallet<T>>::fee_pool_account_id()
    }

    pub fn get_redeem_fee<T: fee::Config>(amount: Wrapped<T>) -> Result<Wrapped<T>, DispatchError> {
        <fee::Pallet<T>>::get_redeem_fee(amount)
    }

    pub fn distribute_wrapped_rewards<T: fee::Config>(amount: Wrapped<T>) -> DispatchResult {
        <fee::Pallet<T>>::distribute_wrapped_rewards(amount)
    }

    pub fn distribute_collateral_rewards<T: fee::Config>(amount: Collateral<T>) -> DispatchResult {
        <fee::Pallet<T>>::distribute_collateral_rewards(amount)
    }

    pub fn get_punishment_fee<T: fee::Config>(amount: Collateral<T>) -> Result<Collateral<T>, DispatchError> {
        <fee::Pallet<T>>::get_punishment_fee(amount)
    }

    pub fn get_premium_redeem_fee<T: fee::Config>(amount: Collateral<T>) -> Result<Collateral<T>, DispatchError> {
        <fee::Pallet<T>>::get_premium_redeem_fee(amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod collateral {
    use crate::Collateral;
    use frame_support::dispatch::DispatchResult;

    type CollateralPallet<T> = currency::Pallet<T, currency::Collateral>;

    pub fn release_collateral<T: currency::Config<currency::Collateral>>(
        sender: &T::AccountId,
        amount: Collateral<T>,
    ) -> DispatchResult {
        CollateralPallet::<T>::release(sender, amount)
    }
}
