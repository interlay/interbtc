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
    use crate::types::{CurrencyId, SignedInner};
    use currency::Amount;
    use frame_support::dispatch::DispatchError;

    pub fn deposit_stake<T: crate::Config>(
        currency_id: CurrencyId<T>,
        vault_id: &T::AccountId,
        nominator_id: &T::AccountId,
        amount: &Amount<T>,
    ) -> Result<(), DispatchError> {
        <staking::Pallet<T>>::deposit_stake(currency_id, vault_id, nominator_id, amount.to_signed_fixed_point()?)
    }

    pub fn withdraw_stake<T: crate::Config>(
        currency_id: CurrencyId<T>,
        vault_id: &T::AccountId,
        nominator_id: &T::AccountId,
        amount: &Amount<T>,
    ) -> Result<(), DispatchError> {
        <staking::Pallet<T>>::withdraw_stake(currency_id, vault_id, nominator_id, amount.to_signed_fixed_point()?)
    }

    pub fn slash_stake<T: crate::Config>(
        currency_id: CurrencyId<T>,
        vault_id: &T::AccountId,
        amount: &Amount<T>,
    ) -> Result<(), DispatchError> {
        <staking::Pallet<T>>::slash_stake(currency_id, vault_id, amount.to_signed_fixed_point()?)
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
    use currency::Amount;
    use frame_support::{dispatch::DispatchError, traits::Get};

    pub fn deposit_stake<T: crate::Config>(vault_id: &T::AccountId, amount: &Amount<T>) -> Result<(), DispatchError> {
        <reward::Pallet<T>>::deposit_stake(
            T::GetWrappedCurrencyId::get(),
            vault_id,
            amount.to_signed_fixed_point()?,
        )
    }

    pub fn withdraw_stake<T: crate::Config>(vault_id: &T::AccountId, amount: &Amount<T>) -> Result<(), DispatchError> {
        <reward::Pallet<T>>::withdraw_stake(
            T::GetWrappedCurrencyId::get(),
            vault_id,
            amount.to_signed_fixed_point()?,
        )
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod fee {
    use currency::Amount;
    use frame_support::{dispatch::DispatchError, traits::Get};

    pub fn get_theft_fee<T: crate::Config>(amount: &Amount<T>) -> Result<Amount<T>, DispatchError> {
        <fee::Pallet<T>>::get_theft_fee(amount)
    }

    pub fn get_theft_fee_max<T: crate::Config>() -> Amount<T> {
        Amount::new(<fee::Pallet<T>>::theft_fee_max(), T::GetWrappedCurrencyId::get())
    }
}
