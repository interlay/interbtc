//! # Collator Setter
//! Updates the Aura authorities of a running chain.

#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{traits::OneSessionHandler, transactional};
use frame_system::ensure_root;
use sp_std::{vec, vec::Vec};

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    /// ## Configuration
    /// The pallet's configuration trait.
    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_aura::Config {}

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // The pallet's dispatchable functions.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(0)]
        #[transactional]
        pub fn set_collators(
            origin: OriginFor<T>,
            validators: Vec<(T::AccountId, T::AuthorityId)>,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            pallet_aura::Pallet::<T>::on_new_session(
                true,
                validators
                    .iter()
                    .map(|(account_id, authority_id)| (account_id, authority_id.clone()))
                    .collect::<Vec<_>>()
                    .into_iter(),
                vec![].into_iter(),
            );
            Ok(().into())
        }
    }
}
