#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod collateral {
    use crate::types::DOT;
    use frame_support::dispatch::DispatchResult;

    pub fn transfer<T: collateral::Config>(
        source: &T::AccountId,
        destination: &T::AccountId,
        amount: DOT<T>,
    ) -> DispatchResult {
        <collateral::Pallet<T>>::transfer(source.clone(), destination.clone(), amount)
    }

    pub fn lock<T: collateral::Config>(sender: &T::AccountId, amount: DOT<T>) -> DispatchResult {
        <collateral::Pallet<T>>::lock_collateral(sender, amount)
    }

    pub fn release_collateral<T: collateral::Config>(sender: &T::AccountId, amount: DOT<T>) -> DispatchResult {
        <collateral::Pallet<T>>::release_collateral(sender, amount)
    }

    pub fn for_account<T: collateral::Config>(id: &T::AccountId) -> DOT<T> {
        <collateral::Pallet<T>>::get_collateral_from_account(id)
    }

    pub fn get_free_balance<T: collateral::Config>(id: &T::AccountId) -> DOT<T> {
        <collateral::Pallet<T>>::get_balance_from_account(id)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod treasury {
    use crate::types::PolkaBTC;
    use frame_support::dispatch::DispatchResult;

    pub fn total_issued<T: treasury::Config>() -> PolkaBTC<T> {
        <treasury::Pallet<T>>::get_total_supply()
    }

    pub fn get_free_balance<T: treasury::Config>(id: T::AccountId) -> PolkaBTC<T> {
        <treasury::Pallet<T>>::get_balance_from_account(id)
    }

    pub fn transfer<T: treasury::Config>(
        sender: T::AccountId,
        receiver: T::AccountId,
        amount: PolkaBTC<T>,
    ) -> DispatchResult {
        <treasury::Pallet<T>>::transfer(sender, receiver, amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod oracle {
    use crate::types::{PolkaBTC, DOT};
    use frame_support::dispatch::DispatchError;

    pub trait Exchangeable: exchange_rate_oracle::Config + ::treasury::Config + ::collateral::Config {}
    impl<T> Exchangeable for T where T: exchange_rate_oracle::Config + ::treasury::Config + ::collateral::Config {}

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
