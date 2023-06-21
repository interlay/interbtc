//! # Supply Module
//! Distributes block rewards to participants.

#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use codec::{Decode, Encode, EncodeLike};
use frame_support::{
    pallet_prelude::DispatchResult,
    traits::{Currency, Get, ReservableCurrency},
    transactional,
    weights::Weight,
    PalletId,
};
use frame_system::ensure_root;
use primitives::TruncateFixedPointToInt;
use scale_info::TypeInfo;
use sp_arithmetic::ArithmeticError;
use sp_runtime::{
    traits::{AccountIdConversion, Saturating},
    FixedPointNumber,
};

mod default_weights;
pub use default_weights::WeightInfo;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    /// ## Configuration
    /// The pallet's configuration trait.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The supply module id, used for deriving its sovereign account ID.
        #[pallet::constant]
        type SupplyPalletId: Get<PalletId>;

        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Unsigned fixed point type.
        type UnsignedFixedPoint: FixedPointNumber<Inner = BalanceOf<Self>>
            + TruncateFixedPointToInt
            + Encode
            + EncodeLike
            + Decode
            + MaybeSerializeDeserialize
            + TypeInfo
            + MaxEncodedLen;

        /// The native currency for emission.
        type Currency: ReservableCurrency<Self::AccountId>;

        /// The period between inflation updates.
        #[pallet::constant]
        type InflationPeriod: Get<Self::BlockNumber>;

        /// Handler for when the total supply has inflated.
        type OnInflation: OnInflation<Self::AccountId, Currency = Self::Currency>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    // The pallet's events
    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        Inflation { total_inflation: BalanceOf<T> },
    }

    #[pallet::error]
    pub enum Error<T> {}

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
        fn on_initialize(n: T::BlockNumber) -> Weight {
            if let Err(e) = Self::begin_block(n) {
                sp_runtime::print(e);
            }
            T::WeightInfo::on_initialize()
        }
    }

    #[pallet::storage]
    #[pallet::whitelist_storage]
    #[pallet::getter(fn start_height)]
    pub type StartHeight<T: Config> = StorageValue<_, T::BlockNumber, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn last_emission)]
    pub type LastEmission<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn inflation)]
    pub type Inflation<T: Config> = StorageValue<_, T::UnsignedFixedPoint, ValueQuery>;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub initial_supply: BalanceOf<T>,
        pub start_height: T::BlockNumber,
        pub inflation: T::UnsignedFixedPoint,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                initial_supply: Default::default(),
                start_height: Default::default(),
                inflation: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            T::Currency::deposit_creating(&T::SupplyPalletId::get().into_account_truncating(), self.initial_supply);
            StartHeight::<T>::put(self.start_height);
            Inflation::<T>::put(self.inflation);
        }
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // The pallet's dispatchable functions.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::set_start_height_and_inflation())]
        #[transactional]
        pub fn set_start_height_and_inflation(
            origin: OriginFor<T>,
            start_height: T::BlockNumber,
            inflation: T::UnsignedFixedPoint,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            StartHeight::<T>::put(start_height);
            Inflation::<T>::put(inflation);
            Ok(().into())
        }
    }
}

// "Internal" functions, callable by code.
impl<T: Config> Pallet<T> {
    pub fn account_id() -> T::AccountId {
        T::SupplyPalletId::get().into_account_truncating()
    }

    pub(crate) fn begin_block(height: T::BlockNumber) -> DispatchResult {
        // ignore if uninitialized or not start height
        if let Some(start_height) = <StartHeight<T>>::get().filter(|&start_height| height == start_height) {
            let end_height = start_height + T::InflationPeriod::get();
            <StartHeight<T>>::put(end_height);

            let total_supply = T::Currency::total_issuance();
            let total_supply_as_fixed =
                T::UnsignedFixedPoint::checked_from_integer(total_supply).ok_or(ArithmeticError::Overflow)?;
            let total_inflation = total_supply_as_fixed
                .saturating_mul(<Inflation<T>>::get())
                .truncate_to_inner()
                .unwrap_or_default();

            <LastEmission<T>>::put(total_inflation);
            let supply_account_id = Self::account_id();
            T::Currency::deposit_creating(&supply_account_id, total_inflation);
            T::OnInflation::on_inflation(&supply_account_id, total_inflation);
            Self::deposit_event(Event::<T>::Inflation { total_inflation });
        }

        Ok(())
    }
}

pub trait OnInflation<AccountId> {
    type Currency: ReservableCurrency<AccountId>;
    fn on_inflation(from: &AccountId, amount: <Self::Currency as Currency<AccountId>>::Balance);
}
