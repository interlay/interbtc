#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod collateral {
    use crate::DOT;
    use frame_support::dispatch::DispatchResult;

    pub fn get_collateral_from_account<T: collateral::Trait>(account: T::AccountId) -> DOT<T> {
        <collateral::Module<T>>::get_collateral_from_account(&account)
    }

    pub fn release_collateral<T: collateral::Trait>(
        sender: T::AccountId,
        amount: DOT<T>,
    ) -> DispatchResult {
        <collateral::Module<T>>::release_collateral(&sender, amount)
    }

    pub fn slash_collateral<T: collateral::Trait>(
        old_vault_id: T::AccountId,
        new_vault_id: T::AccountId,
        collateral: DOT<T>,
    ) -> DispatchResult {
        <collateral::Module<T>>::slash_collateral(old_vault_id, new_vault_id, collateral)
    }

    pub fn lock_collateral<T: collateral::Trait>(
        sender: T::AccountId,
        amount: DOT<T>,
    ) -> DispatchResult {
        <collateral::Module<T>>::lock_collateral(&sender, amount)
    }
}
