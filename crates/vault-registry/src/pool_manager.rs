use crate::*;
use traits::OnExchangeRateChange;

pub struct PoolManager<T>(PhantomData<T>);

impl<T: Config> PoolManager<T> {
    pub fn deposit_collateral(
        vault_id: &DefaultVaultId<T>,
        nominator_id: &T::AccountId,
        amount: &Amount<T>,
    ) -> Result<(), DispatchError> {
        ext::fee::distribute_all_vault_rewards::<T>(vault_id)?;
        ext::staking::deposit_stake(vault_id, nominator_id, amount)?;

        // also propagate to reward & capacity pools
        Self::update_reward_stake(vault_id)
    }

    pub fn withdraw_collateral(
        vault_id: &DefaultVaultId<T>,
        nominator_id: &T::AccountId,
        maybe_amount: Option<Amount<T>>,
        nonce: Option<<T as frame_system::Config>::Nonce>,
    ) -> Result<Amount<T>, DispatchError> {
        ext::fee::distribute_all_vault_rewards::<T>(vault_id)?;
        let amount = ext::staking::withdraw_stake(vault_id, nominator_id, maybe_amount, nonce)?;

        // also propagate to reward & capacity pools
        Self::update_reward_stake(vault_id)?;

        Ok(amount)
    }

    pub fn slash_collateral(vault_id: &DefaultVaultId<T>, amount: &Amount<T>) -> Result<(), DispatchError> {
        ext::fee::distribute_all_vault_rewards::<T>(vault_id)?;
        ext::staking::slash_stake(vault_id, amount)?;

        // also propagate to reward & capacity pools
        Self::update_reward_stake(vault_id)
    }

    pub fn kick_nominators(vault_id: &DefaultVaultId<T>) -> Result<Amount<T>, DispatchError> {
        ext::fee::distribute_all_vault_rewards::<T>(vault_id)?;
        let ret = ext::staking::force_refund::<T>(vault_id)?;

        // also propagate to reward & capacity pools
        Self::update_reward_stake(vault_id)?;

        Ok(ret)
    }

    // hook to be called _after_ one of the vault parameters that influences
    // capacity has been written. This is called when the custom secure threshold or
    // the accept_new_issues setting changes.
    pub fn on_vault_settings_change(vault_id: &DefaultVaultId<T>) -> Result<(), DispatchError> {
        ext::fee::distribute_all_vault_rewards::<T>(vault_id)?;
        Self::update_reward_stake(vault_id)
    }

    // NOTE: temporarily public for reward migration
    pub(crate) fn update_reward_stake(vault_id: &DefaultVaultId<T>) -> Result<(), DispatchError> {
        let vault = Pallet::<T>::get_vault_from_id(vault_id)?;
        let new_reward_stake = if !vault.accepts_new_issues() {
            // if the vault is not accepting new issues it's not getting rewards
            Amount::zero(vault_id.collateral_currency())
        } else {
            let total_collateral = ext::staking::total_current_stake::<T>(vault_id)?;
            let secure_threshold = Pallet::<T>::get_vault_secure_threshold(vault_id)?;

            total_collateral.checked_div(&secure_threshold)?
        };

        ext::reward::set_stake(vault_id, &new_reward_stake)?;

        // also propagate to capacity pool
        Self::update_capacity_stake(vault_id.collateral_currency())
    }

    fn update_capacity_stake(currency_id: CurrencyId<T>) -> Result<(), DispatchError> {
        let total_reward_stake = ext::reward::total_current_stake::<T>(currency_id)?;
        let new_capacity_stake = total_reward_stake.convert_to(T::GetWrappedCurrencyId::get())?;

        ext::capacity::set_stake(currency_id, &new_capacity_stake)
    }
}

impl<T: Config> OnExchangeRateChange<CurrencyId<T>> for PoolManager<T> {
    fn on_exchange_rate_change(currency_id: &CurrencyId<T>) {
        // todo: propagate error
        let _ = Self::update_capacity_stake(currency_id.clone());
    }
}
