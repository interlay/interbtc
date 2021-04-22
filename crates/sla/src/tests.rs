use crate::{
    ext,
    mock::*,
    sp_api_hidden_includes_decl_storage::hidden_include::{StorageMap, StorageValue},
    types::{RelayerEvent, VaultEvent},
    RelayerSla, TotalRelayerScore,
};

use mocktopus::mocking::*;
use sp_arithmetic::{FixedI128, FixedPointNumber};

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;

pub const ALICE_STAKE: i128 = 1_000_000;
pub const BOB_STAKE: i128 = 4_000_000;

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
fn test_event_update_vault_sla_succeeds() {
    run_test(|| {
        let amount = 100u64;
        ext::vault_registry::get_backing_collateral::<Test>.mock_safe(|_| MockResult::Return(Ok(ALICE_STAKE as u64)));
        crate::LifetimeIssued::set(amount.into());

        Sla::event_update_vault_sla(&ALICE, VaultEvent::ExecutedIssue(amount)).unwrap();
        assert_eq!(
            <crate::VaultSla<Test>>::get(ALICE),
            <crate::VaultExecutedIssueMaxSlaChange<Test>>::get()
        );
    })
}

#[test]
fn test_event_update_vault_sla_half_size_increase() {
    run_test(|| {
        let amount = 100u64;
        ext::vault_registry::get_backing_collateral::<Test>.mock_safe(|_| MockResult::Return(Ok(ALICE_STAKE as u64)));
        crate::LifetimeIssued::set(amount.into());

        Sla::event_update_vault_sla(&ALICE, VaultEvent::ExecutedIssue(amount)).unwrap();
        assert_eq!(
            <crate::VaultSla<Test>>::get(ALICE),
            <crate::VaultExecutedIssueMaxSlaChange<Test>>::get() / FixedI128::from(2)
        );
    })
}

#[test]
fn test_event_update_relayer_sla_succeeds() {
    run_test(|| {
        Sla::get_relayer_stake.mock_safe(|_| MockResult::Return(FixedI128::from(ALICE_STAKE)));

        for i in 0..100 {
            Sla::event_update_relayer_sla(&ALICE, RelayerEvent::BlockSubmission).unwrap();
            assert_eq!(<crate::RelayerSla<Test>>::get(ALICE), FixedI128::from(i + 1));
        }

        <crate::RelayerSla<Test>>::insert(ALICE, FixedI128::from(50));
        Sla::event_update_relayer_sla(&ALICE, RelayerEvent::BlockSubmission).unwrap();
        assert_eq!(
            <crate::RelayerSla<Test>>::get(ALICE),
            FixedI128::from(50) + <crate::RelayerBlockSubmission<Test>>::get(),
        );

        <crate::RelayerSla<Test>>::insert(ALICE, FixedI128::from(50));
        Sla::event_update_relayer_sla(&ALICE, RelayerEvent::CorrectNoDataVoteOrReport).unwrap();
        assert_eq!(
            <crate::RelayerSla<Test>>::get(ALICE),
            FixedI128::from(50) + <crate::RelayerCorrectNoDataVoteOrReport<Test>>::get(),
        );

        <crate::RelayerSla<Test>>::insert(ALICE, FixedI128::from(50));
        Sla::event_update_relayer_sla(&ALICE, RelayerEvent::CorrectInvalidVoteOrReport).unwrap();
        assert_eq!(
            <crate::RelayerSla<Test>>::get(ALICE),
            FixedI128::from(50) + <crate::RelayerCorrectInvalidVoteOrReport<Test>>::get(),
        );

        <crate::RelayerSla<Test>>::insert(ALICE, FixedI128::from(50));
        Sla::event_update_relayer_sla(&ALICE, RelayerEvent::CorrectTheftReport).unwrap();
        assert_eq!(
            <crate::RelayerSla<Test>>::get(ALICE),
            FixedI128::from(50) + <crate::RelayerCorrectTheftReport<Test>>::get(),
        );

        <crate::RelayerSla<Test>>::insert(ALICE, FixedI128::from(50));
        Sla::event_update_relayer_sla(&ALICE, RelayerEvent::FalseNoDataVoteOrReport).unwrap();
        assert_eq!(
            <crate::RelayerSla<Test>>::get(ALICE),
            FixedI128::from(50) + <crate::RelayerFalseNoDataVoteOrReport<Test>>::get(),
        );

        <crate::RelayerSla<Test>>::insert(ALICE, FixedI128::from(50));
        Sla::event_update_relayer_sla(&ALICE, RelayerEvent::FalseInvalidVoteOrReport).unwrap();
        assert_eq!(<crate::RelayerSla<Test>>::get(ALICE), FixedI128::from(0));

        <crate::RelayerSla<Test>>::insert(ALICE, FixedI128::from(50));
        Sla::event_update_relayer_sla(&ALICE, RelayerEvent::IgnoredVote).unwrap();
        assert_eq!(
            <crate::RelayerSla<Test>>::get(ALICE),
            FixedI128::from(50) + <crate::RelayerIgnoredVote<Test>>::get(),
        );
    })
}

#[test]
fn test_event_update_relayer_sla_limits() {
    run_test(|| {
        Sla::get_relayer_stake.mock_safe(|_| MockResult::Return(FixedI128::from(ALICE_STAKE)));

        // start at 5, add -100, result should be 0
        <RelayerSla<Test>>::insert(ALICE, FixedI128::from(5));
        Sla::event_update_relayer_sla(&ALICE, RelayerEvent::FalseInvalidVoteOrReport).unwrap();
        assert_eq!(<RelayerSla<Test>>::get(ALICE), FixedI128::from(0));

        // start at 95, add 10, result should be 100
        <RelayerSla<Test>>::insert(ALICE, FixedI128::from(95));
        Sla::event_update_relayer_sla(&ALICE, RelayerEvent::CorrectInvalidVoteOrReport).unwrap();
        assert_eq!(<RelayerSla<Test>>::get(ALICE), FixedI128::from(100));
    })
}

#[test]
fn test_event_update_relayer_total_sla_score() {
    run_test(|| {
        Sla::get_relayer_stake.mock_safe(|x| {
            MockResult::Return(match x {
                &ALICE => FixedI128::from(ALICE_STAKE),
                &BOB => FixedI128::from(BOB_STAKE),
                _ => FixedI128::from(0),
            })
        });

        // total should increase as alice's score increases
        for i in 0..100 {
            Sla::event_update_relayer_sla(&ALICE, RelayerEvent::BlockSubmission).unwrap();
            assert_eq!(
                <TotalRelayerScore<Test>>::get(),
                FixedI128::from(ALICE_STAKE as i128 * (i + 1))
            );
        }

        // there is no change in alice' sla, so total score should not change
        Sla::event_update_relayer_sla(&ALICE, RelayerEvent::BlockSubmission).unwrap();
        assert_eq!(
            <TotalRelayerScore<Test>>::get(),
            FixedI128::from(ALICE_STAKE as i128 * 100)
        );

        // if bob increases score, total _should_ increase
        Sla::event_update_relayer_sla(&BOB, RelayerEvent::BlockSubmission).unwrap();
        assert_eq!(
            <TotalRelayerScore<Test>>::get(),
            FixedI128::from(ALICE_STAKE as i128 * 100 + BOB_STAKE as i128)
        );

        // decrease in alice' score -> total should decrease
        Sla::event_update_relayer_sla(&ALICE, RelayerEvent::FalseNoDataVoteOrReport).unwrap();
        assert_eq!(
            <TotalRelayerScore<Test>>::get(),
            FixedI128::from(ALICE_STAKE as i128 * 90 + BOB_STAKE as i128)
        );

        // decrease in alice' score all the way to 0 -> total be equal to bob's part
        Sla::event_update_relayer_sla(&ALICE, RelayerEvent::FalseInvalidVoteOrReport).unwrap();
        assert_eq!(<TotalRelayerScore<Test>>::get(), FixedI128::from(BOB_STAKE as i128));
    })
}

#[test]
fn test_calculate_reward() {
    run_test(|| {
        Sla::get_relayer_stake.mock_safe(|x| {
            MockResult::Return(match x {
                &ALICE => FixedI128::from(ALICE_STAKE),
                &BOB => FixedI128::from(BOB_STAKE),
                _ => FixedI128::from(0),
            })
        });

        // total should increase as alice's score increases
        for _ in 0..10 {
            Sla::event_update_relayer_sla(&ALICE, RelayerEvent::BlockSubmission).unwrap();
        }
        for _ in 0..10 {
            Sla::event_update_relayer_sla(&BOB, RelayerEvent::BlockSubmission).unwrap();
        }

        // equal sla, but alice and bob have 1:4 staked collateral ratio
        assert_eq!(Sla::_calculate_relayer_reward(&ALICE, 1_000_000), Ok(200_000));

        for _ in 0..30 {
            Sla::event_update_relayer_sla(&ALICE, RelayerEvent::BlockSubmission).unwrap();
        }

        // alice and bob have 4:1 sla ratio, and 1:4 staked collateral ratio, so both get 50%
        assert_eq!(Sla::_calculate_relayer_reward(&ALICE, 1_000_000), Ok(500_000));
    })
}
