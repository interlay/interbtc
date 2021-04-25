#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod collateral {
    use crate::DOT;
    use frame_support::dispatch::DispatchResult;

    pub fn get_free_balance<T: collateral::Config>(id: &T::AccountId) -> DOT<T> {
        <collateral::Pallet<T>>::get_balance_from_account(id)
    }

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

    pub fn get_free_balance<T: treasury::Config>(id: T::AccountId) -> PolkaBTC<T> {
        <treasury::Pallet<T>>::get_balance_from_account(id)
    }

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

    pub fn dot_to_u128<T: sla::Config>(x: DOT<T>) -> Result<u128, DispatchError> {
        <sla::Pallet<T>>::dot_to_u128(x)
    }

    pub fn u128_to_dot<T: sla::Config>(x: u128) -> Result<DOT<T>, DispatchError> {
        <sla::Pallet<T>>::u128_to_dot(x)
    }

    pub fn polkabtc_to_u128<T: sla::Config>(x: PolkaBTC<T>) -> Result<u128, DispatchError> {
        <sla::Pallet<T>>::polkabtc_to_u128(x)
    }

    pub fn u128_to_polkabtc<T: sla::Config>(x: u128) -> Result<PolkaBTC<T>, DispatchError> {
        <sla::Pallet<T>>::u128_to_polkabtc(x)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    use frame_support::dispatch::DispatchError;

    pub fn ensure_parachain_status_not_shutdown<T: security::Config>() -> Result<(), DispatchError> {
        <security::Pallet<T>>::ensure_parachain_status_not_shutdown()
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod nomination {
    use nomination::Nominator;
    use sp_runtime::DispatchError;
    use sp_std::vec::Vec;

    use crate::types::DOT;

    pub fn is_operator<T: nomination::Config>(operator_id: &T::AccountId) -> Result<bool, DispatchError> {
        <nomination::Module<T>>::is_operator(operator_id)
    }

    pub fn scale_amount_by_nominator_proportion<T: nomination::Config>(
        nominator_id: &T::AccountId,
        operator_id: &T::AccountId,
        amount: u128,
    ) -> Result<u128, DispatchError> {
        <nomination::Module<T>>::scale_amount_by_nominator_proportion_of_backing_collateral(
            nominator_id,
            operator_id,
            amount,
        )
    }

    pub fn scale_amount_by_operator_proportion<T: nomination::Config>(
        operator_id: &T::AccountId,
        amount: u128,
    ) -> Result<u128, DispatchError> {
        <nomination::Module<T>>::scale_amount_by_operator_proportion_of_backing_collateral(operator_id, amount)
    }

    pub fn get_nominators<T: nomination::Config>(
        operator_id: &T::AccountId,
    ) -> Result<Vec<Nominator<T::AccountId, T::BlockNumber, DOT<T>>>, DispatchError> {
        <nomination::Module<T>>::get_nominators(operator_id)
    }
}
