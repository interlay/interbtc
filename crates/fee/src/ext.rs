#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod collateral {
    use crate::Collateral;
    use frame_support::dispatch::DispatchResult;

    type CollateralPallet<T> = currency::Pallet<T, currency::Collateral>;

    pub fn transfer<T: currency::Config<currency::Collateral>>(
        sender: &T::AccountId,
        receiver: &T::AccountId,
        amount: Collateral<T>,
    ) -> DispatchResult {
        CollateralPallet::<T>::transfer(sender, receiver, amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod treasury {
    use crate::Wrapped;
    use frame_support::dispatch::DispatchResult;

    type TreasuryPallet<T> = currency::Pallet<T, currency::Wrapped>;

    pub fn transfer<T: currency::Config<currency::Wrapped>>(
        sender: &T::AccountId,
        receiver: &T::AccountId,
        amount: Wrapped<T>,
    ) -> DispatchResult {
        TreasuryPallet::<T>::transfer(sender, receiver, amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    use frame_support::dispatch::DispatchError;

    pub fn ensure_parachain_status_not_shutdown<T: security::Config>() -> Result<(), DispatchError> {
        <security::Pallet<T>>::ensure_parachain_status_not_shutdown()
    }
}
