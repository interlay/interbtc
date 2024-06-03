use super::*;
use frame_support::pallet_prelude::Weight;

pub mod v1 {
    use super::*;
    use frame_support::{dispatch::GetStorageVersion, traits::StorageVersion};
    pub fn migrate_from_v0_to_v1<T: Config>() -> Weight {
        let weight = T::DbWeight::get().reads(1);
        let current_storage_version = Pallet::<T>::current_storage_version();
        let _expected_storage_version = StorageVersion::new(0);
        if matches!(current_storage_version, _expected_storage_version) {
            // Fixme: insert latest btc block as well as the calculated chain work. But for testnet the
            // block header should be different.
            ChainWork::<T>::insert(
                H256Le::from_bytes_le(&[
                    177, 89, 206, 70, 83, 47, 12, 29, 30, 21, 192, 96, 38, 114, 155, 10, 5, 77, 59, 247, 14, 99, 150,
                    79, 228, 250, 72, 71, 124, 92, 197, 19,
                ]),
                U256::zero(),
            );
        }
        StorageVersion::new(1).put::<Pallet<T>>();
        weight
    }
}
