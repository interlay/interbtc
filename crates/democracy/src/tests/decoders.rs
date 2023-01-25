//! The for various partial storage decoders

use super::*;
use frame_support::{
    storage::{migration, unhashed},
    BoundedVec,
};

#[test]
fn test_decode_compact_u32_at() {
    new_test_ext().execute_with(|| {
        let v = codec::Compact(u64::MAX);
        migration::put_storage_value(b"test", b"", &[], v);
        assert_eq!(decode_compact_u32_at(b"test"), None);

        for v in vec![0, 10, u32::MAX] {
            let compact_v = codec::Compact(v);
            unhashed::put(b"test", &compact_v);
            assert_eq!(decode_compact_u32_at(b"test"), Some(v));
        }

        unhashed::kill(b"test");
        assert_eq!(decode_compact_u32_at(b"test"), None);
    })
}

#[test]
fn len_of_deposit_of() {
    new_test_ext().execute_with(|| {
        for l in vec![0, 1, 200, 1000] {
            let value: (BoundedVec<u64, _>, u64) = (
                (0..l)
                    .map(|_| Default::default())
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap(),
                3u64,
            );
            DepositOf::<Test>::insert(2, value);
            assert_eq!(Democracy::len_of_deposit_of(2), Some(l));
        }

        DepositOf::<Test>::remove(2);
        assert_eq!(Democracy::len_of_deposit_of(2), None);
    })
}

#[test]
fn pre_image() {
    new_test_ext().execute_with(|| {
        let key = Default::default();
        let missing = PreimageStatus::Missing(0);
        Preimages::<Test>::insert(key, missing);
        assert_noop!(Democracy::pre_image_data_len(key), Error::<Test>::PreimageMissing);
        assert_eq!(Democracy::check_pre_image_is_missing(key), Ok(()));

        Preimages::<Test>::remove(key);
        assert_noop!(Democracy::pre_image_data_len(key), Error::<Test>::PreimageMissing);
        assert_noop!(Democracy::check_pre_image_is_missing(key), Error::<Test>::NotImminent);

        for l in vec![0, 10, 100, 1000u32] {
            let available = PreimageStatus::Available {
                data: (0..l).map(|i| i as u8).collect(),
                provider: 0,
                deposit: 0,
                since: 0,
                expiry: None,
            };

            Preimages::<Test>::insert(key, available);
            assert_eq!(Democracy::pre_image_data_len(key), Ok(l));
            assert_noop!(
                Democracy::check_pre_image_is_missing(key),
                Error::<Test>::DuplicatePreimage
            );
        }
    })
}
