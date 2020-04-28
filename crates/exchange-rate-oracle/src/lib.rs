#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(test)]
mod tests;

#[cfg(test)]
mod mock;

#[cfg(test)]
extern crate mocktopus;

#[cfg(test)]
use mocktopus::macros::mockable;

use frame_support::dispatch::DispatchResult;
/// # Exchange Rate Oracle implementation
/// This is the implementation of the Exchange Rate Oracle following the spec at:
/// https://interlay.gitlab.io/polkabtc-spec/spec/oracle.html
// Substrate
use frame_support::{decl_event, decl_module, decl_storage, ensure};
use std::time::SystemTime;
use system::ensure_signed;
use x_core::Error;

/// ## Configuration and Constants
/// The pallet's configuration trait.
pub trait Trait: system::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

/// Granularity of exchange rate
pub const GRANULARITY: u128 = 5;

// This pallet's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as ExchangeRateOracle {
    /// ## Storage
        /// Current BTC/DOT exchange rate
        ExchangeRate: u128;

        /// Last exchange rate time
        LastExchangeRateTime: u64;

        /// Maximum delay for the exchange rate to be used
        MaxDelay: u64;

        // Oracle allowed to set the exchange rate
        AuthorizedOracle: T::AccountId;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        // Initializing events
        fn deposit_event() = default;
        pub fn set_exchange_rate(origin, rate: u128) -> DispatchResult {
            Self::ensure_parachain_running()?;

            let sender = ensure_signed(origin)?;

            // fail if the sender is not the authorized oracle
            ensure!(sender == Self::get_authorized_oracle(), Error::InvalidOracleSource);

            Self::internal_set_rate(rate)?;

            Self::deposit_event(Event::<T>::SetExchangeRate(sender, rate));

            Ok(())
        }
    }
}

#[cfg_attr(test, mockable)]
impl<T: Trait> Module<T> {
    /// Public getters
    pub fn get_exchange_rate() -> Result<u128, Error> {
        let max_delay_passed = Self::is_max_delay_passed()?;
        ensure!(!max_delay_passed, Error::MissingExchangeRate);
        Ok(<ExchangeRate>::get())
    }

    pub fn get_last_exchange_rate_time() -> u64 {
        <LastExchangeRateTime>::get()
    }

    /// Private getters and setters
    fn get_max_delay() -> u64 {
        <MaxDelay>::get()
    }

    pub fn internal_set_rate(rate: u128) -> Result<(), Error> {
        Self::set_current_rate(rate);
        // recover if the max delay was already passed
        if Self::is_max_delay_passed()? {
            Self::recover_from_oracle_offline()?;
        }
        let now = Self::seconds_since_epoch()?;
        Self::set_last_exchange_rate_time(now);
        Ok(())
    }

    pub fn set_current_rate(rate: u128) {
        <ExchangeRate>::put(rate);
    }

    fn set_last_exchange_rate_time(time: u64) {
        <LastExchangeRateTime>::put(time);
    }

    fn get_authorized_oracle() -> T::AccountId {
        <AuthorizedOracle<T>>::get()
    }

    /// Other helpers
    /// Returns an error if the parachain is not in running state
    fn ensure_parachain_running() -> Result<(), Error> {
        // TODO: integrate security module
        // ensure!(
        //     !<security::Module<T>>::check_parachain_status(
        //         StatusCode::Shutdown),
        //     Error::Shutdown
        // );
        Ok(())
    }

    fn recover_from_oracle_offline() -> Result<(), Error> {
        // TODO: call recoverFromORACLEOFFLINE in security module
        Ok(())
    }

    /// Returns true if the last update to the exchange rate
    /// was before the maximum allowed delay
    fn is_max_delay_passed() -> Result<bool, Error> {
        let timestamp = Self::seconds_since_epoch()?;
        let last_update = Self::get_last_exchange_rate_time();
        let max_delay = Self::get_max_delay();
        Ok(timestamp - last_update > max_delay)
    }

    /// Returns the number of seconds ellapsed since UNIX epoch
    fn seconds_since_epoch() -> Result<u64, Error> {
        let now = SystemTime::now();
        let epoch_duration = now
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|_e| Error::RuntimeError)?;
        Ok(epoch_duration.as_secs())
    }
}

decl_event! {
    /// ## Events
    pub enum Event<T> where
            AccountId = <T as system::Trait>::AccountId {
        /// Event emitted when exchange rate is set
        SetExchangeRate(AccountId, u128),
    }
}
