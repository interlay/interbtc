use crate::mock::*;
use sp_arithmetic::{traits::*, FixedI128, FixedPointNumber};
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
