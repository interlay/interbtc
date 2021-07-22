//! # Exchange Rate Oracle Module
//! Based on the [specification](https://interlay.gitlab.io/polkabtc-spec/spec/oracle.html).

#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

mod ext;
mod types;

#[cfg(any(feature = "runtime-benchmarks", test))]
mod benchmarking;

mod default_weights;
pub use default_weights::WeightInfo;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod mock;

#[cfg(test)]
extern crate mocktopus;

#[doc(inline)]
pub use crate::types::BtcTxFeesPerByte;

use crate::types::{Collateral, UnsignedFixedPoint, Version, Wrapped};

#[cfg(test)]
use mocktopus::macros::mockable;

use codec::{Decode, Encode, EncodeLike, FullCodec};
use frame_support::{
    dispatch::{DispatchError, DispatchResult},
    ensure, transactional,
    weights::Weight,
};
use frame_system::{ensure_root, ensure_signed};
use security::{ErrorCode, StatusCode};
use sp_runtime::{
    traits::{UniqueSaturatedInto, *},
    FixedPointNumber, FixedPointOperand,
};
use sp_std::{convert::TryInto, fmt::Debug, vec::Vec};

pub use pallet::*;

#[derive(Encode, Decode, Eq, PartialEq, Clone, Copy, Ord, PartialOrd)]
pub struct TimestampedValue<Value, Moment> {
    pub value: Value,
    pub timestamp: Moment,
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    /// ## Configuration
    /// The pallet's configuration trait.
    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_timestamp::Config + security::Config {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The primitive balance type.
        type Balance: AtLeast32BitUnsigned
            + FixedPointOperand
            + MaybeSerializeDeserialize
            + FullCodec
            + Copy
            + Default
            + Debug;

        /// The unsigned fixed point type.
        type UnsignedFixedPoint: FixedPointNumber<Inner = <Self as Config>::Balance> + Encode + EncodeLike + Decode;

        /// Weight information for the extrinsics in this module.
        type WeightInfo: WeightInfo;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    #[pallet::metadata(T::AccountId = "AccountId", UnsignedFixedPoint<T> = "UnsignedFixedPoint")]
    pub enum Event<T: Config> {
        /// Event emitted when exchange rate is set
        SetExchangeRate(T::AccountId, UnsignedFixedPoint<T>),
        /// Event emitted when the btc tx fees are set
        SetBtcTxFeesPerByte(T::AccountId, u32, u32, u32),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Not authorized to set exchange rate
        InvalidOracleSource,
        /// Exchange rate not specified or has expired
        MissingExchangeRate,
        /// Unable to convert value
        TryIntoIntError,
        /// Mathematical operation caused an overflow
        ArithmeticOverflow,
        /// Mathematical operation caused an underflow
        ArithmeticUnderflow,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
        fn on_initialize(n: T::BlockNumber) -> Weight {
            Self::begin_block(n);
            // TODO: calculate weight
            0
        }
    }

    /// Current exchange rate (i.e. Planck per Satoshi)
    #[pallet::storage]
    pub type ExchangeRate<T: Config> = StorageValue<_, UnsignedFixedPoint<T>>;

    #[pallet::storage]
    pub type RawValues<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, TimestampedValue<UnsignedFixedPoint<T>, T::Moment>>;

    #[pallet::storage]
    pub type RawValuesUpdated<T: Config> = StorageValue<_, bool, ValueQuery>;

    /// Last exchange rate time
    #[pallet::storage]
    pub type ValidUntil<T: Config> = StorageValue<_, T::Moment>;

    /// The estimated inclusion time for a Bitcoin transaction in Satoshis per byte
    #[pallet::storage]
    #[pallet::getter(fn satoshi_per_bytes)]
    pub type SatoshiPerBytes<T: Config> = StorageValue<_, BtcTxFeesPerByte, ValueQuery>;

    /// Maximum delay (milliseconds) for the exchange rate to be used
    #[pallet::storage]
    #[pallet::getter(fn max_delay)]
    pub type MaxDelay<T: Config> = StorageValue<_, T::Moment, ValueQuery>;

    // Oracles allowed to set the exchange rate, maps to the name
    #[pallet::storage]
    #[pallet::getter(fn authorized_oracles)]
    pub type AuthorizedOracles<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, Vec<u8>, ValueQuery>;

    #[pallet::type_value]
    pub(super) fn DefaultForStorageVersion() -> Version {
        Version::V0
    }

    /// Build storage at V1 (requires default 0).
    #[pallet::storage]
    #[pallet::getter(fn storage_version)]
    pub(super) type StorageVersion<T: Config> = StorageValue<_, Version, ValueQuery, DefaultForStorageVersion>;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub max_delay: u32,
        pub authorized_oracles: Vec<(T::AccountId, Vec<u8>)>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                max_delay: Default::default(),
                authorized_oracles: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            // T::Moment doesn't implement serialize so we use
            // From<u32> as bound by AtLeast32Bit
            MaxDelay::<T>::put(T::Moment::from(self.max_delay));

            for (ref who, name) in self.authorized_oracles.iter() {
                AuthorizedOracles::<T>::insert(who, name);
            }
        }
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // The pallet's dispatchable functions.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Sets the exchange rate.
        ///
        /// # Arguments
        ///
        /// * `exchange_rate` - i.e. planck_per_satoshi = dot_per_btc * (10**10 / 10**8)
        /// This is the same unit that is stored in the ExchangeRate storage item.
        #[pallet::weight(<T as Config>::WeightInfo::set_exchange_rate())]
        #[transactional]
        pub fn set_exchange_rate(origin: OriginFor<T>, exchange_rate: UnsignedFixedPoint<T>) -> DispatchResult {
            // Check that Parachain is not in SHUTDOWN
            ext::security::ensure_parachain_status_not_shutdown::<T>()?;

            let signer = ensure_signed(origin)?;

            // fail if the signer is not an authorized oracle
            ensure!(Self::is_authorized(&signer), Error::<T>::InvalidOracleSource);

            let timestamped = TimestampedValue {
                timestamp: Self::get_current_time(),
                value: exchange_rate.clone(),
            };
            RawValues::<T>::insert(&signer, timestamped);
            RawValuesUpdated::<T>::set(true);

            Self::deposit_event(Event::<T>::SetExchangeRate(signer, exchange_rate));

            Ok(())
        }

        /// Sets the estimated transaction inclusion fees based on the estimated inclusion time
        ///
        /// # Arguments
        /// * `fast` - The estimated Satoshis per bytes to get included in the next block (~10 min)
        /// * `half` - The estimated Satoshis per bytes to get included in the next 3 blocks (~half hour)
        /// * `hour` - The estimated Satoshis per bytes to get included in the next 6 blocks (~hour)
        #[pallet::weight(<T as Config>::WeightInfo::set_btc_tx_fees_per_byte())]
        #[transactional]
        pub fn set_btc_tx_fees_per_byte(origin: OriginFor<T>, fast: u32, half: u32, hour: u32) -> DispatchResult {
            // Check that Parachain is not in SHUTDOWN
            ext::security::ensure_parachain_status_not_shutdown::<T>()?;

            let signer = ensure_signed(origin)?;

            // fail if the signer is not the authorized oracle
            ensure!(Self::is_authorized(&signer), Error::<T>::InvalidOracleSource);

            // write the new values to storage
            let fees = BtcTxFeesPerByte { fast, half, hour };
            <SatoshiPerBytes<T>>::put(fees);

            Self::deposit_event(Event::<T>::SetBtcTxFeesPerByte(signer, fast, half, hour));

            Ok(())
        }

        /// Adds an authorized oracle account (only executable by the Root account)
        ///
        /// # Arguments
        /// * `account_id` - the account Id of the oracle
        /// * `name` - a descriptive name for the oracle
        #[pallet::weight(<T as Config>::WeightInfo::insert_authorized_oracle())]
        #[transactional]
        pub fn insert_authorized_oracle(
            origin: OriginFor<T>,
            account_id: T::AccountId,
            name: Vec<u8>,
        ) -> DispatchResult {
            ensure_root(origin)?;
            Self::insert_oracle(account_id, name);
            Ok(())
        }

        /// Removes an authorized oracle account (only executable by the Root account)
        ///
        /// # Arguments
        /// * `account_id` - the account Id of the oracle
        #[pallet::weight(<T as Config>::WeightInfo::remove_authorized_oracle())]
        #[transactional]
        pub fn remove_authorized_oracle(origin: OriginFor<T>, account_id: T::AccountId) -> DispatchResult {
            ensure_root(origin)?;
            <AuthorizedOracles<T>>::remove(account_id);
            Ok(())
        }
    }
}

#[cfg_attr(test, mockable)]
impl<T: Config> Pallet<T> {
    fn begin_block(_height: T::BlockNumber) {
        if Self::is_invalidated() {
            RawValuesUpdated::<T>::set(false);

            let mut raw_values: Vec<_> = RawValues::<T>::iter().map(|(_key, value)| value).collect();
            let min_timestamp = Self::get_current_time().saturating_sub(Self::get_max_delay());
            raw_values.retain(|value| value.timestamp >= min_timestamp);
            if raw_values.len() == 0 {
                Self::report_oracle_offline();
            } else {
                let valid_until = raw_values
                    .iter()
                    .map(|x| x.timestamp)
                    .min()
                    .map(|timestamp| timestamp + Self::get_max_delay())
                    .unwrap(); // Won't panic as `values` ensured not empty.

                let mid_index = raw_values.len() / 2;
                let (_, value, _) = raw_values.select_nth_unstable_by(mid_index as usize, |a, b| a.value.cmp(&b.value));

                if ExchangeRate::<T>::get().is_none() {
                    Self::recover_from_oracle_offline();
                }

                ExchangeRate::<T>::set(Some(value.value));
                ValidUntil::<T>::set(Some(valid_until));
            }
        }
    }

    /// Public getters

    /// Get the exchange rate in planck per satoshi
    pub fn get_exchange_rate() -> Result<UnsignedFixedPoint<T>, DispatchError> {
        ExchangeRate::<T>::get().ok_or(Error::<T>::MissingExchangeRate.into())
    }

    pub fn wrapped_to_collateral(amount: Wrapped<T>) -> Result<Collateral<T>, DispatchError> {
        let rate = Self::get_exchange_rate()?;
        let converted = rate.checked_mul_int(amount).ok_or(Error::<T>::ArithmeticOverflow)?;
        let result = converted.try_into().map_err(|_e| Error::<T>::TryIntoIntError)?;
        Ok(result)
    }

    pub fn collateral_to_wrapped(amount: Collateral<T>) -> Result<Wrapped<T>, DispatchError> {
        let rate = Self::get_exchange_rate()?;
        if amount.is_zero() {
            return Ok(Zero::zero());
        }

        // The code below performs `amount/rate`, plus necessary type conversions
        Ok(T::UnsignedFixedPoint::checked_from_integer(amount.into())
            .ok_or(Error::<T>::TryIntoIntError)?
            .checked_div(&rate)
            .ok_or(Error::<T>::ArithmeticUnderflow)?
            .into_inner()
            .checked_div(&UnsignedFixedPoint::<T>::accuracy())
            .ok_or(Error::<T>::ArithmeticUnderflow)?
            .unique_saturated_into())
    }

    /// Private getters and setters

    fn is_invalidated() -> bool {
        if RawValuesUpdated::<T>::get() {
            true
        } else {
            let valid_until = ValidUntil::<T>::get();
            let current_time = Self::get_current_time();
            matches!(valid_until, Some(t) if current_time > t)
        }
    }

    fn get_max_delay() -> T::Moment {
        <MaxDelay<T>>::get()
    }

    /// Set the current exchange rate. ONLY FOR TESTING.
    ///
    /// # Arguments
    ///
    /// * `exchange_rate` - i.e. planck per satoshi
    pub fn _set_exchange_rate(exchange_rate: UnsignedFixedPoint<T>) -> DispatchResult {
        ExchangeRate::<T>::set(Some(exchange_rate));
        Ok(())
    }

    fn report_oracle_offline() {
        ext::security::set_status::<T>(StatusCode::Error);
        ext::security::insert_error::<T>(ErrorCode::OracleOffline);
        ExchangeRate::<T>::kill();
        ValidUntil::<T>::kill();
    }

    fn recover_from_oracle_offline() {
        ext::security::recover_from_oracle_offline::<T>()
    }

    /// Returns the current timestamp
    fn get_current_time() -> T::Moment {
        <pallet_timestamp::Pallet<T>>::get()
    }

    /// Add a new authorized oracle
    fn insert_oracle(oracle: T::AccountId, name: Vec<u8>) {
        <AuthorizedOracles<T>>::insert(oracle, name)
    }

    /// True if oracle is authorized
    fn is_authorized(oracle: &T::AccountId) -> bool {
        <AuthorizedOracles<T>>::contains_key(oracle)
    }
}
