use crate::{mock::*, IssueFee};
use currency::Amount;
use frame_support::{assert_noop, assert_ok, dispatch::DispatchResultWithPostInfo};
use sp_runtime::{DispatchError, FixedPointNumber};

fn test_setter<F1, F2>(f: F1, get_storage_value: F2)
where
    F1: Fn(Origin, UnsignedFixedPoint) -> DispatchResultWithPostInfo,
    F2: Fn() -> UnsignedFixedPoint,
{
    run_test(|| {
        let large_value = UnsignedFixedPoint::checked_from_rational::<u128, u128>(101, 100).unwrap(); // 101%
        assert_noop!(f(RuntimeOrigin::root(), large_value), TestError::AboveMaxExpectedValue);

        let valid_value = UnsignedFixedPoint::checked_from_rational::<u128, u128>(100, 100).unwrap(); // 100%
        assert_noop!(f(RuntimeOrigin::signed(6), valid_value), DispatchError::BadOrigin);
        assert_ok!(f(RuntimeOrigin::root(), valid_value));
        assert_eq!(get_storage_value(), valid_value);
    })
}

#[test]
fn should_get_issue_fee() {
    run_test(|| {
        <IssueFee<Test>>::put(UnsignedFixedPoint::checked_from_rational(10, 100).unwrap());
        assert_ok!(
            Fee::get_issue_fee(&Amount::<Test>::new(100, Token(IBTC))),
            Amount::<Test>::new(10, Token(IBTC))
        );
    })
}

#[test]
fn should_set_issue_fee() {
    test_setter(Fee::set_issue_fee, Fee::issue_fee);
}

#[test]
fn should_set_issue_griefing_collateral() {
    test_setter(Fee::set_issue_griefing_collateral, Fee::issue_griefing_collateral);
}

#[test]
fn should_set_redeem_fee() {
    test_setter(Fee::set_redeem_fee, Fee::redeem_fee);
}

#[test]
fn should_set_premium_redeem_fee() {
    test_setter(Fee::set_premium_redeem_fee, Fee::premium_redeem_fee);
}

#[test]
fn should_set_punishment_fee() {
    test_setter(Fee::set_punishment_fee, Fee::punishment_fee);
}

#[test]
fn should_set_replace_griefing_collateral() {
    test_setter(Fee::set_replace_griefing_collateral, Fee::replace_griefing_collateral);
}
