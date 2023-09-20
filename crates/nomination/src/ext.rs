#[cfg(test)]
use mocktopus::macros::mockable;

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
        amount: Option<Amount<T>>,
    ) -> Result<bool, DispatchError> {
        <vault_registry::Pallet<T>>::is_allowed_to_withdraw_collateral(vault_id, amount)
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

    pub mod pool_manager {
        use super::*;

        pub fn deposit_collateral<T: crate::Config>(
            vault_id: &DefaultVaultId<T>,
            nominator_id: &T::AccountId,
            amount: &Amount<T>,
        ) -> Result<(), DispatchError> {
            <vault_registry::PoolManager<T>>::deposit_collateral(vault_id, nominator_id, amount)
        }

        pub fn withdraw_collateral<T: crate::Config>(
            vault_id: &DefaultVaultId<T>,
            nominator_id: &T::AccountId,
            maybe_amount: Option<Amount<T>>,
            nonce: Option<<T as frame_system::Config>::Nonce>,
        ) -> Result<Amount<T>, DispatchError> {
            <vault_registry::PoolManager<T>>::withdraw_collateral(vault_id, nominator_id, maybe_amount, nonce)
        }

        pub fn kick_nominators<T: crate::Config>(vault_id: &DefaultVaultId<T>) -> Result<Amount<T>, DispatchError> {
            <vault_registry::PoolManager<T>>::kick_nominators(vault_id)
        }
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod staking {
    use crate::BalanceOf;
    use frame_support::dispatch::DispatchError;
    use staking::{RewardsApi, StakingApi};
    use vault_registry::DefaultVaultId;

    pub fn nonce<T: crate::Config>(vault_id: &DefaultVaultId<T>) -> T::Nonce {
        T::VaultStaking::nonce(vault_id)
    }

    pub fn compute_stake<T: vault_registry::Config>(
        vault_id: &DefaultVaultId<T>,
        nominator_id: &T::AccountId,
    ) -> Result<BalanceOf<T>, DispatchError> {
        T::VaultStaking::get_stake(&(None, vault_id.clone()), nominator_id)
    }
}
