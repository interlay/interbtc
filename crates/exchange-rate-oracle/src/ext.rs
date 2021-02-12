#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    use frame_support::dispatch::DispatchResult;

    pub fn ensure_parachain_status_not_shutdown<T: security::Config>() -> DispatchResult {
        <security::Module<T>>::ensure_parachain_status_not_shutdown()
    }

    pub fn recover_from_oracle_offline<T: security::Config>() -> DispatchResult {
        <security::Module<T>>::recover_from_oracle_offline()
    }
}
