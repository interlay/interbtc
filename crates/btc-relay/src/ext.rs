#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    use security::types::StatusCode;
    use x_core::UnitResult;

    pub fn _ensure_parachain_status_running<T: security::Trait>() -> UnitResult {
        <security::Module<T>>::_ensure_parachain_status_running()
    }

    pub fn ensure_parachain_status_not_shutdown<T: security::Trait>() -> UnitResult {
        <security::Module<T>>::_ensure_parachain_status_not_shutdown()
    }

    pub fn _get_parachain_status<T: security::Trait>() -> StatusCode {
        <security::Module<T>>::get_parachain_status()
    }
}
