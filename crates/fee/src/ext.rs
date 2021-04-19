#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod collateral {
    use crate::DOT;
    use frame_support::dispatch::DispatchResult;

    pub fn transfer<T: collateral::Config>(
        sender: T::AccountId,
        receiver: T::AccountId,
        amount: DOT<T>,
    ) -> DispatchResult {
        <collateral::Pallet<T>>::transfer(sender, receiver, amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod treasury {
    use crate::PolkaBTC;
    use frame_support::dispatch::DispatchResult;

    pub fn transfer<T: treasury::Config>(
        sender: T::AccountId,
        receiver: T::AccountId,
        amount: PolkaBTC<T>,
    ) -> DispatchResult {
        <treasury::Pallet<T>>::transfer(sender, receiver, amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod sla {
    use crate::{PolkaBTC, DOT};
    use frame_support::dispatch::DispatchError;
    use sp_std::vec::Vec;

    pub fn get_relayer_rewards<T: sla::Config>(
        total_reward_polka_btc: PolkaBTC<T>,
        total_reward_dot: DOT<T>,
    ) -> Result<Vec<(T::AccountId, PolkaBTC<T>, DOT<T>)>, DispatchError> {
        <sla::Pallet<T>>::get_relayer_rewards(total_reward_polka_btc, total_reward_dot)
    }

    pub fn get_vault_rewards<T: sla::Config>(
        total_reward_for_issued_in_polka_btc: PolkaBTC<T>,
        total_reward_for_locked_in_polka_btc: PolkaBTC<T>,
        total_reward_for_issued_in_dot: DOT<T>,
        total_reward_for_locked_in_dot: DOT<T>,
    ) -> Result<Vec<(T::AccountId, PolkaBTC<T>, DOT<T>)>, DispatchError> {
        <sla::Pallet<T>>::get_vault_rewards(
            total_reward_for_issued_in_polka_btc,
            total_reward_for_locked_in_polka_btc,
            total_reward_for_issued_in_dot,
            total_reward_for_locked_in_dot,
        )
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    use frame_support::dispatch::DispatchError;

    pub fn ensure_parachain_status_not_shutdown<T: security::Config>() -> Result<(), DispatchError> {
        <security::Pallet<T>>::ensure_parachain_status_not_shutdown()
    }
}
