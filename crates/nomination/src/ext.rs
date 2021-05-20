#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    use frame_support::dispatch::DispatchResult;
    use sp_core::H256;

    pub fn get_secure_id<T: security::Config>(id: &T::AccountId) -> H256 {
        <security::Pallet<T>>::get_secure_id(id)
    }

    pub fn ensure_parachain_status_running<T: security::Config>() -> DispatchResult {
        <security::Pallet<T>>::ensure_parachain_status_running()
    }

    pub fn active_block_number<T: security::Config>() -> T::BlockNumber {
        <security::Pallet<T>>::active_block_number()
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod collateral {
    use crate::types::Backing;
    use frame_support::dispatch::DispatchResult;

    type CollateralPallet<T> = currency::Pallet<T, currency::Backing>;

    pub fn transfer_and_lock<T: currency::Config<currency::Backing>>(
        source: T::AccountId,
        destination: T::AccountId,
        amount: Backing<T>,
    ) -> DispatchResult {
        CollateralPallet::<T>::transfer_and_lock(source, destination, amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod vault_registry {
    use crate::Backing;
    pub use ::vault_registry::{DefaultVault, SlashingError, TryDepositCollateral, TryWithdrawCollateral, VaultStatus};
    pub use frame_support::dispatch::{DispatchError, DispatchResult};

    pub fn get_backing_collateral<T: vault_registry::Config>(
        vault_id: &T::AccountId,
    ) -> Result<Backing<T>, DispatchError> {
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

    pub fn compute_collateral<T: vault_registry::Config>(id: &T::AccountId) -> Result<Backing<T>, DispatchError> {
        <vault_registry::Pallet<T>>::compute_collateral(id)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod fee {
    use crate::types::{Backing, UnsignedFixedPoint};
    use frame_support::dispatch::DispatchError;

    pub fn backing_for<T: fee::Config>(
        amount: Backing<T>,
        percentage: UnsignedFixedPoint<T>,
    ) -> Result<Backing<T>, DispatchError> {
        <fee::Module<T>>::backing_for(amount, percentage)
    }
}
