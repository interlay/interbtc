#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod collateral {
    use crate::types::Collateral;
    use frame_support::dispatch::DispatchResult;

    type CollateralPallet<T> = currency::Pallet<T, currency::Collateral>;

    pub fn transfer<T: currency::Config<currency::Collateral>>(
        source: &T::AccountId,
        destination: &T::AccountId,
        amount: Collateral<T>,
    ) -> DispatchResult {
        CollateralPallet::<T>::transfer(source, destination, amount)
    }

    pub fn lock<T: currency::Config<currency::Collateral>>(
        sender: &T::AccountId,
        amount: Collateral<T>,
    ) -> DispatchResult {
        CollateralPallet::<T>::lock(sender, amount)
    }

    pub fn release<T: currency::Config<currency::Collateral>>(
        sender: &T::AccountId,
        amount: Collateral<T>,
    ) -> DispatchResult {
        CollateralPallet::<T>::release(sender, amount)
    }

    pub fn get_reserved_balance<T: currency::Config<currency::Collateral>>(id: &T::AccountId) -> Collateral<T> {
        CollateralPallet::<T>::get_reserved_balance(id)
    }

    pub fn get_free_balance<T: currency::Config<currency::Collateral>>(id: &T::AccountId) -> Collateral<T> {
        CollateralPallet::<T>::get_free_balance(id)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod treasury {
    use crate::types::Wrapped;

    type TreasuryPallet<T> = currency::Pallet<T, currency::Wrapped>;

    pub fn total_issued<T: currency::Config<currency::Wrapped>>() -> Wrapped<T> {
        TreasuryPallet::<T>::get_total_supply()
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod oracle {
    use crate::types::{Collateral, Wrapped};
    use frame_support::dispatch::DispatchError;

    type CollateralInstance = currency::Collateral;
    type WrappedInstance = currency::Wrapped;

    pub trait Exchangeable:
        exchange_rate_oracle::Config + currency::Config<CollateralInstance> + currency::Config<WrappedInstance>
    {
    }
    impl<T> Exchangeable for T where
        T: exchange_rate_oracle::Config + currency::Config<CollateralInstance> + currency::Config<WrappedInstance>
    {
    }

    pub fn wrapped_to_collateral<T: Exchangeable>(amount: Wrapped<T>) -> Result<Collateral<T>, DispatchError> {
        <exchange_rate_oracle::Pallet<T>>::wrapped_to_collateral(amount)
    }

    pub fn collateral_to_wrapped<T: Exchangeable>(amount: Collateral<T>) -> Result<Wrapped<T>, DispatchError> {
        <exchange_rate_oracle::Pallet<T>>::collateral_to_wrapped(amount)
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

#[cfg_attr(test, mockable)]
pub(crate) mod sla {
    use crate::types::{Collateral, UnsignedFixedPoint, Wrapped};
    use frame_support::dispatch::DispatchError;
    pub use sla::types::VaultEvent;

    pub fn calculate_slashed_amount<T: crate::Config>(
        vault_id: &T::AccountId,
        stake: Collateral<T>,
        reimburse: bool,
        liquidation_threshold: UnsignedFixedPoint<T>,
        premium_redeem_threshold: UnsignedFixedPoint<T>,
    ) -> Result<Collateral<T>, DispatchError> {
        <sla::Pallet<T>>::calculate_slashed_amount(
            vault_id,
            stake,
            reimburse,
            liquidation_threshold,
            premium_redeem_threshold,
        )
    }

    pub fn event_update_vault_sla<T: sla::Config>(
        vault_id: &T::AccountId,
        event: VaultEvent<Wrapped<T>, Collateral<T>>,
    ) -> Result<(), DispatchError> {
        <sla::Pallet<T>>::event_update_vault_sla(vault_id, event)
    }
}
