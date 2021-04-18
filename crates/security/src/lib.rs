//! # PolkaBTC Security Module
//! Based on the [specification](https://interlay.gitlab.io/polkabtc-spec/spec/security).

#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

use sp_runtime::traits::*;

pub mod types;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(test)]
extern crate mocktopus;

#[cfg(test)]
use mocktopus::macros::mockable;

#[doc(inline)]
pub use crate::types::{ErrorCode, StatusCode};

use codec::Encode;
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::{DispatchError, DispatchResult},
    transactional,
    weights::Weight,
};
use frame_system::ensure_root;
use primitive_types::H256;
use sha2::{Digest, Sha256};
use sp_core::U256;
use sp_std::{collections::btree_set::BTreeSet, prelude::*};

/// ## Configuration
/// The pallet's configuration trait.
pub trait Config: frame_system::Config {
    /// The overarching event type.
    type Event: From<Event> + Into<<Self as frame_system::Config>::Event>;
}

// This pallet's storage items.
decl_storage! {
    trait Store for Module<T: Config> as SecurityPallet {
        /// Integer/Enum defining the current state of the BTC-Parachain.
        ParachainStatus get(fn status): StatusCode;

        /// Set of ErrorCodes, indicating the reason for an "Error" ParachainStatus.
        Errors get(fn errors): BTreeSet<ErrorCode>;

        /// Integer increment-only counter, used to prevent collisions when generating identifiers
        /// for e.g. issue, redeem or replace requests (for OP_RETURN field in Bitcoin).
        Nonce: U256;

        /// Like frame_system::block_number, but this one only increments if the parachain status is RUNNING.
        /// This variable is used to keep track of durations, such as the issue/redeem/replace expiry. If the
        /// parachain is not RUNNING, no payment proofs can be submitted, and it wouldn't be fair to punish
        /// the user/vault. By using this variable we ensure that they have sufficient time to submit their
        /// proof.
        ActiveBlockCount get(fn active_block_number): T::BlockNumber;
    }
}

// The pallet's dispatchable functions.
decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        fn deposit_event() = default;

        /// Upgrade the runtime depending on the current `StorageVersion`.
        fn on_runtime_upgrade() -> Weight {
            if !ActiveBlockCount::<T>::exists() {
                let chain_height = <frame_system::Pallet<T>>::block_number();
                ActiveBlockCount::<T>::set(chain_height);
            }
            0
        }

        fn on_initialize(_chain_height: T::BlockNumber) -> Weight {
            if Self::status() == StatusCode::Running {
                <ActiveBlockCount<T>>::mutate(|n| {
                    *n = n.saturating_add(1u32.into());
                });
            }

            0
        }

        /// Set the parachain status code.
        ///
        /// # Arguments
        ///
        /// * `origin` - the dispatch origin of this call (must be _Root_)
        /// * `status_code` - the status code to set
        ///
        /// # Weight: `O(1)`
        #[weight = 0]
        #[transactional]
        pub fn set_parachain_status(origin, status_code: StatusCode) {
            ensure_root(origin)?;
            Self::set_status(status_code);
        }

        /// Insert a new parachain error.
        ///
        /// # Arguments
        ///
        /// * `origin` - the dispatch origin of this call (must be _Root_)
        /// * `error_code` - the error code to insert
        ///
        /// # Weight: `O(1)`
        #[weight = 0]
        #[transactional]
        pub fn insert_parachain_error(origin, error_code: ErrorCode) {
            ensure_root(origin)?;
            Self::insert_error(error_code);
        }

        /// Remove a parachain error.
        ///
        /// # Arguments
        ///
        /// * `origin` - the dispatch origin of this call (must be _Root_)
        /// * `error_code` - the error code to remove
        ///
        /// # Weight: `O(1)`
        #[weight = 0]
        #[transactional]
        pub fn remove_parachain_error(origin, error_code: ErrorCode) {
            ensure_root(origin)?;
            Self::remove_error(error_code);
        }
    }
}

// "Internal" functions, callable by code.
#[cfg_attr(test, mockable)]
impl<T: Config> Module<T> {
    /// Ensures the Parachain is RUNNING
    pub fn ensure_parachain_status_running() -> DispatchResult {
        if <ParachainStatus>::get() == StatusCode::Running {
            Ok(())
        } else {
            Err(Error::<T>::ParachainNotRunning.into())
        }
    }

    /// Ensures the Parachain is not SHUTDOWN
    pub fn ensure_parachain_status_not_shutdown() -> DispatchResult {
        if <ParachainStatus>::get() != StatusCode::Shutdown {
            Ok(())
        } else {
            Err(Error::<T>::ParachainShutdown.into())
        }
    }

    /// Checks if the Parachain has a NoDataBTCRelay Error state
    pub fn is_parachain_error_no_data_btcrelay() -> bool {
        <ParachainStatus>::get() == StatusCode::Error && <Errors>::get().contains(&ErrorCode::NoDataBTCRelay)
    }

    /// Checks if the Parachain has a InvalidBTCRelay Error state
    pub fn is_parachain_error_invalid_btcrelay() -> bool {
        <ParachainStatus>::get() == StatusCode::Error && <Errors>::get().contains(&ErrorCode::InvalidBTCRelay)
    }

    /// Checks if the Parachain has a OracleOffline Error state
    pub fn is_parachain_error_oracle_offline() -> bool {
        <ParachainStatus>::get() == StatusCode::Error && <Errors>::get().contains(&ErrorCode::OracleOffline)
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
    pub fn set_status(status_code: StatusCode) {
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

    pub fn has_expired(opentime: T::BlockNumber, period: T::BlockNumber) -> Result<bool, DispatchError> {
        let expiration_block = opentime.checked_add(&period).ok_or(Error::<T>::ArithmeticOverflow)?;
        Ok(Self::active_block_number() > expiration_block)
    }

    fn recover_from_(error_codes: Vec<ErrorCode>) {
        for error_code in error_codes.clone() {
            Self::remove_error(error_code);
        }

        if Self::get_errors().is_empty() {
            Self::set_status(StatusCode::Running);
        }

        Self::deposit_event(Event::RecoverFromErrors(Self::get_parachain_status(), error_codes));
    }

    /// Recovers the BTC Parachain state from an `ORACLE_OFFLINE` error
    /// and sets ParachainStatus to `RUNNING` if there are no other errors.
    pub fn recover_from_oracle_offline() {
        Self::recover_from_(vec![ErrorCode::OracleOffline])
    }

    /// Recovers the BTC Parachain state from a `NO_DATA_BTC_RELAY` or `INVALID_BTC_RELAY` error
    /// (when a chain reorganization occurs and the new main chain has no errors)
    /// and sets ParachainStatus to `RUNNING` if there are no other errors.
    pub fn recover_from_btc_relay_failure() {
        Self::recover_from_(vec![ErrorCode::InvalidBTCRelay, ErrorCode::NoDataBTCRelay])
    }

    /// Increment and return the `Nonce`.
    fn get_nonce() -> U256 {
        <Nonce>::mutate(|n| {
            let (res, _) = (*n).overflowing_add(U256::one());
            *n = res;
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
        hasher.input(frame_system::Pallet::<T>::parent_hash());
        let mut result = [0; 32];
        result.copy_from_slice(&hasher.result()[..]);
        H256(result)
    }

    /// for testing purposes only!
    pub fn set_active_block_number(n: T::BlockNumber) {
        ActiveBlockCount::<T>::set(n);
    }
}

decl_event!(
    pub enum Event {
        RecoverFromErrors(StatusCode, Vec<ErrorCode>),
    }
);

decl_error! {
    pub enum Error for Module<T: Config> {
        NoDataBTCRelay,
        InvalidBTCRelay,
        ParachainNotRunning,
        ParachainShutdown,
        ParachainNotRunningOrLiquidation,
        ParachainOracleOfflineError,
        ParachainLiquidationError,
        InvalidErrorCode,
        ArithmeticOverflow
    }
}

impl<T: Config> From<ErrorCode> for Error<T> {
    fn from(error_code: ErrorCode) -> Self {
        match error_code {
            ErrorCode::NoDataBTCRelay => Error::NoDataBTCRelay,
            ErrorCode::InvalidBTCRelay => Error::InvalidBTCRelay,
            ErrorCode::OracleOffline => Error::ParachainOracleOfflineError,
            _ => Error::InvalidErrorCode,
        }
    }
}
