use crate::{mock::*, IssueFee};
use currency::Amount;
use frame_support::assert_ok;
use sp_runtime::FixedPointNumber;

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
