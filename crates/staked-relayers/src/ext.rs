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

    pub(crate) fn get_collateral_from_account<T: collateral::Trait>(
        sender: &T::AccountId,
    ) -> DOT<T> {
        <collateral::Module<T>>::get_collateral_from_account(sender)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod oracle {
    use crate::types::{PolkaBTC, DOT};
    use frame_support::dispatch::DispatchError;

    pub(crate) fn is_max_delay_passed<T: exchange_rate_oracle::Trait>() -> bool {
        <exchange_rate_oracle::Module<T>>::is_max_delay_passed()
    }

    pub fn btc_to_dots<T: exchange_rate_oracle::Trait>(
        amount: PolkaBTC<T>,
    ) -> Result<DOT<T>, DispatchError> {
        <exchange_rate_oracle::Module<T>>::btc_to_dots(amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod vault_registry {
    use crate::types::PolkaBTC;
    use ::vault_registry::VaultStatus;
    use frame_support::dispatch::{DispatchError, DispatchResult};

    pub fn get_vault_from_id<T: vault_registry::Trait>(
        vault_id: &T::AccountId,
    ) -> Result<
        vault_registry::types::Vault<T::AccountId, T::BlockNumber, PolkaBTC<T>>,
        DispatchError,
    > {
        <vault_registry::Module<T>>::_get_vault_from_id(vault_id)
    }

    pub fn get_liquidation_collateral_threshold<T: vault_registry::Trait>() -> u128 {
        <vault_registry::Module<T>>::_get_liquidation_collateral_threshold()
    }

    pub fn liquidate_vault<T: vault_registry::Trait>(vault_id: &T::AccountId) -> DispatchResult {
        <vault_registry::Module<T>>::_liquidate_vault(vault_id)
    }

    pub fn liquidate_theft_vault<T: vault_registry::Trait>(
        vault_id: &T::AccountId,
    ) -> DispatchResult {
        <vault_registry::Module<T>>::_liquidate_vault_with_status(
            vault_id,
            VaultStatus::CommittedTheft,
        )
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    use frame_support::dispatch::DispatchResult;
    use security::types::{ErrorCode, StatusCode};
    use sp_std::collections::btree_set::BTreeSet;

    pub(crate) fn get_parachain_status<T: security::Trait>() -> StatusCode {
        <security::Module<T>>::get_parachain_status()
    }

    pub(crate) fn set_parachain_status<T: security::Trait>(status_code: StatusCode) {
        <security::Module<T>>::set_parachain_status(status_code)
    }

    pub(crate) fn mutate_errors<T, F>(f: F) -> DispatchResult
    where
        T: security::Trait,
        F: for<'a> FnOnce(&'a mut BTreeSet<ErrorCode>) -> DispatchResult,
    {
        <security::Module<T>>::mutate_errors(f)
    }

    #[cfg(test)]
    pub(crate) fn get_errors<T: security::Trait>() -> BTreeSet<ErrorCode> {
        <security::Module<T>>::get_errors()
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod btc_relay {
    use bitcoin::types::H256Le;
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
        <btc_relay::Module<T>>::_verify_transaction_inclusion(tx_id, raw_merkle_proof, 0, false)
    }

    pub(crate) fn block_header_exists<T: btc_relay::Trait>(block_hash: H256Le) -> bool {
        <btc_relay::Module<T>>::block_header_exists(block_hash)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod redeem {
    use crate::types::{PolkaBTC, DOT};
    use frame_support::dispatch::DispatchError;
    use primitive_types::H256;
    use redeem::types::RedeemRequest;

    pub(crate) fn get_redeem_request_from_id<T: redeem::Trait>(
        id: &H256,
    ) -> Result<RedeemRequest<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>, DispatchError>
    {
        <redeem::Module<T>>::get_redeem_request_from_id(id)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod replace {
    use crate::types::{PolkaBTC, DOT};
    use frame_support::dispatch::DispatchError;
    use primitive_types::H256;
    use replace::types::Replace;

    pub(crate) fn get_replace_request<T: replace::Trait>(
        id: &H256,
    ) -> Result<Replace<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>, DispatchError> {
        <replace::Module<T>>::get_replace_request(id)
    }
}
