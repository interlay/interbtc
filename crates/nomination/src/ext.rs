#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    use frame_support::dispatch::DispatchResult;
    use primitive_types::H256;
    use security::types::{ErrorCode, StatusCode};
    use sp_std::collections::btree_set::BTreeSet;

    pub(crate) fn get_parachain_status<T: security::Config>() -> StatusCode {
        <security::Module<T>>::get_parachain_status()
    }

    pub(crate) fn set_status<T: security::Config>(status_code: StatusCode) {
        <security::Module<T>>::set_status(status_code)
    }

    pub(crate) fn insert_error<T: security::Config>(error_code: ErrorCode) {
        <security::Module<T>>::insert_error(error_code)
    }

    pub(crate) fn remove_error<T: security::Config>(error_code: ErrorCode) {
        <security::Module<T>>::remove_error(error_code)
    }

    pub(crate) fn get_errors<T: security::Config>() -> BTreeSet<ErrorCode> {
        <security::Module<T>>::get_errors()
    }

    pub fn get_secure_id<T: security::Config>(id: &T::AccountId) -> H256 {
        <security::Module<T>>::get_secure_id(id)
    }

    pub fn ensure_parachain_status_running<T: security::Config>() -> DispatchResult {
        <security::Module<T>>::ensure_parachain_status_running()
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod collateral {
    use crate::types::DOT;
    use frame_support::dispatch::DispatchResult;

    pub fn transfer<T: collateral::Config>(
        source: &T::AccountId,
        destination: &T::AccountId,
        amount: DOT<T>,
    ) -> DispatchResult {
        <collateral::Module<T>>::transfer(source.clone(), destination.clone(), amount)
    }

    pub fn lock<T: collateral::Config>(sender: &T::AccountId, amount: DOT<T>) -> DispatchResult {
        <collateral::Module<T>>::lock_collateral(sender, amount)
    }

    pub fn release_collateral<T: collateral::Config>(
        sender: &T::AccountId,
        amount: DOT<T>,
    ) -> DispatchResult {
        <collateral::Module<T>>::release_collateral(sender, amount)
    }

    pub fn for_account<T: collateral::Config>(id: &T::AccountId) -> DOT<T> {
        <collateral::Module<T>>::get_collateral_from_account(id)
    }

    pub fn get_free_balance<T: collateral::Config>(id: &T::AccountId) -> DOT<T> {
        <collateral::Module<T>>::get_balance_from_account(id)
    }
}
