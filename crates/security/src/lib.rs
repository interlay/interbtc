//! # Security Module
//! Based on the [specification](https://spec.interlay.io/spec/security.html).

#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

use sp_runtime::{traits::*, ArithmeticError};
use sp_std::convert::TryInto;

mod default_weights;
pub use default_weights::WeightInfo;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(test)]
extern crate mocktopus;

#[cfg(test)]
use mocktopus::macros::mockable;

use codec::Encode;
use frame_support::{dispatch::DispatchError, weights::Weight};
use frame_system::pallet_prelude::BlockNumberFor;
pub use pallet::*;
use sha2::{Digest, Sha256};
use sp_core::{H256, U256};
use sp_std::vec;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    /// ## Configuration
    /// The pallet's configuration trait.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Weight information for the extrinsics in this module.
        type WeightInfo: WeightInfo;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        UpdateActiveBlock { block_number: BlockNumberFor<T> },
        Activated,
        Deactivated,
    }

    #[pallet::error]
    pub enum Error<T> {}

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
            Self::increment_active_block();
            <T as Config>::WeightInfo::on_initialize()
        }
    }

    /// Integer increment-only counter, used to prevent collisions when generating identifiers
    /// for e.g. issue, redeem or replace requests (for OP_RETURN field in Bitcoin).
    #[pallet::storage]
    pub type Nonce<T: Config> = StorageValue<_, U256, ValueQuery>;

    /// Like frame_system::block_number, but this one only increments if the parachain status is RUNNING.
    /// This variable is used to keep track of durations, such as the issue/redeem/replace expiry. If the
    /// parachain is not RUNNING, no payment proofs can be submitted, and it wouldn't be fair to punish
    /// the user/vault. By using this variable we ensure that they have sufficient time to submit their
    /// proof.
    #[pallet::storage]
    #[pallet::getter(fn active_block_number)]
    pub type ActiveBlockCount<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

    #[pallet::storage]
    pub type IsDeactivated<T: Config> = StorageValue<_, bool, ValueQuery>;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // The pallet's dispatchable functions.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Activate or deactivate active block counting.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::activate_counter())]
        pub fn activate_counter(origin: OriginFor<T>, is_active: bool) -> DispatchResult {
            ensure_root(origin)?;

            // IsDeactivated is negative so that we don't need migration
            IsDeactivated::<T>::set(!is_active);

            if is_active {
                Self::deposit_event(Event::Activated);
            } else {
                Self::deposit_event(Event::Deactivated);
            }
            Ok(())
        }
    }
}
// "Internal" functions, callable by code.
#[cfg_attr(test, mockable)]
impl<T: Config> Pallet<T> {
    pub fn parachain_block_expired(
        opentime: BlockNumberFor<T>,
        period: BlockNumberFor<T>,
    ) -> Result<bool, DispatchError> {
        let expiration_block = opentime.checked_add(&period).ok_or(ArithmeticError::Overflow)?;
        Ok(Self::active_block_number() > expiration_block)
    }

    /// Increment and return the `Nonce`.
    fn get_nonce() -> U256 {
        <Nonce<T>>::mutate(|n| {
            let (res, _) = (*n).overflowing_add(U256::one());
            *n = res;
            *n
        })
    }

    fn increment_active_block() {
        if IsDeactivated::<T>::get() {
            return;
        }

        let height = <ActiveBlockCount<T>>::mutate(|n| {
            *n = n.saturating_add(1u32.into());
            *n
        });
        Self::deposit_event(Event::UpdateActiveBlock { block_number: height });
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
    pub fn set_active_block_number(n: BlockNumberFor<T>) {
        ActiveBlockCount::<T>::set(n);
    }
}
