#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    #[cfg(test)]
    use security::types::ErrorCode;
    #[cfg(test)]
    use security::types::StatusCode;

    use frame_support::dispatch::DispatchError;
    type UnitResult = Result<(), DispatchError>;

    pub fn ensure_parachain_status_not_shutdown<T: security::Config>() -> UnitResult {
        <security::Pallet<T>>::ensure_parachain_status_not_shutdown()
    }

    pub fn is_parachain_error_invalid_btcrelay<T: security::Config>() -> bool {
        <security::Pallet<T>>::is_parachain_error_invalid_btcrelay()
    }

    pub fn is_parachain_error_no_data_btcrelay<T: security::Config>() -> bool {
        <security::Pallet<T>>::is_parachain_error_no_data_btcrelay()
    }

    pub fn recover_from_btc_relay_failure<T: security::Config>() {
        <security::Pallet<T>>::recover_from_btc_relay_failure()
    }

    #[cfg(test)]
    pub fn set_status<T: security::Config>(status: StatusCode) {
        <security::Pallet<T>>::set_status(status)
    }

    #[cfg(test)]
    pub fn insert_error<T: security::Config>(error: ErrorCode) {
        <security::Pallet<T>>::insert_error(error)
    }

    pub fn active_block_number<T: security::Config>() -> T::BlockNumber {
        <security::Pallet<T>>::active_block_number()
    }
}
