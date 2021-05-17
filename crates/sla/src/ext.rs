#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod vault_registry {
    use crate::{Backing, Issuing};
    use frame_support::dispatch::DispatchError;
    use vault_registry::types::Vault;

    pub fn get_vault_from_id<T: vault_registry::Config>(
        vault_id: &T::AccountId,
    ) -> Result<Vault<T::AccountId, T::BlockNumber, Issuing<T>, Backing<T>, T::SignedFixedPoint>, DispatchError> {
        <vault_registry::Pallet<T>>::get_vault_from_id(vault_id)
    }

    pub fn premium_redeem_threshold<T: vault_registry::Config>() -> <T as vault_registry::Config>::UnsignedFixedPoint {
        <vault_registry::Pallet<T>>::premium_redeem_threshold()
    }

    pub fn liquidation_collateral_threshold<T: vault_registry::Config>(
    ) -> <T as vault_registry::Config>::UnsignedFixedPoint {
        <vault_registry::Pallet<T>>::liquidation_collateral_threshold()
    }

    pub fn get_total_issued_tokens<T: vault_registry::Config>(
        include_liquidation_vault: bool,
    ) -> Result<Issuing<T>, DispatchError> {
        <vault_registry::Pallet<T>>::get_total_issued_tokens(include_liquidation_vault)
    }

    pub fn get_total_backing_collateral<T: vault_registry::Config>(
        include_liquidation_vault: bool,
    ) -> Result<Backing<T>, DispatchError> {
        <vault_registry::Pallet<T>>::get_total_backing_collateral(include_liquidation_vault)
    }

    pub fn get_backing_collateral<T: vault_registry::Config>(
        vault_id: &T::AccountId,
    ) -> Result<Backing<T>, DispatchError> {
        <vault_registry::Pallet<T>>::get_backing_collateral(vault_id)
    }
}
