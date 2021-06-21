//! # Nomination Module

#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

mod ext;
mod types;

mod default_weights;

use ext::vault_registry::{DefaultVault, SlashingError, TryDepositCollateral, TryWithdrawCollateral};
use frame_support::{
    dispatch::{DispatchError, DispatchResult},
    ensure, transactional,
    weights::Weight,
};
use frame_system::{ensure_root, ensure_signed};
use reward::RewardPool;
use sp_runtime::{
    traits::{CheckedAdd, CheckedDiv, CheckedSub, One, Zero},
    FixedPointNumber,
};
use sp_std::convert::TryInto;
pub use types::Nominator;
use types::{
    BalanceOf, Collateral, DefaultNominator, RichNominator, SignedFixedPoint, SignedInner, UnsignedFixedPoint,
};

pub trait WeightInfo {
    fn set_nomination_enabled() -> Weight;
    fn opt_in_to_nomination() -> Weight;
    fn opt_out_of_nomination() -> Weight;
    fn deposit_collateral() -> Weight;
    fn withdraw_collateral() -> Weight;
}

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    /// ## Configuration
    /// The pallet's configuration trait.
    #[pallet::config]
    pub trait Config:
        frame_system::Config
        + security::Config
        + vault_registry::Config
        + fee::Config<UnsignedFixedPoint = UnsignedFixedPoint<Self>, UnsignedInner = BalanceOf<Self>>
    {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Weight information for the extrinsics in this module.
        type WeightInfo: WeightInfo;

        /// Vault reward pool for the collateral currency.
        type CollateralVaultRewards: reward::Rewards<Self::AccountId, SignedFixedPoint = SignedFixedPoint<Self>>;

        /// Vault reward pool for the wrapped currency.
        type WrappedVaultRewards: reward::Rewards<Self::AccountId, SignedFixedPoint = SignedFixedPoint<Self>>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    #[pallet::metadata(T::AccountId = "AccountId", Collateral<T> = "Collateral")]
    pub enum Event<T: Config> {
        // [vault_id]
        NominationOptIn(T::AccountId),
        // [vault_id]
        NominationOptOut(T::AccountId),
        // [nominator_id, vault_id, collateral]
        DepositCollateral(T::AccountId, T::AccountId, Collateral<T>),
        // [nominator_id, vault_id, collateral]
        WithdrawCollateral(T::AccountId, T::AccountId, Collateral<T>),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Account has insufficient balance
        InsufficientFunds,
        ArithmeticOverflow,
        ArithmeticUnderflow,
        NominatorNotFound,
        VaultAlreadyOptedInToNomination,
        VaultNotOptedInToNomination,
        VaultNotFound,
        TryIntoIntError,
        InsufficientCollateral,
        VaultNominationDisabled,
        DepositViolatesMaxNominationRatio,
        HasNominatedCollateral,
    }

    impl<T: Config> From<SlashingError> for Error<T> {
        fn from(err: SlashingError) -> Self {
            match err {
                SlashingError::ArithmeticOverflow => Error::<T>::ArithmeticOverflow,
                SlashingError::ArithmeticUnderflow => Error::<T>::ArithmeticUnderflow,
                SlashingError::TryIntoIntError => Error::<T>::TryIntoIntError,
                SlashingError::InsufficientFunds => Error::<T>::InsufficientCollateral,
            }
        }
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

    /// Map of Nominators
    #[pallet::storage]
    pub(super) type Nominators<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        (T::AccountId, T::AccountId),
        Nominator<T::AccountId, Collateral<T>, SignedFixedPoint<T>>,
        ValueQuery,
    >;

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
            let sender = ensure_signed(origin)?;
            ext::security::ensure_parachain_status_running::<T>()?;
            Self::_deposit_collateral(sender, vault_id, amount)?;
            Ok(().into())
        }

        #[pallet::weight(<T as Config>::WeightInfo::withdraw_collateral())]
        #[transactional]
        pub fn withdraw_collateral(
            origin: OriginFor<T>,
            vault_id: T::AccountId,
            amount: Collateral<T>,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            ext::security::ensure_parachain_status_running::<T>()?;
            Self::_withdraw_collateral(sender, vault_id, amount)?;
            Ok(().into())
        }
    }
}

// "Internal" functions, callable by code.
impl<T: Config> Pallet<T> {
    pub fn _withdraw_collateral(
        nominator_id: T::AccountId,
        vault_id: T::AccountId,
        amount: Collateral<T>,
    ) -> DispatchResult {
        ensure!(Self::is_nomination_enabled(), Error::<T>::VaultNominationDisabled);
        ensure!(
            Self::is_nominatable(&vault_id)?,
            Error::<T>::VaultNotOptedInToNomination
        );

        // we can only withdraw nominated collateral if the vault is still
        // above the secure threshold for issued + to_be_issued tokens
        ensure!(
            ext::vault_registry::is_allowed_to_withdraw_collateral::<T>(&vault_id, amount)?,
            Error::<T>::InsufficientCollateral
        );

        // Withdraw all vault rewards first, to prevent the nominator from withdrawing rewards from the past.
        ext::fee::withdraw_all_vault_rewards::<T>(&vault_id)?;
        // Withdraw `amount` of stake from both vault reward pools
        Self::withdraw_pool_stake::<<T as pallet::Config>::CollateralVaultRewards>(&nominator_id, &vault_id, amount)?;
        Self::withdraw_pool_stake::<<T as pallet::Config>::WrappedVaultRewards>(&nominator_id, &vault_id, amount)?;

        let mut nominator: RichNominator<T> = Self::get_nominator(&nominator_id, &vault_id)?.into();
        nominator.try_withdraw_collateral(amount)?;
        ext::collateral::unlock_and_transfer::<T>(&vault_id, &nominator_id, amount)?;

        Self::deposit_event(Event::<T>::WithdrawCollateral(nominator_id, vault_id, amount));
        Ok(())
    }

    pub fn _deposit_collateral(
        nominator_id: T::AccountId,
        vault_id: T::AccountId,
        amount: Collateral<T>,
    ) -> DispatchResult {
        ensure!(Self::is_nomination_enabled(), Error::<T>::VaultNominationDisabled);
        ensure!(
            Self::is_nominatable(&vault_id)?,
            Error::<T>::VaultNotOptedInToNomination
        );

        let vault_backing_collateral = ext::vault_registry::get_backing_collateral::<T>(&vault_id)?;
        let total_nominated_collateral = Self::get_total_nominated_collateral(&vault_id)?;
        let new_nominated_collateral = total_nominated_collateral
            .checked_add(&amount)
            .ok_or(Error::<T>::ArithmeticOverflow)?;
        ensure!(
            new_nominated_collateral <= Self::get_max_nominatable_collateral(vault_backing_collateral)?,
            Error::<T>::DepositViolatesMaxNominationRatio
        );

        // Withdraw all vault rewards first, to prevent the nominator from withdrawing rewards from the past.
        ext::fee::withdraw_all_vault_rewards::<T>(&vault_id)?;
        // Deposit `amount` of stake to both vault reward pools
        Self::deposit_pool_stake::<<T as pallet::Config>::CollateralVaultRewards>(&nominator_id, &vault_id, amount)?;
        Self::deposit_pool_stake::<<T as pallet::Config>::WrappedVaultRewards>(&nominator_id, &vault_id, amount)?;

        let mut nominator: RichNominator<T> = Self::register_or_get_nominator(&nominator_id, &vault_id)?.into();
        nominator
            .try_deposit_collateral(amount)
            .map_err(|e| Error::<T>::from(e))?;
        ext::collateral::transfer_and_lock::<T>(&nominator_id, &vault_id, amount)?;

        Self::deposit_event(Event::<T>::DepositCollateral(nominator_id, vault_id, amount));
        Ok(())
    }

    /// Vault is to allow nominated collateral
    ///
    /// # Arguments
    /// * `vault_id` - the id of the vault to allow nomination for
    pub fn _opt_in_to_nomination(vault_id: &T::AccountId) -> DispatchResult {
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

    pub fn _opt_out_of_nomination(vault_id: &T::AccountId) -> DispatchResult {
        // TODO: force refund
        ensure!(
            Self::get_total_nominated_collateral(vault_id)?.is_zero(),
            Error::<T>::HasNominatedCollateral
        );
        <Vaults<T>>::remove(vault_id);
        Self::deposit_event(Event::<T>::NominationOptOut(vault_id.clone()));
        Ok(())
    }

    pub fn is_nominatable(vault_id: &T::AccountId) -> Result<bool, DispatchError> {
        Ok(<Vaults<T>>::contains_key(&vault_id))
    }

    pub fn is_nominator(nominator_id: &T::AccountId, vault_id: &T::AccountId) -> Result<bool, DispatchError> {
        Ok(<Nominators<T>>::contains_key((&nominator_id, &vault_id)))
    }

    pub fn get_total_nominated_collateral(vault_id: &T::AccountId) -> Result<Collateral<T>, DispatchError> {
        let vault: DefaultVault<T> = ext::vault_registry::get_vault_from_id::<T>(vault_id)?;
        let vault_actual_collateral = ext::vault_registry::compute_collateral::<T>(vault_id)?;
        Ok(vault
            .backing_collateral
            .checked_sub(&vault_actual_collateral)
            .ok_or(Error::<T>::ArithmeticUnderflow)?)
    }

    pub fn get_max_nomination_ratio() -> Result<UnsignedFixedPoint<T>, DispatchError> {
        let secure_collateral_threshold = ext::vault_registry::get_secure_collateral_threshold::<T>();
        let premium_redeem_threshold = ext::vault_registry::get_premium_redeem_threshold::<T>();
        Ok(secure_collateral_threshold
            .checked_div(&premium_redeem_threshold)
            .ok_or(Error::<T>::ArithmeticUnderflow)?
            .checked_sub(&UnsignedFixedPoint::<T>::one())
            .ok_or(Error::<T>::ArithmeticUnderflow)?)
    }

    pub fn get_nominator(
        nominator_id: &T::AccountId,
        vault_id: &T::AccountId,
    ) -> Result<DefaultNominator<T>, DispatchError> {
        ensure!(
            Self::is_nominator(&nominator_id, &vault_id)?,
            Error::<T>::NominatorNotFound
        );
        Ok(<Nominators<T>>::get((nominator_id, vault_id)))
    }

    pub fn get_rich_nominator(
        nominator_id: &T::AccountId,
        vault_id: &T::AccountId,
    ) -> Result<RichNominator<T>, DispatchError> {
        Ok(Self::get_nominator(&nominator_id, &vault_id)?.into())
    }

    pub fn get_nominator_collateral(
        nominator_id: &T::AccountId,
        vault_id: &T::AccountId,
    ) -> Result<Collateral<T>, DispatchError> {
        let nominator = Self::get_rich_nominator(nominator_id, vault_id)?;
        Ok(nominator.compute_collateral()?)
    }

    pub fn register_or_get_nominator(
        nominator_id: &T::AccountId,
        vault_id: &T::AccountId,
    ) -> Result<DefaultNominator<T>, DispatchError> {
        if !Self::is_nominator(&nominator_id, &vault_id)? {
            let nominator = Nominator::new(nominator_id.clone(), vault_id.clone());
            <Nominators<T>>::insert((nominator_id, vault_id), nominator.clone());
            Ok(nominator)
        } else {
            Ok(<Nominators<T>>::get((&nominator_id, &vault_id)))
        }
    }

    pub fn get_max_nominatable_collateral(vault_collateral: Collateral<T>) -> Result<Collateral<T>, DispatchError> {
        ext::fee::collateral_for::<T>(vault_collateral, Self::get_max_nomination_ratio()?)
    }

    fn collateral_to_fixed(x: Collateral<T>) -> Result<SignedFixedPoint<T>, DispatchError> {
        let signed_inner = TryInto::<SignedInner<T>>::try_into(x).map_err(|_| Error::<T>::TryIntoIntError)?;
        let signed_fixed_point =
            SignedFixedPoint::<T>::checked_from_integer(signed_inner).ok_or(Error::<T>::TryIntoIntError)?;
        Ok(signed_fixed_point)
    }

    fn withdraw_pool_stake<R: reward::Rewards<T::AccountId, SignedFixedPoint = SignedFixedPoint<T>>>(
        account_id: &T::AccountId,
        vault_id: &T::AccountId,
        amount: Collateral<T>,
    ) -> Result<(), DispatchError> {
        let amount_fixed = Self::collateral_to_fixed(amount)?;
        if amount_fixed > SignedFixedPoint::<T>::zero() {
            R::withdraw_stake(RewardPool::Local(vault_id.clone()), account_id, amount_fixed)?;
        }
        Ok(())
    }

    fn deposit_pool_stake<R: reward::Rewards<T::AccountId, SignedFixedPoint = SignedFixedPoint<T>>>(
        account_id: &T::AccountId,
        vault_id: &T::AccountId,
        amount: Collateral<T>,
    ) -> Result<(), DispatchError> {
        let amount_fixed = Self::collateral_to_fixed(amount)?;
        R::deposit_stake(RewardPool::Local(vault_id.clone()), account_id, amount_fixed)?;
        Ok(())
    }
}
