use crate::mock::*;
use frame_support::assert_ok;
use frame_system::RawOrigin;
use sp_core::H256;

#[test]
fn test_get_secure_id() {
    run_test(|| {
        frame_system::Pallet::<Test>::set_parent_hash(H256::zero());
        assert_eq!(
            Security::get_secure_id(&1),
            H256::from_slice(&[
                71, 121, 67, 63, 246, 65, 71, 242, 66, 184, 148, 234, 23, 56, 62, 52, 108, 82, 213, 33, 160, 200, 214,
                1, 13, 46, 37, 138, 95, 245, 117, 109
            ])
        );
    })
}

#[test]
fn test_get_increment_active_block_succeeds() {
    run_test(|| {
        let initial_active_block = Security::active_block_number();
        Security::increment_active_block();
        assert_eq!(Security::active_block_number(), initial_active_block + 1);
    })
}

#[test]
fn test_get_active_block_not_incremented_if_inactive() {
    run_test(|| {
        let initial_active_block = Security::active_block_number();

        // not updated if there is an error
        assert_ok!(Security::activate_counter(RawOrigin::Root.into(), false));
        Security::increment_active_block();
        assert_eq!(Security::active_block_number(), initial_active_block);

        assert_ok!(Security::activate_counter(RawOrigin::Root.into(), true));
        Security::increment_active_block();
        assert_eq!(Security::active_block_number(), initial_active_block + 1);
    })
}

mod spec_based_tests {
    use super::*;
    use sp_core::U256;

    #[test]
    fn test_generate_secure_id() {
        run_test(|| {
            let get_secure_id_with = |account, nonce: u32, parent| {
                crate::Nonce::<Test>::set(U256::from(nonce));

                frame_system::Pallet::<Test>::set_parent_hash(H256::from_slice(&[parent; 32]));
                Security::get_secure_id(&account)
            };

            let test_secure_id_with = |account, nonce: u32, parent| {
                let result1 = get_secure_id_with(account, nonce, parent);
                let result2 = get_secure_id_with(account, nonce, parent);
                // test that the result ONLY depend on account, nonce and parent
                assert_eq!(result1, result2);
                result1
            };

            let mut results = vec![];

            for i in 0..2 {
                for j in 0..2 {
                    for k in 0..2 {
                        let result = test_secure_id_with(i, j, k);
                        results.push(result);
                    }
                }
            }
            results.sort(); // required because dedup only remove duplicate _consecutive_ values
            results.dedup();

            // postcondition: MUST return the 256-bit hash of the account, nonce, and parent_hash
            // test that each combination of account, nonce, and parent_hash gives a unique result
            assert_eq!(results.len(), 8);

            // postcondition: Nonce MUST be incremented by one.
            let initial = crate::Nonce::<Test>::get();
            Security::get_secure_id(&1);
            assert_eq!(crate::Nonce::<Test>::get(), initial + 1);
        })
    }

    #[test]
    fn test_has_expired() {
        run_test(|| {
            let test_parachain_block_expired_postcondition = |opentime, period, active_block_count| {
                // postcondition: MUST return True if opentime + period < ActiveBlockCount, False otherwise.
                let expected = opentime + period < active_block_count;

                crate::ActiveBlockCount::<Test>::set(active_block_count);

                assert_eq!(expected, Security::parachain_block_expired(opentime, period).unwrap())
            };

            for i in 0..4 {
                for j in 0..4 {
                    for k in 1..3 {
                        // precondition: The ActiveBlockCount MUST be greater than 0.
                        test_parachain_block_expired_postcondition(i, j, k);
                    }
                }
            }
        })
    }
}
