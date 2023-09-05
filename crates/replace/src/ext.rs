#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod btc_relay {
    use bitcoin::types::{FullTransactionProof, Value};
    use btc_relay::BtcAddress;
    use frame_support::dispatch::DispatchError;
    use frame_system::pallet_prelude::BlockNumberFor;
    use sp_core::H256;
    use sp_std::convert::TryInto;

    pub fn verify_and_validate_op_return_transaction<T: crate::Config, V: TryInto<Value>>(
        unchecked_transaction: FullTransactionProof,
        recipient_btc_address: BtcAddress,
        expected_btc: V,
        op_return_id: H256,
    ) -> Result<(), DispatchError> {
        <btc_relay::Pallet<T>>::verify_and_validate_op_return_transaction(
            unchecked_transaction,
            recipient_btc_address,
            expected_btc,
            op_return_id,
        )
    }

    pub fn get_best_block_height<T: crate::Config>() -> u32 {
        <btc_relay::Pallet<T>>::get_best_block_height()
    }

    pub fn has_request_expired<T: crate::Config>(
        opentime: BlockNumberFor<T>,
        btc_open_height: u32,
        period: BlockNumberFor<T>,
    ) -> Result<bool, DispatchError> {
        <btc_relay::Pallet<T>>::has_request_expired(opentime, btc_open_height, period)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod vault_registry {
    use crate::DefaultVaultId;
    use currency::Amount;
    use frame_support::dispatch::{DispatchError, DispatchResult};
    use vault_registry::types::CurrencySource;

    pub fn transfer_funds<T: crate::Config>(
        from: CurrencySource<T>,
        to: CurrencySource<T>,
        amount: &Amount<T>,
    ) -> DispatchResult {
        <vault_registry::Pallet<T>>::transfer_funds(from, to, amount)
    }

    pub fn replace_tokens<T: crate::Config>(
        old_vault_id: &DefaultVaultId<T>,
        new_vault_id: &DefaultVaultId<T>,
        tokens: &Amount<T>,
        collateral: &Amount<T>,
    ) -> DispatchResult {
        <vault_registry::Pallet<T>>::replace_tokens(old_vault_id, new_vault_id, tokens, collateral)
    }

    pub fn cancel_replace_tokens<T: crate::Config>(
        old_vault_id: &DefaultVaultId<T>,
        new_vault_id: &DefaultVaultId<T>,
        tokens: &Amount<T>,
    ) -> DispatchResult {
        <vault_registry::Pallet<T>>::cancel_replace_tokens(old_vault_id, new_vault_id, tokens)
    }

    pub fn is_vault_liquidated<T: crate::Config>(vault_id: &DefaultVaultId<T>) -> Result<bool, DispatchError> {
        <vault_registry::Pallet<T>>::is_vault_liquidated(vault_id)
    }

    pub fn try_increase_to_be_redeemed_tokens<T: crate::Config>(
        vault_id: &DefaultVaultId<T>,
        tokens: &Amount<T>,
    ) -> DispatchResult {
        <vault_registry::Pallet<T>>::try_increase_to_be_redeemed_tokens(vault_id, tokens)
    }

    pub fn ensure_not_banned<T: crate::Config>(vault_id: &DefaultVaultId<T>) -> DispatchResult {
        <vault_registry::Pallet<T>>::_ensure_not_banned(vault_id)
    }

    pub fn try_increase_to_be_issued_tokens<T: crate::Config>(
        vault_id: &DefaultVaultId<T>,
        amount: &Amount<T>,
    ) -> Result<(), DispatchError> {
        <vault_registry::Pallet<T>>::try_increase_to_be_issued_tokens(vault_id, amount)
    }

    pub fn requestable_to_be_replaced_tokens<T: crate::Config>(
        vault_id: &DefaultVaultId<T>,
    ) -> Result<Amount<T>, DispatchError> {
        <vault_registry::Pallet<T>>::requestable_to_be_replaced_tokens(vault_id)
    }

    pub fn try_increase_to_be_replaced_tokens<T: crate::Config>(
        vault_id: &DefaultVaultId<T>,
        amount: &Amount<T>,
    ) -> Result<Amount<T>, DispatchError> {
        <vault_registry::Pallet<T>>::try_increase_to_be_replaced_tokens(vault_id, amount)
    }

    pub fn decrease_to_be_replaced_tokens<T: crate::Config>(
        vault_id: &DefaultVaultId<T>,
        tokens: &Amount<T>,
    ) -> Result<(Amount<T>, Amount<T>), DispatchError> {
        <vault_registry::Pallet<T>>::decrease_to_be_replaced_tokens(vault_id, tokens)
    }

    pub fn withdraw_replace_request<T: crate::Config>(
        vault_id: &DefaultVaultId<T>,
        amount: &Amount<T>,
    ) -> Result<(Amount<T>, Amount<T>), DispatchError> {
        <vault_registry::Pallet<T>>::withdraw_replace_request(vault_id, amount)
    }

    pub fn try_deposit_collateral<T: crate::Config>(
        vault_id: &DefaultVaultId<T>,
        amount: &Amount<T>,
    ) -> Result<(), DispatchError> {
        <vault_registry::Pallet<T>>::try_deposit_collateral(vault_id, amount)
    }

    pub fn force_withdraw_collateral<T: crate::Config>(
        vault_id: &DefaultVaultId<T>,
        amount: &Amount<T>,
    ) -> Result<(), DispatchError> {
        <vault_registry::Pallet<T>>::force_withdraw_collateral(vault_id, amount)
    }

    pub fn is_allowed_to_withdraw_collateral<T: crate::Config>(
        vault_id: &DefaultVaultId<T>,
        amount: Option<Amount<T>>,
    ) -> Result<bool, DispatchError> {
        <vault_registry::Pallet<T>>::is_allowed_to_withdraw_collateral(vault_id, amount)
    }

    pub fn calculate_collateral<T: crate::Config>(
        collateral: &Amount<T>,
        numerator: &Amount<T>,
        denominator: &Amount<T>,
    ) -> Result<Amount<T>, DispatchError> {
        <vault_registry::Pallet<T>>::calculate_collateral(collateral, numerator, denominator)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    use frame_system::pallet_prelude::BlockNumberFor;
    use sp_core::H256;

    pub fn get_secure_id<T: crate::Config>(id: &T::AccountId) -> H256 {
        <security::Pallet<T>>::get_secure_id(id)
    }

    pub fn active_block_number<T: crate::Config>() -> BlockNumberFor<T> {
        <security::Pallet<T>>::active_block_number()
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod fee {
    use currency::Amount;
    use frame_support::dispatch::DispatchError;

    pub fn get_replace_griefing_collateral<T: crate::Config>(amount: &Amount<T>) -> Result<Amount<T>, DispatchError> {
        <fee::Pallet<T>>::get_replace_griefing_collateral(amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod nomination {
    use crate::DefaultVaultId;

    pub fn is_nominatable<T: crate::Config>(vault_id: &DefaultVaultId<T>) -> bool {
        <nomination::Pallet<T>>::is_opted_in(vault_id)
    }
}
