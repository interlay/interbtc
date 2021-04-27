#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod btc_relay {
    use bitcoin::types::H256Le;
    use btc_relay::BtcAddress;
    use frame_support::dispatch::{DispatchError, DispatchResult};
    use sp_std::vec::Vec;

    pub fn verify_transaction_inclusion<T: btc_relay::Config>(tx_id: H256Le, merkle_proof: Vec<u8>) -> DispatchResult {
        <btc_relay::Pallet<T>>::_verify_transaction_inclusion(tx_id, merkle_proof, None)
    }

    pub fn validate_transaction<T: btc_relay::Config>(
        raw_tx: Vec<u8>,
        minimum_btc: Option<i64>,
        btc_address: BtcAddress,
        redeem_id: Option<Vec<u8>>,
    ) -> Result<(BtcAddress, i64), DispatchError> {
        <btc_relay::Pallet<T>>::_validate_transaction(raw_tx, minimum_btc, btc_address, redeem_id)
    }

    pub fn get_best_block_height<T: btc_relay::Config>() -> u32 {
        <btc_relay::Pallet<T>>::get_best_block_height()
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod vault_registry {
    use crate::types::{PolkaBTC, DOT};
    use frame_support::dispatch::{DispatchError, DispatchResult};
    use vault_registry::types::CurrencySource;

    pub fn get_backing_collateral<T: vault_registry::Config>(vault_id: &T::AccountId) -> Result<DOT<T>, DispatchError> {
        <vault_registry::Pallet<T>>::get_backing_collateral(vault_id)
    }

    pub fn slash_collateral<T: vault_registry::Config>(
        from: CurrencySource<T>,
        to: CurrencySource<T>,
        amount: DOT<T>,
    ) -> DispatchResult {
        <vault_registry::Pallet<T>>::slash_collateral(from, to, amount)
    }

    pub fn slash_collateral_saturated<T: vault_registry::Config>(
        from: CurrencySource<T>,
        to: CurrencySource<T>,
        amount: DOT<T>,
    ) -> Result<DOT<T>, DispatchError> {
        <vault_registry::Pallet<T>>::slash_collateral_saturated(from, to, amount)
    }

    pub fn get_vault_from_id<T: vault_registry::Config>(
        vault_id: &T::AccountId,
    ) -> Result<vault_registry::types::Vault<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>, DispatchError> {
        <vault_registry::Pallet<T>>::get_vault_from_id(vault_id)
    }

    pub fn try_increase_to_be_redeemed_tokens<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        amount: PolkaBTC<T>,
    ) -> DispatchResult {
        <vault_registry::Pallet<T>>::try_increase_to_be_redeemed_tokens(vault_id, amount)
    }

    pub fn redeem_tokens<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        tokens: PolkaBTC<T>,
        premium: DOT<T>,
        redeemer_id: &T::AccountId,
    ) -> DispatchResult {
        <vault_registry::Pallet<T>>::redeem_tokens(vault_id, tokens, premium, redeemer_id)
    }

    pub fn decrease_tokens<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        user_id: &T::AccountId,
        tokens: PolkaBTC<T>,
    ) -> DispatchResult {
        <vault_registry::Pallet<T>>::decrease_tokens(vault_id, user_id, tokens)
    }

    pub fn redeem_tokens_liquidation<T: vault_registry::Config>(
        redeemer_id: &T::AccountId,
        amount: PolkaBTC<T>,
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
        tokens: PolkaBTC<T>,
    ) -> DispatchResult {
        <vault_registry::Pallet<T>>::decrease_to_be_redeemed_tokens(vault_id, tokens)
    }

    pub fn calculate_collateral<T: vault_registry::Config>(
        collateral: DOT<T>,
        numerator: PolkaBTC<T>,
        denominator: PolkaBTC<T>,
    ) -> Result<DOT<T>, DispatchError> {
        <vault_registry::Pallet<T>>::calculate_collateral(collateral, numerator, denominator)
    }

    pub fn try_increase_to_be_issued_tokens<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        amount: PolkaBTC<T>,
    ) -> Result<(), DispatchError> {
        <vault_registry::Pallet<T>>::try_increase_to_be_issued_tokens(vault_id, amount)
    }

    pub fn issue_tokens<T: vault_registry::Config>(vault_id: &T::AccountId, amount: PolkaBTC<T>) -> DispatchResult {
        <vault_registry::Pallet<T>>::issue_tokens(vault_id, amount)
    }

    pub fn decrease_to_be_replaced_tokens<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        tokens: PolkaBTC<T>,
    ) -> Result<(PolkaBTC<T>, DOT<T>), DispatchError> {
        <vault_registry::Pallet<T>>::decrease_to_be_replaced_tokens(vault_id, tokens)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod sla {
    use crate::types::{PolkaBTC, DOT};
    use frame_support::dispatch::DispatchError;
    pub use sla::types::VaultEvent;

    pub fn event_update_vault_sla<T: sla::Config>(
        vault_id: &T::AccountId,
        event: VaultEvent<PolkaBTC<T>>,
    ) -> Result<(), DispatchError> {
        <sla::Pallet<T>>::event_update_vault_sla(vault_id, event)
    }

    pub fn calculate_slashed_amount<T: sla::Config>(
        vault_id: &T::AccountId,
        stake: DOT<T>,
        reimburse: bool,
    ) -> Result<DOT<T>, DispatchError> {
        <sla::Pallet<T>>::calculate_slashed_amount(vault_id, stake, reimburse)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod treasury {
    use crate::types::PolkaBTC;
    use frame_support::dispatch::DispatchResult;

    pub fn get_balance<T: treasury::Config>(account: T::AccountId) -> PolkaBTC<T> {
        <treasury::Pallet<T>>::get_balance_from_account(account)
    }

    pub fn lock<T: treasury::Config>(redeemer: T::AccountId, amount: PolkaBTC<T>) -> DispatchResult {
        <treasury::Pallet<T>>::lock(redeemer, amount)
    }

    pub fn unlock<T: treasury::Config>(account: T::AccountId, amount: PolkaBTC<T>) -> DispatchResult {
        <treasury::Pallet<T>>::unlock(account, amount)
    }

    pub fn burn<T: treasury::Config>(redeemer: T::AccountId, amount: PolkaBTC<T>) -> DispatchResult {
        <treasury::Pallet<T>>::burn(redeemer, amount)
    }

    pub fn unlock_and_transfer<T: treasury::Config>(
        source: T::AccountId,
        destination: T::AccountId,
        amount: PolkaBTC<T>,
    ) -> DispatchResult {
        <treasury::Pallet<T>>::unlock_and_transfer(source, destination, amount)
    }

    pub fn mint<T: treasury::Config>(requester: T::AccountId, amount: PolkaBTC<T>) {
        <treasury::Pallet<T>>::mint(requester, amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    use frame_support::dispatch::DispatchError;
    use primitive_types::H256;

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
    use crate::types::{PolkaBTC, DOT};
    use exchange_rate_oracle::BtcTxFeesPerByte;
    use frame_support::dispatch::DispatchError;

    pub fn satoshi_per_bytes<T: fee::Config>() -> BtcTxFeesPerByte {
        <exchange_rate_oracle::Pallet<T>>::satoshi_per_bytes()
    }

    pub fn btc_to_dots<T: exchange_rate_oracle::Config>(amount: PolkaBTC<T>) -> Result<DOT<T>, DispatchError> {
        <exchange_rate_oracle::Pallet<T>>::btc_to_dots(amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod fee {
    use crate::types::{PolkaBTC, DOT};
    use frame_support::dispatch::DispatchError;

    pub fn fee_pool_account_id<T: fee::Config>() -> T::AccountId {
        <fee::Pallet<T>>::fee_pool_account_id()
    }

    pub fn get_redeem_fee<T: fee::Config>(amount: PolkaBTC<T>) -> Result<PolkaBTC<T>, DispatchError> {
        <fee::Pallet<T>>::get_redeem_fee(amount)
    }

    pub fn increase_polka_btc_rewards_for_epoch<T: fee::Config>(amount: PolkaBTC<T>) {
        <fee::Pallet<T>>::increase_polka_btc_rewards_for_epoch(amount)
    }

    pub fn increase_dot_rewards_for_epoch<T: fee::Config>(amount: DOT<T>) {
        <fee::Pallet<T>>::increase_dot_rewards_for_epoch(amount)
    }

    pub fn get_punishment_fee<T: fee::Config>(amount: DOT<T>) -> Result<DOT<T>, DispatchError> {
        <fee::Pallet<T>>::get_punishment_fee(amount)
    }

    pub fn get_premium_redeem_fee<T: fee::Config>(amount: DOT<T>) -> Result<DOT<T>, DispatchError> {
        <fee::Pallet<T>>::get_premium_redeem_fee(amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod collateral {
    use crate::DOT;
    use frame_support::dispatch::DispatchResult;

    pub fn release_collateral<T: collateral::Config>(sender: &T::AccountId, amount: DOT<T>) -> DispatchResult {
        <collateral::Pallet<T>>::release_collateral(sender, amount)
    }
}
