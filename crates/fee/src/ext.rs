#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod treasury {
    use crate::Wrapped;
    use currency::ParachainCurrency;
    use frame_support::dispatch::DispatchResult;

    pub fn transfer<T: crate::Config>(
        sender: &T::AccountId,
        receiver: &T::AccountId,
        amount: Wrapped<T>,
    ) -> DispatchResult {
        T::Wrapped::transfer(sender, receiver, amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    use frame_support::dispatch::DispatchError;

    pub fn ensure_parachain_status_not_shutdown<T: crate::Config>() -> Result<(), DispatchError> {
        <security::Pallet<T>>::ensure_parachain_status_not_shutdown()
    }
}
