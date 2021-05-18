use crate::{mock::*, *};
use frame_support::{assert_err, assert_ok};
use sp_arithmetic::{FixedPointNumber, FixedU128};

#[test]
fn test_calculate_for() {
    run_test(|| {
        let tests: Vec<(u128, FixedU128, u128)> = vec![
            (
                1 * 10u128.pow(8),                               // 1 BTC
                FixedU128::checked_from_rational(1, 2).unwrap(), // 50%
                50000000,
            ),
            (
                50000000,                                          // 0.5 BTC
                FixedU128::checked_from_rational(5, 100).unwrap(), // 5%
                2500000,
            ),
            (
                25000000,                                           // 0.25 BTC
                FixedU128::checked_from_rational(5, 1000).unwrap(), // 0.5%
                125000,
            ),
            (
                12500000,                                             // 0.125 BTC
                FixedU128::checked_from_rational(5, 100000).unwrap(), // 0.005%
                625,
            ),
            (
                1 * 10u128.pow(10),                               // 1 DOT
                FixedU128::checked_from_rational(1, 10).unwrap(), // 10%
                1000000000,
            ),
        ];

        for (amount, percent, expected) in tests {
            let actual = Fee::calculate_for(amount, percent).unwrap();
            assert_eq!(actual, expected);
        }
    })
}

#[test]
fn test_ensure_rationals_sum_to_one_fails() {
    run_test(|| {
        assert_err!(
            Fee::ensure_rationals_sum_to_one(vec![
                FixedU128::checked_from_rational(45, 100).unwrap(),
                FixedU128::checked_from_rational(3, 100).unwrap(),
                FixedU128::checked_from_integer(0).unwrap(),
                FixedU128::checked_from_integer(0).unwrap(),
            ]),
            TestError::InvalidRewardDist
        );
    })
}

#[test]
fn test_ensure_rationals_sum_to_one_succeeds() {
    run_test(|| {
        assert_ok!(Fee::ensure_rationals_sum_to_one(vec![
            FixedU128::checked_from_rational(77, 100).unwrap(),
            FixedU128::checked_from_rational(3, 100).unwrap(),
            FixedU128::checked_from_rational(15, 100).unwrap(),
            FixedU128::checked_from_rational(5, 100).unwrap(),
        ],),);
    })
}
