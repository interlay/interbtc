#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    use frame_support::dispatch::DispatchResult;
    use security::types::ErrorCode::InvalidBTCRelay;
    use sp_std::vec;

    pub fn ensure_parachain_status_not_shutdown<T: security::Trait>() -> DispatchResult {
        <security::Module<T>>::_ensure_parachain_status_not_shutdown()
    }

    pub fn ensure_parachain_error_not_invalid<T: security::Trait>() -> DispatchResult {
        <security::Module<T>>::_ensure_parachain_status_has_not_specific_errors(vec![
            InvalidBTCRelay,
        ])
    }

    pub fn recover_from_oracle_offline<T: security::Trait>() -> DispatchResult {
        <security::Module<T>>::recover_from_oracle_offline()
    }
}
