#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod currency {
    use crate::types::CurrencyId;
    use currency::Amount;

    pub fn get_free_balance<T: crate::Config>(currency_id: CurrencyId<T>, id: &T::AccountId) -> Amount<T> {
        currency::get_free_balance::<T>(currency_id, id)
    }

    pub fn get_reserved_balance<T: crate::Config>(currency_id: CurrencyId<T>, id: &T::AccountId) -> Amount<T> {
        currency::get_reserved_balance::<T>(currency_id, id)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod oracle {
    use crate::CurrencyId;
    use frame_support::dispatch::DispatchError;
    use oracle::{types::UnsignedFixedPoint, OracleKey};

    pub fn get_price<T: crate::Config>(currency_id: CurrencyId<T>) -> Result<UnsignedFixedPoint<T>, DispatchError> {
        <oracle::Pallet<T>>::get_price(OracleKey::ExchangeRate(currency_id))
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    pub fn active_block_number<T: crate::Config>() -> T::BlockNumber {
        <security::Pallet<T>>::active_block_number()
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod staking {
    use crate::{types::BalanceOf, DefaultVaultId};
    use currency::Amount;
    use frame_support::dispatch::{DispatchError, DispatchResult};
    use staking::{RewardsApi, StakingApi};

    pub fn deposit_stake<T: crate::Config>(
        vault_id: &DefaultVaultId<T>,
        nominator_id: &T::AccountId,
        amount: &Amount<T>,
    ) -> DispatchResult {
        T::VaultStaking::deposit_stake(&(None, vault_id.clone()), nominator_id, amount.amount())
    }

    pub fn withdraw_stake<T: crate::Config>(
        vault_id: &DefaultVaultId<T>,
        nominator_id: &T::AccountId,
        amount: &Amount<T>,
    ) -> DispatchResult {
        T::VaultStaking::withdraw_stake(&(None, vault_id.clone()), nominator_id, amount.amount())
    }

    pub fn slash_stake<T: crate::Config>(vault_id: &DefaultVaultId<T>, amount: &Amount<T>) -> DispatchResult {
        T::VaultStaking::slash_stake(vault_id, amount.amount())
    }

    pub fn compute_stake<T: crate::Config>(
        vault_id: &DefaultVaultId<T>,
        nominator_id: &T::AccountId,
    ) -> Result<BalanceOf<T>, DispatchError> {
        T::VaultStaking::get_stake(&(None, vault_id.clone()), nominator_id)
    }

    pub fn total_current_stake<T: crate::Config>(vault_id: &DefaultVaultId<T>) -> Result<BalanceOf<T>, DispatchError> {
        T::VaultStaking::get_total_stake(&(None, vault_id.clone()))
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod reward {
    use crate::DefaultVaultId;
    use currency::Amount;
    use frame_support::dispatch::DispatchError;
    use reward::RewardsApi;

    pub fn set_stake<T: crate::Config>(vault_id: &DefaultVaultId<T>, amount: &Amount<T>) -> Result<(), DispatchError> {
        T::VaultRewards::set_stake(&(), vault_id, amount.amount())
    }

    #[cfg(feature = "integration-tests")]
    pub fn get_stake<T: crate::Config>(vault_id: &DefaultVaultId<T>) -> Result<crate::BalanceOf<T>, DispatchError> {
        T::VaultRewards::get_stake(&(), vault_id)
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
