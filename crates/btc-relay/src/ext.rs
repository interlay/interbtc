#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    use primitive_types::H256;
    use x_core::UnitResult;

    pub fn ensure_parachain_status_running<T: security::Trait>() -> UnitResult {
        <security::Module<T>>::_ensure_parachain_status_running()
    }

    pub fn ensure_parachain_status_running<T: security::Trait>() -> UnitResult {
        <security::Module<T>>::get_parachain_status()
    }

    pub fn ensure_parachain_status_running<T: security::Trait>() -> UnitResult {
        <security::Module<T>>::get_parachain_status()
    }
}
