#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod collateral {
    use crate::types::DOT;
    use frame_support::dispatch::DispatchResult;

    type CollateralPallet<T> = currency::Pallet<T, currency::Collateral>;

    pub fn transfer<T: currency::Config<currency::Collateral>>(
        source: &T::AccountId,
        destination: &T::AccountId,
        amount: DOT<T>,
    ) -> DispatchResult {
        CollateralPallet::<T>::transfer(source, destination, amount)
    }

    pub fn lock<T: currency::Config<currency::Collateral>>(sender: &T::AccountId, amount: DOT<T>) -> DispatchResult {
        CollateralPallet::<T>::lock(sender, amount)
    }

    pub fn release_collateral<T: currency::Config<currency::Collateral>>(
        sender: &T::AccountId,
        amount: DOT<T>,
    ) -> DispatchResult {
        CollateralPallet::<T>::release(sender, amount)
    }

    pub fn for_account<T: currency::Config<currency::Collateral>>(id: &T::AccountId) -> DOT<T> {
        CollateralPallet::<T>::get_reserved_balance(id)
    }

    pub fn get_free_balance<T: currency::Config<currency::Collateral>>(id: &T::AccountId) -> DOT<T> {
        CollateralPallet::<T>::get_free_balance(id)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod treasury {
    use crate::types::PolkaBTC;
    use frame_support::dispatch::DispatchResult;

    type TreasuryPallet<T> = currency::Pallet<T, currency::Treasury>;

    pub fn total_issued<T: currency::Config<currency::Treasury>>() -> PolkaBTC<T> {
        TreasuryPallet::<T>::get_total_supply()
    }

    pub fn get_free_balance<T: currency::Config<currency::Treasury>>(id: T::AccountId) -> PolkaBTC<T> {
        TreasuryPallet::<T>::get_free_balance(&id)
    }

    pub fn transfer<T: currency::Config<currency::Treasury>>(
        sender: T::AccountId,
        receiver: T::AccountId,
        amount: PolkaBTC<T>,
    ) -> DispatchResult {
        TreasuryPallet::<T>::transfer(&sender, &receiver, amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod oracle {
    use crate::types::{PolkaBTC, DOT};
    use frame_support::dispatch::DispatchError;

    type Collateral = currency::Instance1;
    type Treasury = currency::Instance2;

    pub trait Exchangeable:
        exchange_rate_oracle::Config + currency::Config<Collateral> + currency::Config<Treasury>
    {
    }
    impl<T> Exchangeable for T where
        T: exchange_rate_oracle::Config + currency::Config<Collateral> + currency::Config<Treasury>
    {
    }

    pub fn btc_to_dots<T: Exchangeable>(amount: PolkaBTC<T>) -> Result<DOT<T>, DispatchError> {
        <exchange_rate_oracle::Pallet<T>>::btc_to_dots(amount)
    }

    pub fn dots_to_btc<T: Exchangeable>(amount: DOT<T>) -> Result<PolkaBTC<T>, DispatchError> {
        <exchange_rate_oracle::Pallet<T>>::dots_to_btc(amount)
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
