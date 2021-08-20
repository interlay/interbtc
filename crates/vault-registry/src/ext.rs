#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod currency {
    use crate::types::{Collateral, CurrencyId};
    use frame_support::dispatch::DispatchResult;

    pub fn lock<T: crate::Config>(
        currency_id: CurrencyId<T>,
        account: &T::AccountId,
        amount: Collateral<T>,
    ) -> DispatchResult {
        currency::with_currency_id::lock::<T>(currency_id, account, amount)
    }

    pub fn unlock<T: crate::Config>(
        currency_id: CurrencyId<T>,
        account: &T::AccountId,
        amount: Collateral<T>,
    ) -> DispatchResult {
        currency::with_currency_id::unlock::<T>(currency_id, account, amount)
    }

    pub fn get_free_balance<T: crate::Config>(currency_id: CurrencyId<T>, id: &T::AccountId) -> Collateral<T> {
        currency::with_currency_id::get_free_balance::<T>(currency_id, id)
    }

    pub fn get_reserved_balance<T: crate::Config>(currency_id: CurrencyId<T>, id: &T::AccountId) -> Collateral<T> {
        currency::with_currency_id::get_reserved_balance::<T>(currency_id, id)
    }

    pub fn transfer<T: crate::Config>(
        currency_id: CurrencyId<T>,
        source: &T::AccountId,
        destination: &T::AccountId,
        amount: Collateral<T>,
    ) -> DispatchResult {
        currency::with_currency_id::transfer::<T>(currency_id, source, destination, amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod treasury {
    use crate::{types::Wrapped, Config};
    use currency::ParachainCurrency;

    pub fn total_issued<T: crate::Config>() -> Wrapped<T> {
        <T as Config>::Wrapped::get_total_supply()
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod oracle {
    use crate::types::{Collateral, CurrencyId, Wrapped};
    use frame_support::dispatch::DispatchError;

    pub fn wrapped_to_collateral<T: crate::Config>(
        amount: Wrapped<T>,
        currency_id: CurrencyId<T>,
    ) -> Result<Collateral<T>, DispatchError> {
        <exchange_rate_oracle::Pallet<T>>::wrapped_to_collateral(amount, currency_id)
    }

    pub fn collateral_to_wrapped<T: crate::Config>(
        amount: Collateral<T>,
        currency_id: CurrencyId<T>,
    ) -> Result<Wrapped<T>, DispatchError> {
        <exchange_rate_oracle::Pallet<T>>::collateral_to_wrapped(amount, currency_id)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    use frame_support::dispatch::DispatchResult;

    pub fn ensure_parachain_status_not_shutdown<T: crate::Config>() -> DispatchResult {
        <security::Pallet<T>>::ensure_parachain_status_not_shutdown()
    }

    pub fn active_block_number<T: crate::Config>() -> T::BlockNumber {
        <security::Pallet<T>>::active_block_number()
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod staking {
    use crate::{
        types::{BalanceOf, CurrencyId, SignedInner},
        Pallet,
    };
    use frame_support::dispatch::DispatchError;

    pub fn deposit_stake<T: crate::Config>(
        currency_id: CurrencyId<T>,
        vault_id: &T::AccountId,
        nominator_id: &T::AccountId,
        amount: BalanceOf<T>,
    ) -> Result<(), DispatchError> {
        <staking::Pallet<T>>::deposit_stake(
            currency_id,
            vault_id,
            nominator_id,
            Pallet::<T>::currency_to_fixed(amount)?,
        )
    }

    pub fn withdraw_stake<T: crate::Config>(
        currency_id: CurrencyId<T>,
        vault_id: &T::AccountId,
        nominator_id: &T::AccountId,
        amount: BalanceOf<T>,
    ) -> Result<(), DispatchError> {
        <staking::Pallet<T>>::withdraw_stake(
            currency_id,
            vault_id,
            nominator_id,
            Pallet::<T>::currency_to_fixed(amount)?,
        )
    }

    pub fn slash_stake<T: crate::Config>(
        currency_id: CurrencyId<T>,
        vault_id: &T::AccountId,
        amount: BalanceOf<T>,
    ) -> Result<(), DispatchError> {
        <staking::Pallet<T>>::slash_stake(currency_id, vault_id, Pallet::<T>::currency_to_fixed(amount)?)
    }

    pub fn compute_stake<T: crate::Config>(
        currency_id: CurrencyId<T>,
        vault_id: &T::AccountId,
        nominator_id: &T::AccountId,
    ) -> Result<SignedInner<T>, DispatchError> {
        <staking::Pallet<T>>::compute_stake(currency_id, vault_id, nominator_id)
    }

    pub fn total_current_stake<T: crate::Config>(
        currency_id: CurrencyId<T>,
        vault_id: &T::AccountId,
    ) -> Result<SignedInner<T>, DispatchError> {
        <staking::Pallet<T>>::total_current_stake(currency_id, vault_id)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod reward {
    use crate::{types::BalanceOf, Pallet};
    use frame_support::{dispatch::DispatchError, traits::Get};

    pub fn deposit_stake<T: crate::Config>(vault_id: &T::AccountId, amount: BalanceOf<T>) -> Result<(), DispatchError> {
        <reward::Pallet<T>>::deposit_stake(
            T::GetRewardsCurrencyId::get(),
            vault_id,
            Pallet::<T>::currency_to_fixed(amount)?,
        )
    }

    pub fn withdraw_stake<T: crate::Config>(
        vault_id: &T::AccountId,
        amount: BalanceOf<T>,
    ) -> Result<(), DispatchError> {
        <reward::Pallet<T>>::withdraw_stake(
            T::GetRewardsCurrencyId::get(),
            vault_id,
            Pallet::<T>::currency_to_fixed(amount)?,
        )
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod fee {
    use crate::{Collateral, Wrapped};
    use frame_support::dispatch::DispatchError;

    pub fn get_theft_fee<T: crate::Config>(amount: Collateral<T>) -> Result<Collateral<T>, DispatchError> {
        <fee::Pallet<T>>::get_theft_fee(amount)
    }

    pub fn get_theft_fee_max<T: crate::Config>() -> Wrapped<T> {
        <fee::Pallet<T>>::theft_fee_max()
    }
}
