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

#[cfg_attr(test, mockable)]
pub(crate) mod sla {
    use crate::PolkaBTC;
    use frame_support::dispatch::DispatchError;
    use sp_std::vec::Vec;

    pub fn get_relayer_rewards<T: sla::Trait>(
        total_reward: PolkaBTC<T>,
    ) -> Vec<(T::AccountId, Result<PolkaBTC<T>, DispatchError>)> {
        <sla::Module<T>>::get_relayer_rewards(total_reward)
    }
}
