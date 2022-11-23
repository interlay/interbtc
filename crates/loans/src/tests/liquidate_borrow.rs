use crate::{
    mock::{new_test_ext, with_price, CurrencyConvert, Loans, RuntimeOrigin, Test, Tokens, ALICE, BOB},
    tests::unit,
    Error, MarketState,
};
use currency::CurrencyConversion;
use frame_support::{assert_noop, assert_ok, traits::fungibles::Inspect};
use mocktopus::mocking::Mockable;
use primitives::{
    CurrencyId::{self, Token},
    Rate, DOT as DOT_CURRENCY, KBTC as KBTC_CURRENCY, KSM as KSM_CURRENCY,
};
use sp_runtime::FixedPointNumber;

const DOT: CurrencyId = Token(DOT_CURRENCY);
const KSM: CurrencyId = Token(KSM_CURRENCY);
const USDT: CurrencyId = Token(KBTC_CURRENCY);

#[test]
fn liquidate_borrow_allowed_works() {
    new_test_ext().execute_with(|| {
        // Borrower should have a positive shortfall
        let dot_market = Loans::market(DOT).unwrap();
        assert_noop!(
            Loans::liquidate_borrow_allowed(&ALICE, DOT, 100, &dot_market),
            Error::<Test>::InsufficientShortfall
        );
        initial_setup();
        alice_borrows_100_ksm();
        // Adjust KSM price to make shortfall
        CurrencyConvert::convert.mock_safe(with_price(Some((KSM, 2.into()))));
        let ksm_market = Loans::market(KSM).unwrap();
        // Here the balance sheet of Alice is:
        // Collateral   Loans
        // USDT $110    KSM $200
        assert_noop!(
            Loans::liquidate_borrow_allowed(&ALICE, KSM, unit(51), &ksm_market),
            Error::<Test>::TooMuchRepay
        );
        assert_ok!(Loans::liquidate_borrow_allowed(&ALICE, KSM, unit(50), &ksm_market));
    })
}

#[test]
fn deposit_of_borrower_must_be_collateral() {
    new_test_ext().execute_with(|| {
        initial_setup();
        alice_borrows_100_ksm();
        // Adjust KSM price to make shortfall
        CurrencyConvert::convert.mock_safe(with_price(Some((KSM, 2.into()))));
        let market = Loans::market(KSM).unwrap();
        assert_noop!(
            Loans::liquidate_borrow_allowed(&ALICE, KSM, unit(51), &market),
            Error::<Test>::TooMuchRepay
        );

        // Previously (in Parallel's original implementation), this extrinsic call used to
        // return a `DepositsAreNotCollateral` error.
        // However, because the collateral "toggle" has been removed, the extrinsic looks
        // directly inside the `AccountDeposits` map, which no longer represents lend_token holdings
        // but rather lend_tokens that have been locked as collateral.
        // Since no KSM lend_tokens have been locked as collateral in this test, there will be zero
        // collateral available for paying the liquidator, thus producing the error below.
        assert_noop!(
            Loans::liquidate_borrow(RuntimeOrigin::signed(BOB), ALICE, KSM, 10, DOT),
            Error::<Test>::InsufficientCollateral
        );
    })
}

#[test]
fn collateral_value_must_be_greater_than_liquidation_value() {
    new_test_ext().execute_with(|| {
        initial_setup();
        alice_borrows_100_ksm();
        CurrencyConvert::convert.mock_safe(with_price(Some((KSM, Rate::from_float(2000.0)))));
        Loans::mutate_market(KSM, |market| {
            market.liquidate_incentive = Rate::from_float(200.0);
            market.clone()
        })
        .unwrap();
        assert_noop!(
            Loans::liquidate_borrow(RuntimeOrigin::signed(BOB), ALICE, KSM, unit(50), USDT),
            Error::<Test>::InsufficientCollateral
        );
    })
}

#[test]
fn full_workflow_works_as_expected() {
    new_test_ext().execute_with(|| {
        initial_setup();
        alice_borrows_100_ksm();
        // adjust KSM price to make ALICE generate shortfall
        CurrencyConvert::convert.mock_safe(with_price(Some((KSM, 2.into()))));
        // BOB repay the KSM borrow balance and get DOT from ALICE
        assert_ok!(Loans::liquidate_borrow(
            RuntimeOrigin::signed(BOB),
            ALICE,
            KSM,
            unit(50),
            USDT
        ));

        // KSM price = 2
        // incentive = repay KSM value * 1.1 = (50 * 2) * 1.1 = 110
        // Alice USDT: cash - deposit = 1000 - 200 = 800
        // Alice USDT collateral: deposit - incentive = 200 - 110 = 90
        // Alice KSM: cash + borrow = 1000 + 100 = 1100
        // Alice KSM borrow balance: origin borrow balance - liquidate amount = 100 - 50 = 50
        // Bob KSM: cash - deposit - repay = 1000 - 200 - 50 = 750
        // Bob DOT collateral: incentive = 110-(110/1.1*0.03)=107
        assert_eq!(Tokens::balance(USDT, &ALICE), unit(800),);
        assert_eq!(
            Loans::exchange_rate(USDT).saturating_mul_int(Tokens::balance(Loans::lend_token_id(USDT).unwrap(), &ALICE)),
            unit(90),
        );
        assert_eq!(Tokens::balance(KSM, &ALICE), unit(1100),);
        assert_eq!(Loans::account_borrows(KSM, ALICE).principal, unit(50));
        assert_eq!(Tokens::balance(KSM, &BOB), unit(750));
        assert_eq!(
            Loans::exchange_rate(USDT).saturating_mul_int(Tokens::balance(Loans::lend_token_id(USDT).unwrap(), &BOB)),
            unit(107),
        );
        // 3 dollar reserved in our incentive reward account
        let incentive_reward_account = Loans::incentive_reward_account_id().unwrap();
        println!("incentive reserve account:{:?}", incentive_reward_account.clone());
        assert_eq!(
            Loans::exchange_rate(USDT).saturating_mul_int(Tokens::balance(
                Loans::lend_token_id(USDT).unwrap(),
                &incentive_reward_account.clone()
            )),
            unit(3),
        );
        assert_eq!(Tokens::balance(USDT, &ALICE), unit(800),);
        // reduce 2 dollar from incentive reserve to alice account
        assert_ok!(Loans::reduce_incentive_reserves(
            RuntimeOrigin::root(),
            ALICE,
            USDT,
            unit(2),
        ));
        // still 1 dollar left in reserve account
        assert_eq!(
            Loans::exchange_rate(USDT).saturating_mul_int(Tokens::balance(
                Loans::lend_token_id(USDT).unwrap(),
                &incentive_reward_account
            )),
            unit(1),
        );
        // 2 dollar transfer to alice
        assert_eq!(Tokens::balance(USDT, &ALICE), unit(800) + unit(2),);
    })
}

#[test]
fn liquidator_cannot_take_inactive_market_currency() {
    new_test_ext().execute_with(|| {
        initial_setup();
        alice_borrows_100_ksm();
        // Adjust KSM price to make shortfall
        CurrencyConvert::convert.mock_safe(with_price(Some((KSM, 2.into()))));
        assert_ok!(Loans::mutate_market(DOT, |stored_market| {
            stored_market.state = MarketState::Supervision;
            stored_market.clone()
        }));
        assert_noop!(
            Loans::liquidate_borrow(RuntimeOrigin::signed(BOB), ALICE, KSM, unit(50), DOT),
            Error::<Test>::MarketNotActivated
        );
    })
}

#[test]
fn liquidator_can_not_repay_more_than_the_close_factor_pct_multiplier() {
    new_test_ext().execute_with(|| {
        initial_setup();
        alice_borrows_100_ksm();
        CurrencyConvert::convert.mock_safe(with_price(Some((KSM, 20.into()))));
        assert_noop!(
            Loans::liquidate_borrow(RuntimeOrigin::signed(BOB), ALICE, KSM, unit(51), DOT),
            Error::<Test>::TooMuchRepay
        );
    })
}

#[test]
fn liquidator_must_not_be_borrower() {
    new_test_ext().execute_with(|| {
        initial_setup();
        assert_noop!(
            Loans::liquidate_borrow(RuntimeOrigin::signed(ALICE), ALICE, KSM, 1, DOT),
            Error::<Test>::LiquidatorIsBorrower
        );
    })
}

fn alice_borrows_100_ksm() {
    assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), KSM, unit(100)));
}

fn initial_setup() {
    // Bob deposits 200 KSM
    assert_ok!(Loans::mint(RuntimeOrigin::signed(BOB), KSM, unit(200)));
    // Alice deposits 200 DOT as collateral
    assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), USDT, unit(200)));
    assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(ALICE), USDT));
}
