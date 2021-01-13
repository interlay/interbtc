#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod collateral {
    use crate::DOT;

    pub fn get_collateral_from_account<T: collateral::Config>(account: T::AccountId) -> DOT<T> {
        <collateral::Module<T>>::get_collateral_from_account(&account)
    }

    pub fn get_total_collateral<T: collateral::Config>() -> DOT<T> {
        <collateral::Module<T>>::get_total_collateral()
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod treasury {
    use crate::PolkaBTC;

    pub fn get_total_supply<T: treasury::Config>() -> PolkaBTC<T> {
        <treasury::Module<T>>::get_total_supply()
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod vault_registry {
    use crate::PolkaBTC;
    use frame_support::dispatch::DispatchError;

    pub fn get_active_vault_from_id<T: vault_registry::Config>(
        vault_id: &T::AccountId,
    ) -> Result<
        vault_registry::types::Vault<T::AccountId, T::BlockNumber, PolkaBTC<T>>,
        DispatchError,
    > {
        <vault_registry::Module<T>>::get_active_vault_from_id(vault_id)
    }

    pub fn get_premium_redeem_threshold<T: vault_registry::Config>(
    ) -> <T as vault_registry::Config>::UnsignedFixedPoint {
        <vault_registry::Module<T>>::get_premium_redeem_threshold()
    }

    pub fn get_liquidation_collateral_threshold<T: vault_registry::Config>(
    ) -> <T as vault_registry::Config>::UnsignedFixedPoint {
        <vault_registry::Module<T>>::get_liquidation_collateral_threshold()
    }
}
