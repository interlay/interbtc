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
pub(crate) mod currency {
    use crate::types::Collateral;
    use frame_support::dispatch::DispatchResult;
    use vault_registry::types::CurrencyId;

    pub fn transfer_and_lock<T: crate::Config>(
        currency_id: CurrencyId<T>,
        from: &T::AccountId,
        to: &T::AccountId,
        amount: Collateral<T>,
    ) -> DispatchResult {
        currency::with_currency_id::transfer_and_lock::<T>(currency_id, from, to, amount)
    }

    pub fn unlock_and_transfer<T: crate::Config>(
        currency_id: CurrencyId<T>,
        from: &T::AccountId,
        to: &T::AccountId,
        amount: Collateral<T>,
    ) -> DispatchResult {
        currency::with_currency_id::unlock_and_transfer::<T>(currency_id, from, to, amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod vault_registry {
    use crate::Collateral;
    pub use frame_support::dispatch::{DispatchError, DispatchResult};
    pub use vault_registry::{types::CurrencyId, DefaultVault, VaultStatus};

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

    pub fn get_max_nominatable_collateral<T: crate::Config>(
        currency_id: CurrencyId<T>,
        vault_collateral: Collateral<T>,
    ) -> Result<Collateral<T>, DispatchError> {
        <vault_registry::Pallet<T>>::get_max_nominatable_collateral(currency_id, vault_collateral)
    }

    pub fn get_collateral_currency<T: crate::Config>(vault_id: &T::AccountId) -> Result<CurrencyId<T>, DispatchError> {
        <vault_registry::Pallet<T>>::get_collateral_currency(vault_id)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod fee {
    use frame_support::dispatch::DispatchResult;

    pub fn withdraw_all_vault_rewards<T: fee::Config>(account_id: &T::AccountId) -> DispatchResult {
        <fee::Pallet<T>>::withdraw_all_vault_rewards(account_id)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod staking {
    use crate::types::{SignedFixedPoint, SignedInner};
    use frame_support::dispatch::DispatchError;
    use vault_registry::types::CurrencyId;

    pub fn deposit_stake<T: crate::Config>(
        currency_id: CurrencyId<T>,
        vault_id: &T::AccountId,
        nominator_id: &T::AccountId,
        amount: SignedFixedPoint<T>,
    ) -> Result<(), DispatchError> {
        <staking::Pallet<T>>::deposit_stake(currency_id, vault_id, nominator_id, amount)
    }

    pub fn withdraw_stake<T: crate::Config>(
        currency_id: CurrencyId<T>,
        vault_id: &T::AccountId,
        nominator_id: &T::AccountId,
        amount: SignedFixedPoint<T>,
    ) -> Result<(), DispatchError> {
        <staking::Pallet<T>>::withdraw_stake(currency_id, vault_id, nominator_id, amount)
    }

    pub fn compute_stake<T: crate::Config>(
        currency_id: CurrencyId<T>,
        vault_id: &T::AccountId,
        nominator_id: &T::AccountId,
    ) -> Result<SignedInner<T>, DispatchError> {
        <staking::Pallet<T>>::compute_stake(currency_id, vault_id, nominator_id)
    }

    pub fn force_refund<T: crate::Config>(
        currency_id: CurrencyId<T>,
        vault_id: &T::AccountId,
    ) -> Result<(), DispatchError> {
        <staking::Pallet<T>>::force_refund(currency_id, vault_id)
    }
}
