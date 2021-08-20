use crate::mock::*;
use frame_support::{assert_err, assert_ok};
use sp_arithmetic::{FixedPointNumber, FixedU128};

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
