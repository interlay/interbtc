#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    #[cfg(test)]
    use security::types::ErrorCode;
    use security::types::StatusCode;
    use x_core::{Error, UnitResult};

    pub fn _ensure_parachain_status_running<T: security::Trait>() -> UnitResult {
        <security::Module<T>>::_ensure_parachain_status_running()
    }

    pub fn ensure_parachain_status_not_shutdown<T: security::Trait>() -> UnitResult {
        <security::Module<T>>::_ensure_parachain_status_not_shutdown()
    }

    pub fn _get_parachain_status<T: security::Trait>() -> StatusCode {
        <security::Module<T>>::get_parachain_status()
    }

    pub fn _is_parachain_error_invalid_btcrelay<T: security::Trait>() -> bool {
        <security::Module<T>>::_is_parachain_error_invalid_btcrelay()
    }

    pub fn _is_parachain_error_no_data_btcrelay<T: security::Trait>() -> bool {
        <security::Module<T>>::_is_parachain_error_no_data_btcrelay()
    }

    pub fn recover_from_btc_relay_failure<T: security::Trait>() -> UnitResult {
        <security::Module<T>>::recover_from_btc_relay_failure().map_err(|_e| Error::RuntimeError)
    }

    #[cfg(test)]
    pub fn set_parachain_status<T: security::Trait>(status: StatusCode) -> () {
        <security::Module<T>>::set_parachain_status(status)
    }

    #[cfg(test)]
    pub fn insert_error<T: security::Trait>(error: ErrorCode) -> () {
        <security::Module<T>>::insert_error(error)
    }
}
