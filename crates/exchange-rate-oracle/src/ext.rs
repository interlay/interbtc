#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    use frame_support::dispatch::DispatchResult;
    use x_core::UnitResult;

    pub fn ensure_parachain_status_running<T: security::Trait>() -> UnitResult {
        <security::Module<T>>::_ensure_parachain_status_running()
    }

    pub fn recover_from_oracle_offline<T: security::Trait>() -> DispatchResult {
        <security::Module<T>>::recover_from_oracle_offline()
    }
}
