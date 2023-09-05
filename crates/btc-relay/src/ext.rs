#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    use frame_support::dispatch::DispatchError;
    use frame_system::pallet_prelude::BlockNumberFor;

    #[cfg(feature = "runtime-benchmarks")]
    pub fn set_active_block_number<T: crate::Config>(n: BlockNumberFor<T>) {
        <security::Pallet<T>>::set_active_block_number(n)
    }

    pub fn active_block_number<T: crate::Config>() -> BlockNumberFor<T> {
        <security::Pallet<T>>::active_block_number()
    }

    pub fn parachain_block_expired<T: crate::Config>(
        opentime: BlockNumberFor<T>,
        period: BlockNumberFor<T>,
    ) -> Result<bool, DispatchError> {
        <security::Pallet<T>>::parachain_block_expired(opentime, period)
    }
}
