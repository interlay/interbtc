#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod treasury {
    use crate::PolkaBTC;

    pub fn get_total_supply<T: treasury::Config>() -> PolkaBTC<T> {
        <treasury::Module<T>>::get_total_supply()
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod vault_registry {
    use crate::{PolkaBTC, DOT};
    use frame_support::dispatch::DispatchError;

    pub fn get_vault_from_id<T: vault_registry::Config>(
        vault_id: &T::AccountId,
    ) -> Result<
        vault_registry::types::Vault<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>,
        DispatchError,
    > {
        <vault_registry::Module<T>>::get_vault_from_id(vault_id)
    }

    pub fn get_premium_redeem_threshold<T: vault_registry::Config>(
    ) -> <T as vault_registry::Config>::UnsignedFixedPoint {
        <vault_registry::Module<T>>::get_premium_redeem_threshold()
    }

    pub fn get_liquidation_collateral_threshold<T: vault_registry::Config>(
    ) -> <T as vault_registry::Config>::UnsignedFixedPoint {
        <vault_registry::Module<T>>::get_liquidation_collateral_threshold()
    }

    pub fn get_total_issued_tokens<T: vault_registry::Config>(
        include_liquidation_vault: bool,
    ) -> Result<PolkaBTC<T>, DispatchError> {
        <vault_registry::Module<T>>::get_total_issued_tokens(include_liquidation_vault)
    }

    pub fn get_total_backing_collateral<T: vault_registry::Config>(
        include_liquidation_vault: bool,
    ) -> Result<DOT<T>, DispatchError> {
        <vault_registry::Module<T>>::get_total_backing_collateral(include_liquidation_vault)
    }

    pub fn get_backing_collateral<T: vault_registry::Config>(
        vault_id: &T::AccountId,
    ) -> Result<DOT<T>, DispatchError> {
        <vault_registry::Module<T>>::get_backing_collateral(vault_id)
    }
}
