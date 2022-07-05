#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod vault_registry {
    use crate::{BalanceOf, DefaultVaultId, DefaultVaultStatus};
    use frame_support::dispatch::{DispatchError, DispatchResult};
    use vault_registry::types::DefaultVault;

    pub fn get_active_vault_from_id<T: crate::Config>(
        vault_id: &DefaultVaultId<T>,
    ) -> Result<DefaultVault<T>, DispatchError> {
        <vault_registry::Pallet<T>>::get_active_vault_from_id(vault_id)
    }

    pub fn get_vault_from_id<T: crate::Config>(vault_id: &DefaultVaultId<T>) -> Result<DefaultVault<T>, DispatchError> {
        <vault_registry::Pallet<T>>::get_vault_from_id(vault_id)
    }

    pub fn report_vault_theft<T: crate::Config>(
        vault_id: &DefaultVaultId<T>,
        reported_amount: BalanceOf<T>,
        reporter_id: &T::AccountId,
    ) -> DispatchResult {
        <vault_registry::Pallet<T>>::report_vault_theft(vault_id, reported_amount, reporter_id)
    }

    pub fn recover_vault_theft<T: crate::Config>(
        vault_id: &DefaultVaultId<T>,
        recovered_amount: BalanceOf<T>,
    ) -> Result<DefaultVaultStatus<T>, DispatchError> {
        Ok(<vault_registry::Pallet<T>>::recover_vault_theft(
            vault_id,
            recovered_amount,
        )?)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod btc_relay {
    use bitcoin::types::{BlockHeader, H256Le, MerkleProof, RawBlockHeader};
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

    pub(crate) fn verify_transaction_inclusion<T: crate::Config>(
        tx_id: H256Le,
        merkle_proof: MerkleProof,
    ) -> DispatchResult {
        <btc_relay::Pallet<T>>::_verify_transaction_inclusion(tx_id, merkle_proof, None)
    }

    pub fn parse_raw_block_header<T: btc_relay::Config>(
        raw_block_header: &RawBlockHeader,
    ) -> Result<BlockHeader, DispatchError> {
        <btc_relay::Pallet<T>>::parse_raw_block_header(raw_block_header)
    }

    pub fn parse_merkle_proof<T: btc_relay::Config>(raw_merkle_proof: &[u8]) -> Result<MerkleProof, DispatchError> {
        <btc_relay::Pallet<T>>::parse_merkle_proof(raw_merkle_proof)
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
