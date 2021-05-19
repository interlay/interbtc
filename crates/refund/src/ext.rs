#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod fee {
    use crate::types::Issuing;
    use frame_support::dispatch::DispatchError;

    pub fn get_refund_fee_from_total<T: fee::Config>(amount: Issuing<T>) -> Result<Issuing<T>, DispatchError> {
        <fee::Pallet<T>>::get_refund_fee_from_total(amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod sla {
    use crate::types::{Backing, Issuing};
    use frame_support::dispatch::DispatchError;
    pub use sla::types::VaultEvent;

    pub fn event_update_vault_sla<T: sla::Config>(
        vault_id: &T::AccountId,
        event: VaultEvent<Issuing<T>, Backing<T>>,
    ) -> Result<(), DispatchError> {
        <sla::Pallet<T>>::event_update_vault_sla(vault_id, event)
    }
}

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
}

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    use frame_support::dispatch::DispatchResult;
    use sp_core::H256;

    pub fn get_secure_id<T: security::Config>(id: &T::AccountId) -> H256 {
        <security::Pallet<T>>::get_secure_id(id)
    }

    pub fn ensure_parachain_status_not_shutdown<T: security::Config>() -> DispatchResult {
        <security::Pallet<T>>::ensure_parachain_status_not_shutdown()
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod treasury {
    use crate::types::Issuing;

    type TreasuryPallet<T> = currency::Pallet<T, currency::Issuing>;

    pub fn mint<T: currency::Config<currency::Issuing>>(requester: T::AccountId, amount: Issuing<T>) {
        TreasuryPallet::<T>::mint(requester, amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod vault_registry {
    use crate::types::Issuing;
    use frame_support::dispatch::{DispatchError, DispatchResult};

    pub fn try_increase_to_be_issued_tokens<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        amount: Issuing<T>,
    ) -> Result<(), DispatchError> {
        <vault_registry::Pallet<T>>::try_increase_to_be_issued_tokens(vault_id, amount)
    }

    pub fn issue_tokens<T: vault_registry::Config>(vault_id: &T::AccountId, amount: Issuing<T>) -> DispatchResult {
        <vault_registry::Pallet<T>>::issue_tokens(vault_id, amount)
    }
}
