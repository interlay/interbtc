#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    use frame_support::dispatch::DispatchResult;
    use primitive_types::H256;

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
pub(crate) mod vault_registry {
    use crate::Backing;
    pub use ::vault_registry::VaultStatus;
    pub use frame_support::dispatch::{DispatchError, DispatchResult};
    use sp_std::vec::Vec;
    use vault_registry::LiquidationTarget;

    pub fn get_backing_collateral<T: vault_registry::Config>(
        vault_id: &T::AccountId,
    ) -> Result<Backing<T>, DispatchError> {
        <vault_registry::Module<T>>::get_backing_collateral(vault_id)
    }

    pub fn liquidate_vault_with_status<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        status: VaultStatus,
    ) -> Result<Backing<T>, DispatchError> {
        <vault_registry::Module<T>>::liquidate_vault_with_status(vault_id, status)
    }

    pub fn increase_backing_collateral<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        amount: Backing<T>,
    ) -> DispatchResult {
        <vault_registry::Module<T>>::try_increase_backing_collateral(vault_id, amount)
    }

    pub fn decrease_backing_collateral<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        amount: Backing<T>,
    ) -> DispatchResult {
        <vault_registry::Module<T>>::try_decrease_backing_collateral(vault_id, amount)
    }

    pub fn lock_additional_collateral_from_address<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        collateral: Backing<T>,
        depositor_id: &T::AccountId,
    ) -> DispatchResult {
        <vault_registry::Module<T>>::_lock_additional_collateral_from_address(vault_id, collateral, depositor_id)
    }

    pub fn withdraw_collateral_to_address<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        collateral: Backing<T>,
        payee_id: &T::AccountId,
    ) -> DispatchResult {
        <vault_registry::Module<T>>::try_withdraw_collateral_to_address(vault_id, collateral, payee_id)
    }

    pub fn vault_exists<T: vault_registry::Config>(id: &T::AccountId) -> bool {
        <vault_registry::Module<T>>::vault_exists(id)
    }

    pub fn set_is_nomination_operator<T: vault_registry::Config>(vault_id: &T::AccountId, is_operator: bool) {
        <vault_registry::Module<T>>::set_is_nomination_operator(vault_id, is_operator)
    }

    pub fn liquidate_undercollateralized_vaults<T: vault_registry::Config>(
        liquidation_target: LiquidationTarget,
    ) -> (u32, Vec<(T::AccountId, Backing<T>)>) {
        <vault_registry::Module<T>>::liquidate_undercollateralized_vaults(liquidation_target)
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
