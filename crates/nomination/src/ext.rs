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
pub(crate) mod vault_registry {
    use currency::Amount;
    pub use frame_support::dispatch::{DispatchError, DispatchResult};
    pub use vault_registry::{types::CurrencyId, DefaultVault, VaultStatus};
    use vault_registry::{types::DefaultVaultCurrencyPair, DefaultVaultId};

    pub fn get_backing_collateral<T: crate::Config>(vault_id: &DefaultVaultId<T>) -> Result<Amount<T>, DispatchError> {
        <vault_registry::Pallet<T>>::get_backing_collateral(vault_id)
    }

    pub fn vault_exists<T: crate::Config>(vault_id: &DefaultVaultId<T>) -> bool {
        <vault_registry::Pallet<T>>::vault_exists(vault_id)
    }

    pub fn compute_collateral<T: crate::Config>(vault_id: &DefaultVaultId<T>) -> Result<Amount<T>, DispatchError> {
        <vault_registry::Pallet<T>>::compute_collateral(vault_id)
    }

    pub fn is_allowed_to_withdraw_collateral<T: crate::Config>(
        vault_id: &DefaultVaultId<T>,
        amount: &Amount<T>,
    ) -> Result<bool, DispatchError> {
        <vault_registry::Pallet<T>>::is_allowed_to_withdraw_collateral(vault_id, amount)
    }

    pub fn get_max_nominatable_collateral<T: crate::Config>(
        vault_collateral: &Amount<T>,
        currency_pair: &DefaultVaultCurrencyPair<T>,
    ) -> Result<Amount<T>, DispatchError> {
        <vault_registry::Pallet<T>>::get_max_nominatable_collateral(vault_collateral, currency_pair)
    }

    pub fn try_increase_total_backing_collateral<T: crate::Config>(
        currency_pair: &DefaultVaultCurrencyPair<T>,
        amount: &Amount<T>,
    ) -> DispatchResult {
        <vault_registry::Pallet<T>>::try_increase_total_backing_collateral(currency_pair, amount)
    }

    pub fn decrease_total_backing_collateral<T: crate::Config>(
        currency_pair: &DefaultVaultCurrencyPair<T>,
        amount: &Amount<T>,
    ) -> DispatchResult {
        <vault_registry::Pallet<T>>::decrease_total_backing_collateral(currency_pair, amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod fee {
    use crate::DefaultVaultId;
    use frame_support::dispatch::DispatchResult;

    pub fn withdraw_all_vault_rewards<T: crate::Config>(vault_id: &DefaultVaultId<T>) -> DispatchResult {
        <fee::Pallet<T>>::withdraw_all_vault_rewards(vault_id)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod staking {
    use crate::BalanceOf;
    use frame_support::dispatch::DispatchError;
    use staking::Staking;
    use vault_registry::DefaultVaultId;

    pub fn nonce<T: crate::Config>(vault_id: &DefaultVaultId<T>) -> T::Index {
        T::VaultStaking::nonce(vault_id)
    }

    pub fn deposit_stake<T: crate::Config>(
        vault_id: &DefaultVaultId<T>,
        nominator_id: &T::AccountId,
        amount: BalanceOf<T>,
    ) -> Result<(), DispatchError> {
        T::VaultStaking::deposit_stake(vault_id, nominator_id, amount)
    }

    pub fn withdraw_stake<T: crate::Config>(
        vault_id: &DefaultVaultId<T>,
        nominator_id: &T::AccountId,
        amount: BalanceOf<T>,
        index: Option<T::Index>,
    ) -> Result<(), DispatchError> {
        T::VaultStaking::withdraw_stake(vault_id, nominator_id, amount, index)
    }

    pub fn compute_stake<T: vault_registry::Config>(
        vault_id: &DefaultVaultId<T>,
        nominator_id: &T::AccountId,
    ) -> Result<BalanceOf<T>, DispatchError> {
        T::VaultStaking::compute_stake(vault_id, nominator_id)
    }

    pub fn force_refund<T: crate::Config>(vault_id: &DefaultVaultId<T>) -> Result<BalanceOf<T>, DispatchError> {
        T::VaultStaking::force_refund(vault_id)
    }
}
