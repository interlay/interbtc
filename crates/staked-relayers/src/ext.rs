#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod collateral {
    use crate::types::DOT;
    use frame_support::dispatch::{DispatchError, DispatchResult};

    pub(crate) fn lock_collateral<T: collateral::Trait>(
        sender: &T::AccountId,
        amount: DOT<T>,
    ) -> Result<(), DispatchError> {
        <collateral::Module<T>>::lock_collateral(sender, amount)
    }

    pub(crate) fn release_collateral<T: collateral::Trait>(
        sender: &T::AccountId,
        amount: DOT<T>,
    ) -> Result<(), DispatchError> {
        <collateral::Module<T>>::release_collateral(sender, amount)
    }

    pub fn slash_collateral<T: collateral::Trait>(
        sender: T::AccountId,
        receiver: T::AccountId,
        amount: DOT<T>,
    ) -> DispatchResult {
        <collateral::Module<T>>::slash_collateral(sender, receiver, amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod oracle {
    pub(crate) fn is_max_delay_passed<T: exchange_rate_oracle::Trait>() -> bool {
        <exchange_rate_oracle::Module<T>>::is_max_delay_passed()
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod vault_registry {
    use crate::PolkaBTC;
    use ::vault_registry::VaultStatus;
    use frame_support::dispatch::{DispatchError, DispatchResult};

    pub fn get_vault_from_id<T: vault_registry::Trait>(
        vault_id: &T::AccountId,
    ) -> Result<
        vault_registry::types::Vault<T::AccountId, T::BlockNumber, PolkaBTC<T>>,
        DispatchError,
    > {
        <vault_registry::Module<T>>::get_vault_from_id(vault_id)
    }

    pub fn is_vault_below_liquidation_threshold<T: vault_registry::Trait>(
        vault_id: &T::AccountId,
    ) -> Result<bool, DispatchError> {
        <vault_registry::Module<T>>::is_vault_below_liquidation_threshold(vault_id)
    }

    pub fn liquidate_vault<T: vault_registry::Trait>(vault_id: &T::AccountId) -> DispatchResult {
        <vault_registry::Module<T>>::liquidate_vault(vault_id)
    }

    pub fn liquidate_theft_vault<T: vault_registry::Trait>(
        vault_id: &T::AccountId,
    ) -> DispatchResult {
        <vault_registry::Module<T>>::liquidate_vault_with_status(
            vault_id,
            VaultStatus::CommittedTheft,
        )
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    use security::types::{ErrorCode, StatusCode};
    use sp_std::collections::btree_set::BTreeSet;

    pub(crate) fn get_parachain_status<T: security::Trait>() -> StatusCode {
        <security::Module<T>>::get_parachain_status()
    }

    pub(crate) fn set_parachain_status<T: security::Trait>(status_code: StatusCode) {
        <security::Module<T>>::set_parachain_status(status_code)
    }

    pub(crate) fn insert_error<T: security::Trait>(error_code: ErrorCode) {
        <security::Module<T>>::insert_error(error_code)
    }

    pub(crate) fn remove_error<T: security::Trait>(error_code: ErrorCode) {
        <security::Module<T>>::remove_error(error_code)
    }

    pub(crate) fn get_errors<T: security::Trait>() -> BTreeSet<ErrorCode> {
        <security::Module<T>>::get_errors()
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod btc_relay {
    use bitcoin::types::{H256Le, Transaction};
    use btc_relay::BtcAddress;
    use frame_support::dispatch::DispatchResult;
    use security::types::ErrorCode;
    use sp_std::prelude::*;

    pub(crate) fn flag_block_error<T: btc_relay::Trait>(
        block_hash: H256Le,
        error: ErrorCode,
    ) -> DispatchResult {
        <btc_relay::Module<T>>::flag_block_error(block_hash, error)
    }

    pub(crate) fn clear_block_error<T: btc_relay::Trait>(
        block_hash: H256Le,
        error: ErrorCode,
    ) -> DispatchResult {
        <btc_relay::Module<T>>::clear_block_error(block_hash, error)
    }

    pub(crate) fn verify_transaction_inclusion<T: btc_relay::Trait>(
        tx_id: H256Le,
        raw_merkle_proof: Vec<u8>,
    ) -> DispatchResult {
        <btc_relay::Module<T>>::_verify_transaction_inclusion(tx_id, raw_merkle_proof, None)
    }

    pub(crate) fn block_header_exists<T: btc_relay::Trait>(block_hash: H256Le) -> bool {
        <btc_relay::Module<T>>::block_header_exists(block_hash)
    }

    pub(crate) fn extract_outputs<T: btc_relay::Trait>(
        tx: Transaction,
    ) -> Result<(Vec<(i64, BtcAddress)>, Vec<(i64, Vec<u8>)>), btc_relay::Error<T>> {
        <btc_relay::Module<T>>::extract_outputs(tx)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod redeem {
    use crate::types::{PolkaBTC, DOT};
    use frame_support::dispatch::DispatchError;
    use primitive_types::H256;
    use redeem::types::RedeemRequest;

    pub(crate) fn get_open_or_completed_redeem_request_from_id<T: redeem::Trait>(
        id: &H256,
    ) -> Result<RedeemRequest<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>, DispatchError>
    {
        <redeem::Module<T>>::get_open_or_completed_redeem_request_from_id(id)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod replace {
    use crate::types::{PolkaBTC, DOT};
    use frame_support::dispatch::DispatchError;
    use primitive_types::H256;
    use replace::types::ReplaceRequest;

    pub(crate) fn get_open_or_completed_replace_request<T: replace::Trait>(
        id: &H256,
    ) -> Result<ReplaceRequest<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>, DispatchError>
    {
        <replace::Module<T>>::get_open_or_completed_replace_request(id)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod refund {
    use crate::types::PolkaBTC;
    use frame_support::dispatch::DispatchError;
    use primitive_types::H256;
    use refund::types::RefundRequest;

    pub(crate) fn get_open_or_completed_refund_request_from_id<T: refund::Trait>(
        id: &H256,
    ) -> Result<RefundRequest<T::AccountId, PolkaBTC<T>>, DispatchError> {
        <refund::Module<T>>::get_open_or_completed_refund_request_from_id(id)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod sla {
    use frame_support::dispatch::DispatchError;
    pub use sla::types::RelayerEvent;

    pub fn event_update_relayer_sla<T: sla::Trait>(
        relayer_id: T::AccountId,
        event: RelayerEvent,
    ) -> Result<(), DispatchError> {
        <sla::Module<T>>::event_update_relayer_sla(relayer_id, event)
    }
}
