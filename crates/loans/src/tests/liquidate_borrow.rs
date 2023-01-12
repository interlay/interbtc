use crate::{
    mock::{
        new_test_ext, with_price, CurrencyConvert, Loans, RuntimeOrigin, Test, Tokens, _run_to_block, market_mock,
        new_test_ext_no_markets, ALICE, BOB, DEFAULT_WRAPPED_CURRENCY, LEND_KBTC, LEND_KSM,
    },
    tests::unit,
    Amount, Error, Market, MarketState,
};
use currency::CurrencyConversion;
use frame_support::{assert_noop, assert_ok, traits::fungibles::Inspect};
use mocktopus::mocking::Mockable;
use primitives::{
    Balance,
    CurrencyId::{self, Token},
    Rate, Ratio, DOT as DOT_CURRENCY, KBTC as KBTC_CURRENCY, KSM as KSM_CURRENCY,
};
use sp_runtime::FixedPointNumber;
use traits::LoansApi;

const DOT: CurrencyId = Token(DOT_CURRENCY);
const KSM: CurrencyId = Token(KSM_CURRENCY);
const KBTC: CurrencyId = Token(KBTC_CURRENCY);

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

        assert_noop!(
            Loans::liquidate_borrow(RuntimeOrigin::signed(BOB), ALICE, KSM, 10, DOT),
            Error::<Test>::DepositsAreNotCollateral
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
            Loans::liquidate_borrow(RuntimeOrigin::signed(BOB), ALICE, KSM, unit(50), KBTC),
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
            KBTC
        ));

        // KSM price = 2
        // incentive = repay KSM value * 1.1 = (50 * 2) * 1.1 = 110
        // Alice KBTC: cash - deposit = 1000 - 200 = 800
        // Alice KBTC collateral: deposit - incentive = 200 - 110 = 90
        // Alice KSM: cash + borrow = 1000 + 100 = 1100
        // Alice KSM borrow balance: origin borrow balance - liquidate amount = 100 - 50 = 50
        // Bob KSM: cash - deposit - repay = 1000 - 200 - 50 = 750
        // Bob KBTC collateral: incentive = 110-(110/1.1*0.03)=107
        assert_eq!(Tokens::balance(KBTC, &ALICE), unit(800),);
        assert_eq!(
            Loans::exchange_rate(KBTC).saturating_mul_int(Tokens::balance(Loans::lend_token_id(KBTC).unwrap(), &ALICE)),
            unit(90),
        );
        assert_eq!(Tokens::balance(KSM, &ALICE), unit(1100),);
        assert_eq!(Loans::account_borrows(KSM, ALICE).principal, unit(50));
        assert_eq!(Tokens::balance(KSM, &BOB), unit(750));
        assert_eq!(
            Loans::exchange_rate(KBTC).saturating_mul_int(Tokens::balance(Loans::lend_token_id(KBTC).unwrap(), &BOB)),
            unit(107),
        );
        // 3 dollar reserved in our incentive reward account
        let incentive_reward_account = Loans::incentive_reward_account_id().unwrap();
        println!("incentive reserve account:{:?}", incentive_reward_account.clone());
        assert_eq!(
            Loans::exchange_rate(KBTC).saturating_mul_int(Tokens::balance(
                Loans::lend_token_id(KBTC).unwrap(),
                &incentive_reward_account.clone()
            )),
            unit(3),
        );
        assert_eq!(Tokens::balance(KBTC, &ALICE), unit(800),);
        // reduce 2 dollar from incentive reserve to alice account
        assert_ok!(Loans::reduce_incentive_reserves(
            RuntimeOrigin::root(),
            ALICE,
            KBTC,
            unit(2),
        ));
        // still 1 dollar left in reserve account
        assert_eq!(
            Loans::exchange_rate(KBTC).saturating_mul_int(Tokens::balance(
                Loans::lend_token_id(KBTC).unwrap(),
                &incentive_reward_account
            )),
            unit(1),
        );
        // 2 dollar transfer to alice
        assert_eq!(Tokens::balance(KBTC, &ALICE), unit(800) + unit(2),);
    })
}

#[test]
fn withdrawing_incentive_reserve_accrues_interest() {
    new_test_ext().execute_with(|| {
        let incentive_reward_account = Loans::incentive_reward_account_id().unwrap();
        initial_setup();
        alice_borrows_100_ksm();
        assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(BOB), KSM));
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(BOB), KBTC, unit(100)));
        // adjust KSM price to make ALICE generate shortfall
        CurrencyConvert::convert.mock_safe(with_price(Some((KSM, 2.into()))));
        // BOB repay the KSM borrow balance and get DOT from ALICE
        assert_ok!(Loans::liquidate_borrow(
            RuntimeOrigin::signed(BOB),
            ALICE,
            KSM,
            unit(50),
            KBTC
        ));
        assert_eq!(
            Loans::exchange_rate(KBTC).saturating_mul_int(Tokens::balance(
                Loans::lend_token_id(KBTC).unwrap(),
                &incentive_reward_account
            )),
            unit(3),
        );

        _run_to_block(10000);

        // Can reduce more than 3 dollars because interest is accrued just before the reserve is withdrawn
        assert_ok!(Loans::reduce_incentive_reserves(
            RuntimeOrigin::root(),
            ALICE,
            KBTC,
            3000169788955,
        ));
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
fn repay_currency_auto_locking_works() {
    new_test_ext().execute_with(|| {
        initial_setup();
        assert_ok!(Loans::mint(RuntimeOrigin::signed(BOB), KBTC, 1));
        assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(BOB), KBTC));
        alice_borrows_100_ksm();
        CurrencyConvert::convert.mock_safe(with_price(Some((KSM, 2.into()))));
        let initial_borrower_locked_collateral =
            Amount::<Test>::new(Loans::account_deposits(LEND_KBTC, &ALICE), LEND_KBTC);
        let initial_liquidator_locked_collateral =
            Amount::<Test>::new(Loans::account_deposits(LEND_KBTC, &BOB), LEND_KBTC);
        assert_ok!(Loans::liquidate_borrow(
            RuntimeOrigin::signed(BOB),
            ALICE,
            KSM,
            unit(50),
            KBTC
        ),);
        let final_borrower_locked_collateral =
            Amount::<Test>::new(Loans::account_deposits(LEND_KBTC, &ALICE), LEND_KBTC);
        let final_liquidator_locked_collateral =
            Amount::<Test>::new(Loans::account_deposits(LEND_KBTC, &BOB), LEND_KBTC);
        let actual_liquidator_reward = final_liquidator_locked_collateral
            .checked_sub(&initial_liquidator_locked_collateral)
            .unwrap();
        let borrower_slash = initial_borrower_locked_collateral
            .checked_sub(&final_borrower_locked_collateral)
            .unwrap();
        let Market {
            liquidate_incentive,
            liquidate_incentive_reserved_factor,
            ..
        } = Loans::market(KSM).unwrap();
        let reserved_reward = liquidate_incentive_reserved_factor
            .mul_floor(borrower_slash.checked_div(&liquidate_incentive).unwrap().amount());
        // The amount received by the liquidator should be whatever is slashed
        // from the borrower minus the reserve's share.
        let expected_liquidator_reward = borrower_slash.amount().checked_sub(reserved_reward).unwrap();

        assert!(
            actual_liquidator_reward.amount().eq(&expected_liquidator_reward),
            "The entirety of the liquidator's reward should have been auto-locked as collateral"
        );
    })
}

#[test]
fn liquidated_transfer_reduces_locked_collateral() {
    new_test_ext().execute_with(|| {
        initial_setup();
        alice_borrows_100_ksm();
        CurrencyConvert::convert.mock_safe(with_price(Some((KSM, 2.into()))));
        let amount_to_liquidate = Amount::<Test>::new(unit(50), KSM);
        let initial_locked_collateral = Amount::<Test>::new(Loans::account_deposits(LEND_KBTC, &ALICE), LEND_KBTC);
        let initial_locked_underlying = Loans::recompute_underlying_amount(&initial_locked_collateral).unwrap();
        assert_ok!(Loans::liquidate_borrow(
            RuntimeOrigin::signed(BOB),
            ALICE,
            KSM,
            amount_to_liquidate.amount(),
            KBTC
        ),);
        let final_locked_collateral = Amount::<Test>::new(Loans::account_deposits(LEND_KBTC, &ALICE), LEND_KBTC);
        let final_locked_underlying = Loans::recompute_underlying_amount(&final_locked_collateral).unwrap();
        // The total liquidated KSM includes the market's liquidation incentive
        let liquidation_incentive = Loans::market(KSM).unwrap().liquidate_incentive;
        let liquidated_ksm = amount_to_liquidate
            .checked_fixed_point_mul(&liquidation_incentive)
            .unwrap();
        let liquidated_ksm_as_wrapped = liquidated_ksm.convert_to(DEFAULT_WRAPPED_CURRENCY).unwrap();
        // The borrower's locked collateral (as tracked by the `AccountDeposits` storage item) must have been decreased
        // by the liquidated amount.
        let borrower_collateral_difference_as_wrapped = initial_locked_underlying
            .checked_sub(&final_locked_underlying)
            .unwrap()
            .convert_to(DEFAULT_WRAPPED_CURRENCY)
            .unwrap();
        assert!(liquidated_ksm_as_wrapped
            .eq(&borrower_collateral_difference_as_wrapped)
            .unwrap());
    })
}

#[test]
fn close_factor_may_require_multiple_liquidations_to_clear_bad_debt() {
    new_test_ext_no_markets().execute_with(|| {
        fn conservative_mock_market(lend_token: CurrencyId) -> Market<Balance> {
            Market {
                collateral_factor: Ratio::from_percent(40),
                liquidation_threshold: Ratio::from_percent(45),
                ..market_mock(lend_token)
            }
        }
        Loans::add_market(RuntimeOrigin::root(), KSM, conservative_mock_market(LEND_KSM)).unwrap();
        Loans::activate_market(RuntimeOrigin::root(), KSM).unwrap();
        Loans::add_market(RuntimeOrigin::root(), KBTC, conservative_mock_market(LEND_KBTC)).unwrap();
        Loans::activate_market(RuntimeOrigin::root(), KBTC).unwrap();
        // Market setup:
        // secure ratio        = 40%
        // liquidation ratio   = 45%
        // liquidation premium = 110%
        // close factor        = 50%
        initial_setup();
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), KSM, unit(80)));
        // Step 1.
        // Collateral = 200 KBTC (worth 200 reference units)
        // Debt       =  80 KSM  (worth  80 reference units)
        // Collateral ratio = 40%
        CurrencyConvert::convert.mock_safe(with_price(Some((KSM, 2.into()))));

        // Step 2.
        // Collateral = 200 KBTC (worth 200 reference units)
        // Debt       =  80 KSM  (worth 160 reference units)
        // Collateral ratio = 80% (unhealthy)
        assert_noop!(
            Loans::liquidate_borrow(RuntimeOrigin::signed(BOB), ALICE, KSM, unit(41), KBTC),
            Error::<Test>::TooMuchRepay
        );
        assert_ok!(Loans::liquidate_borrow(
            RuntimeOrigin::signed(BOB),
            ALICE,
            KSM,
            unit(40),
            KBTC
        ),);
        // There is still shortfall in spite of liquidating the max available amount
        let shortfall = Loans::get_account_liquidation_threshold_liquidity(&ALICE)
            .unwrap()
            .shortfall();
        assert!(
            !shortfall.is_zero(),
            "Shortfall should be greater than zero, because the close factor limited the max liquidatable amount"
        );

        // Step 3.
        // Collateral = 112 KBTC (worth 112 reference units)
        // Debt       =  40 KSM  (worth 80 reference units)
        // Collateral ratio = 71% (unhealthy)
        assert_ok!(Loans::liquidate_borrow(
            RuntimeOrigin::signed(BOB),
            ALICE,
            KSM,
            unit(20),
            KBTC
        ),);

        // Step 4.
        // Collateral =  68 KBTC (worth 68 reference units)
        // Debt       =  20 KSM  (worth 40 reference units)
        // Collateral ratio = 59% (unhealthy)
        assert_ok!(Loans::liquidate_borrow(
            RuntimeOrigin::signed(BOB),
            ALICE,
            KSM,
            unit(10),
            KBTC
        ),);

        // Step 5.
        // Collateral =  24 KBTC (worth 24 reference units)
        // Debt       =  10 KSM  (worth 10 reference units)
        // Collateral ratio = 42% (healthy)
        // At this point the debt is healthy, cannot liquidate even one planck
        assert_noop!(
            Loans::liquidate_borrow(RuntimeOrigin::signed(BOB), ALICE, KSM, 1, KBTC),
            Error::<Test>::InsufficientShortfall
        );
        let shortfall_final = Loans::get_account_liquidation_threshold_liquidity(&ALICE)
            .unwrap()
            .shortfall();
        assert!(
            shortfall_final.is_zero(),
            "The borrower should have no shortfall after repeated max liquidations"
        );

        // The borrower can take on no additional debt because they exceeded the 40% secure ratio
        assert_noop!(
            Loans::borrow(RuntimeOrigin::signed(ALICE), KSM, 1),
            Error::<Test>::InsufficientLiquidity
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
    // Alice deposits 200 KBTC as collateral
    assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), KBTC, unit(200)));
    assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(ALICE), KBTC));
}
