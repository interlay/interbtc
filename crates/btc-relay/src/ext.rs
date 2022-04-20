#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    use frame_support::dispatch::DispatchError;
    use security::{ErrorCode, StatusCode};

    pub fn active_block_number<T: crate::Config>() -> T::BlockNumber {
        <security::Pallet<T>>::active_block_number()
    }

    pub fn parachain_block_expired<T: crate::Config>(
        opentime: T::BlockNumber,
        period: T::BlockNumber,
    ) -> Result<bool, DispatchError> {
        <security::Pallet<T>>::parachain_block_expired(opentime, period)
    }

    pub fn insert_ongoing_fork_error<T: crate::Config>() {
        <security::Pallet<T>>::set_status(StatusCode::Error);
        <security::Pallet<T>>::insert_error(ErrorCode::OngoingFork)
    }

    pub fn remove_ongoing_fork_error<T: crate::Config>() {
        <security::Pallet<T>>::recover_from_ongoing_fork()
    }
}
