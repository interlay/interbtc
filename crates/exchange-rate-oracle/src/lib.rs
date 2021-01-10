#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

/// # Exchange Rate Oracle implementation
/// This is the implementation of the Exchange Rate Oracle following the spec at:
/// https://interlay.gitlab.io/polkabtc-spec/spec/oracle.html
mod ext;

#[cfg(any(feature = "runtime-benchmarks", test))]
mod benchmarking;

mod default_weights;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod mock;

#[cfg(test)]
extern crate mocktopus;

#[cfg(test)]
use mocktopus::macros::mockable;

use codec::{Decode, Encode, EncodeLike};
use frame_support::dispatch::{DispatchError, DispatchResult};
use frame_support::traits::Currency;
use frame_support::weights::Weight;
use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure};
use frame_system::{ensure_root, ensure_signed};
use sp_arithmetic::traits::UniqueSaturatedInto;
use sp_arithmetic::traits::*;
use sp_arithmetic::FixedPointNumber;
use sp_std::convert::TryInto;
use sp_std::vec::Vec;

pub(crate) type DOT<T> =
    <<T as collateral::Trait>::DOT as Currency<<T as frame_system::Trait>::AccountId>>::Balance;

pub(crate) type PolkaBTC<T> =
    <<T as treasury::Trait>::PolkaBTC as Currency<<T as frame_system::Trait>::AccountId>>::Balance;

pub(crate) type UnsignedFixedPoint<T> = <T as Trait>::UnsignedFixedPoint;

pub(crate) type Inner<T> = <<T as Trait>::UnsignedFixedPoint as FixedPointNumber>::Inner;

/// Storage version.
#[derive(Encode, Decode, Eq, PartialEq)]
pub enum Version {
    /// Initial version.
    V0,
    /// BtcAddress type with script format.
    V1,
}

pub trait WeightInfo {
    fn set_exchange_rate() -> Weight;
    fn set_btc_tx_fees_per_byte() -> Weight;
    fn insert_authorized_oracle() -> Weight;
    fn remove_authorized_oracle() -> Weight;
}

const BTC_DECIMALS: u32 = 8;
const DOT_DECIMALS: u32 = 10;

/// ## Configuration and Constants
/// The pallet's configuration trait.
pub trait Trait:
    frame_system::Trait + timestamp::Trait + treasury::Trait + collateral::Trait + security::Trait
{
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    type UnsignedFixedPoint: FixedPointNumber + Encode + EncodeLike + Decode;

    /// Weight information for the extrinsics in this module.
    type WeightInfo: WeightInfo;
}

#[derive(Encode, Decode, Default)]
pub struct BtcTxFeesPerByte {
    /// The estimated Satoshis per bytes to get included in the next block (~10 min)
    pub fast: u32,
    /// The estimated Satoshis per bytes to get included in the next 3 blocks (~half hour)
    pub half: u32,
    /// The estimated Satoshis per bytes to get included in the next 6 blocks (~hour)
    pub hour: u32,
}

// This pallet's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as ExchangeRateOracle {
        /// ## Storage
        /// Current planck per satoshi rate
        ExchangeRate: UnsignedFixedPoint<T>;

        /// Last exchange rate time
        LastExchangeRateTime: T::Moment;

        SatoshiPerBytes get(fn satoshi_per_bytes): BtcTxFeesPerByte;

        /// Maximum delay (milliseconds) for the exchange rate to be used
        MaxDelay get(fn max_delay) config(): T::Moment;

        // Oracles allowed to set the exchange rate, maps to the name
        AuthorizedOracles get(fn authorized_oracles) config(): map hasher(blake2_128_concat) T::AccountId => Vec<u8>;

        /// Build storage at V1 (requires default 0).
        StorageVersion get(fn storage_version) build(|_| Version::V1): Version = Version::V0;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        // Initializing events
        fn deposit_event() = default;

        // Errors must be initialized if they are used by the pallet.
        type Error = Error<T>;

        /// Upgrade the runtime depending on the current `StorageVersion`.
        fn on_runtime_upgrade() -> Weight {
            use frame_support::{Twox128, StorageHasher, migration::take_storage_item};
            use sp_std::vec;

            if Self::storage_version() == Version::V0 {

                fn take_storage_value<T: Decode + Sized>(module: &[u8], item: &[u8]) -> Option<T> {
                    let mut key = vec![0u8; 32];
                    key[0..16].copy_from_slice(&Twox128::hash(module));
                    key[16..32].copy_from_slice(&Twox128::hash(item));
                    frame_support::storage::unhashed::take::<T>(&key)
                }

                if let Some(account_id) = take_storage_value::<T::AccountId>(b"ExchangeRateOracle", b"AuthorizedOracle") {
                    let name = take_storage_item::<T::AccountId, Vec<u8>, Twox128>(b"ExchangeRateOracle", b"OracleNames", account_id.clone()).unwrap_or(vec![]);
                    <AuthorizedOracles<T>>::insert(account_id, name);
                }

                StorageVersion::put(Version::V1);
            }

            0
        }

        /// Sets the exchange rate.
        ///
        /// # Arguments
        ///
        /// * `dot_per_btc` - exchange rate in dot per btc. Note that this is _not_ the same unit
        /// that is stored in the ExchangeRate storage item.
        #[weight = <T as Trait>::WeightInfo::set_exchange_rate()]
        pub fn set_exchange_rate(origin, dot_per_btc: UnsignedFixedPoint<T>) -> DispatchResult {
            // Check that Parachain is not in SHUTDOWN
            ext::security::ensure_parachain_status_not_shutdown::<T>()?;

            let signer = ensure_signed(origin)?;

            // fail if the signer is not an authorized oracle
            ensure!(Self::is_authorized(&signer), Error::<T>::InvalidOracleSource);

            let planck_per_satoshi = Self::dot_per_btc_to_planck_per_satoshi(dot_per_btc)?;
            Self::_set_exchange_rate(planck_per_satoshi)?;

            Self::deposit_event(Event::<T>::SetExchangeRate(signer, dot_per_btc));

            Ok(())
        }

        #[weight = <T as Trait>::WeightInfo::set_btc_tx_fees_per_byte()]
        pub fn set_btc_tx_fees_per_byte(origin, fast: u32, half: u32, hour: u32) -> DispatchResult {
            // Check that Parachain is not in SHUTDOWN
            ext::security::ensure_parachain_status_not_shutdown::<T>()?;

            let signer = ensure_signed(origin)?;

            // fail if the signer is not the authorized oracle
            ensure!(Self::is_authorized(&signer), Error::<T>::InvalidOracleSource);

            // write the new values to storage
            let fees = BtcTxFeesPerByte{fast, half, hour};
            <SatoshiPerBytes>::put(fees);

            Self::deposit_event(Event::<T>::SetBtcTxFeesPerByte(signer, fast, half, hour));

            Ok(())
        }

        #[weight = <T as Trait>::WeightInfo::insert_authorized_oracle()]
        pub fn insert_authorized_oracle(origin, account_id: T::AccountId, name: Vec<u8>) -> DispatchResult {
            ensure_root(origin)?;
            Self::insert_oracle(account_id, name);
            Ok(())
        }

        #[weight = <T as Trait>::WeightInfo::remove_authorized_oracle()]
        pub fn remove_authorized_oracle(origin, account_id: T::AccountId) -> DispatchResult {
            ensure_root(origin)?;
            <AuthorizedOracles<T>>::remove(account_id);
            Ok(())
        }
    }
}

#[cfg_attr(test, mockable)]
impl<T: Trait> Module<T> {
    /// Public getters

    /// Get the exchange rate in planck per satoshi
    pub fn get_exchange_rate() -> Result<UnsignedFixedPoint<T>, DispatchError> {
        let max_delay_passed = Self::is_max_delay_passed();
        ensure!(!max_delay_passed, Error::<T>::MissingExchangeRate);
        Ok(<ExchangeRate<T>>::get())
    }

    /// Convert the dot per btc to planck per satoshi
    fn dot_per_btc_to_planck_per_satoshi(
        dot_per_btc: UnsignedFixedPoint<T>,
    ) -> Result<UnsignedFixedPoint<T>, DispatchError> {
        // safe to unwrap because we only use constants
        let conversion_factor = UnsignedFixedPoint::<T>::checked_from_rational(
            10_u128.pow(DOT_DECIMALS),
            10_u128.pow(BTC_DECIMALS),
        )
        .unwrap();

        dot_per_btc
            .checked_mul(&conversion_factor)
            .ok_or(Error::<T>::ArithmeticOverflow.into())
    }

    fn into_u128<I: TryInto<u128>>(x: I) -> Result<u128, DispatchError> {
        TryInto::<u128>::try_into(x).map_err(|_e| Error::<T>::TryIntoIntError.into())
    }

    pub fn btc_to_dots(amount: PolkaBTC<T>) -> Result<DOT<T>, DispatchError> {
        let rate = Self::get_exchange_rate()?;
        let raw_amount = Self::into_u128(amount)?;
        let converted = rate
            .checked_mul_int(raw_amount)
            .ok_or(Error::<T>::ArithmeticOverflow)?;
        let result = converted
            .try_into()
            .map_err(|_e| Error::<T>::TryIntoIntError)?;
        Ok(result)
    }

    pub fn dots_to_btc(amount: DOT<T>) -> Result<PolkaBTC<T>, DispatchError> {
        let rate = Self::get_exchange_rate()?;
        let raw_amount = Self::into_u128(amount)?;
        if raw_amount == 0 {
            return Ok(0.into());
        }

        // The code below performs `raw_amount/rate`, plus necessary type conversions
        let dot_as_inner: Inner<T> = raw_amount
            .try_into()
            .map_err(|_| Error::<T>::TryIntoIntError)?;
        let btc_raw: u128 = T::UnsignedFixedPoint::checked_from_integer(dot_as_inner)
            .ok_or(Error::<T>::TryIntoIntError)?
            .checked_div(&rate)
            .ok_or(Error::<T>::ArithmeticUnderflow)?
            .into_inner()
            .checked_div(&UnsignedFixedPoint::<T>::accuracy())
            .ok_or(Error::<T>::ArithmeticUnderflow)?
            .unique_saturated_into();
        btc_raw
            .try_into()
            .map_err(|_e| Error::<T>::TryIntoIntError.into())
    }

    pub fn get_last_exchange_rate_time() -> T::Moment {
        <LastExchangeRateTime<T>>::get()
    }

    /// Private getters and setters
    fn get_max_delay() -> T::Moment {
        <MaxDelay<T>>::get()
    }

    /// Set the current exchange rate
    ///
    /// # Arguments
    ///
    /// * `planck_per_satoshi` - exchange rate in planck per satoshi
    pub fn _set_exchange_rate(planck_per_satoshi: UnsignedFixedPoint<T>) -> DispatchResult {
        <ExchangeRate<T>>::put(planck_per_satoshi);
        // recover if the max delay was already passed
        if Self::is_max_delay_passed() {
            Self::recover_from_oracle_offline()?;
        }
        let now = Self::get_current_time();
        Self::set_last_exchange_rate_time(now);
        Ok(())
    }

    fn set_last_exchange_rate_time(time: T::Moment) {
        <LastExchangeRateTime<T>>::put(time);
    }

    fn recover_from_oracle_offline() -> DispatchResult {
        ext::security::recover_from_oracle_offline::<T>()
    }

    /// Returns true if the last update to the exchange rate
    /// was before the maximum allowed delay
    pub fn is_max_delay_passed() -> bool {
        let timestamp = Self::get_current_time();
        let last_update = Self::get_last_exchange_rate_time();
        let max_delay = Self::get_max_delay();
        last_update + max_delay < timestamp
    }

    /// Returns the current timestamp
    fn get_current_time() -> T::Moment {
        <timestamp::Module<T>>::get()
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

decl_event! {
    /// ## Events
    pub enum Event<T> where
            AccountId = <T as frame_system::Trait>::AccountId,
            UnsignedFixedPoint = UnsignedFixedPoint<T>,
        {
        /// Event emitted when exchange rate is set
        SetExchangeRate(AccountId, UnsignedFixedPoint),
        /// Event emitted when the btc tx fees are set
        SetBtcTxFeesPerByte(AccountId, u32, u32, u32),
    }
}

decl_error! {
    pub enum Error for Module<T: Trait> {
        /// Not authorized to set exchange rate
        InvalidOracleSource,
        /// Exchange rate not specified or has expired
        MissingExchangeRate,
        /// Unable to convert value
        TryIntoIntError,
        ArithmeticOverflow,
        ArithmeticUnderflow,
    }
}
