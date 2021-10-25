#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    use frame_support::dispatch::DispatchResult;
    use security::{ErrorCode, StatusCode};
    use sp_std::collections::btree_set::BTreeSet;

    pub fn ensure_parachain_status_running<T: crate::Config>() -> DispatchResult {
        <security::Pallet<T>>::ensure_parachain_status_running()
    }

    pub fn recover_from_oracle_offline<T: crate::Config>() {
        <security::Pallet<T>>::recover_from_oracle_offline()
    }

    pub(crate) fn set_status<T: crate::Config>(status_code: StatusCode) {
        <security::Pallet<T>>::set_status(status_code)
    }

    pub(crate) fn insert_error<T: crate::Config>(error_code: ErrorCode) {
        <security::Pallet<T>>::insert_error(error_code)
    }

    pub(crate) fn get_errors<T: crate::Config>() -> BTreeSet<ErrorCode> {
        <security::Pallet<T>>::get_errors()
    }
}
