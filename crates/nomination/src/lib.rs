//! # Nomination Module

#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
extern crate mocktopus;

#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg(test)]
mod tests;

mod ext;
mod types;

mod default_weights;

use frame_support::{
    dispatch::{DispatchError, DispatchResult},
    ensure,
    traits::Get,
    transactional,
    weights::Weight,
};
use frame_system::{ensure_root, ensure_signed};
use sp_std::convert::TryInto;
use types::{Collateral, SignedFixedPoint};

pub trait WeightInfo {
    fn set_nomination_enabled() -> Weight;
    fn opt_in_to_nomination() -> Weight;
    fn opt_out_of_nomination() -> Weight;
    fn deposit_collateral() -> Weight;
    fn withdraw_collateral() -> Weight;
}

use currency::Amount;
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    /// ## Configuration
    /// The pallet's configuration trait.
    #[pallet::config]
    pub trait Config: frame_system::Config + security::Config + vault_registry::Config + fee::Config {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Weight information for the extrinsics in this module.
        type WeightInfo: WeightInfo;

        /// Vault reward pool for the wrapped currency.
        type VaultRewards: reward::Rewards<Self::AccountId, SignedFixedPoint = SignedFixedPoint<Self>>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    #[pallet::metadata(T::AccountId = "AccountId", Collateral<T> = "Collateral")]
    pub enum Event<T: Config> {
        // [vault_id]
        NominationOptIn(T::AccountId),
        // [vault_id]
        NominationOptOut(T::AccountId),
        // [vault_id, nominator_id, collateral]
        DepositCollateral(T::AccountId, T::AccountId, Collateral<T>),
        // [vault_id, nominator_id, collateral]
        WithdrawCollateral(T::AccountId, T::AccountId, Collateral<T>),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Account has insufficient balance
        InsufficientFunds,
        ArithmeticOverflow,
        ArithmeticUnderflow,
        VaultAlreadyOptedInToNomination,
        VaultNotOptedInToNomination,
        VaultNotFound,
        TryIntoIntError,
        InsufficientCollateral,
        VaultNominationDisabled,
        DepositViolatesMaxNominationRatio,
        HasNominatedCollateral,
        CollateralizationTooLow,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

    /// Flag indicating whether this feature is enabled
    #[pallet::storage]
    #[pallet::getter(fn is_nomination_enabled)]
    pub type NominationEnabled<T: Config> = StorageValue<_, bool, ValueQuery>;

    /// Map of Vaults who have enabled nomination
    #[pallet::storage]
    pub(super) type Vaults<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, bool, ValueQuery>;

    #[pallet::genesis_config]
    pub struct GenesisConfig {
        pub is_nomination_enabled: bool,
    }

    #[cfg(feature = "std")]
    impl Default for GenesisConfig {
        fn default() -> Self {
            Self {
                is_nomination_enabled: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig {
        fn build(&self) {
            {
                NominationEnabled::<T>::put(self.is_nomination_enabled);
            }
        }
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // The pallet's dispatchable functions.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(<T as Config>::WeightInfo::set_nomination_enabled())]
        #[transactional]
        pub fn set_nomination_enabled(origin: OriginFor<T>, enabled: bool) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            <NominationEnabled<T>>::set(enabled);
            Ok(().into())
        }

        /// Allow nomination for this vault
        #[pallet::weight(<T as Config>::WeightInfo::opt_in_to_nomination())]
        #[transactional]
        pub fn opt_in_to_nomination(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            ext::security::ensure_parachain_status_running::<T>()?;
            Self::_opt_in_to_nomination(&ensure_signed(origin)?)?;
            Ok(().into())
        }

        /// Disallow nomination for this vault
        #[pallet::weight(<T as Config>::WeightInfo::opt_out_of_nomination())]
        #[transactional]
        pub fn opt_out_of_nomination(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            ext::security::ensure_parachain_status_running::<T>()?;
            Self::_opt_out_of_nomination(&ensure_signed(origin)?)?;
            Ok(().into())
        }

        #[pallet::weight(<T as Config>::WeightInfo::deposit_collateral())]
        #[transactional]
        pub fn deposit_collateral(
            origin: OriginFor<T>,
            vault_id: T::AccountId,
            amount: Collateral<T>,
        ) -> DispatchResultWithPostInfo {
            let nominator_id = ensure_signed(origin)?;
            ext::security::ensure_parachain_status_running::<T>()?;
            Self::_deposit_collateral(vault_id, nominator_id, amount)?;
            Ok(().into())
        }

        #[pallet::weight(<T as Config>::WeightInfo::withdraw_collateral())]
        #[transactional]
        pub fn withdraw_collateral(
            origin: OriginFor<T>,
            vault_id: T::AccountId,
            amount: Collateral<T>,
        ) -> DispatchResultWithPostInfo {
            let nominator_id = ensure_signed(origin)?;
            ext::security::ensure_parachain_status_running::<T>()?;
            Self::_withdraw_collateral(vault_id, nominator_id, amount)?;
            Ok(().into())
        }
    }
}

// "Internal" functions, callable by code.
#[cfg_attr(test, mockable)]
impl<T: Config> Pallet<T> {
    pub fn _withdraw_collateral(
        vault_id: T::AccountId,
        nominator_id: T::AccountId,
        amount: Collateral<T>,
    ) -> DispatchResult {
        ensure!(Self::is_nomination_enabled(), Error::<T>::VaultNominationDisabled);
        ensure!(Self::is_opted_in(&vault_id)?, Error::<T>::VaultNotOptedInToNomination);

        let currency_id = ext::vault_registry::get_collateral_currency::<T>(&vault_id)?;
        let amount = Amount::new(amount, currency_id);

        // we can only withdraw nominated collateral if the vault is still
        // above the secure threshold for issued + to_be_issued tokens
        ensure!(
            ext::vault_registry::is_allowed_to_withdraw_collateral::<T>(&vault_id, &amount)?,
            Error::<T>::InsufficientCollateral
        );

        // Withdraw all vault rewards first, to prevent the nominator from withdrawing past rewards
        ext::fee::withdraw_all_vault_rewards::<T>(&vault_id)?;
        // Withdraw `amount` of stake from the vault staking pool
        ext::staking::withdraw_stake::<T>(
            T::GetWrappedCurrencyId::get(),
            &vault_id,
            &nominator_id,
            amount.to_signed_fixed_point()?,
        )?;
        amount.unlock(&vault_id)?;
        amount.transfer(&vault_id, &nominator_id)?;
        ext::vault_registry::decrease_total_backing_collateral(&amount)?;

        Self::deposit_event(Event::<T>::WithdrawCollateral(vault_id, nominator_id, amount.amount()));
        Ok(())
    }

    pub fn _deposit_collateral(
        vault_id: T::AccountId,
        nominator_id: T::AccountId,
        amount: Collateral<T>,
    ) -> DispatchResult {
        ensure!(Self::is_nomination_enabled(), Error::<T>::VaultNominationDisabled);
        ensure!(Self::is_opted_in(&vault_id)?, Error::<T>::VaultNotOptedInToNomination);

        let currency_id = ext::vault_registry::get_collateral_currency::<T>(&vault_id)?;

        let amount = Amount::new(amount, currency_id);

        let vault_backing_collateral = ext::vault_registry::get_backing_collateral::<T>(&vault_id)?;
        let total_nominated_collateral = Self::get_total_nominated_collateral(&vault_id)?;
        let new_nominated_collateral = total_nominated_collateral.checked_add(&amount)?;
        let max_nominatable_collateral =
            ext::vault_registry::get_max_nominatable_collateral::<T>(&vault_backing_collateral)?;
        ensure!(
            new_nominated_collateral.le(&max_nominatable_collateral)?,
            Error::<T>::DepositViolatesMaxNominationRatio
        );

        // Withdraw all vault rewards first, to prevent the nominator from withdrawing past rewards
        ext::fee::withdraw_all_vault_rewards::<T>(&vault_id)?;

        // Deposit `amount` of stake into the vault staking pool
        ext::staking::deposit_stake::<T>(
            T::GetWrappedCurrencyId::get(),
            &vault_id,
            &nominator_id,
            amount.to_signed_fixed_point()?,
        )?;
        amount.transfer(&nominator_id, &vault_id)?;
        amount.lock(&vault_id)?;
        ext::vault_registry::try_increase_total_backing_collateral(&amount)?;

        Self::deposit_event(Event::<T>::DepositCollateral(vault_id, nominator_id, amount.amount()));
        Ok(())
    }

    /// Vault is to allow nominated collateral
    ///
    /// # Arguments
    /// * `vault_id` - the id of the vault to allow nomination for
    fn _opt_in_to_nomination(vault_id: &T::AccountId) -> DispatchResult {
        ensure!(Self::is_nomination_enabled(), Error::<T>::VaultNominationDisabled);
        ensure!(
            ext::vault_registry::vault_exists::<T>(&vault_id),
            Error::<T>::VaultNotFound
        );
        ensure!(
            !<Vaults<T>>::contains_key(vault_id),
            Error::<T>::VaultAlreadyOptedInToNomination
        );
        <Vaults<T>>::insert(vault_id, true);
        Self::deposit_event(Event::<T>::NominationOptIn(vault_id.clone()));
        Ok(())
    }

    fn _opt_out_of_nomination(vault_id: &T::AccountId) -> DispatchResult {
        ensure!(Self::is_opted_in(&vault_id)?, Error::<T>::VaultNotOptedInToNomination);
        let total_nominated_collateral = Self::get_total_nominated_collateral(&vault_id)?;
        ensure!(
            ext::vault_registry::is_allowed_to_withdraw_collateral::<T>(&vault_id, &total_nominated_collateral)?,
            Error::<T>::CollateralizationTooLow
        );

        let refunded_collateral = ext::staking::force_refund::<T>(T::GetWrappedCurrencyId::get(), vault_id)?
            .try_into()
            .map_err(|_| Error::<T>::TryIntoIntError)?;

        // Update the system-wide total backing collateral
        let vault_currency_id = ext::vault_registry::get_collateral_currency::<T>(&vault_id)?;
        let refunded_collateral = Amount::<T>::new(refunded_collateral, vault_currency_id);
        ext::vault_registry::decrease_total_backing_collateral(&refunded_collateral)?;

        <Vaults<T>>::remove(vault_id);
        Self::deposit_event(Event::<T>::NominationOptOut(vault_id.clone()));
        Ok(())
    }

    pub fn is_opted_in(vault_id: &T::AccountId) -> Result<bool, DispatchError> {
        Ok(<Vaults<T>>::contains_key(&vault_id))
    }

    pub fn get_total_nominated_collateral(vault_id: &T::AccountId) -> Result<Amount<T>, DispatchError> {
        let vault_backing_collateral = ext::vault_registry::get_backing_collateral::<T>(vault_id)?;
        let vault_actual_collateral = ext::vault_registry::compute_collateral::<T>(vault_id)?;
        vault_backing_collateral.checked_sub(&vault_actual_collateral)
    }

    pub fn get_nominator_collateral(
        vault_id: &T::AccountId,
        nominator_id: &T::AccountId,
    ) -> Result<Amount<T>, DispatchError> {
        let collateral = ext::staking::compute_stake::<T>(T::GetWrappedCurrencyId::get(), vault_id, nominator_id)?;

        let amount = collateral.try_into().map_err(|_| Error::<T>::TryIntoIntError)?;

        let vault_currency_id = ext::vault_registry::get_collateral_currency::<T>(&vault_id)?;
        Ok(Amount::new(amount, vault_currency_id))
    }
}
