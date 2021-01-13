#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod fee {
    use crate::types::PolkaBTC;
    use frame_support::dispatch::DispatchError;

    pub fn get_refund_fee_from_total<T: fee::Config>(
        amount: PolkaBTC<T>,
    ) -> Result<PolkaBTC<T>, DispatchError> {
        <fee::Module<T>>::get_refund_fee_from_total(amount)
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
        refund_id: Option<Vec<u8>>,
    ) -> Result<(BtcAddress, i64), DispatchError> {
        <btc_relay::Module<T>>::_validate_transaction(raw_tx, amount, btc_address, refund_id)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    use primitive_types::H256;

    pub fn get_secure_id<T: security::Config>(id: &T::AccountId) -> H256 {
        <security::Module<T>>::get_secure_id(id)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod treasury {
    use crate::types::PolkaBTC;

    pub fn mint<T: treasury::Config>(requester: T::AccountId, amount: PolkaBTC<T>) {
        <treasury::Module<T>>::mint(requester, amount)
    }
}
