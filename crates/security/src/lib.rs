#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

pub mod types;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(test)]
extern crate mocktopus;

#[cfg(test)]
use mocktopus::macros::mockable;

pub use crate::types::{ErrorCode, StatusCode};
use codec::Encode;
/// # Security module implementation
/// This is the implementation of the BTC Parachain Security module following the spec at:
/// https://interlay.gitlab.io/polkabtc-spec/spec/security
///
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::{DispatchError, DispatchResult},
};
use primitive_types::H256;
use sha2::{Digest, Sha256};
use sp_core::U256;
use sp_std::collections::btree_set::BTreeSet;
use sp_std::iter::FromIterator;
use sp_std::prelude::*;

/// ## Configuration
/// The pallet's configuration trait.
pub trait Trait: frame_system::Trait {
    /// The overarching event type.
    type Event: From<Event> + Into<<Self as frame_system::Trait>::Event>;
}

// This pallet's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as SecurityModule {
        /// Integer/Enum defining the current state of the BTC-Parachain.
        ParachainStatus get(fn status): StatusCode;

        /// Set of ErrorCodes, indicating the reason for an "Error" ParachainStatus.
        Errors get(fn errors): BTreeSet<ErrorCode>;

        /// Integer increment-only counter, used to prevent collisions when generating identifiers
        /// for e.g. issue, redeem or replace requests (for OP_RETURN field in Bitcoin).
        Nonce: U256;
    }
}

// The pallet's dispatchable functions.
decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        fn deposit_event() = default;
    }
}

// "Internal" functions, callable by code.
#[cfg_attr(test, mockable)]
impl<T: Trait> Module<T> {
    /// Ensures the Parachain is RUNNING
    pub fn ensure_parachain_status_running() -> Result<(), DispatchError> {
        if <ParachainStatus>::get() == StatusCode::Running {
            Ok(())
        } else {
            Err(Error::<T>::ParachainNotRunning.into())
        }
    }

    /// Ensures the Parachain is not SHUTDOWN
    pub fn ensure_parachain_status_not_shutdown() -> Result<(), DispatchError> {
        if <ParachainStatus>::get() != StatusCode::Shutdown {
            Ok(())
        } else {
            Err(Error::<T>::ParachainShutdown.into())
        }
    }

    /// Ensures that the parachain DOES NOT have the given errors
    ///
    /// # Arguments
    ///
    ///   * `error_codes` - list of `ErrorCode` to be checked
    ///
    /// Returns the first error that is encountered, or Ok(()) if none of the errors were found
    pub fn ensure_parachain_does_not_have_errors(
        error_codes: Vec<ErrorCode>,
    ) -> Result<(), DispatchError> {
        if <ParachainStatus>::get() == StatusCode::Error {
            for error_code in error_codes {
                if <Errors>::get().contains(&error_code) {
                    return Err(Error::<T>::from(error_code).into());
                }
            }
        }
        Ok(())
    }

    /// Ensures that the parachain HAS ONLY SPECIFIC errors or NO error AT ALL
    ///
    /// # Arguments
    ///
    ///   * `error_codes` - list of `ErrorCode` to be checked
    ///
    /// Returns the first unexpected error that is encountered,
    /// or Ok(()) if only expected errors / no errors at all were found
    pub fn ensure_parachain_only_has_errors(
        error_codes: Vec<ErrorCode>,
    ) -> Result<(), DispatchError> {
        if <ParachainStatus>::get() == StatusCode::Error {
            let error_set: BTreeSet<ErrorCode> = FromIterator::from_iter(error_codes);
            for error_code in <Errors>::get().iter() {
                // check if error is set
                if !error_set.contains(&error_code) {
                    return Err(Error::<T>::from(error_code.clone()).into());
                }
            }
        }
        Ok(())
    }

    /// Ensures the Parachain is not in an ERROR state due to OracleOffline error
    pub fn _ensure_parachain_status_not_error_oracle_offline() -> Result<(), DispatchError> {
        if <ParachainStatus>::get() == StatusCode::Error
            && <Errors>::get().contains(&ErrorCode::OracleOffline)
        {
            return Err(Error::<T>::ParachainOracleOfflineError.into());
        }
        Ok(())
    }

    /// Checks if the Parachain has a NoDataBTCRelay Error state
    pub fn is_parachain_error_no_data_btcrelay() -> bool {
        <ParachainStatus>::get() == StatusCode::Error
            && <Errors>::get().contains(&ErrorCode::NoDataBTCRelay)
    }

    /// Checks if the Parachain has a InvalidBTCRelay Error state
    pub fn is_parachain_error_invalid_btcrelay() -> bool {
        <ParachainStatus>::get() == StatusCode::Error
            && <Errors>::get().contains(&ErrorCode::InvalidBTCRelay)
    }

    /// Checks if the Parachain has a OracleOffline Error state
    pub fn is_parachain_error_oracle_offline() -> bool {
        <ParachainStatus>::get() == StatusCode::Error
            && <Errors>::get().contains(&ErrorCode::OracleOffline)
    }

    /// Checks if the Parachain has a Liquidation Error state
    pub fn is_parachain_error_liquidation() -> bool {
        <ParachainStatus>::get() == StatusCode::Error
            && <Errors>::get().contains(&ErrorCode::Liquidation)
    }

    /// Gets the current `StatusCode`.
    pub fn get_parachain_status() -> StatusCode {
        <ParachainStatus>::get()
    }

    /// Sets the given `StatusCode`.
    ///
    /// # Arguments
    ///
    /// * `status_code` - to set in storage.
    pub fn set_parachain_status(status_code: StatusCode) {
        <ParachainStatus>::set(status_code);
    }

    /// Get the current set of `ErrorCode`.
    pub fn get_errors() -> BTreeSet<ErrorCode> {
        <Errors>::get()
    }

    /// Inserts the given `ErrorCode`.
    ///
    /// # Arguments
    ///
    /// * `error_code` - the error to insert.
    pub fn insert_error(error_code: ErrorCode) {
        <Errors>::mutate(|errors| {
            errors.insert(error_code);
        })
    }

    /// Removes the given `ErrorCode`.
    ///
    /// # Arguments
    ///
    /// * `error_code` - the error to remove.
    pub fn remove_error(error_code: ErrorCode) {
        <Errors>::mutate(|errors| {
            errors.remove(&error_code);
        })
    }

    fn recover_from_(error_codes: Vec<ErrorCode>) -> DispatchResult {
        for error_code in error_codes.clone() {
            Self::remove_error(error_code);
        }

        if Self::get_errors().is_empty() {
            Self::set_parachain_status(StatusCode::Running);
        }

        Self::deposit_event(Event::RecoverFromErrors(
            Self::get_parachain_status(),
            error_codes,
        ));

        Ok(())
    }

    /// Recovers the BTC Parachain state from a `LIQUIDATION` error
    /// and sets ParachainStatus to `RUNNING` if there are no other errors.
    pub fn recover_from_liquidation() -> DispatchResult {
        Self::recover_from_(vec![ErrorCode::Liquidation])
    }

    /// Recovers the BTC Parachain state from an `ORACLE_OFFLINE` error
    /// and sets ParachainStatus to `RUNNING` if there are no other errors.
    pub fn recover_from_oracle_offline() -> DispatchResult {
        Self::recover_from_(vec![ErrorCode::OracleOffline])
    }

    /// Recovers the BTC Parachain state from a `NO_DATA_BTC_RELAY` or `INVALID_BTC_RELAY` error
    /// (when a chain reorganization occurs and the new main chain has no errors)
    /// and sets ParachainStatus to `RUNNING` if there are no other errors.
    pub fn recover_from_btc_relay_failure() -> DispatchResult {
        Self::recover_from_(vec![ErrorCode::InvalidBTCRelay, ErrorCode::NoDataBTCRelay])
    }

    /// Increment and return the `Nonce`.
    fn get_nonce() -> U256 {
        <Nonce>::mutate(|n| {
            *n += U256::one();
            *n
        })
    }

    /// Generates a 256-bit unique hash from an `AccountId` and the
    /// internal (auto-incrementing) `Nonce` to prevent replay attacks.
    ///
    /// # Arguments
    ///
    /// * `id`: Parachain account identifier.
    pub fn get_secure_id(id: &T::AccountId) -> H256 {
        let mut hasher = Sha256::default();
        hasher.input(id.encode());
        hasher.input(Self::get_nonce().encode());
        // supplement with prev block hash to prevent replays
        // even if the `Nonce` is reset (i.e. purge-chain)
        hasher.input(frame_system::Module::<T>::parent_hash());
        let mut result = [0; 32];
        result.copy_from_slice(&hasher.result()[..]);
        H256(result)
    }
}

decl_event!(
    pub enum Event {
        RecoverFromErrors(StatusCode, Vec<ErrorCode>),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        NoDataBTCRelay,
        InvalidBTCRelay,
        ParachainNotRunning,
        ParachainShutdown,
        ParachainNotRunningOrLiquidation,
        ParachainOracleOfflineError,
        ParachainLiquidationError,
        InvalidErrorCode,
    }
}

impl<T: Trait> From<ErrorCode> for Error<T> {
    fn from(error_code: ErrorCode) -> Self {
        match error_code {
            ErrorCode::NoDataBTCRelay => Error::NoDataBTCRelay,
            ErrorCode::InvalidBTCRelay => Error::InvalidBTCRelay,
            ErrorCode::OracleOffline => Error::ParachainOracleOfflineError,
            ErrorCode::Liquidation => Error::ParachainLiquidationError,
            _ => Error::InvalidErrorCode,
        }
    }
}
