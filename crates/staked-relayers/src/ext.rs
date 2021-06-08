#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod vault_registry {
    use crate::{Collateral, Wrapped};
    use ::vault_registry::VaultStatus;
    use frame_support::dispatch::{DispatchError, DispatchResult};
    use vault_registry::types::Vault;

    pub fn get_active_vault_from_id<T: crate::Config>(
        vault_id: &T::AccountId,
    ) -> Result<
        Vault<T::AccountId, T::BlockNumber, Wrapped<T>, Collateral<T>, <T as vault_registry::Config>::SignedFixedPoint>,
        DispatchError,
    > {
        <vault_registry::Pallet<T>>::get_active_vault_from_id(vault_id)
    }

    pub fn liquidate_theft_vault<T: crate::Config>(vault_id: &T::AccountId) -> DispatchResult {
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
    pub(crate) fn get_errors<T: crate::Config>() -> BTreeSet<ErrorCode> {
        <security::Pallet<T>>::get_errors()
    }

    pub fn ensure_parachain_status_not_shutdown<T: crate::Config>() -> DispatchResult {
        <security::Pallet<T>>::ensure_parachain_status_not_shutdown()
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod btc_relay {
    use bitcoin::types::{H256Le, RawBlockHeader};
    use frame_support::dispatch::DispatchResult;
    use sp_std::prelude::*;

    pub fn initialize<T: crate::Config>(
        relayer: T::AccountId,
        raw_block_header: RawBlockHeader,
        block_height: u32,
    ) -> DispatchResult {
        <btc_relay::Pallet<T>>::initialize(relayer, raw_block_header, block_height)
    }

    pub fn store_block_header<T: crate::Config>(
        relayer: &T::AccountId,
        raw_block_header: RawBlockHeader,
    ) -> DispatchResult {
        <btc_relay::Pallet<T>>::store_block_header(relayer, raw_block_header)
    }

    pub(crate) fn verify_transaction_inclusion<T: crate::Config>(
        tx_id: H256Le,
        raw_merkle_proof: Vec<u8>,
    ) -> DispatchResult {
        <btc_relay::Pallet<T>>::_verify_transaction_inclusion(tx_id, raw_merkle_proof, None)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod redeem {
    use crate::types::{Collateral, Wrapped};
    use frame_support::dispatch::DispatchError;
    use redeem::types::RedeemRequest;
    use sp_core::H256;

    pub(crate) fn get_open_or_completed_redeem_request_from_id<T: crate::Config>(
        id: &H256,
    ) -> Result<RedeemRequest<T::AccountId, T::BlockNumber, Wrapped<T>, Collateral<T>>, DispatchError> {
        <redeem::Pallet<T>>::get_open_or_completed_redeem_request_from_id(id)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod replace {
    use crate::types::{Collateral, Wrapped};
    use frame_support::dispatch::DispatchError;
    use replace::types::ReplaceRequest;
    use sp_core::H256;

    pub(crate) fn get_open_or_completed_replace_request<T: crate::Config>(
        id: &H256,
    ) -> Result<ReplaceRequest<T::AccountId, T::BlockNumber, Wrapped<T>, Collateral<T>>, DispatchError> {
        <replace::Pallet<T>>::get_open_or_completed_replace_request(id)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod refund {
    use crate::types::Wrapped;
    use frame_support::dispatch::DispatchError;
    use refund::types::RefundRequest;
    use sp_core::H256;

    pub(crate) fn get_open_or_completed_refund_request_from_id<T: crate::Config>(
        id: &H256,
    ) -> Result<RefundRequest<T::AccountId, Wrapped<T>>, DispatchError> {
        <refund::Pallet<T>>::get_open_or_completed_refund_request_from_id(id)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod sla {
    use frame_support::dispatch::DispatchError;
    pub use sla::types::RelayerEvent;

    pub fn event_update_relayer_sla<T: crate::Config>(
        relayer_id: &T::AccountId,
        event: RelayerEvent,
    ) -> Result<(), DispatchError> {
        <sla::Pallet<T>>::event_update_relayer_sla(relayer_id, event)
    }
}
