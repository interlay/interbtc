#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    use frame_support::dispatch::DispatchResult;

    pub fn ensure_parachain_status_running<T: security::Config>() -> DispatchResult {
        <security::Pallet<T>>::ensure_parachain_status_running()
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod collateral {
    use crate::types::Collateral;
    use frame_support::dispatch::DispatchResult;

    type CollateralPallet<T> = currency::Pallet<T, currency::Collateral>;

    pub fn transfer_and_lock<T: currency::Config<currency::Collateral>>(
        source: &T::AccountId,
        destination: &T::AccountId,
        amount: Collateral<T>,
    ) -> DispatchResult {
        CollateralPallet::<T>::transfer_and_lock(source, destination, amount)
    }

    pub fn unlock_and_transfer<T: currency::Config<currency::Collateral>>(
        source: &T::AccountId,
        destination: &T::AccountId,
        amount: Collateral<T>,
    ) -> DispatchResult {
        CollateralPallet::<T>::unlock_and_transfer(source, destination, amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod vault_registry {
    use crate::Collateral;
    pub use ::vault_registry::{
        DefaultVault, Slashable, SlashingError, TryDepositCollateral, TryWithdrawCollateral, VaultStatus,
    };
    pub use frame_support::dispatch::{DispatchError, DispatchResult};

    pub fn get_backing_collateral<T: vault_registry::Config>(
        vault_id: &T::AccountId,
    ) -> Result<Collateral<T>, DispatchError> {
        <vault_registry::Pallet<T>>::get_backing_collateral(vault_id)
    }

    pub fn get_vault_from_id<T: vault_registry::Config>(
        vault_id: &T::AccountId,
    ) -> Result<DefaultVault<T>, DispatchError> {
        <vault_registry::Pallet<T>>::get_vault_from_id(vault_id)
    }

    pub fn vault_exists<T: vault_registry::Config>(id: &T::AccountId) -> bool {
        <vault_registry::Pallet<T>>::vault_exists(id)
    }

    pub fn get_secure_collateral_threshold<T: vault_registry::Config>(
    ) -> <T as vault_registry::Config>::UnsignedFixedPoint {
        <vault_registry::Pallet<T>>::secure_collateral_threshold()
    }

    pub fn get_premium_redeem_threshold<T: vault_registry::Config>() -> <T as vault_registry::Config>::UnsignedFixedPoint
    {
        <vault_registry::Pallet<T>>::premium_redeem_threshold()
    }

    pub fn insert_vault<T: vault_registry::Config>(id: &T::AccountId, vault: DefaultVault<T>) {
        <vault_registry::Pallet<T>>::insert_vault(id, vault)
    }

    pub fn compute_collateral<T: vault_registry::Config>(id: &T::AccountId) -> Result<Collateral<T>, DispatchError> {
        <vault_registry::Pallet<T>>::compute_collateral(id)
    }

    pub fn is_allowed_to_withdraw_collateral<T: vault_registry::Config>(
        id: &T::AccountId,
        amount: Collateral<T>,
    ) -> Result<bool, DispatchError> {
        <vault_registry::Pallet<T>>::is_allowed_to_withdraw_collateral(id, amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod fee {
    use crate::types::{Collateral, UnsignedFixedPoint};
    use frame_support::dispatch::DispatchError;

    pub fn collateral_for<T: fee::Config>(
        amount: Collateral<T>,
        percentage: UnsignedFixedPoint<T>,
    ) -> Result<Collateral<T>, DispatchError> {
        <fee::Module<T>>::collateral_for(amount, percentage)
    }
}
