#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod collateral {
    use crate::DOT;
    use frame_support::dispatch::DispatchResult;

    pub fn transfer<T: collateral::Trait>(
        sender: T::AccountId,
        receiver: T::AccountId,
        amount: DOT<T>,
    ) -> DispatchResult {
        <collateral::Module<T>>::transfer(sender, receiver, amount)
    }
}

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
    use crate::{PolkaBTC, DOT};
    use frame_support::dispatch::DispatchError;
    use sp_std::vec::Vec;

    pub fn get_relayer_rewards<T: sla::Trait>(
        total_reward_polka_btc: PolkaBTC<T>,
        total_reward_dot: DOT<T>,
    ) -> Result<Vec<(T::AccountId, PolkaBTC<T>, DOT<T>)>, DispatchError> {
        <sla::Module<T>>::get_relayer_rewards(total_reward_polka_btc, total_reward_dot)
    }

    pub fn get_vault_rewards<T: sla::Trait>(
        total_reward_for_issued_in_polka_btc: PolkaBTC<T>,
        total_reward_for_locked_in_polka_btc: PolkaBTC<T>,
        total_reward_for_issued_in_dot: DOT<T>,
        total_reward_for_locked_in_dot: DOT<T>,
    ) -> Result<Vec<(T::AccountId, PolkaBTC<T>, DOT<T>)>, DispatchError> {
        <sla::Module<T>>::get_vault_rewards(
            total_reward_for_issued_in_polka_btc,
            total_reward_for_locked_in_polka_btc,
            total_reward_for_issued_in_dot,
            total_reward_for_locked_in_dot,
        )
    }
}
