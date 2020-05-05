#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod vault_registry {
    use crate::{PolkaBTC, DOT};
    use x_core::{Result, UnitResult};

    pub fn replace_tokens<T: vault_registry::Trait>(
        old_vault_id: T::AccountId,
        new_vault_id: T::AccountId,
        tokens: PolkaBTC<T>,
        collateral: DOT<T>,
    ) -> UnitResult {
        <vault_registry::Module<T>>::_replace_tokens(
            &old_vault_id,
            &new_vault_id,
            tokens,
            collateral,
        )
    }

    pub fn decrease_to_be_redeemed_tokens<T: vault_registry::Trait>(
        vault_id: T::AccountId,
        tokens: PolkaBTC<T>,
    ) -> UnitResult {
        <vault_registry::Module<T>>::_decrease_to_be_redeemed_tokens(&vault_id, tokens)
    }

    pub fn get_vault_from_id<T: vault_registry::Trait>(
        vault_id: &T::AccountId,
    ) -> Result<vault_registry::types::Vault<T::AccountId, T::BlockNumber, PolkaBTC<T>>> {
        <vault_registry::Module<T>>::_get_vault_from_id(vault_id)
    }

    pub fn increase_to_be_redeemed_tokens<T: vault_registry::Trait>(
        vault_id: &T::AccountId,
        tokens: PolkaBTC<T>,
    ) -> Result<()> {
        <vault_registry::Module<T>>::_increase_to_be_redeemed_tokens(vault_id, tokens)
    }

    pub fn is_over_minimum_collateral<T: vault_registry::Trait>(collateral: DOT<T>) -> bool {
        <vault_registry::Module<T>>::_is_over_minimum_collateral(collateral)
    }

    pub fn is_vault_below_auction_threshold<T: vault_registry::Trait>(
        _vault_id: T::AccountId,
    ) -> Result<bool> {
        // FIXME: call from vault registry
        unimplemented!()
    }

    pub fn is_collateral_below_secure_threshold<T: collateral::Trait>(
        _collateral: DOT<T>,
    ) -> Result<bool> {
        //FIXME:
        unimplemented!()
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod collateral {
    use crate::DOT;
    use x_core::UnitResult;

    pub fn get_collateral_from_account<T: collateral::Trait>(account: T::AccountId) -> DOT<T> {
        <collateral::Module<T>>::get_collateral_from_account(&account)
    }

    pub fn release_collateral<T: collateral::Trait>(
        sender: T::AccountId,
        amount: DOT<T>,
    ) -> UnitResult {
        <collateral::Module<T>>::release_collateral(&sender, amount)
    }

    pub fn slash_collateral<T: collateral::Trait>(
        old_vault_id: T::AccountId,
        new_vault_id: T::AccountId,
        collateral: DOT<T>,
    ) -> UnitResult {
        <collateral::Module<T>>::slash_collateral(old_vault_id, new_vault_id, collateral)
    }

    pub fn lock_collateral<T: collateral::Trait>(
        sender: T::AccountId,
        amount: DOT<T>,
    ) -> UnitResult {
        <collateral::Module<T>>::lock_collateral(&sender, amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    use primitive_types::H256;
    pub fn gen_secure_id<T: security::Trait>(_id: T::AccountId) -> H256 {
        unimplemented!()
    }
}
