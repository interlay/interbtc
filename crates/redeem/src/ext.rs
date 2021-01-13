#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod btc_relay {
    use bitcoin::types::H256Le;
    use btc_relay::BtcAddress;
    use frame_support::dispatch::{DispatchError, DispatchResult};
    use sp_std::vec::Vec;

    pub fn verify_transaction_inclusion<T: btc_relay::Config>(
        tx_id: H256Le,
        merkle_proof: Vec<u8>,
    ) -> DispatchResult {
        <btc_relay::Module<T>>::_verify_transaction_inclusion(tx_id, merkle_proof, None)
    }

    pub fn validate_transaction<T: btc_relay::Config>(
        raw_tx: Vec<u8>,
        amount: i64,
        btc_address: BtcAddress,
        redeem_id: Option<Vec<u8>>,
    ) -> Result<(BtcAddress, i64), DispatchError> {
        <btc_relay::Module<T>>::_validate_transaction(raw_tx, amount, btc_address, redeem_id)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod vault_registry {
    use crate::types::{PolkaBTC, DOT};
    use frame_support::dispatch::{DispatchError, DispatchResult};

    pub fn get_active_vault_from_id<T: vault_registry::Config>(
        vault_id: &T::AccountId,
    ) -> Result<
        vault_registry::types::Vault<T::AccountId, T::BlockNumber, PolkaBTC<T>>,
        DispatchError,
    > {
        <vault_registry::Module<T>>::get_active_vault_from_id(vault_id)
    }

    pub fn increase_to_be_redeemed_tokens<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        amount: PolkaBTC<T>,
    ) -> DispatchResult {
        <vault_registry::Module<T>>::increase_to_be_redeemed_tokens(vault_id, amount)
    }

    pub fn redeem_tokens<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        tokens: PolkaBTC<T>,
    ) -> DispatchResult {
        <vault_registry::Module<T>>::redeem_tokens(vault_id, tokens)
    }

    pub fn redeem_tokens_premium<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        tokens: PolkaBTC<T>,
        premium: DOT<T>,
        redeemer_id: &T::AccountId,
    ) -> DispatchResult {
        <vault_registry::Module<T>>::redeem_tokens_premium(vault_id, tokens, premium, redeemer_id)
    }

    pub fn redeem_tokens_liquidation<T: vault_registry::Config>(
        redeemer_id: &T::AccountId,
        amount: PolkaBTC<T>,
    ) -> DispatchResult {
        <vault_registry::Module<T>>::redeem_tokens_liquidation(redeemer_id, amount)
    }

    pub fn decrease_tokens<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        user_id: &T::AccountId,
        tokens: PolkaBTC<T>,
    ) -> DispatchResult {
        <vault_registry::Module<T>>::decrease_tokens(vault_id, user_id, tokens)
    }

    pub fn ban_vault<T: vault_registry::Config>(vault_id: T::AccountId) -> DispatchResult {
        <vault_registry::Module<T>>::ban_vault(vault_id)
    }

    pub fn ensure_not_banned<T: vault_registry::Config>(
        vault: &T::AccountId,
        height: T::BlockNumber,
    ) -> DispatchResult {
        <vault_registry::Module<T>>::_ensure_not_banned(vault, height)
    }

    pub fn is_vault_below_premium_threshold<T: vault_registry::Config>(
        vault_id: &T::AccountId,
    ) -> Result<bool, DispatchError> {
        <vault_registry::Module<T>>::is_vault_below_premium_threshold(&vault_id)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod collateral {
    use crate::types::DOT;
    use frame_support::dispatch::DispatchResult;

    pub fn slash_collateral<T: collateral::Config>(
        sender: &T::AccountId,
        receiver: &T::AccountId,
        amount: DOT<T>,
    ) -> DispatchResult {
        <collateral::Module<T>>::slash_collateral(sender.clone(), receiver.clone(), amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod sla {
    use crate::types::{PolkaBTC, DOT};
    use frame_support::dispatch::DispatchError;
    pub use sla::types::VaultEvent;

    pub fn event_update_vault_sla<T: sla::Config>(
        vault_id: T::AccountId,
        event: VaultEvent<PolkaBTC<T>>,
    ) -> Result<(), DispatchError> {
        <sla::Module<T>>::event_update_vault_sla(vault_id, event)
    }

    pub fn calculate_slashed_amount<T: sla::Config>(
        vault_id: T::AccountId,
        stake: DOT<T>,
    ) -> Result<DOT<T>, DispatchError> {
        <sla::Module<T>>::calculate_slashed_amount(vault_id, stake)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod treasury {
    use crate::types::PolkaBTC;
    use frame_support::dispatch::DispatchResult;

    pub fn get_balance<T: treasury::Config>(account: T::AccountId) -> PolkaBTC<T> {
        <treasury::Module<T>>::get_balance_from_account(account)
    }

    pub fn lock<T: treasury::Config>(
        redeemer: T::AccountId,
        amount: PolkaBTC<T>,
    ) -> DispatchResult {
        <treasury::Module<T>>::lock(redeemer, amount)
    }

    pub fn burn<T: treasury::Config>(
        redeemer: T::AccountId,
        amount: PolkaBTC<T>,
    ) -> DispatchResult {
        <treasury::Module<T>>::burn(redeemer, amount)
    }

    pub fn unlock_and_transfer<T: treasury::Config>(
        source: T::AccountId,
        destination: T::AccountId,
        amount: PolkaBTC<T>,
    ) -> DispatchResult {
        <treasury::Module<T>>::unlock_and_transfer(source, destination, amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    use frame_support::dispatch::DispatchResult;
    use primitive_types::H256;
    use security::ErrorCode;
    use sp_std::vec::Vec;

    pub fn get_secure_id<T: security::Config>(id: &T::AccountId) -> H256 {
        <security::Module<T>>::get_secure_id(id)
    }

    pub fn ensure_parachain_is_running_or_only_has_errors<T: security::Config>(
        error_codes: Vec<ErrorCode>,
    ) -> DispatchResult {
        <security::Module<T>>::ensure_parachain_is_running_or_only_has_errors(error_codes)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod oracle {
    use crate::types::{PolkaBTC, DOT};
    use frame_support::dispatch::DispatchError;

    pub fn btc_to_dots<T: exchange_rate_oracle::Config>(
        amount: PolkaBTC<T>,
    ) -> Result<DOT<T>, DispatchError> {
        <exchange_rate_oracle::Module<T>>::btc_to_dots(amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod fee {
    use crate::types::{PolkaBTC, DOT};
    use frame_support::dispatch::DispatchError;

    pub fn fee_pool_account_id<T: fee::Config>() -> T::AccountId {
        <fee::Module<T>>::fee_pool_account_id()
    }

    pub fn get_redeem_fee<T: fee::Config>(
        amount: PolkaBTC<T>,
    ) -> Result<PolkaBTC<T>, DispatchError> {
        <fee::Module<T>>::get_redeem_fee(amount)
    }

    pub fn increase_polka_btc_rewards_for_epoch<T: fee::Config>(amount: PolkaBTC<T>) {
        <fee::Module<T>>::increase_polka_btc_rewards_for_epoch(amount)
    }

    pub fn increase_dot_rewards_for_epoch<T: fee::Config>(amount: DOT<T>) {
        <fee::Module<T>>::increase_dot_rewards_for_epoch(amount)
    }

    pub fn get_punishment_fee<T: fee::Config>(amount: DOT<T>) -> Result<DOT<T>, DispatchError> {
        <fee::Module<T>>::get_punishment_fee(amount)
    }

    pub fn get_premium_redeem_fee<T: fee::Config>(amount: DOT<T>) -> Result<DOT<T>, DispatchError> {
        <fee::Module<T>>::get_premium_redeem_fee(amount)
    }
}
