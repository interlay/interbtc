#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod collateral {
    use crate::DOT;
    use frame_support::dispatch::DispatchResult;

    pub fn get_collateral_from_account<T: collateral::Trait>(account: T::AccountId) -> DOT<T> {
        <collateral::Module<T>>::get_collateral_from_account(&account)
    }

    pub fn get_total_collateral<T: collateral::Trait>() -> DOT<T> {
        <collateral::Module<T>>::get_total_collateral()
    }
}
