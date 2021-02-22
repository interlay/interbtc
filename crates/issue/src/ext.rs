#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod btc_relay {
    use bitcoin::types::H256Le;
    use btc_relay::BtcAddress;
    use frame_support::dispatch::DispatchError;
    use sp_std::vec::Vec;

    pub fn verify_transaction_inclusion<T: btc_relay::Config>(
        tx_id: H256Le,
        merkle_proof: Vec<u8>,
    ) -> Result<(), DispatchError> {
        <btc_relay::Module<T>>::_verify_transaction_inclusion(tx_id, merkle_proof, None)
    }

    pub fn validate_transaction<T: btc_relay::Config>(
        raw_tx: Vec<u8>,
        amount: i64,
        btc_address: BtcAddress,
        issue_id: Option<Vec<u8>>,
    ) -> Result<(BtcAddress, i64), DispatchError> {
        <btc_relay::Module<T>>::_validate_transaction(raw_tx, amount, btc_address, issue_id)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod vault_registry {
    use crate::types::PolkaBTC;
    use btc_relay::BtcAddress;
    use frame_support::dispatch::{DispatchError, DispatchResult};
    use sp_core::H256;

    pub fn get_active_vault_from_id<T: vault_registry::Config>(
        vault_id: &T::AccountId,
    ) -> Result<
        vault_registry::types::Vault<T::AccountId, T::BlockNumber, PolkaBTC<T>>,
        DispatchError,
    > {
        <vault_registry::Module<T>>::get_active_vault_from_id(vault_id)
    }

    pub fn increase_to_be_issued_tokens<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        amount: PolkaBTC<T>,
    ) -> Result<(), DispatchError> {
        <vault_registry::Module<T>>::increase_to_be_issued_tokens(vault_id, amount)
    }

    pub fn _register_address<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        secure_id: H256,
    ) -> Result<BtcAddress, DispatchError> {
        <vault_registry::Module<T>>::_register_address(vault_id, secure_id)
    }

    pub fn issue_tokens<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        amount: PolkaBTC<T>,
    ) -> DispatchResult {
        <vault_registry::Module<T>>::issue_tokens(vault_id, amount)
    }

    pub fn decrease_to_be_issued_tokens<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        amount: PolkaBTC<T>,
    ) -> DispatchResult {
        <vault_registry::Module<T>>::decrease_to_be_issued_tokens(vault_id, amount)
    }

    pub fn ensure_not_banned<T: vault_registry::Config>(
        vault: &T::AccountId,
        height: T::BlockNumber,
    ) -> DispatchResult {
        <vault_registry::Module<T>>::_ensure_not_banned(vault, height)
    }

    pub fn liquidation_vault_force_decrease_to_be_issued_tokens<T: vault_registry::Config>(
        amount: PolkaBTC<T>,
    ) -> DispatchResult {
        <vault_registry::Module<T>>::liquidation_vault_force_decrease_to_be_issued_tokens(amount)
    }

    pub fn liquidation_vault_force_increase_issued_tokens<T: vault_registry::Config>(
        amount: PolkaBTC<T>,
    ) -> DispatchResult {
        <vault_registry::Module<T>>::liquidation_vault_force_increase_issued_tokens(amount)
    }

    pub fn get_vault_from_id<T: vault_registry::Config>(
        vault_id: &T::AccountId,
    ) -> Result<
        vault_registry::types::Vault<T::AccountId, T::BlockNumber, PolkaBTC<T>>,
        DispatchError,
    > {
        <vault_registry::Module<T>>::get_vault_from_id(vault_id)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod collateral {
    use crate::types::DOT;
    use frame_support::dispatch::DispatchResult;

    pub fn lock_collateral<T: collateral::Config>(
        sender: &T::AccountId,
        amount: DOT<T>,
    ) -> DispatchResult {
        <collateral::Module<T>>::lock_collateral(sender, amount)
    }

    pub fn slash_collateral<T: collateral::Config>(
        sender: &T::AccountId,
        receiver: &T::AccountId,
        amount: DOT<T>,
    ) -> DispatchResult {
        <collateral::Module<T>>::slash_collateral(sender.clone(), receiver.clone(), amount)
    }

    pub fn release_collateral<T: collateral::Config>(
        sender: &T::AccountId,
        amount: DOT<T>,
    ) -> DispatchResult {
        <collateral::Module<T>>::release_collateral(sender, amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod treasury {
    use crate::types::PolkaBTC;

    pub fn mint<T: treasury::Config>(requester: T::AccountId, amount: PolkaBTC<T>) {
        <treasury::Module<T>>::mint(requester, amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    use frame_support::dispatch::DispatchResult;
    use primitive_types::H256;

    pub fn get_secure_id<T: security::Config>(id: &T::AccountId) -> H256 {
        <security::Module<T>>::get_secure_id(id)
    }

    pub fn ensure_parachain_status_running<T: security::Config>() -> DispatchResult {
        <security::Module<T>>::ensure_parachain_status_running()
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
pub(crate) mod sla {
    use crate::types::PolkaBTC;
    use frame_support::dispatch::DispatchError;
    pub use sla::types::VaultEvent;

    pub fn event_update_vault_sla<T: sla::Config>(
        vault_id: T::AccountId,
        event: VaultEvent<PolkaBTC<T>>,
    ) -> Result<(), DispatchError> {
        <sla::Module<T>>::event_update_vault_sla(vault_id, event)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod fee {
    use crate::types::{PolkaBTC, DOT};
    use frame_support::dispatch::DispatchError;

    pub fn fee_pool_account_id<T: fee::Config>() -> T::AccountId {
        <fee::Module<T>>::fee_pool_account_id()
    }

    pub fn get_issue_fee<T: fee::Config>(
        amount: PolkaBTC<T>,
    ) -> Result<PolkaBTC<T>, DispatchError> {
        <fee::Module<T>>::get_issue_fee(amount)
    }

    pub fn get_issue_fee_from_total<T: fee::Config>(
        amount: PolkaBTC<T>,
    ) -> Result<PolkaBTC<T>, DispatchError> {
        <fee::Module<T>>::get_issue_fee_from_total(amount)
    }

    pub fn get_issue_griefing_collateral<T: fee::Config>(
        amount: DOT<T>,
    ) -> Result<DOT<T>, DispatchError> {
        <fee::Module<T>>::get_issue_griefing_collateral(amount)
    }

    pub fn increase_polka_btc_rewards_for_epoch<T: fee::Config>(amount: PolkaBTC<T>) {
        <fee::Module<T>>::increase_polka_btc_rewards_for_epoch(amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod refund {
    use crate::types::PolkaBTC;
    use btc_relay::BtcAddress;
    use frame_support::dispatch::DispatchError;
    use primitive_types::H256;

    pub fn request_refund<T: refund::Config>(
        total_amount_btc: PolkaBTC<T>,
        vault_id: T::AccountId,
        issuer: T::AccountId,
        btc_address: BtcAddress,
        issue_id: H256,
    ) -> Result<(), DispatchError> {
        <refund::Module<T>>::request_refund(
            total_amount_btc,
            vault_id,
            issuer,
            btc_address,
            issue_id,
        )
    }
}
