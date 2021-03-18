#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    use frame_support::dispatch::DispatchResult;
    use security::{ErrorCode, StatusCode};

    pub fn ensure_parachain_status_not_shutdown<T: security::Config>() -> DispatchResult {
        <security::Module<T>>::ensure_parachain_status_not_shutdown()
    }

    pub fn recover_from_oracle_offline<T: security::Config>() {
        <security::Module<T>>::recover_from_oracle_offline()
    }

    pub(crate) fn set_status<T: security::Config>(status_code: StatusCode) {
        <security::Module<T>>::set_status(status_code)
    }

    pub(crate) fn insert_error<T: security::Config>(error_code: ErrorCode) {
        <security::Module<T>>::insert_error(error_code)
    }
}
