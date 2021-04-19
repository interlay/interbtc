#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    use frame_support::dispatch::DispatchResult;
    use primitive_types::H256;

    pub fn get_secure_id<T: security::Config>(id: &T::AccountId) -> H256 {
        <security::Module<T>>::get_secure_id(id)
    }

    pub fn ensure_parachain_status_running<T: security::Config>() -> DispatchResult {
        <security::Module<T>>::ensure_parachain_status_running()
    }

    pub fn active_block_number<T: security::Config>() -> T::BlockNumber {
        <security::Module<T>>::active_block_number()
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod vault_registry {
    use crate::DOT;
    pub use ::vault_registry::VaultStatus;
    pub use frame_support::dispatch::{DispatchError, DispatchResult};
    use sp_std::vec::Vec;
    use vault_registry::LiquidationTarget;

    pub fn get_backing_collateral<T: vault_registry::Config>(vault_id: &T::AccountId) -> Result<DOT<T>, DispatchError> {
        <vault_registry::Module<T>>::get_backing_collateral(vault_id)
    }

    pub fn liquidate_vault_with_status<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        status: VaultStatus,
    ) -> Result<DOT<T>, DispatchError> {
        <vault_registry::Module<T>>::liquidate_vault_with_status(vault_id, status)
    }

    pub fn increase_backing_collateral<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        amount: DOT<T>,
    ) -> DispatchResult {
        <vault_registry::Module<T>>::increase_backing_collateral(vault_id, amount)
    }

    pub fn decrease_backing_collateral<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        amount: DOT<T>,
    ) -> DispatchResult {
        <vault_registry::Module<T>>::decrease_backing_collateral(vault_id, amount)
    }

    pub fn lock_additional_collateral_from_address<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        collateral: DOT<T>,
        depositor_id: &T::AccountId,
    ) -> DispatchResult {
        <vault_registry::Module<T>>::try_lock_additional_collateral_from_address(vault_id, collateral, depositor_id)
    }

    pub fn withdraw_collateral_to_address<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        collateral: DOT<T>,
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
    ) -> (u32, Vec<(T::AccountId, DOT<T>)>) {
        <vault_registry::Module<T>>::liquidate_undercollateralized_vaults(liquidation_target)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod fee {
    use crate::types::{Inner, DOT};

    use frame_support::dispatch::DispatchError;

    pub fn inner_to_dot<T: fee::Config>(x: Inner<T>) -> Result<DOT<T>, DispatchError> {
        <fee::Module<T>>::inner_to_dot(x)
    }

    pub fn dot_to_inner<T: fee::Config>(x: DOT<T>) -> Result<Inner<T>, DispatchError> {
        <fee::Module<T>>::dot_to_inner(x)
    }
}
