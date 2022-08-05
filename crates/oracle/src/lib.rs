//! # Oracle Pallet
//! Based on the [specification](https://spec.interlay.io/spec/oracle.html).

#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

mod ext;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

mod default_weights;
pub use default_weights::WeightInfo;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod mock;

pub mod types;

#[cfg(test)]
extern crate mocktopus;

#[cfg(test)]
use mocktopus::macros::mockable;

use crate::types::{BalanceOf, UnsignedFixedPoint, Version};
use codec::{Decode, Encode, MaxEncodedLen};
use currency::Amount;
use frame_support::{
    dispatch::{DispatchError, DispatchResult},
    ensure,
    traits::Get,
    transactional,
    weights::Weight,
};
use frame_system::{ensure_root, ensure_signed};
use scale_info::TypeInfo;
use security::{ErrorCode, StatusCode};
use sp_runtime::{
    traits::{UniqueSaturatedInto, *},
    ArithmeticError, FixedPointNumber,
};
use sp_std::{convert::TryInto, vec::Vec};

pub use pallet::*;
pub use primitives::{oracle::Key as OracleKey, CurrencyId, TruncateFixedPointToInt};

#[derive(Encode, Decode, Eq, PartialEq, Clone, Copy, Ord, PartialOrd, TypeInfo, MaxEncodedLen)]
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
    pub trait Config:
        frame_system::Config + pallet_timestamp::Config + security::Config + currency::Config<CurrencyId = CurrencyId>
    {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Weight information for the extrinsics in this module.
        type WeightInfo: WeightInfo;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Event emitted when exchange rate is set
        FeedValues {
            oracle_id: T::AccountId,
            values: Vec<(OracleKey, T::UnsignedFixedPoint)>,
        },
        AggregateUpdated {
            values: Vec<(OracleKey, Option<T::UnsignedFixedPoint>)>,
        },
        OracleAdded {
            oracle_id: T::AccountId,
            name: Vec<u8>,
        },
        OracleRemoved {
            oracle_id: T::AccountId,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Not authorized to set exchange rate
        InvalidOracleSource,
        /// Exchange rate not specified or has expired
        MissingExchangeRate,
        /// Unable to convert value
        TryIntoIntError,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
        fn on_initialize(n: T::BlockNumber) -> Weight {
            Self::begin_block(n);
            <T as Config>::WeightInfo::on_initialize()
        }
    }

    /// Current medianized value for the given key
    #[pallet::storage]
    pub type Aggregate<T: Config> = StorageMap<_, Blake2_128Concat, OracleKey, UnsignedFixedPoint<T>>;

    #[pallet::storage]
    pub type RawValues<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        OracleKey,
        Blake2_128Concat,
        T::AccountId,
        TimestampedValue<UnsignedFixedPoint<T>, T::Moment>,
    >;

    #[pallet::storage]
    /// if a key is present, it means the values have been updated
    pub type RawValuesUpdated<T: Config> = StorageMap<_, Blake2_128Concat, OracleKey, bool>;

    /// Time until which the aggregate is valid
    #[pallet::storage]
    pub type ValidUntil<T: Config> = StorageMap<_, Blake2_128Concat, OracleKey, T::Moment>;

    /// Maximum delay (milliseconds) for a reported value to be used
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
    #[pallet::without_storage_info] // MaxEncodedLen not implemented for vecs
    pub struct Pallet<T>(_);

    // The pallet's dispatchable functions.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Feeds data from the oracles, e.g., the exchange rates. This function
        /// is intended to be API-compatible with orml-oracle.
        ///
        /// # Arguments
        ///
        /// * `values` - a vector of (key, value) pairs to submit
        #[pallet::weight(<T as Config>::WeightInfo::feed_values(values.len() as u32))]
        pub fn feed_values(
            origin: OriginFor<T>,
            values: Vec<(OracleKey, T::UnsignedFixedPoint)>,
        ) -> DispatchResultWithPostInfo {
            let signer = ensure_signed(origin)?;

            // fail if the signer is not an authorized oracle
            ensure!(Self::is_authorized(&signer), Error::<T>::InvalidOracleSource);

            Self::_feed_values(signer, values);
            Ok(Pays::No.into())
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
            Self::insert_oracle(account_id.clone(), name.clone());
            Self::deposit_event(Event::OracleAdded {
                oracle_id: account_id,
                name,
            });
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
            <AuthorizedOracles<T>>::remove(account_id.clone());
            Self::deposit_event(Event::OracleRemoved { oracle_id: account_id });
            Ok(())
        }
    }
}

#[cfg_attr(test, mockable)]
impl<T: Config> Pallet<T> {
    // public only for testing purposes
    pub fn begin_block(_height: T::BlockNumber) {
        // read to a temporary value, because we can't alter the map while we iterate over it
        let raw_values_updated: Vec<_> = RawValuesUpdated::<T>::iter().collect();

        let current_time = Self::get_current_time();

        let mut updated_items = Vec::new();
        for (key, is_updated) in raw_values_updated.iter() {
            if *is_updated || Self::is_outdated(key, current_time) {
                let new_value = Self::update_aggregate(key);
                updated_items.push((key.clone(), new_value));
            }
        }

        if !updated_items.is_empty() {
            Self::deposit_event(Event::<T>::AggregateUpdated { values: updated_items });
        }

        let current_status_is_online = Self::is_oracle_online();
        let new_status_is_online = raw_values_updated.len() > 0
            && raw_values_updated
                .iter()
                .all(|(key, _)| Aggregate::<T>::get(key).is_some());

        if current_status_is_online != new_status_is_online {
            if new_status_is_online {
                Self::recover_from_oracle_offline();
            } else {
                Self::report_oracle_offline();
            }
        }
    }

    // public only for testing purposes
    pub fn _feed_values(oracle: T::AccountId, values: Vec<(OracleKey, T::UnsignedFixedPoint)>) {
        for (key, value) in values.iter() {
            let timestamped = TimestampedValue {
                timestamp: Self::get_current_time(),
                value: value.clone(),
            };
            RawValues::<T>::insert(key, &oracle, timestamped);
            RawValuesUpdated::<T>::insert(key, true);
        }

        Self::deposit_event(Event::<T>::FeedValues {
            oracle_id: oracle,
            values,
        });
    }

    /// Public getters

    /// Get the exchange rate in planck per satoshi
    pub fn get_price(key: OracleKey) -> Result<UnsignedFixedPoint<T>, DispatchError> {
        ext::security::ensure_parachain_status_running::<T>()?;

        Aggregate::<T>::get(key).ok_or(Error::<T>::MissingExchangeRate.into())
    }

    pub fn convert(amount: &Amount<T>, currency_id: T::CurrencyId) -> Result<Amount<T>, DispatchError> {
        let converted = match (amount.currency(), currency_id) {
            (x, y) if x == y => amount.amount(),
            (x, _) if x == T::GetWrappedCurrencyId::get() => {
                // convert interbtc to collateral
                Self::wrapped_to_collateral(amount.amount(), currency_id)?
            }
            (from_currency, x) if x == T::GetWrappedCurrencyId::get() => {
                // convert collateral to interbtc
                Self::collateral_to_wrapped(amount.amount(), from_currency)?
            }
            (_, _) => {
                // first convert to btc, then convert the btc to the desired currency
                let base = Self::collateral_to_wrapped(amount.amount(), amount.currency())?;
                Self::wrapped_to_collateral(base, currency_id)?
            }
        };
        Ok(Amount::new(converted, currency_id))
    }

    pub fn wrapped_to_collateral(amount: BalanceOf<T>, currency_id: CurrencyId) -> Result<BalanceOf<T>, DispatchError> {
        let rate = Self::get_price(OracleKey::ExchangeRate(currency_id))?;
        let converted = rate.checked_mul_int(amount).ok_or(ArithmeticError::Overflow)?;
        let result = converted.try_into().map_err(|_e| Error::<T>::TryIntoIntError)?;
        Ok(result)
    }

    pub fn collateral_to_wrapped(amount: BalanceOf<T>, currency_id: CurrencyId) -> Result<BalanceOf<T>, DispatchError> {
        let rate = Self::get_price(OracleKey::ExchangeRate(currency_id))?;
        if amount.is_zero() {
            return Ok(Zero::zero());
        }

        // The code below performs `amount/rate`, plus necessary type conversions
        Ok(T::UnsignedFixedPoint::checked_from_integer(amount)
            .ok_or(Error::<T>::TryIntoIntError)?
            .checked_div(&rate)
            .ok_or(ArithmeticError::Underflow)?
            .truncate_to_inner()
            .ok_or(Error::<T>::TryIntoIntError)?
            .unique_saturated_into())
    }

    fn update_aggregate(key: &OracleKey) -> Option<T::UnsignedFixedPoint> {
        RawValuesUpdated::<T>::insert(key, false);
        let mut raw_values: Vec<_> = RawValues::<T>::iter_prefix(key).map(|(_, value)| value).collect();
        let min_timestamp = Self::get_current_time().saturating_sub(Self::get_max_delay());
        raw_values.retain(|value| value.timestamp >= min_timestamp);
        if raw_values.len() == 0 {
            Aggregate::<T>::remove(key);
            ValidUntil::<T>::remove(key);
            None
        } else {
            let valid_until = raw_values
                .iter()
                .map(|x| x.timestamp)
                .min()
                .map(|timestamp| timestamp + Self::get_max_delay())
                .unwrap_or_default(); // Unwrap will never fail, but if somehow it did, we retry next block

            let mid_index = raw_values.len() / 2;
            let (_, value, _) = raw_values.select_nth_unstable_by(mid_index as usize, |a, b| a.value.cmp(&b.value));

            Aggregate::<T>::insert(key, value.value);
            ValidUntil::<T>::insert(key, valid_until);
            Some(value.value)
        }
    }

    /// Private getters and setters

    fn is_outdated(key: &OracleKey, current_time: T::Moment) -> bool {
        let valid_until = ValidUntil::<T>::get(key);
        matches!(valid_until, Some(t) if current_time > t)
    }

    fn get_max_delay() -> T::Moment {
        <MaxDelay<T>>::get()
    }

    /// Set the current exchange rate. ONLY FOR TESTING.
    ///
    /// # Arguments
    ///
    /// * `exchange_rate` - i.e. planck per satoshi
    pub fn _set_exchange_rate(currency_id: CurrencyId, exchange_rate: UnsignedFixedPoint<T>) -> DispatchResult {
        Aggregate::<T>::insert(OracleKey::ExchangeRate(currency_id), exchange_rate);
        // this is useful for benchmark tests
        Self::recover_from_oracle_offline();
        Ok(())
    }

    fn is_oracle_online() -> bool {
        !ext::security::get_errors::<T>().contains(&ErrorCode::OracleOffline)
    }

    fn report_oracle_offline() {
        ext::security::set_status::<T>(StatusCode::Error);
        ext::security::insert_error::<T>(ErrorCode::OracleOffline);
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
