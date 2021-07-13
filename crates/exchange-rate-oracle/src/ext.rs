#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    use frame_support::dispatch::DispatchResult;
    use security::{ErrorCode, StatusCode};

    pub fn ensure_parachain_status_not_shutdown<T: crate::Config>() -> DispatchResult {
        <security::Pallet<T>>::ensure_parachain_status_not_shutdown()
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
}

// #[cfg_attr(test, mockable)]
// pub(crate) mod orml_oracle {
//     pub fn feed_value<T: crate::Config>(
//         who: T::AccountId,
//         key: T::OracleKey,
//         value: T::OracleValue
//     ) -> DispatchResult {
//         <orml_oracle::Pallet<T>>::feed_value(who, key, value)
//     }
// }
