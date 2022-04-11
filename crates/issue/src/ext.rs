#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod btc_relay {
    use bitcoin::types::{MerkleProof, Transaction, Value};
    use btc_relay::BtcAddress;
    use frame_support::dispatch::DispatchError;
    use sp_std::convert::TryFrom;

    pub fn get_and_verify_issue_payment<T: crate::Config, V: TryFrom<Value>>(
        merkle_proof: MerkleProof,
        transaction: Transaction,
        recipient_btc_address: BtcAddress,
    ) -> Result<(BtcAddress, V), DispatchError> {
        <btc_relay::Pallet<T>>::get_and_verify_issue_payment(merkle_proof, transaction, recipient_btc_address)
    }

    pub fn get_best_block_height<T: crate::Config>() -> u32 {
        <btc_relay::Pallet<T>>::get_best_block_height()
    }

    pub fn is_fully_initialized<T: crate::Config>() -> Result<bool, DispatchError> {
        <btc_relay::Pallet<T>>::is_fully_initialized()
    }

    pub fn parse_transaction<T: btc_relay::Config>(raw_tx: &[u8]) -> Result<Transaction, DispatchError> {
        <btc_relay::Pallet<T>>::parse_transaction(raw_tx)
    }

    pub fn parse_merkle_proof<T: btc_relay::Config>(raw_merkle_proof: &[u8]) -> Result<MerkleProof, DispatchError> {
        <btc_relay::Pallet<T>>::parse_merkle_proof(raw_merkle_proof)
    }

    pub fn has_request_expired<T: crate::Config>(
        opentime: T::BlockNumber,
        btc_open_height: u32,
        period: T::BlockNumber,
    ) -> Result<bool, DispatchError> {
        <btc_relay::Pallet<T>>::has_request_expired(opentime, btc_open_height, period)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod vault_registry {
    use crate::DefaultVaultId;
    use btc_relay::BtcAddress;
    use frame_support::dispatch::{DispatchError, DispatchResult};
    use sp_core::H256;
    use vault_registry::{
        types::{CurrencySource, DefaultVault},
        Amount, BtcPublicKey,
    };

    pub fn transfer_funds<T: crate::Config>(
        from: CurrencySource<T>,
        to: CurrencySource<T>,
        amount: &Amount<T>,
    ) -> DispatchResult {
        <vault_registry::Pallet<T>>::transfer_funds(from, to, amount)
    }

    pub fn is_vault_liquidated<T: crate::Config>(vault_id: &DefaultVaultId<T>) -> Result<bool, DispatchError> {
        <vault_registry::Pallet<T>>::is_vault_liquidated(vault_id)
    }

    pub fn get_active_vault_from_id<T: crate::Config>(
        vault_id: &DefaultVaultId<T>,
    ) -> Result<DefaultVault<T>, DispatchError> {
        <vault_registry::Pallet<T>>::get_active_vault_from_id(vault_id)
    }

    pub fn try_increase_to_be_issued_tokens<T: crate::Config>(
        vault_id: &DefaultVaultId<T>,
        amount: &Amount<T>,
    ) -> Result<(), DispatchError> {
        <vault_registry::Pallet<T>>::try_increase_to_be_issued_tokens(vault_id, amount)
    }

    pub fn register_deposit_address<T: crate::Config>(
        vault_id: &DefaultVaultId<T>,
        secure_id: H256,
    ) -> Result<BtcAddress, DispatchError> {
        <vault_registry::Pallet<T>>::register_deposit_address(vault_id, secure_id)
    }

    pub fn get_bitcoin_public_key<T: crate::Config>(account_id: &T::AccountId) -> Result<BtcPublicKey, DispatchError> {
        <vault_registry::Pallet<T>>::get_bitcoin_public_key(account_id)
    }

    pub fn issue_tokens<T: crate::Config>(vault_id: &DefaultVaultId<T>, amount: &Amount<T>) -> DispatchResult {
        <vault_registry::Pallet<T>>::issue_tokens(vault_id, amount)
    }

    pub fn ensure_not_banned<T: crate::Config>(vault_id: &DefaultVaultId<T>) -> DispatchResult {
        <vault_registry::Pallet<T>>::_ensure_not_banned(vault_id)
    }

    pub fn decrease_to_be_issued_tokens<T: crate::Config>(
        vault_id: &DefaultVaultId<T>,
        tokens: &Amount<T>,
    ) -> DispatchResult {
        <vault_registry::Pallet<T>>::decrease_to_be_issued_tokens(vault_id, tokens)
    }

    pub fn calculate_collateral<T: crate::Config>(
        collateral: &Amount<T>,
        numerator: &Amount<T>,
        denominator: &Amount<T>,
    ) -> Result<Amount<T>, DispatchError> {
        <vault_registry::Pallet<T>>::calculate_collateral(collateral, numerator, denominator)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    use sp_core::H256;

    pub fn get_secure_id<T: crate::Config>(id: &T::AccountId) -> H256 {
        <security::Pallet<T>>::get_secure_id(id)
    }

    pub fn active_block_number<T: crate::Config>() -> T::BlockNumber {
        <security::Pallet<T>>::active_block_number()
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod fee {
    use currency::Amount;
    use frame_support::dispatch::{DispatchError, DispatchResult};

    pub fn fee_pool_account_id<T: crate::Config>() -> T::AccountId {
        <fee::Pallet<T>>::fee_pool_account_id()
    }

    pub fn get_issue_fee<T: crate::Config>(amount: &Amount<T>) -> Result<Amount<T>, DispatchError> {
        <fee::Pallet<T>>::get_issue_fee(amount)
    }

    pub fn get_issue_griefing_collateral<T: crate::Config>(amount: &Amount<T>) -> Result<Amount<T>, DispatchError> {
        <fee::Pallet<T>>::get_issue_griefing_collateral(amount)
    }

    pub fn distribute_rewards<T: crate::Config>(amount: &Amount<T>) -> DispatchResult {
        <fee::Pallet<T>>::distribute_rewards(amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod refund {
    use crate::DefaultVaultId;
    use btc_relay::BtcAddress;
    use currency::Amount;
    use frame_support::dispatch::DispatchError;
    use sp_core::H256;

    pub fn request_refund<T: crate::Config>(
        total_amount_btc: &Amount<T>,
        vault_id: DefaultVaultId<T>,
        issuer: T::AccountId,
        btc_address: BtcAddress,
        issue_id: H256,
    ) -> Result<Option<H256>, DispatchError> {
        <refund::Pallet<T>>::request_refund(total_amount_btc, vault_id, issuer, btc_address, issue_id)
    }
}
