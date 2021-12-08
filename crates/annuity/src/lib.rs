//! # Annuity Module
//! Distributes block rewards to participants.

#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use frame_support::{
    dispatch::{DispatchError, DispatchResult},
    traits::{Currency, ExistenceRequirement, Get, ReservableCurrency},
    transactional,
    weights::Weight,
    PalletId,
};
use sp_runtime::traits::{CheckedDiv, Convert};

pub use pallet::*;

type BalanceOf<T, I> = <<T as Config<I>>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::{ensure_signed, pallet_prelude::*};

    /// ## Configuration
    /// The pallet's configuration trait.
    #[pallet::config]
    pub trait Config<I: 'static = ()>: frame_system::Config {
        /// The annuity module id, used for deriving its sovereign account ID.
        #[pallet::constant]
        type AnnuityPalletId: Get<PalletId>;

        /// The overarching event type.
        type Event: From<Event<Self, I>> + IsType<<Self as frame_system::Config>::Event>;

        /// The native currency for emission.
        type Currency: ReservableCurrency<Self::AccountId>;

        /// The block reward provider.
        type BlockRewardProvider: BlockRewardProvider<Self::AccountId, Currency = Self::Currency>;

        /// Convert the block number into a balance.
        type BlockNumberToBalance: Convert<Self::BlockNumber, BalanceOf<Self, I>>;

        /// The emission period for block rewards.
        #[pallet::constant]
        type EmissionPeriod: Get<Self::BlockNumber>;
    }

    // The pallet's events
    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config<I>, I: 'static = ()> {
        BlockReward(BalanceOf<T, I>),
    }

    #[pallet::error]
    pub enum Error<T, I = ()> {}

    #[pallet::hooks]
    impl<T: Config<I>, I: 'static> Hooks<T::BlockNumber> for Pallet<T, I> {
        fn on_initialize(n: T::BlockNumber) -> Weight {
            if let Err(e) = Self::begin_block(n) {
                sp_runtime::print(e);
            }
            0
        }
    }

    #[pallet::storage]
    #[pallet::getter(fn annuity_pallet_id)]
    pub type AnnuityPalletId<T: Config<I>, I: 'static = ()> = StorageValue<_, T::AccountId, ValueQuery>;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config<I>, I: 'static = ()>(PhantomData<(T, I)>);

    #[cfg(feature = "std")]
    impl<T: Config<I>, I: 'static> Default for GenesisConfig<T, I> {
        fn default() -> Self {
            Self(PhantomData {})
        }
    }

    #[pallet::genesis_build]
    impl<T: Config<I>, I: 'static> GenesisBuild<T, I> for GenesisConfig<T, I> {
        fn build(&self) {
            let annuity_pallet_id =
                sp_runtime::traits::AccountIdConversion::into_account(&<T as Config<I>>::AnnuityPalletId::get());
            AnnuityPalletId::<T, I>::put::<T::AccountId>(annuity_pallet_id);
        }
    }

    #[pallet::pallet]
    pub struct Pallet<T, I = ()>(_);

    // The pallet's dispatchable functions.
    #[pallet::call]
    impl<T: Config<I>, I: 'static> Pallet<T, I> {
        #[pallet::weight(0)]
        #[transactional]
        pub fn withdraw_reward(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            let dest = ensure_signed(origin)?;
            let value = T::BlockRewardProvider::withdraw_reward(&dest)?;
            let _ = T::Currency::transfer(
                &Self::annuity_pallet_id(),
                &dest,
                value,
                ExistenceRequirement::KeepAlive,
            );
            Ok(().into())
        }
    }
}

// "Internal" functions, callable by code.
impl<T: Config<I>, I: 'static> Pallet<T, I> {
    pub(crate) fn begin_block(_height: T::BlockNumber) -> DispatchResult {
        let annuity_pallet_id = Self::annuity_pallet_id();
        let reward_per_block = Self::reward_per_block();
        Self::deposit_event(Event::<T, I>::BlockReward(reward_per_block));
        T::BlockRewardProvider::distribute_block_reward(&annuity_pallet_id, reward_per_block)
    }

    fn reward_per_block() -> BalanceOf<T, I> {
        let emission_period = T::BlockNumberToBalance::convert(T::EmissionPeriod::get());
        let total_balance = T::Currency::total_balance(&Self::annuity_pallet_id());
        let reward_per_block = total_balance.checked_div(&emission_period).unwrap_or_default();
        reward_per_block
    }
}

pub trait BlockRewardProvider<AccountId> {
    type Currency: ReservableCurrency<AccountId>;
    fn distribute_block_reward(
        from: &AccountId,
        amount: <Self::Currency as Currency<AccountId>>::Balance,
    ) -> DispatchResult;
    fn withdraw_reward(who: &AccountId) -> Result<<Self::Currency as Currency<AccountId>>::Balance, DispatchError>;
}
