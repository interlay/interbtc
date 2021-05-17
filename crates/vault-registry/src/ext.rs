#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod collateral {
    use crate::types::Backing;
    use frame_support::dispatch::DispatchResult;

    type CollateralPallet<T> = currency::Pallet<T, currency::Backing>;

    pub fn transfer<T: currency::Config<currency::Backing>>(
        source: &T::AccountId,
        destination: &T::AccountId,
        amount: Backing<T>,
    ) -> DispatchResult {
        CollateralPallet::<T>::transfer(source, destination, amount)
    }

    pub fn lock<T: currency::Config<currency::Backing>>(sender: &T::AccountId, amount: Backing<T>) -> DispatchResult {
        CollateralPallet::<T>::lock(sender, amount)
    }

    pub fn release_collateral<T: currency::Config<currency::Backing>>(
        sender: &T::AccountId,
        amount: Backing<T>,
    ) -> DispatchResult {
        CollateralPallet::<T>::release(sender, amount)
    }

    pub fn get_reserved_balance<T: currency::Config<currency::Backing>>(id: &T::AccountId) -> Backing<T> {
        CollateralPallet::<T>::get_reserved_balance(id)
    }

    pub fn get_free_balance<T: currency::Config<currency::Backing>>(id: &T::AccountId) -> Backing<T> {
        CollateralPallet::<T>::get_free_balance(id)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod treasury {
    use crate::types::Issuing;

    type TreasuryPallet<T> = currency::Pallet<T, currency::Issuing>;

    pub fn total_issued<T: currency::Config<currency::Issuing>>() -> Issuing<T> {
        TreasuryPallet::<T>::get_total_supply()
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod oracle {
    use crate::types::{Backing, Issuing};
    use frame_support::dispatch::DispatchError;

    type Collateral = currency::Backing;
    type Treasury = currency::Issuing;

    pub trait Exchangeable:
        exchange_rate_oracle::Config + currency::Config<Collateral> + currency::Config<Treasury>
    {
    }
    impl<T> Exchangeable for T where
        T: exchange_rate_oracle::Config + currency::Config<Collateral> + currency::Config<Treasury>
    {
    }

    pub fn issuing_to_backing<T: Exchangeable>(amount: Issuing<T>) -> Result<Backing<T>, DispatchError> {
        <exchange_rate_oracle::Pallet<T>>::issuing_to_backing(amount)
    }

    pub fn backing_to_issuing<T: Exchangeable>(amount: Backing<T>) -> Result<Issuing<T>, DispatchError> {
        <exchange_rate_oracle::Pallet<T>>::backing_to_issuing(amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    use frame_support::dispatch::DispatchResult;

    pub fn ensure_parachain_status_not_shutdown<T: security::Config>() -> DispatchResult {
        <security::Pallet<T>>::ensure_parachain_status_not_shutdown()
    }

    pub fn active_block_number<T: security::Config>() -> T::BlockNumber {
        <security::Pallet<T>>::active_block_number()
    }
}
