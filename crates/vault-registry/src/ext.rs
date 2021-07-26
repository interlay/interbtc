#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod collateral {
    use crate::types::Collateral;
    use currency::ParachainCurrency;
    use frame_support::dispatch::DispatchResult;

    pub fn transfer<T: crate::Config>(
        source: &T::AccountId,
        destination: &T::AccountId,
        amount: Collateral<T>,
    ) -> DispatchResult {
        T::Collateral::transfer(source, destination, amount)
    }

    pub fn lock<T: crate::Config>(sender: &T::AccountId, amount: Collateral<T>) -> DispatchResult {
        T::Collateral::lock(sender, amount)
    }

    pub fn unlock<T: crate::Config>(sender: &T::AccountId, amount: Collateral<T>) -> DispatchResult {
        T::Collateral::unlock(sender, amount)
    }

    pub fn get_reserved_balance<T: crate::Config>(id: &T::AccountId) -> Collateral<T> {
        T::Collateral::get_reserved_balance(id)
    }

    pub fn get_free_balance<T: crate::Config>(id: &T::AccountId) -> Collateral<T> {
        T::Collateral::get_free_balance(id)
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
    use crate::types::{Collateral, Wrapped};
    use frame_support::dispatch::DispatchError;

    pub fn wrapped_to_collateral<T: crate::Config>(amount: Wrapped<T>) -> Result<Collateral<T>, DispatchError> {
        <exchange_rate_oracle::Pallet<T>>::wrapped_to_collateral(amount)
    }

    pub fn collateral_to_wrapped<T: crate::Config>(amount: Collateral<T>) -> Result<Wrapped<T>, DispatchError> {
        <exchange_rate_oracle::Pallet<T>>::collateral_to_wrapped(amount)
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
pub(crate) mod sla {
    use crate::types::{BalanceOf, UnsignedFixedPoint};
    use frame_support::dispatch::DispatchError;
    pub use sla::Action;

    pub fn calculate_slashed_amount<T: crate::Config>(
        vault_id: &T::AccountId,
        stake: BalanceOf<T>,
        reimburse: bool,
        liquidation_threshold: UnsignedFixedPoint<T>,
        premium_redeem_threshold: UnsignedFixedPoint<T>,
    ) -> Result<BalanceOf<T>, DispatchError> {
        <sla::Pallet<T>>::calculate_slashed_amount(
            vault_id,
            stake,
            reimburse,
            liquidation_threshold,
            premium_redeem_threshold,
        )
    }

    pub fn event_update_vault_sla<T: crate::Config>(
        vault_id: &T::AccountId,
        action: Action<BalanceOf<T>>,
    ) -> Result<(), DispatchError> {
        <sla::Pallet<T>>::event_update_vault_sla(vault_id, action)
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

    pub fn slash_stake<T: crate::Config>(
        vault_id: &T::AccountId,
        amount: SignedFixedPoint<T>,
    ) -> Result<(), DispatchError> {
        <staking::Pallet<T>>::slash_stake(T::GetRewardsCurrencyId::get(), vault_id, amount)
    }

    pub fn unslash_stake<T: crate::Config>(
        vault_id: &T::AccountId,
        amount: SignedFixedPoint<T>,
    ) -> Result<(), DispatchError> {
        <staking::Pallet<T>>::unslash_stake(T::GetRewardsCurrencyId::get(), vault_id, amount)
    }

    pub fn compute_stake<T: crate::Config>(
        vault_id: &T::AccountId,
        nominator_id: &T::AccountId,
    ) -> Result<SignedInner<T>, DispatchError> {
        <staking::Pallet<T>>::compute_stake(T::GetRewardsCurrencyId::get(), vault_id, nominator_id)
    }

    pub fn total_current_stake<T: crate::Config>(vault_id: &T::AccountId) -> Result<SignedInner<T>, DispatchError> {
        <staking::Pallet<T>>::total_current_stake(T::GetRewardsCurrencyId::get(), vault_id)
    }
}
