use crate::ext;
use crate::mock::*;
use crate::sp_api_hidden_includes_decl_storage::hidden_include::StorageMap;
use crate::sp_api_hidden_includes_decl_storage::hidden_include::StorageValue;
use crate::{types::RelayerEvent, RelayerSla, TotalRelayerScore};

use mocktopus::mocking::*;
use sp_arithmetic::{traits::*, FixedI128, FixedPointNumber};

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;

pub const ALICE_STAKE: u64 = 1_000_000;
pub const BOB_STAKE: u64 = 4_000_000;

#[test]
fn test_calculate_slashed_amount_best_sla() {
    run_test(|| {
        assert_eq!(
            Sla::_calculate_slashed_amount(
                FixedI128::checked_from_rational(100, 1).unwrap(),
                Sla::u128_to_dot(1_000_000_000).unwrap(),
                FixedI128::checked_from_rational(110, 100).unwrap(),
                FixedI128::checked_from_rational(130, 100).unwrap(),
            ),
            Ok(Sla::u128_to_dot(1_100_000_000).unwrap()),
        );
    })
}

#[test]
fn test_calculate_slashed_amount_worst_sla() {
    run_test(|| {
        assert_eq!(
            Sla::_calculate_slashed_amount(
                FixedI128::zero(),
                Sla::u128_to_dot(1_000_000_000).unwrap(),
                FixedI128::checked_from_rational(110, 100).unwrap(),
                FixedI128::checked_from_rational(130, 100).unwrap(),
            ),
            Ok(Sla::u128_to_dot(1_300_000_000).unwrap()),
        );
    })
}
#[test]
fn test_calculate_slashed_amount_mediocre_sla() {
    run_test(|| {
        assert_eq!(
            Sla::_calculate_slashed_amount(
                FixedI128::from(25),
                Sla::u128_to_dot(1_000_000_000).unwrap(),
                FixedI128::checked_from_rational(110, 100).unwrap(),
                FixedI128::checked_from_rational(130, 100).unwrap(),
            ),
            Ok(Sla::u128_to_dot(1_250_000_000).unwrap()),
        );
    })
}

#[test]
fn test_calculate_slashed_amount_big_stake() {
    run_test(|| {
        assert_eq!(
            Sla::_calculate_slashed_amount(
                FixedI128::from(100),
                Sla::u128_to_dot(u64::MAX as u128).unwrap(),
                FixedI128::checked_from_rational(100, 100).unwrap(),
                FixedI128::checked_from_rational(200000000000000u128, 100).unwrap(),
            ),
            Ok(Sla::u128_to_dot(u64::MAX as u128).unwrap()),
        );
    })
}

#[test]
fn test_event_update_relayer_sla_succeeds() {
    run_test(|| {
        ext::collateral::get_collateral_from_account::<Test>
            .mock_safe(|_| MockResult::Return(ALICE_STAKE.into()));

        for i in 0..100 {
            Sla::event_update_relayer_sla(ALICE, RelayerEvent::BlockSubmission).unwrap();
            assert_eq!(
                <crate::RelayerSla<Test>>::get(ALICE),
                FixedI128::from(i + 1)
            );
        }
    })
}

#[test]
fn test_event_update_relayer_sla_limits() {
    run_test(|| {
        ext::collateral::get_collateral_from_account::<Test>
            .mock_safe(|_| MockResult::Return(ALICE_STAKE.into()));

        // start at 5, add -100, result should be 0
        <RelayerSla<Test>>::insert(ALICE, FixedI128::from(5));
        Sla::event_update_relayer_sla(ALICE, RelayerEvent::FalseInvalidVoteOrReport).unwrap();
        assert_eq!(<RelayerSla<Test>>::get(ALICE), FixedI128::from(0));

        // start at 95, add 10, result should be 100
        <RelayerSla<Test>>::insert(ALICE, FixedI128::from(95));
        Sla::event_update_relayer_sla(ALICE, RelayerEvent::CorrectInvalidVoteOrReport).unwrap();
        assert_eq!(<RelayerSla<Test>>::get(ALICE), FixedI128::from(100));
    })
}

#[test]
fn test_event_update_relayer_total_sla_score() {
    run_test(|| {
        ext::collateral::get_collateral_from_account::<Test>.mock_safe(|x| {
            MockResult::Return(match x {
                ALICE => ALICE_STAKE.into(),
                BOB => BOB_STAKE.into(),
                _ => 0u64,
            })
        });

        // total should increase as alice's score increases
        for i in 0..100 {
            Sla::event_update_relayer_sla(ALICE, RelayerEvent::BlockSubmission).unwrap();
            assert_eq!(
                <TotalRelayerScore<Test>>::get(),
                FixedI128::from(ALICE_STAKE as i128 * (i + 1))
            );
        }

        // there is no change in alice' sla, so total score should not change
        Sla::event_update_relayer_sla(ALICE, RelayerEvent::BlockSubmission).unwrap();
        assert_eq!(
            <TotalRelayerScore<Test>>::get(),
            FixedI128::from(ALICE_STAKE as i128 * 100)
        );

        // if bob increases score, total _should_ increase
        Sla::event_update_relayer_sla(BOB, RelayerEvent::BlockSubmission).unwrap();
        assert_eq!(
            <TotalRelayerScore<Test>>::get(),
            FixedI128::from(ALICE_STAKE as i128 * 100 + BOB_STAKE as i128)
        );

        // decrease in alice' score -> total should decrease
        Sla::event_update_relayer_sla(ALICE, RelayerEvent::FalseNoDataVoteOrReport).unwrap();
        assert_eq!(
            <TotalRelayerScore<Test>>::get(),
            FixedI128::from(ALICE_STAKE as i128 * 90 + BOB_STAKE as i128)
        );

        // decrease in alice' score all the way to 0 -> total be equal to bob's part
        Sla::event_update_relayer_sla(ALICE, RelayerEvent::FalseInvalidVoteOrReport).unwrap();
        assert_eq!(
            <TotalRelayerScore<Test>>::get(),
            FixedI128::from(BOB_STAKE as i128)
        );
    })
}

#[test]
fn test_calculate_reward() {
    run_test(|| {
        ext::collateral::get_collateral_from_account::<Test>.mock_safe(|x| {
            MockResult::Return(match x {
                ALICE => ALICE_STAKE.into(),
                BOB => BOB_STAKE.into(),
                _ => 0u64,
            })
        });

        // total should increase as alice's score increases
        for i in 0..10 {
            Sla::event_update_relayer_sla(ALICE, RelayerEvent::BlockSubmission).unwrap();
        }
        for i in 0..10 {
            Sla::event_update_relayer_sla(BOB, RelayerEvent::BlockSubmission).unwrap();
        }

        // equal sla, but alice and bob have 1:4 staked collateral ratio
        assert_eq!(Sla::calculate_relayer_reward(ALICE, 1_000_000), Ok(200_000));

        for i in 0..30 {
            Sla::event_update_relayer_sla(ALICE, RelayerEvent::BlockSubmission).unwrap();
        }

        // alice and bob have 4:1 sla ratio, and 1:4 staked collateral ratio, so both get 50%
        assert_eq!(Sla::calculate_relayer_reward(ALICE, 1_000_000), Ok(500_000));
    })
}
