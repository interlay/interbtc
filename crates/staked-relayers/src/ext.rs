#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod collateral {
    use crate::types::DOT;
    use x_core::{Error, UnitResult};

    pub(crate) fn lock_collateral<T: collateral::Trait>(
        sender: &T::AccountId,
        amount: DOT<T>,
    ) -> Result<(), Error> {
        <collateral::Module<T>>::lock_collateral(sender, amount)
    }

    pub(crate) fn release_collateral<T: collateral::Trait>(
        sender: &T::AccountId,
        amount: DOT<T>,
    ) -> Result<(), Error> {
        <collateral::Module<T>>::release_collateral(sender, amount)
    }

    pub fn slash_collateral<T: collateral::Trait>(
        sender: T::AccountId,
        receiver: T::AccountId,
        amount: DOT<T>,
    ) -> UnitResult {
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
    use x_core::Result;

    pub(crate) fn is_max_delay_passed<T: exchange_rate_oracle::Trait>() -> Result<bool> {
        // FIXME
        // <exchange_rate_oracle::Module<T>>::is_max_delay_passed()
        Ok(true)
    }

    pub fn btc_to_dots<T: exchange_rate_oracle::Trait>(amount: PolkaBTC<T>) -> Result<DOT<T>> {
        <exchange_rate_oracle::Module<T>>::btc_to_dots(amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod vault_registry {
    use crate::types::PolkaBTC;
    use x_core::{Result, UnitResult};

    pub fn get_vault_from_id<T: vault_registry::Trait>(
        vault_id: &T::AccountId,
    ) -> Result<vault_registry::types::Vault<T::AccountId, T::BlockNumber, PolkaBTC<T>>> {
        <vault_registry::Module<T>>::_get_vault_from_id(vault_id)
    }

    pub fn get_liquidation_collateral_threshold<T: vault_registry::Trait>() -> u128 {
        // FIXME: add function to vault registry
        110000
    }

    pub fn liquidate_vault<T: vault_registry::Trait>(vault_id: &T::AccountId) -> UnitResult {
        <vault_registry::Module<T>>::_liquidate_vault(vault_id)
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
    use security::types::ErrorCode;
    use x_core::UnitResult;

    pub(crate) fn flag_block_error<T: btc_relay::Trait>(
        block_hash: H256Le,
        error: ErrorCode,
    ) -> UnitResult {
        <btc_relay::Module<T>>::flag_block_error(block_hash, error)
    }

    pub(crate) fn clear_block_error<T: btc_relay::Trait>(
        block_hash: H256Le,
        error: ErrorCode,
    ) -> UnitResult {
        <btc_relay::Module<T>>::clear_block_error(block_hash, error)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod redeem {
    use sp_std::vec::Vec;

    pub(crate) fn get_redeem_request_by_id(_id: &Vec<u8>) -> bool {
        // FIXME
        true
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod replace {
    use sp_std::vec::Vec;

    pub(crate) fn get_replace_request_by_id(_id: &Vec<u8>) -> bool {
        // FIXME
        true
    }
}
