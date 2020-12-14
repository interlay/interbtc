#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod collateral {
    use crate::DOT;

    pub fn get_collateral_from_account<T: collateral::Trait>(account: T::AccountId) -> DOT<T> {
        <collateral::Module<T>>::get_collateral_from_account(&account)
    }

    pub fn get_total_collateral<T: collateral::Trait>() -> DOT<T> {
        <collateral::Module<T>>::get_total_collateral()
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod vault_registry {
    pub fn get_liquidation_collateral_threshold<T: vault_registry::Trait>() -> u128 {
        <vault_registry::Module<T>>::_get_liquidation_collateral_threshold()
    }

    pub fn get_premium_redeem_threshold<T: vault_registry::Trait>() -> u128 {
        <vault_registry::Module<T>>::_get_premium_redeem_threshold()
    }
}
