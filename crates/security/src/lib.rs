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
use frame_support::{decl_module, decl_storage, dispatch::DispatchResult};
use primitive_types::H256;
use sha2::{Digest, Sha256};
use sp_core::U256;
use sp_std::collections::btree_set::BTreeSet;

/// ## Configuration
/// The pallet's configuration trait.
pub trait Trait: system::Trait {}

// This pallet's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as SecurityModule {
        /// Integer/Enum defining the current state of the BTC-Parachain.
        ParachainStatus get(parachain_status): StatusCode;

        /// Set of ErrorCodes, indicating the reason for an "Error" ParachainStatus.
        Errors get(fn error): BTreeSet<ErrorCode>;

        /// Integer increment-only counter, used to prevent collisions when generating identifiers
        /// for e.g. issue, redeem or replace requests (for OP_RETURN field in Bitcoin).
        Nonce get(fn nonce): U256;
    }
}

// The pallet's dispatchable functions.
decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
    }
}

// "Internal" functions, callable by code.
#[cfg_attr(test, mockable)]
impl<T: Trait> Module<T> {
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

    /// Mutates the set of `ErrorCode`.
    ///
    /// # Arguments
    ///
    /// * `f` - callback to manipulate errors.
    pub fn mutate_errors<F>(f: F) -> DispatchResult
    where
        F: FnOnce(&mut BTreeSet<ErrorCode>) -> DispatchResult,
    {
        <Errors>::mutate(f)
    }

    /// Get the current set of `ErrorCode`.
    pub fn get_errors() -> BTreeSet<ErrorCode> {
        <Errors>::get()
    }

    /// Increment and return the `Nonce`.
    fn get_nonce() -> U256 {
        <Nonce>::mutate(|n| {
            *n += U256::one();
            *n
        })
    }

    /// Generates a unique ID using an account identifier and the `Nonce`.
    ///
    /// # Arguments
    ///
    /// * `id`: Parachain account identifier.
    pub fn get_secure_id(id: &T::AccountId) -> H256 {
        let mut hasher = Sha256::default();
        hasher.input(id.encode());
        hasher.input(Self::get_nonce().encode());
        let mut result = [0; 32];
        result.copy_from_slice(&hasher.result()[..]);
        H256(result)
    }
}
