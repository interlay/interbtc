#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod vault_registry {
    use crate::DefaultVaultId;
    use frame_support::dispatch::DispatchError;
    use vault_registry::types::DefaultVault;

    pub fn get_active_vault_from_id<T: crate::Config>(
        vault_id: &DefaultVaultId<T>,
    ) -> Result<DefaultVault<T>, DispatchError> {
        <vault_registry::Pallet<T>>::get_active_vault_from_id(vault_id)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod btc_relay {
    use bitcoin::types::{BlockHeader, RawBlockHeader};
    use frame_support::dispatch::DispatchResult;
    use sp_runtime::DispatchError;

    pub fn initialize<T: crate::Config>(
        relayer: T::AccountId,
        block_header: BlockHeader,
        block_height: u32,
    ) -> DispatchResult {
        <btc_relay::Pallet<T>>::initialize(relayer, block_header, block_height)
    }

    pub fn store_block_header<T: crate::Config>(relayer: &T::AccountId, block_header: BlockHeader) -> DispatchResult {
        <btc_relay::Pallet<T>>::store_block_header(relayer, block_header)
    }

    pub fn parse_raw_block_header<T: btc_relay::Config>(
        raw_block_header: &RawBlockHeader,
    ) -> Result<BlockHeader, DispatchError> {
        <btc_relay::Pallet<T>>::parse_raw_block_header(raw_block_header)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod redeem {
    use frame_support::dispatch::DispatchError;
    use redeem::types::DefaultRedeemRequest;
    use sp_core::H256;

    pub(crate) fn get_open_or_completed_redeem_request_from_id<T: crate::Config>(
        id: &H256,
    ) -> Result<DefaultRedeemRequest<T>, DispatchError> {
        <redeem::Pallet<T>>::get_open_or_completed_redeem_request_from_id(id)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod replace {
    use frame_support::dispatch::DispatchError;
    use replace::types::DefaultReplaceRequest;
    use sp_core::H256;

    pub(crate) fn get_open_or_completed_replace_request<T: crate::Config>(
        id: &H256,
    ) -> Result<DefaultReplaceRequest<T>, DispatchError> {
        <replace::Pallet<T>>::get_open_or_completed_replace_request(id)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod refund {
    use frame_support::dispatch::DispatchError;
    use refund::types::DefaultRefundRequest;
    use sp_core::H256;

    pub(crate) fn get_open_or_completed_refund_request_from_id<T: crate::Config>(
        id: &H256,
    ) -> Result<DefaultRefundRequest<T>, DispatchError> {
        <refund::Pallet<T>>::get_open_or_completed_refund_request_from_id(id)
    }
}
