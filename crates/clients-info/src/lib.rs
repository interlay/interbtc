//! # ClientsInfo Module
//! Stores information about clients that comprise the network, such as vaults and oracles.

#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use scale_info::TypeInfo;

mod default_weights;

pub use default_weights::WeightInfo;

use frame_support::{dispatch::DispatchResult, traits::Get, transactional};

use sp_std::vec::Vec;

#[cfg(test)]
mod mock;

#[derive(Encode, Decode, Eq, PartialEq, Clone, Default, TypeInfo, Debug)]
pub struct ClientRelease<Hash> {
    /// URI to the client release binary.
    pub uri: Vec<u8>,
    /// The SHA256 checksum of the client binary.
    pub checksum: Hash,
}

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use crate::*;

    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::pallet]
    #[pallet::generate_store(trait Store)]
    #[pallet::without_storage_info] // ClientRelease struct contains vec which doesn't implement MaxEncodedLen
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type Event: From<Event<Self>>
            + Into<<Self as frame_system::Config>::Event>
            + IsType<<Self as frame_system::Config>::Event>;

        /// Weight information for the extrinsics in this module.
        type WeightInfo: WeightInfo;
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_runtime_upgrade() -> frame_support::weights::Weight {
            crate::upgrade_client_releases::try_upgrade_current_client_releases::<T>()
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Sets the current client release version, in case of a bug fix or patch.
        /// Clients incude the vault, oracle, and faucet.
        ///
        /// # Arguments
        /// * `client_name` - raw byte string representation of the client name (e.g. `b"vault"`, `b"oracle"`,
        ///   `b"faucet"`)
        /// * `release` - The release information for the given `client_name`
        #[pallet::weight(<T as Config>::WeightInfo::set_current_client_release())]
        #[transactional]
        pub fn set_current_client_release(
            origin: OriginFor<T>,
            client_name: Vec<u8>,
            release: ClientRelease<T::Hash>,
        ) -> DispatchResult {
            ensure_root(origin)?;
            CurrentClientReleases::<T>::insert(client_name, release.clone());
            Self::deposit_event(Event::<T>::ApplyClientRelease { release });
            Ok(())
        }

        /// Sets the pending client release version. To be batched alongside the
        /// `parachainSystem.authorizeUpgrade` Cumulus call.
        /// Clients incude the vault, oracle, and faucet.
        ///
        /// # Arguments
        /// * `client_name` - raw byte string representation of the client name (e.g. `b"vault"`, `b"oracle"`,
        ///   `b"faucet"`)
        /// * `release` - The release information for the given `client_name`
        #[pallet::weight(<T as Config>::WeightInfo::set_pending_client_release())]
        #[transactional]
        pub fn set_pending_client_release(
            origin: OriginFor<T>,
            client_name: Vec<u8>,
            release: ClientRelease<T::Hash>,
        ) -> DispatchResult {
            ensure_root(origin)?;
            PendingClientReleases::<T>::insert(client_name, release.clone());
            Self::deposit_event(Event::<T>::NotifyClientRelease { release });
            Ok(())
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        NotifyClientRelease { release: ClientRelease<T::Hash> },
        ApplyClientRelease { release: ClientRelease<T::Hash> },
    }

    #[pallet::error]
    pub enum Error<T> {}

    /// Mapping of client name (string literal represented as bytes) to its release details.
    #[pallet::storage]
    #[pallet::getter(fn current_client_release)]
    pub(super) type CurrentClientReleases<T: Config> =
        StorageMap<_, Blake2_128Concat, Vec<u8>, ClientRelease<T::Hash>, OptionQuery>;

    /// Mapping of client name (string literal represented as bytes) to its pending release details.
    #[pallet::storage]
    #[pallet::getter(fn pending_client_release)]
    pub(super) type PendingClientReleases<T: Config> =
        StorageMap<_, Blake2_128Concat, Vec<u8>, ClientRelease<T::Hash>, OptionQuery>;
}

pub mod upgrade_client_releases {

    use crate::*;
    use frame_support::weights::Weight;

    /// For each pending client release, set the current release to that.
    /// The pending release entry is removed.
    pub fn try_upgrade_current_client_releases<T: Config>() -> Weight {
        let mut reads: Weight = 0;
        for (key, release) in PendingClientReleases::<T>::drain() {
            log::info!("Upgrading client release for key {:?}", key);
            CurrentClientReleases::<T>::insert(key, release.clone());
            Pallet::<T>::deposit_event(Event::<T>::ApplyClientRelease { release });
            reads += 1;
        }
        T::DbWeight::get().reads_writes(reads, reads * 2)
    }

    #[cfg(test)]
    #[test]
    fn test_client_pending_release_migration() {
        use std::collections::HashMap;

        use sp_core::H256;

        use sp_std::vec;

        use crate::mock::Test;

        crate::mock::run_test(|| {
            let vault_key = b"vault".to_vec();
            let oracle_key = b"oracle".to_vec();
            let faucet_key = b"faucet".to_vec();

            let pre_migration_pending_releases: HashMap<_, _> = vec![
                (vault_key.clone(), ClientRelease {
                    uri: b"https://github.com/interlay/interbtc-clients/releases/download/1.15.0/vault-standalone-metadata"
                        .to_vec(),
                    checksum: H256::default(),
                }),
                (oracle_key.clone(), ClientRelease {
                    uri: b"https://github.com/interlay/interbtc-clients/releases/download/1.15.0/oracle-standalone-metadata"
                        .to_vec(),
                    checksum: H256::default(),
                })
            ].into_iter().collect();
            pre_migration_pending_releases.iter().for_each(|(key, value)| {
                PendingClientReleases::<Test>::insert(key, value.clone());
            });

            let pre_migration_current_releases: HashMap<_, _> = vec![
                (vault_key.clone(), ClientRelease {
                    uri: b"https://github.com/interlay/interbtc-clients/releases/download/1.14.0/vault-standalone-metadata"
                        .to_vec(),
                    checksum: H256::default(),
                }),
                (oracle_key.clone(), ClientRelease {
                    uri: b"https://github.com/interlay/interbtc-clients/releases/download/1.14.0/oracle-standalone-metadata"
                        .to_vec(),
                    checksum: H256::default(),
                }),
                (faucet_key.clone(), ClientRelease {
                    uri: b"https://github.com/interlay/interbtc-clients/releases/download/1.14.0/faucet-standalone-metadata"
                        .to_vec(),
                    checksum: H256::default(),
                })
            ].into_iter().collect();
            pre_migration_current_releases.iter().for_each(|(key, value)| {
                CurrentClientReleases::<Test>::insert(key, value.clone());
            });

            try_upgrade_current_client_releases::<Test>();

            let pending_releases = PendingClientReleases::<Test>::iter_values().collect::<Vec<_>>();
            assert_eq!(pending_releases.is_empty(), true);

            let current_releases = CurrentClientReleases::<Test>::iter().collect::<HashMap<_, _>>();
            assert_eq!(
                current_releases.get(&vault_key),
                pre_migration_pending_releases.get(&vault_key)
            );
            assert_eq!(
                current_releases.get(&oracle_key),
                pre_migration_pending_releases.get(&oracle_key)
            );
            // The faucet release should not be updated
            assert_eq!(
                current_releases.get(&faucet_key),
                pre_migration_current_releases.get(&faucet_key)
            );
        });
    }
}
