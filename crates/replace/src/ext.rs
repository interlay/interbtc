#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod btc_relay {
    use bitcoin::types::H256Le;
    use btc_relay::BtcAddress;
    use frame_support::dispatch::{DispatchError, DispatchResult};
    use sp_std::vec::Vec;

    pub fn verify_transaction_inclusion<T: btc_relay::Config>(tx_id: H256Le, merkle_proof: Vec<u8>) -> DispatchResult {
        <btc_relay::Module<T>>::_verify_transaction_inclusion(tx_id, merkle_proof, None)
    }

    pub fn validate_transaction<T: btc_relay::Config>(
        raw_tx: Vec<u8>,
        minimum_btc: Option<i64>,
        btc_address: BtcAddress,
        replace_id: Option<Vec<u8>>,
    ) -> Result<(BtcAddress, i64), DispatchError> {
        <btc_relay::Module<T>>::_validate_transaction(raw_tx, minimum_btc, btc_address, replace_id)
    }

    pub fn get_best_block_height<T: btc_relay::Config>() -> u32 {
        <btc_relay::Module<T>>::get_best_block_height()
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod vault_registry {
    use crate::{PolkaBTC, DOT};
    use btc_relay::BtcAddress;
    use frame_support::dispatch::{DispatchError, DispatchResult};
    use vault_registry::types::CurrencySource;

    pub fn slash_collateral<T: vault_registry::Config>(
        from: CurrencySource<T>,
        to: CurrencySource<T>,
        amount: DOT<T>,
    ) -> DispatchResult {
        <vault_registry::Module<T>>::slash_collateral(from, to, amount)
    }
    pub fn replace_tokens<T: vault_registry::Config>(
        old_vault_id: T::AccountId,
        new_vault_id: T::AccountId,
        tokens: PolkaBTC<T>,
        collateral: DOT<T>,
    ) -> DispatchResult {
        <vault_registry::Module<T>>::replace_tokens(&old_vault_id, &new_vault_id, tokens, collateral)
    }

    pub fn get_auctionable_tokens<T: vault_registry::Config>(
        vault_id: &T::AccountId,
    ) -> Result<PolkaBTC<T>, DispatchError> {
        <vault_registry::Module<T>>::get_auctionable_tokens(vault_id)
    }

    pub fn cancel_replace_tokens<T: vault_registry::Config>(
        old_vault_id: &T::AccountId,
        new_vault_id: &T::AccountId,
        tokens: PolkaBTC<T>,
    ) -> DispatchResult {
        <vault_registry::Module<T>>::cancel_replace_tokens(old_vault_id, new_vault_id, tokens)
    }

    pub fn is_vault_liquidated<T: vault_registry::Config>(vault_id: &T::AccountId) -> Result<bool, DispatchError> {
        <vault_registry::Module<T>>::is_vault_liquidated(vault_id)
    }

    pub fn try_increase_to_be_redeemed_tokens<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        tokens: PolkaBTC<T>,
    ) -> DispatchResult {
        <vault_registry::Module<T>>::try_increase_to_be_redeemed_tokens(vault_id, tokens)
    }

    pub fn is_vault_below_auction_threshold<T: vault_registry::Config>(
        vault_id: T::AccountId,
    ) -> Result<bool, DispatchError> {
        <vault_registry::Module<T>>::is_vault_below_auction_threshold(&vault_id)
    }

    pub fn ensure_not_banned<T: vault_registry::Config>(vault: &T::AccountId) -> DispatchResult {
        <vault_registry::Module<T>>::_ensure_not_banned(vault)
    }

    pub fn insert_vault_deposit_address<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        btc_address: BtcAddress,
    ) -> DispatchResult {
        <vault_registry::Module<T>>::insert_vault_deposit_address(vault_id, btc_address)
    }

    pub fn try_increase_to_be_issued_tokens<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        amount: PolkaBTC<T>,
    ) -> Result<(), DispatchError> {
        <vault_registry::Module<T>>::try_increase_to_be_issued_tokens(vault_id, amount)
    }

    pub fn requestable_to_be_replaced_tokens<T: vault_registry::Config>(
        vault_id: &T::AccountId,
    ) -> Result<PolkaBTC<T>, DispatchError> {
        <vault_registry::Module<T>>::requestable_to_be_replaced_tokens(vault_id)
    }

    pub fn try_increase_to_be_replaced_tokens<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        amount: PolkaBTC<T>,
        griefing_collateral: DOT<T>,
    ) -> Result<(PolkaBTC<T>, DOT<T>), DispatchError> {
        <vault_registry::Module<T>>::try_increase_to_be_replaced_tokens(vault_id, amount, griefing_collateral)
    }

    pub fn decrease_to_be_replaced_tokens<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        tokens: PolkaBTC<T>,
    ) -> Result<(PolkaBTC<T>, DOT<T>), DispatchError> {
        <vault_registry::Module<T>>::decrease_to_be_replaced_tokens(vault_id, tokens)
    }

    pub fn try_lock_additional_collateral<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        amount: DOT<T>,
    ) -> Result<(), DispatchError> {
        <vault_registry::Module<T>>::try_lock_additional_collateral(vault_id, amount)
    }

    pub fn force_withdraw_collateral<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        amount: DOT<T>,
    ) -> Result<(), DispatchError> {
        <vault_registry::Module<T>>::force_withdraw_collateral(vault_id, amount)
    }

    pub fn is_allowed_to_withdraw_collateral<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        amount: DOT<T>,
    ) -> Result<bool, DispatchError> {
        <vault_registry::Module<T>>::is_allowed_to_withdraw_collateral(vault_id, amount)
    }

    pub fn calculate_collateral<T: vault_registry::Config>(
        collateral: DOT<T>,
        numerator: PolkaBTC<T>,
        denominator: PolkaBTC<T>,
    ) -> Result<DOT<T>, DispatchError> {
        <vault_registry::Module<T>>::calculate_collateral(collateral, numerator, denominator)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod collateral {
    use crate::DOT;
    use frame_support::dispatch::DispatchResult;

    pub fn release_collateral<T: collateral::Config>(sender: &T::AccountId, amount: DOT<T>) -> DispatchResult {
        <collateral::Module<T>>::release_collateral(sender, amount)
    }

    pub fn lock_collateral<T: collateral::Config>(sender: T::AccountId, amount: DOT<T>) -> DispatchResult {
        <collateral::Module<T>>::lock_collateral(&sender, amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    use frame_support::dispatch::{DispatchError, DispatchResult};
    use primitive_types::H256;

    pub fn get_secure_id<T: security::Config>(id: &T::AccountId) -> H256 {
        <security::Module<T>>::get_secure_id(id)
    }

    pub fn ensure_parachain_status_running<T: security::Config>() -> DispatchResult {
        <security::Module<T>>::ensure_parachain_status_running()
    }

    pub fn active_block_number<T: security::Config>() -> T::BlockNumber {
        <security::Module<T>>::active_block_number()
    }

    pub fn has_expired<T: security::Config>(
        opentime: T::BlockNumber,
        period: T::BlockNumber,
    ) -> Result<bool, DispatchError> {
        <security::Module<T>>::has_expired(opentime, period)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod oracle {
    use crate::types::{PolkaBTC, DOT};
    use frame_support::dispatch::DispatchError;

    pub fn btc_to_dots<T: exchange_rate_oracle::Config>(amount: PolkaBTC<T>) -> Result<DOT<T>, DispatchError> {
        <exchange_rate_oracle::Module<T>>::btc_to_dots(amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod fee {
    use crate::types::DOT;
    use frame_support::dispatch::DispatchError;

    pub fn get_replace_griefing_collateral<T: fee::Config>(amount: DOT<T>) -> Result<DOT<T>, DispatchError> {
        <fee::Module<T>>::get_replace_griefing_collateral(amount)
    }

    pub fn get_auction_redeem_fee<T: fee::Config>(amount: DOT<T>) -> Result<DOT<T>, DispatchError> {
        <fee::Module<T>>::get_auction_redeem_fee(amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod nomination {
    use sp_runtime::DispatchError;

    use crate::types::DOT;

    pub fn is_nomination_enabled<T: nomination::Config>() -> Result<bool, DispatchError> {
        <nomination::Module<T>>::is_nomination_enabled()
    }

    pub fn get_total_nominated_collateral<T: nomination::Config>(
        operator_id: &T::AccountId,
    ) -> Result<DOT<T>, DispatchError> {
        <nomination::Module<T>>::get_total_nominated_collateral(operator_id)
    }

    pub fn is_operator<T: nomination::Config>(
        operator_id: &T::AccountId,
    ) -> Result<bool, DispatchError> {
        <nomination::Module<T>>::is_operator(operator_id)
    }
}
