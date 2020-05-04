#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod vault_registry {
    use crate::PolkaBTC;
    use x_core::Result;

    pub fn get_vault_from_id<T: vault_registry::Trait>(
        vault_id: &T::AccountId,
    ) -> Result<vault_registry::types::Vault<T::AccountId, T::BlockNumber, PolkaBTC<T>>> {
        <vault_registry::Module<T>>::_get_vault_from_id(vault_id)
    }

    /*
    pub fn increase_to_be_issued_tokens<T: vault_registry::Trait>(
        vault_id: &T::AccountId,
        amount: PolkaBTC<T>,
    ) -> Result<H160> {
        <vault_registry::Module<T>>::_increase_to_be_issued_tokens(vault_id, amount)
    }

    pub fn issue_tokens<T: vault_registry::Trait>(
        vault_id: &T::AccountId,
        amount: PolkaBTC<T>,
    ) -> UnitResult {
        <vault_registry::Module<T>>::_issue_tokens(vault_id, amount)
    }

    pub fn decrease_to_be_issued_tokens<T: vault_registry::Trait>(
        vault_id: &T::AccountId,
        amount: PolkaBTC<T>,
    ) -> UnitResult {
        <vault_registry::Module<T>>::_decrease_to_be_issued_tokens(vault_id, amount)
    }
    */
}
