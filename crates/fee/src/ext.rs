#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod collateral {
    use crate::Backing;
    use frame_support::dispatch::DispatchResult;

    type CollateralPallet<T> = currency::Pallet<T, currency::Collateral>;

    pub fn transfer<T: currency::Config<currency::Collateral>>(
        sender: T::AccountId,
        receiver: T::AccountId,
        amount: Backing<T>,
    ) -> DispatchResult {
        CollateralPallet::<T>::transfer(&sender, &receiver, amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod treasury {
    use crate::Issuing;
    use frame_support::dispatch::DispatchResult;

    type TreasuryPallet<T> = currency::Pallet<T, currency::Treasury>;

    pub fn transfer<T: currency::Config<currency::Treasury>>(
        sender: T::AccountId,
        receiver: T::AccountId,
        amount: Issuing<T>,
    ) -> DispatchResult {
        TreasuryPallet::<T>::transfer(&sender, &receiver, amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod sla {
    use crate::{Backing, Issuing};
    use frame_support::dispatch::DispatchError;
    use sp_std::vec::Vec;

    pub fn get_relayer_rewards<T: sla::Config>(
        total_reward_issuing: Issuing<T>,
        total_reward_backing: Backing<T>,
    ) -> Result<Vec<(T::AccountId, Issuing<T>, Backing<T>)>, DispatchError> {
        <sla::Pallet<T>>::get_relayer_rewards(total_reward_issuing, total_reward_backing)
    }

    pub fn get_vault_rewards<T: sla::Config>(
        total_reward_for_issued_in_issuing: Issuing<T>,
        total_reward_for_locked_in_issuing: Issuing<T>,
        total_reward_for_issued_in_backing: Backing<T>,
        total_reward_for_locked_in_backing: Backing<T>,
    ) -> Result<Vec<(T::AccountId, Issuing<T>, Backing<T>)>, DispatchError> {
        <sla::Pallet<T>>::get_vault_rewards(
            total_reward_for_issued_in_issuing,
            total_reward_for_locked_in_issuing,
            total_reward_for_issued_in_backing,
            total_reward_for_locked_in_backing,
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
