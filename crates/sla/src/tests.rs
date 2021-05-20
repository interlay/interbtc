use crate::{
    mock::*,
    sp_api_hidden_includes_decl_storage::hidden_include::{StorageMap, StorageValue},
    types::{RelayerEvent, VaultEvent},
    RelayerSla,
};

use frame_support::assert_ok;
use sp_arithmetic::{traits::Zero, FixedI128, FixedPointNumber};

pub const ALICE: AccountId = 1;
// pub const BOB: AccountId = 2;

#[test]
fn test_calculate_slashed_amount_best_sla() {
    run_test(|| {
        assert_eq!(
            Sla::_calculate_slashed_amount(
                FixedI128::checked_from_rational(100, 1).unwrap(),
                1_000_000_000,
                FixedI128::checked_from_rational(110, 100).unwrap(),
                FixedI128::checked_from_rational(130, 100).unwrap(),
            ),
            Ok(1_100_000_000),
        );
    })
}

#[test]
fn test_calculate_slashed_amount_worst_sla() {
    run_test(|| {
        assert_eq!(
            Sla::_calculate_slashed_amount(
                FixedI128::zero(),
                1_000_000_000,
                FixedI128::checked_from_rational(110, 100).unwrap(),
                FixedI128::checked_from_rational(130, 100).unwrap(),
            ),
            Ok(1_300_000_000),
        );
    })
}
#[test]
fn test_calculate_slashed_amount_mediocre_sla() {
    run_test(|| {
        assert_eq!(
            Sla::_calculate_slashed_amount(
                FixedI128::from(25),
                1_000_000_000,
                FixedI128::checked_from_rational(110, 100).unwrap(),
                FixedI128::checked_from_rational(130, 100).unwrap(),
            ),
            Ok(1_250_000_000),
        );
    })
}

#[test]
fn test_calculate_slashed_amount_big_stake() {
    run_test(|| {
        assert_eq!(
            Sla::_calculate_slashed_amount(
                FixedI128::from(100),
                u64::MAX as u128,
                FixedI128::checked_from_rational(100, 100).unwrap(),
                FixedI128::checked_from_rational(200000000000000u128, 100).unwrap(),
            ),
            Ok(u64::MAX as u128),
        );
    })
}

#[test]
fn test_event_update_vault_sla_succeeds() {
    run_test(|| {
        let amount = 100u128;
        crate::LifetimeIssued::set(amount.into());

        Sla::event_update_vault_sla(&ALICE, VaultEvent::ExecuteIssue(amount)).unwrap();
        assert_eq!(
            <crate::VaultSla<Test>>::get(ALICE),
            <crate::VaultExecuteIssueMaxSlaChange<Test>>::get()
        );
    })
}

#[test]
fn test_event_update_vault_sla_half_size_increase() {
    run_test(|| {
        let amount = 100u128;
        crate::LifetimeIssued::set(amount.into());

        Sla::event_update_vault_sla(&ALICE, VaultEvent::ExecuteIssue(amount)).unwrap();
        assert_eq!(
            <crate::VaultSla<Test>>::get(ALICE),
            <crate::VaultExecuteIssueMaxSlaChange<Test>>::get() / FixedI128::from(2)
        );
    })
}

#[test]
fn test_event_update_relayer_sla_succeeds() {
    run_test(|| {
        for i in 0..100 {
            Sla::event_update_relayer_sla(&ALICE, RelayerEvent::StoreBlock).unwrap();
            assert_eq!(<crate::RelayerSla<Test>>::get(ALICE), FixedI128::from(i + 1));
        }

        <crate::RelayerSla<Test>>::insert(ALICE, FixedI128::from(50));
        Sla::event_update_relayer_sla(&ALICE, RelayerEvent::StoreBlock).unwrap();
        assert_eq!(
            <crate::RelayerSla<Test>>::get(ALICE),
            FixedI128::from(50) + <crate::RelayerStoreBlock<Test>>::get(),
        );

        <crate::RelayerSla<Test>>::insert(ALICE, FixedI128::from(50));
        Sla::event_update_relayer_sla(&ALICE, RelayerEvent::TheftReport).unwrap();
        assert_eq!(
            <crate::RelayerSla<Test>>::get(ALICE),
            FixedI128::from(50) + <crate::RelayerTheftReport<Test>>::get(),
        );
    })
}

#[test]
fn test_event_update_relayer_sla_limits() {
    run_test(|| {
        // start at 99.5, add 1, result should be 100
        <RelayerSla<Test>>::insert(ALICE, FixedI128::checked_from_rational(9950, 100).unwrap());
        Sla::event_update_relayer_sla(&ALICE, RelayerEvent::StoreBlock).unwrap();
        assert_eq!(<RelayerSla<Test>>::get(ALICE), FixedI128::from(100));
    })
}

#[test]
fn test_deposit_sla_change() {
    run_test(|| {
        assert_ok!(Sla::_deposit_sla_change(3756), FixedI128::from(4));
        // 1.990400000000000000
        assert_ok!(
            Sla::_deposit_sla_change(1244),
            FixedI128::from_inner(1990400000000000000)
        );
        assert_ok!(Sla::_deposit_sla_change(6866), FixedI128::from(4));
        // 0.067136623027861696
        assert_ok!(Sla::_deposit_sla_change(50), FixedI128::from_inner(67136623027861696));
    })
}

#[test]
fn test_withdraw_sla_change() {
    run_test(|| {
        assert_ok!(Sla::_withdraw_sla_change(1000), FixedI128::from(-4));
        assert_ok!(Sla::_withdraw_sla_change(2000), FixedI128::from(-4));
        assert_ok!(Sla::_withdraw_sla_change(1500), FixedI128::from(-4));
        // -2.909090909090909092
        assert_ok!(
            Sla::_withdraw_sla_change(1000),
            FixedI128::from_inner(-2909090909090909092)
        );
    })
}
