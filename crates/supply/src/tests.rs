/// Tests for Supply
use crate::mock::*;
use frame_support::{assert_err, assert_ok, traits::Currency};
use sp_arithmetic::ArithmeticError;

#[test]
fn should_inflate_supply_from_start_height() {
    run_test(|| {
        assert_ok!(Supply::begin_block(0));
        let mut start_height = 100;
        assert_eq!(Supply::start_height(), Some(start_height));
        assert_eq!(Supply::last_emission(), 0);

        for emission in [200_000, 204_000] {
            assert_ok!(Supply::begin_block(start_height));
            start_height += YEARS;
            assert_eq!(Supply::start_height(), Some(start_height));
            assert_eq!(Supply::last_emission(), emission);
        }
    })
}

#[test]
fn should_not_inflate_total_supply() {
    run_test(|| {
        Balances::make_free_balance_be(&Supply::account_id(), u128::MAX);

        let start_height = 100;
        assert_ok!(Supply::set_start_height_and_inflation(
            RuntimeOrigin::root(),
            start_height,
            UnsignedFixedPoint::checked_from_rational(110, 100).unwrap()
        ));
        assert_eq!(Supply::start_height(), Some(start_height));
        assert_eq!(Supply::last_emission(), 0);

        assert_err!(Supply::begin_block(start_height), ArithmeticError::Overflow);
        assert_eq!(Balances::total_issuance(), u128::MAX);
    })
}
