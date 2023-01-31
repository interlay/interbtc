use crate::*;
use frame_support::{pallet_prelude::StorageVersion, traits::OnRuntimeUpgrade};
use sp_std::vec::Vec;

/// The log target.
const TARGET: &'static str = "zenlink_protocol::migration";

pub mod v0 {
	use super::*;

	#[derive(Clone, Copy, Encode, Decode, RuntimeDebug, PartialEq, Eq, MaxEncodedLen, TypeInfo)]
	pub enum PairStatus<Balance, BlockNumber, Account> {
		Trading(PairMetadata<Balance, Account>),
		Bootstrap(BootstrapParameter<Balance, BlockNumber, Account>),
		Disable,
	}

	impl<Balance, BlockNumber, Account> Default for PairStatus<Balance, BlockNumber, Account> {
		fn default() -> Self {
			Self::Disable
		}
	}

	#[derive(Encode, Decode, Clone, Copy, RuntimeDebug, PartialEq, Eq, MaxEncodedLen, TypeInfo)]
	pub struct PairMetadata<Balance, Account> {
		pub pair_account: Account,
		pub total_supply: Balance,
	}

	#[frame_support::storage_alias]
	pub(crate) type PairStatuses<T: Config> = StorageMap<
		Pallet<T>,
		Twox64Concat,
		(<T as Config>::AssetId, <T as Config>::AssetId),
		PairStatus<
			AssetBalance,
			<T as frame_system::Config>::BlockNumber,
			<T as frame_system::Config>::AccountId,
		>,
		ValueQuery,
	>;
}

pub mod v1 {
	use super::*;

	/// Migrate the pallet from V0 to V1.
	pub struct MigrateToV1<T>(sp_std::marker::PhantomData<T>);

	impl<T: Config> OnRuntimeUpgrade for MigrateToV1<T> {
		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
			assert_eq!(StorageVersion::get::<Pallet<T>>(), 0, "Can only upgrade from version 0");

			let old_entries = v0::PairStatuses::<T>::iter().count();
			log::info!(target: TARGET, "{old_entries} entries will be migrated");

			Ok((old_entries as u32).encode())
		}

		fn on_runtime_upgrade() -> Weight {
			let version = StorageVersion::get::<Pallet<T>>();
			if version != 0 {
				log::warn!(
					target: TARGET,
					"skipping v0 to v1 migration: executed on wrong storage version.\
            				Expected version 0, found {:?}",
					version,
				);
				return T::DbWeight::get().reads(1)
			}

			let mut weight = T::DbWeight::get().reads_writes(2, 1);

			let pair_statuses_storage_map_v0 = v0::PairStatuses::<T>::drain().collect::<Vec<_>>();
			weight.saturating_accrue(
				T::DbWeight::get().reads(pair_statuses_storage_map_v0.len() as u64),
			);
			for (pair, old_pair_status) in pair_statuses_storage_map_v0.into_iter() {
				let new_pair_status = match old_pair_status {
					v0::PairStatus::Trading(metadata) => PairStatus::Trading(PairMetadata {
						pair_account: metadata.pair_account,
						total_supply: metadata.total_supply,
						fee_rate: DEFAULT_FEE_RATE,
					}),
					v0::PairStatus::Bootstrap(params) => PairStatus::Bootstrap(params),
					v0::PairStatus::Disable => PairStatus::Disable,
				};
				PairStatuses::<T>::insert(pair, new_pair_status);
				weight.saturating_accrue(T::DbWeight::get().writes(1));
			}

			log::info!(target: TARGET, "Finished migration...");

			StorageVersion::new(1).put::<Pallet<T>>();
			weight.saturating_add(T::DbWeight::get().reads_writes(1, 2))
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(state: Vec<u8>) -> Result<(), &'static str> {
			let old_entries: u32 =
				Decode::decode(&mut &state[..]).expect("pre_upgrade provides a valid state; qed");
			let new_entries = PairStatuses::<T>::iter().count() as u32;
			if new_entries != old_entries {
				log::error!(
					target: TARGET,
					"migrated {} entries, expected {}",
					new_entries,
					old_entries
				);
			}
			assert_eq!(StorageVersion::get::<Pallet<T>>(), 1, "Must upgrade");
			Ok(())
		}
	}
}

#[cfg(test)]
#[cfg(feature = "try-runtime")]
mod test {
	use super::*;
	use crate::fee::mock::*;
	use frame_support::assert_ok;

	const DOT_ASSET_ID: AssetId = AssetId { chain_id: 200, asset_type: LOCAL, asset_index: 2 };
	const BTC_ASSET_ID: AssetId = AssetId { chain_id: 300, asset_type: RESERVED, asset_index: 3 };

	#[test]
	#[allow(deprecated)]
	fn migration_v0_to_v1_works() {
		new_test_ext().execute_with(|| {
			// assume that we are at v0
			StorageVersion::new(0).put::<Zenlink>();

			let pair = (DOT_ASSET_ID, BTC_ASSET_ID);
			v0::PairStatuses::<Test>::insert(
				pair,
				v0::PairStatus::Trading(v0::PairMetadata { pair_account: 1, total_supply: 100 }),
			);

			let state = v1::MigrateToV1::<Test>::pre_upgrade().unwrap();
			let _w = v1::MigrateToV1::<Test>::on_runtime_upgrade();
			assert_ok!(v1::MigrateToV1::<Test>::post_upgrade(state));

			assert_eq!(
				PairStatuses::<Test>::get(pair),
				PairStatus::Trading(PairMetadata {
					pair_account: 1,
					total_supply: 100,
					fee_rate: DEFAULT_FEE_RATE
				})
			);

			assert_eq!(StorageVersion::get::<Zenlink>(), 1);
		});
	}
}
