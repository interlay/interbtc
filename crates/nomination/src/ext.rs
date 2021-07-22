#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    use frame_support::dispatch::DispatchResult;

    pub fn ensure_parachain_status_running<T: crate::Config>() -> DispatchResult {
        <security::Pallet<T>>::ensure_parachain_status_running()
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod collateral {
    use crate::types::Collateral;
    use currency::ParachainCurrency;
    use frame_support::dispatch::DispatchResult;

    pub fn transfer_and_lock<T: crate::Config>(
        source: &T::AccountId,
        destination: &T::AccountId,
        amount: Collateral<T>,
    ) -> DispatchResult {
        <T as vault_registry::Config>::Collateral::transfer_and_lock(source, destination, amount)
    }

    pub fn unlock_and_transfer<T: crate::Config>(
        source: &T::AccountId,
        destination: &T::AccountId,
        amount: Collateral<T>,
    ) -> DispatchResult {
        <T as vault_registry::Config>::Collateral::unlock_and_transfer(source, destination, amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod vault_registry {
    use crate::{types::UnsignedFixedPoint, Collateral};
    pub use frame_support::dispatch::{DispatchError, DispatchResult};
    pub use vault_registry::{DefaultVault, VaultStatus};

    pub fn get_backing_collateral<T: crate::Config>(vault_id: &T::AccountId) -> Result<Collateral<T>, DispatchError> {
        <vault_registry::Pallet<T>>::get_backing_collateral(vault_id)
    }

    pub fn vault_exists<T: crate::Config>(id: &T::AccountId) -> bool {
        <vault_registry::Pallet<T>>::vault_exists(id)
    }

    pub fn compute_collateral<T: crate::Config>(vault_id: &T::AccountId) -> Result<Collateral<T>, DispatchError> {
        <vault_registry::Pallet<T>>::compute_collateral(vault_id)
    }

    pub fn is_allowed_to_withdraw_collateral<T: crate::Config>(
        vault_id: &T::AccountId,
        amount: Collateral<T>,
    ) -> Result<bool, DispatchError> {
        <vault_registry::Pallet<T>>::is_allowed_to_withdraw_collateral(vault_id, amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod fee {
    use crate::types::{Collateral, UnsignedFixedPoint};
    use frame_support::dispatch::{DispatchError, DispatchResult};

    pub fn collateral_for<T: crate::Config>(
        amount: Collateral<T>,
        percentage: UnsignedFixedPoint<T>,
    ) -> Result<Collateral<T>, DispatchError> {
        <fee::Pallet<T>>::collateral_for(amount, percentage)
    }

    pub fn withdraw_all_vault_rewards<T: fee::Config>(account_id: &T::AccountId) -> DispatchResult {
        <fee::Pallet<T>>::withdraw_all_vault_rewards(account_id)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod staking {
    use crate::types::{SignedFixedPoint, SignedInner};
    use frame_support::{dispatch::DispatchError, traits::Get};

    pub fn deposit_stake<T: crate::Config>(
        vault_id: &T::AccountId,
        nominator_id: &T::AccountId,
        amount: SignedFixedPoint<T>,
    ) -> Result<(), DispatchError> {
        <staking::Pallet<T>>::deposit_stake(T::GetRewardsCurrencyId::get(), vault_id, nominator_id, amount)
    }

    pub fn withdraw_stake<T: crate::Config>(
        vault_id: &T::AccountId,
        nominator_id: &T::AccountId,
        amount: SignedFixedPoint<T>,
    ) -> Result<(), DispatchError> {
        <staking::Pallet<T>>::withdraw_stake(T::GetRewardsCurrencyId::get(), vault_id, nominator_id, amount)
    }

    pub fn compute_stake<T: crate::Config>(
        vault_id: &T::AccountId,
        nominator_id: &T::AccountId,
    ) -> Result<SignedInner<T>, DispatchError> {
        <staking::Pallet<T>>::compute_stake(T::GetRewardsCurrencyId::get(), vault_id, nominator_id)
    }

    pub fn force_refund<T: crate::Config>(vault_id: &T::AccountId) -> Result<(), DispatchError> {
        <staking::Pallet<T>>::force_refund(T::GetRewardsCurrencyId::get(), vault_id)
    }
}
