#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod treasury {
    use crate::PolkaBTC;
    use frame_support::dispatch::DispatchResult;

    pub fn transfer<T: treasury::Trait>(
        sender: T::AccountId,
        receiver: T::AccountId,
        amount: PolkaBTC<T>,
    ) -> DispatchResult {
        <treasury::Module<T>>::transfer(sender, receiver, amount)
    }
}
