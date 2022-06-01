use crate::{mock::*, IssueFee, IssueGriefingCollateral, RedeemFee, RefundFee, PremiumRedeemFee, PunishmentFee, TheftFee, TheftFeeMax, ReplaceGriefingCollateral};
use currency::Amount;
use frame_support::{assert_ok, assert_noop};
use sp_runtime::{FixedPointNumber, DispatchError};

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
    run_test(|| {
        let fee = UnsignedFixedPoint::checked_from_rational(100, 100).unwrap(); // 1%
        assert_noop!(
            Fee::set_issue_fee(Origin::signed(6), fee),
            DispatchError::BadOrigin
        );
        assert_ok!(
            Fee::set_issue_fee(Origin::root(), fee)
        );
        assert_eq!(
            <IssueFee<Test>>::get(),
            fee
        );
    })
}

#[test]
fn should_set_issue_griefing_collateral() {
    run_test(|| {
        let griefing_collateral = UnsignedFixedPoint::checked_from_rational(100, 100).unwrap(); // 1%
        assert_noop!(
            Fee::set_issue_griefing_collateral(Origin::signed(6), griefing_collateral),
            DispatchError::BadOrigin
        );
        assert_ok!(
            Fee::set_issue_griefing_collateral(Origin::root(), griefing_collateral)
        );
        assert_eq!(
            <IssueGriefingCollateral<Test>>::get(),
            griefing_collateral
        );
    })
}

#[test]
fn should_set_redeem_fee() {
    run_test(|| {
        let fee = UnsignedFixedPoint::checked_from_rational(100, 100).unwrap(); // 1%    
        assert_noop!(
            Fee::set_redeem_fee(Origin::signed(6), fee),
            DispatchError::BadOrigin
        );
        assert_ok!(
            Fee::set_redeem_fee(Origin::root(), fee)
        );
        assert_eq!(
            <RedeemFee<Test>>::get(),
            fee
        );
    })
}

#[test]
fn should_set_refund_fee() {
    run_test(|| {
        let fee = UnsignedFixedPoint::checked_from_rational(100, 100).unwrap(); // 1%
        assert_noop!(
            Fee::set_refund_fee(Origin::signed(6), fee),
            DispatchError::BadOrigin
        );
        assert_ok!(
            Fee::set_refund_fee(Origin::root(), fee)
        );
        assert_eq!(
            <RefundFee<Test>>::get(),
            fee
        );
    })
}

#[test]
fn should_set_premium_redeem_fee() {
    run_test(|| {
        let fee = UnsignedFixedPoint::checked_from_rational(100, 100).unwrap(); // 1%
        assert_noop!(
            Fee::set_premium_redeem_fee(Origin::signed(6), fee),
            DispatchError::BadOrigin
        );
        assert_ok!(
            Fee::set_premium_redeem_fee(Origin::root(), fee)
        );
        assert_eq!(
            <PremiumRedeemFee<Test>>::get(),
            fee
        );
    })
}

#[test]
fn should_set_punishment_fee() {
    run_test(|| {
        let fee = UnsignedFixedPoint::checked_from_rational(100, 100).unwrap(); // 1%
        assert_noop!(
            Fee::set_punishment_fee(Origin::signed(6), fee),
            DispatchError::BadOrigin
        );
        assert_ok!(
            Fee::set_punishment_fee(Origin::root(), fee)
        );
        assert_eq!(
            <PunishmentFee<Test>>::get(),
            fee
        );
    })
}

#[test]
fn should_set_replace_griefing_collateral() {
    run_test(|| {
        let griefing_collateral = UnsignedFixedPoint::checked_from_rational(100, 100).unwrap(); // 1%
        assert_noop!(
            Fee::set_replace_griefing_collateral(Origin::signed(6), griefing_collateral),
            DispatchError::BadOrigin
        );
        assert_ok!(
            Fee::set_replace_griefing_collateral(Origin::root(), griefing_collateral)
        );
        assert_eq!(
            <ReplaceGriefingCollateral<Test>>::get(),
            griefing_collateral
        );
    })
}

#[test]
fn should_set_theft_fee() {
    run_test(|| {
        let fee = UnsignedFixedPoint::checked_from_rational(100, 100).unwrap(); // 1%
        assert_noop!(
            Fee::set_theft_fee(Origin::signed(6), fee),
            DispatchError::BadOrigin
        );
        assert_ok!(
            Fee::set_theft_fee(Origin::root(), fee)
        );
        assert_eq!(
            <TheftFee<Test>>::get(),
            fee
        );
    })
}

#[test]
fn should_set_theft_fee_max() {
    run_test(|| {
        let fee_max = 1;
        assert_noop!(
            Fee::set_theft_fee_max(Origin::signed(6), fee_max),
            DispatchError::BadOrigin
        );
        assert_ok!(
            Fee::set_theft_fee_max(Origin::root(), fee_max)
        );
        assert_eq!(
            <TheftFeeMax<Test>>::get(),
            fee_max
        );
    })
}
