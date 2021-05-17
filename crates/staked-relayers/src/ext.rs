#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod collateral {
    use crate::types::Backing;
    use frame_support::dispatch::{DispatchError, DispatchResult};

    type CollateralPallet<T> = currency::Pallet<T, currency::Collateral>;

    pub(crate) fn lock_collateral<T: currency::Config<currency::Collateral>>(
        sender: &T::AccountId,
        amount: Backing<T>,
    ) -> Result<(), DispatchError> {
        CollateralPallet::<T>::lock(sender, amount)
    }

    pub(crate) fn release_collateral<T: currency::Config<currency::Collateral>>(
        sender: &T::AccountId,
        amount: Backing<T>,
    ) -> Result<(), DispatchError> {
        CollateralPallet::<T>::release(sender, amount)
    }

    pub fn slash_collateral<T: currency::Config<currency::Collateral>>(
        sender: T::AccountId,
        receiver: T::AccountId,
        amount: Backing<T>,
    ) -> DispatchResult {
        CollateralPallet::<T>::slash(sender, receiver, amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod vault_registry {
    use crate::{Backing, Issuing};
    use ::vault_registry::VaultStatus;
    use frame_support::dispatch::{DispatchError, DispatchResult};
    use vault_registry::types::Vault;

    pub fn get_active_vault_from_id<T: vault_registry::Config>(
        vault_id: &T::AccountId,
    ) -> Result<Vault<T::AccountId, T::BlockNumber, Issuing<T>, Backing<T>, T::SignedFixedPoint>, DispatchError> {
        <vault_registry::Pallet<T>>::get_active_vault_from_id(vault_id)
    }

    pub fn liquidate_theft_vault<T: vault_registry::Config>(vault_id: &T::AccountId) -> DispatchResult {
        let _ = <vault_registry::Pallet<T>>::liquidate_vault_with_status(vault_id, VaultStatus::CommittedTheft)?;
        Ok(())
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    use frame_support::dispatch::DispatchResult;
    use security::types::ErrorCode;
    use sp_std::collections::btree_set::BTreeSet;

    #[allow(dead_code)]
    pub(crate) fn get_errors<T: security::Config>() -> BTreeSet<ErrorCode> {
        <security::Pallet<T>>::get_errors()
    }

    pub fn ensure_parachain_status_not_shutdown<T: security::Config>() -> DispatchResult {
        <security::Pallet<T>>::ensure_parachain_status_not_shutdown()
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod fee {
    pub fn fee_pool_account_id<T: fee::Config>() -> T::AccountId {
        <fee::Pallet<T>>::fee_pool_account_id()
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod btc_relay {
    use bitcoin::types::{H256Le, RawBlockHeader, Transaction};
    use btc_relay::BtcAddress;
    use frame_support::dispatch::DispatchResult;
    use sp_std::prelude::*;

    pub fn initialize<T: btc_relay::Config>(
        relayer: T::AccountId,
        raw_block_header: RawBlockHeader,
        block_height: u32,
    ) -> DispatchResult {
        <btc_relay::Pallet<T>>::initialize(relayer, raw_block_header, block_height)
    }

    pub fn store_block_header<T: btc_relay::Config>(
        relayer: &T::AccountId,
        raw_block_header: RawBlockHeader,
    ) -> DispatchResult {
        <btc_relay::Pallet<T>>::store_block_header(relayer, raw_block_header)
    }

    pub(crate) fn verify_transaction_inclusion<T: btc_relay::Config>(
        tx_id: H256Le,
        raw_merkle_proof: Vec<u8>,
    ) -> DispatchResult {
        <btc_relay::Pallet<T>>::_verify_transaction_inclusion(tx_id, raw_merkle_proof, None)
    }

    pub(crate) fn extract_outputs<T: btc_relay::Config>(
        tx: Transaction,
    ) -> Result<(Vec<(i64, BtcAddress)>, Vec<(i64, Vec<u8>)>), btc_relay::Error<T>> {
        <btc_relay::Pallet<T>>::extract_outputs(tx)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod redeem {
    use crate::types::{Backing, Issuing};
    use frame_support::dispatch::DispatchError;
    use primitive_types::H256;
    use redeem::types::RedeemRequest;

    pub(crate) fn get_open_or_completed_redeem_request_from_id<T: redeem::Config>(
        id: &H256,
    ) -> Result<RedeemRequest<T::AccountId, T::BlockNumber, Issuing<T>, Backing<T>>, DispatchError> {
        <redeem::Pallet<T>>::get_open_or_completed_redeem_request_from_id(id)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod replace {
    use crate::types::{Backing, Issuing};
    use frame_support::dispatch::DispatchError;
    use primitive_types::H256;
    use replace::types::ReplaceRequest;

    pub(crate) fn get_open_or_completed_replace_request<T: replace::Config>(
        id: &H256,
    ) -> Result<ReplaceRequest<T::AccountId, T::BlockNumber, Issuing<T>, Backing<T>>, DispatchError> {
        <replace::Pallet<T>>::get_open_or_completed_replace_request(id)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod refund {
    use crate::types::Issuing;
    use frame_support::dispatch::DispatchError;
    use primitive_types::H256;
    use refund::types::RefundRequest;

    pub(crate) fn get_open_or_completed_refund_request_from_id<T: refund::Config>(
        id: &H256,
    ) -> Result<RefundRequest<T::AccountId, Issuing<T>>, DispatchError> {
        <refund::Pallet<T>>::get_open_or_completed_refund_request_from_id(id)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod sla {
    use crate::types::Backing;
    use frame_support::dispatch::DispatchError;
    pub use sla::types::RelayerEvent;

    pub fn event_update_relayer_sla<T: sla::Config>(
        relayer_id: &T::AccountId,
        event: RelayerEvent,
    ) -> Result<(), DispatchError> {
        <sla::Pallet<T>>::event_update_relayer_sla(relayer_id, event)
    }

    pub fn initialize_relayer_stake<T: sla::Config>(
        relayer_id: &T::AccountId,
        stake: Backing<T>,
    ) -> Result<(), DispatchError> {
        <sla::Pallet<T>>::initialize_relayer_stake(relayer_id, stake)
    }
}
