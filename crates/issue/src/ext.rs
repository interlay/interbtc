#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod btc_relay {
    use bitcoin::types::H256Le;
    use btc_relay::BtcAddress;
    use frame_support::dispatch::DispatchError;
    use sp_std::vec::Vec;

    pub fn verify_transaction_inclusion<T: btc_relay::Trait>(
        tx_id: H256Le,
        merkle_proof: Vec<u8>,
    ) -> Result<(), DispatchError> {
        <btc_relay::Module<T>>::_verify_transaction_inclusion(tx_id, merkle_proof, 0, false)
    }

    pub fn validate_transaction<T: btc_relay::Trait>(
        raw_tx: Vec<u8>,
        amount: i64,
        btc_address: BtcAddress,
        issue_id: Vec<u8>,
    ) -> Result<(), DispatchError> {
        <btc_relay::Module<T>>::_validate_transaction(raw_tx, amount, btc_address, issue_id)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod vault_registry {
    use crate::types::PolkaBTC;
    use btc_relay::BtcAddress;
    use frame_support::dispatch::{DispatchError, DispatchResult};

    pub fn get_vault_from_id<T: vault_registry::Trait>(
        vault_id: &T::AccountId,
    ) -> Result<
        vault_registry::types::Vault<T::AccountId, T::BlockNumber, PolkaBTC<T>>,
        DispatchError,
    > {
        <vault_registry::Module<T>>::_get_vault_from_id(vault_id)
    }

    pub fn increase_to_be_issued_tokens<T: vault_registry::Trait>(
        vault_id: &T::AccountId,
        amount: PolkaBTC<T>,
    ) -> Result<BtcAddress, DispatchError> {
        <vault_registry::Module<T>>::_increase_to_be_issued_tokens(vault_id, amount)
    }

    pub fn issue_tokens<T: vault_registry::Trait>(
        vault_id: &T::AccountId,
        amount: PolkaBTC<T>,
    ) -> DispatchResult {
        <vault_registry::Module<T>>::_issue_tokens(vault_id, amount)
    }

    pub fn decrease_to_be_issued_tokens<T: vault_registry::Trait>(
        vault_id: &T::AccountId,
        amount: PolkaBTC<T>,
    ) -> DispatchResult {
        <vault_registry::Module<T>>::_decrease_to_be_issued_tokens(vault_id, amount)
    }

    pub fn ensure_not_banned<T: vault_registry::Trait>(
        vault: &T::AccountId,
        height: T::BlockNumber,
    ) -> DispatchResult {
        <vault_registry::Module<T>>::_ensure_not_banned(vault, height)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod collateral {
    use crate::types::DOT;
    use frame_support::dispatch::DispatchResult;

    pub fn lock_collateral<T: collateral::Trait>(
        sender: &T::AccountId,
        amount: DOT<T>,
    ) -> DispatchResult {
        <collateral::Module<T>>::lock_collateral(sender, amount)
    }

    pub fn slash_collateral<T: collateral::Trait>(
        sender: &T::AccountId,
        receiver: &T::AccountId,
        amount: DOT<T>,
    ) -> DispatchResult {
        <collateral::Module<T>>::slash_collateral(sender.clone(), receiver.clone(), amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod treasury {
    use crate::types::PolkaBTC;

    pub fn mint<T: treasury::Trait>(requester: T::AccountId, amount: PolkaBTC<T>) {
        <treasury::Module<T>>::mint(requester, amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    use frame_support::dispatch::DispatchResult;
    use primitive_types::H256;

    pub fn get_secure_id<T: security::Trait>(id: &T::AccountId) -> H256 {
        <security::Module<T>>::get_secure_id(id)
    }

    pub fn ensure_parachain_status_running<T: security::Trait>() -> DispatchResult {
        <security::Module<T>>::ensure_parachain_status_running()
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod oracle {
    use crate::types::{PolkaBTC, DOT};
    use frame_support::dispatch::DispatchError;

    pub fn btc_to_dots<T: exchange_rate_oracle::Trait>(
        amount: PolkaBTC<T>,
    ) -> Result<DOT<T>, DispatchError> {
        <exchange_rate_oracle::Module<T>>::btc_to_dots(amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod sla {
    use frame_support::dispatch::DispatchError;
    pub use sla::types::VaultEvent;

    pub fn event_update_vault_sla<T: sla::Trait>(
        vault_id: T::AccountId,
        event: VaultEvent,
    ) -> Result<(), DispatchError> {
        <sla::Module<T>>::event_update_vault_sla(vault_id, event)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod fee {
    use crate::types::{PolkaBTC, DOT};
    use frame_support::dispatch::DispatchError;

    pub fn account_id<T: fee::Trait>() -> T::AccountId {
        <fee::Module<T>>::account_id()
    }

    pub fn get_issue_fee<T: fee::Trait>(amount: PolkaBTC<T>) -> Result<PolkaBTC<T>, DispatchError> {
        <fee::Module<T>>::get_issue_fee(amount)
    }

    pub fn get_issue_griefing_collateral<T: fee::Trait>(
        amount: DOT<T>,
    ) -> Result<DOT<T>, DispatchError> {
        <fee::Module<T>>::get_issue_griefing_collateral(amount)
    }

    pub fn increase_rewards_for_epoch<T: fee::Trait>(amount: PolkaBTC<T>) {
        <fee::Module<T>>::increase_rewards_for_epoch(amount)
    }
}
