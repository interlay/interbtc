use super::*;
use crate::{mock::*, tests::Loans, Error};
use frame_support::{assert_err, assert_ok};
use primitives::{
    CurrencyId::{ForeignAsset, Token},
    DOT, IBTC, KSM,
};
use sp_runtime::FixedPointNumber;

#[test]
fn exceeded_supply_cap() {
    new_test_ext().execute_with(|| {
        Tokens::set_balance(Origin::root(), ALICE, Token(DOT), million_unit(1001), 0).unwrap();
        let amount = million_unit(501);
        assert_ok!(Loans::mint(Origin::signed(ALICE), Token(DOT), amount));
        // Exceed upper bound.
        assert_err!(
            Loans::mint(Origin::signed(ALICE), Token(DOT), amount),
            Error::<Test>::SupplyCapacityExceeded
        );

        Loans::redeem(Origin::signed(ALICE), Token(DOT), amount).unwrap();
        // Here should work, cause we redeemed already.
        assert_ok!(Loans::mint(Origin::signed(ALICE), Token(DOT), amount));
    })
}

#[test]
fn repay_borrow_all_no_underflow() {
    new_test_ext().execute_with(|| {
        // Alice deposits 200 KSM as collateral
        assert_ok!(Loans::mint(Origin::signed(ALICE), Token(KSM), unit(200)));
        assert_ok!(Loans::collateral_asset(Origin::signed(ALICE), Token(KSM), true));

        // Alice borrow only 1/1e5 KSM which is hard to accrue total borrows interest in 100 seconds
        assert_ok!(Loans::borrow(Origin::signed(ALICE), Token(KSM), 10_u128.pow(7)));

        accrue_interest_per_block(Token(KSM), 100, 9);

        assert_eq!(Loans::current_borrow_balance(&ALICE, Token(KSM)), Ok(10000005));
        // FIXME since total_borrows is too small and we accrue internal on it every 100 seconds
        // accrue_interest fails every time
        // as you can see the current borrow balance is not equal to total_borrows anymore
        assert_eq!(Loans::total_borrows(Token(KSM)), 10000000);

        // Alice repay all borrow balance. total_borrows = total_borrows.saturating_sub(10000005) = 0.
        assert_ok!(Loans::repay_borrow_all(Origin::signed(ALICE), Token(KSM)));

        assert_eq!(Tokens::balance(Token(KSM), &ALICE), unit(800) - 5);

        assert_eq!(
            Loans::exchange_rate(Token(DOT))
                .saturating_mul_int(Loans::account_deposits(Token(KSM), ALICE).voucher_balance),
            unit(200)
        );

        let borrow_snapshot = Loans::account_borrows(Token(KSM), ALICE);
        assert_eq!(borrow_snapshot.principal, 0);
        assert_eq!(borrow_snapshot.borrow_index, Loans::borrow_index(Token(KSM)));
    })
}

#[test]
fn ensure_capacity_fails_when_market_not_existed() {
    new_test_ext().execute_with(|| {
        assert_err!(
            Loans::ensure_under_supply_cap(ForeignAsset(987997280), unit(100)),
            Error::<Test>::MarketDoesNotExist
        );
    });
}

#[test]
fn redeem_all_should_be_accurate() {
    new_test_ext().execute_with(|| {
        assert_ok!(Loans::mint(Origin::signed(ALICE), Token(KSM), unit(200)));
        assert_ok!(Loans::collateral_asset(Origin::signed(ALICE), Token(KSM), true));
        assert_ok!(Loans::borrow(Origin::signed(ALICE), Token(KSM), unit(50)));

        // let exchange_rate greater than 0.02
        accrue_interest_per_block(Token(KSM), 6, 2);
        assert_eq!(Loans::exchange_rate(Token(KSM)), Rate::from_inner(20000000036387000));

        assert_ok!(Loans::repay_borrow_all(Origin::signed(ALICE), Token(KSM)));
        // It failed with InsufficientLiquidity before #839
        assert_ok!(Loans::redeem_all(Origin::signed(ALICE), Token(KSM)));
    })
}

#[test]
fn prevent_the_exchange_rate_attack() {
    new_test_ext().execute_with(|| {
        // Initialize Eve's balance
        assert_ok!(<Tokens as Transfer<AccountId>>::transfer(
            Token(DOT),
            &ALICE,
            &EVE,
            unit(200),
            false
        ));
        // Eve deposits a small amount
        assert_ok!(Loans::mint(Origin::signed(EVE), Token(DOT), 1));
        // !!! Eve transfer a big amount to Loans::account_id
        assert_ok!(<Tokens as Transfer<AccountId>>::transfer(
            Token(DOT),
            &EVE,
            &Loans::account_id(),
            unit(100),
            false
        ));
        assert_eq!(Tokens::balance(Token(DOT), &EVE), 99999999999999);
        assert_eq!(Tokens::balance(Token(DOT), &Loans::account_id()), 100000000000001);
        assert_eq!(
            Loans::total_supply(Token(DOT)),
            1 * 50, // 1 / 0.02
        );
        TimestampPallet::set_timestamp(12000);
        // Eve can not let the exchange rate greater than 1
        assert!(Loans::accrue_interest(Token(DOT)).is_err());

        // Mock a BIG exchange_rate: 100000000000.02
        ExchangeRate::<Test>::insert(Token(DOT), Rate::saturating_from_rational(100000000000020u128, 20 * 50));
        // Bob can not deposit 0.1 DOT because the voucher_balance can not be 0.
        assert_noop!(
            Loans::mint(Origin::signed(BOB), Token(DOT), 100000000000),
            Error::<Test>::InvalidExchangeRate
        );
    })
}
