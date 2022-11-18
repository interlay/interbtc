use crate::{
    mock::{
        market_mock, new_test_ext, AccountId, Loans, RuntimeOrigin, Test, Tokens, ALICE, DAVE, LEND_KBTC, LEND_KINT,
        LEND_KSM,
    },
    tests::unit,
    Error,
};
use frame_support::{assert_err, assert_noop, assert_ok, traits::tokens::fungibles::Inspect};
use orml_traits::{MultiCurrency, MultiReservableCurrency};
use primitives::{
    Balance,
    CurrencyId::{self, ForeignAsset, Token},
    KBTC as KBTC_CURRENCY, KINT as KINT_CURRENCY, KSM as KSM_CURRENCY,
};
use sp_runtime::{FixedPointNumber, TokenError};
use traits::LoansApi;

const KINT: CurrencyId = Token(KINT_CURRENCY);
const KSM: CurrencyId = Token(KSM_CURRENCY);
const KBTC: CurrencyId = Token(KBTC_CURRENCY);

pub fn free_balance(currency_id: CurrencyId, account_id: &AccountId) -> Balance {
    <Tokens as MultiCurrency<<Test as frame_system::Config>::AccountId>>::free_balance(currency_id, account_id)
}

pub fn reserved_balance(currency_id: CurrencyId, account_id: &AccountId) -> Balance {
    <Tokens as MultiReservableCurrency<<Test as frame_system::Config>::AccountId>>::reserved_balance(
        currency_id,
        account_id,
    )
}

#[test]
fn trait_inspect_methods_works() {
    new_test_ext().execute_with(|| {
        // No Deposits can't not withdraw
        assert_err!(
            Loans::can_withdraw(LEND_KINT, &DAVE, 100).into_result(),
            TokenError::NoFunds
        );
        assert_eq!(Loans::total_issuance(LEND_KINT), 0);
        assert_eq!(Loans::total_issuance(LEND_KSM), 0);

        let minimum_balance = Loans::minimum_balance(LEND_KINT);
        assert_eq!(minimum_balance, 0);

        assert_eq!(Loans::balance(LEND_KINT, &DAVE), 0);

        // DAVE Deposit 100 KINT
        assert_ok!(Loans::mint(RuntimeOrigin::signed(DAVE), KINT, unit(100)));
        assert_eq!(Tokens::balance(LEND_KINT, &DAVE), unit(100) * 50);
        assert_eq!(Tokens::total_issuance(LEND_KINT), unit(100) * 50);
        // Check entries from orml-tokens directly
        assert_eq!(free_balance(LEND_KINT, &DAVE), unit(100) * 50);
        assert_eq!(reserved_balance(LEND_KINT, &DAVE), 0);

        // No collateral deposited yet, therefore no reducible balance
        assert_eq!(Loans::reducible_balance(LEND_KINT, &DAVE, true), 0);

        assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(DAVE), KINT));
        assert_eq!(Loans::reducible_balance(LEND_KINT, &DAVE, true), unit(100) * 50);
        // Check entries from orml-tokens directly
        assert_eq!(free_balance(LEND_KINT, &DAVE), 0);
        assert_eq!(reserved_balance(LEND_KINT, &DAVE), unit(100) * 50);

        // Borrow 25 KINT will reduce 25 KINT liquidity for collateral_factor is 50%
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(DAVE), KINT, unit(25)));

        assert_eq!(
            Loans::exchange_rate(KINT)
                .saturating_mul_int(Loans::account_deposits(Loans::lend_token_id(KINT).unwrap(), DAVE)),
            unit(100)
        );

        // DAVE Deposit 100 KINT, Borrow 25 KINT
        // Liquidity KINT 25
        // Formula: lend_tokens = liquidity / price(1) / collateral(0.5) / exchange_rate(0.02)
        assert_eq!(Loans::reducible_balance(LEND_KINT, &DAVE, true), unit(25) * 2 * 50);

        // Multi-asset case, additional deposit KBTC
        // DAVE Deposit 100 KINT, 50 KBTC, Borrow 25 KINT
        // Liquidity KINT = 25, KBTC = 25
        // lend_tokens = dollar(25 + 25) / 1 / 0.5 / 0.02 = dollar(50) * 100
        assert_ok!(Loans::mint(RuntimeOrigin::signed(DAVE), KBTC, unit(50)));
        assert_eq!(Tokens::balance(LEND_KBTC, &DAVE), unit(50) * 50);
        // Check entries from orml-tokens directly
        assert_eq!(free_balance(LEND_KBTC, &DAVE), unit(50) * 50);
        assert_eq!(reserved_balance(LEND_KBTC, &DAVE), 0);

        // `reducible_balance()` checks how much collateral can be withdrawn from the amount deposited.
        // Since no collateral has been deposited yet, this value is zero.
        assert_eq!(Loans::reducible_balance(LEND_KBTC, &DAVE, true), 0);
        // enable KBTC collateral
        assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(DAVE), KBTC));
        assert_eq!(Loans::reducible_balance(LEND_KINT, &DAVE, true), unit(25 + 25) * 2 * 50);
        assert_eq!(Loans::reducible_balance(LEND_KBTC, &DAVE, true), unit(50) * 50);
        // Check entries from orml-tokens directly
        assert_eq!(free_balance(LEND_KBTC, &DAVE), 0);
        assert_eq!(reserved_balance(LEND_KBTC, &DAVE), unit(50) * 50);

        assert_ok!(Loans::borrow(RuntimeOrigin::signed(DAVE), KINT, unit(50)));
        assert_eq!(Loans::reducible_balance(LEND_KINT, &DAVE, true), 0);
        assert_eq!(Loans::reducible_balance(LEND_KBTC, &DAVE, true), 0);

        assert_eq!(Loans::total_issuance(LEND_KINT), unit(100) * 50);
        assert_ok!(Loans::can_deposit(LEND_KINT, &DAVE, 100, true).into_result());
        assert_ok!(Loans::can_withdraw(LEND_KINT, &DAVE, 1000).into_result());
    })
}

#[test]
fn lend_token_unique_works() {
    new_test_ext().execute_with(|| {
        // lend_token_id already exists in `UnderlyingAssetId`
        assert_noop!(
            Loans::add_market(RuntimeOrigin::root(), ForeignAsset(1000000), market_mock(LEND_KINT)),
            Error::<Test>::InvalidLendTokenId
        );

        // lend_token_id cannot as the same as the asset id in `Markets`
        assert_noop!(
            Loans::add_market(RuntimeOrigin::root(), ForeignAsset(1000000), market_mock(KSM)),
            Error::<Test>::InvalidLendTokenId
        );
    })
}

#[test]
fn transfer_lend_token_works() {
    new_test_ext().execute_with(|| {
        // DAVE Deposit 100 KINT
        assert_ok!(Loans::mint(RuntimeOrigin::signed(DAVE), KINT, unit(100)));

        // DAVE KINT collateral: deposit = 100
        // KINT: cash - deposit = 1000 - 100 = 900
        assert_eq!(
            Loans::exchange_rate(KINT).saturating_mul_int(Tokens::balance(Loans::lend_token_id(KINT).unwrap(), &DAVE)),
            unit(100)
        );

        // ALICE KINT collateral: deposit = 0
        assert_eq!(
            Loans::exchange_rate(KINT).saturating_mul_int(Tokens::balance(Loans::lend_token_id(KINT).unwrap(), &ALICE)),
            unit(0)
        );

        // Transfer lend_tokens from DAVE to ALICE
        Loans::transfer(LEND_KINT, &DAVE, &ALICE, unit(50) * 50, true).unwrap();
        // Loans::transfer_lend_tokens(RuntimeOrigin::signed(DAVE), ALICE, KINT, dollar(50) * 50).unwrap();

        // DAVE KINT collateral: deposit = 50
        assert_eq!(
            Loans::exchange_rate(KINT).saturating_mul_int(Tokens::balance(Loans::lend_token_id(KINT).unwrap(), &DAVE)),
            unit(50)
        );
        // DAVE Redeem 51 KINT should cause InsufficientDeposit
        assert_noop!(
            Loans::redeem_allowed(KINT, &DAVE, unit(51) * 50),
            Error::<Test>::InsufficientDeposit
        );

        // ALICE KINT collateral: deposit = 50
        assert_eq!(
            Loans::exchange_rate(KINT).saturating_mul_int(Tokens::balance(Loans::lend_token_id(KINT).unwrap(), &ALICE)),
            unit(50)
        );
        // ALICE Redeem 50 KINT should be succeeded
        assert_ok!(Loans::redeem_allowed(KINT, &ALICE, unit(50) * 50));
    })
}

#[test]
fn transfer_lend_tokens_under_collateral_does_not_work() {
    new_test_ext().execute_with(|| {
        // DAVE Deposit 100 KINT
        assert_ok!(Loans::mint(RuntimeOrigin::signed(DAVE), KINT, unit(100)));
        // Check entries from orml-tokens directly
        assert_eq!(free_balance(LEND_KINT, &DAVE), unit(100) * 50);
        assert_eq!(reserved_balance(LEND_KINT, &DAVE), 0);

        assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(DAVE), KINT));
        // Check entries from orml-tokens directly
        assert_eq!(free_balance(LEND_KINT, &DAVE), 0);
        assert_eq!(reserved_balance(LEND_KINT, &DAVE), unit(100) * 50);

        // Borrow 50 KINT will reduce 50 KINT liquidity for collateral_factor is 50%
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(DAVE), KINT, unit(50)));
        // Repay 40 KINT
        assert_ok!(Loans::repay_borrow(RuntimeOrigin::signed(DAVE), KINT, unit(40)));

        // Allowed to redeem 20 lend_tokens
        assert_ok!(Loans::redeem_allowed(KINT, &DAVE, unit(20) * 50,));
        // Not allowed to transfer the same 20 lend_tokens because they are locked
        assert_noop!(
            Loans::transfer(LEND_KINT, &DAVE, &ALICE, unit(20) * 50, true),
            Error::<Test>::InsufficientCollateral
        );
        // First, withdraw some tokens. Note that directly withdrawing part of the locked
        // lend_tokens is not possible through extrinsics.
        assert_ok!(Loans::do_withdraw_collateral(&DAVE, LEND_KINT, unit(20) * 50));
        // Check entries from orml-tokens directly
        assert_eq!(free_balance(LEND_KINT, &DAVE), unit(20) * 50);
        assert_eq!(reserved_balance(LEND_KINT, &DAVE), unit(80) * 50);
        // Then transfer them
        assert_ok!(Loans::transfer(LEND_KINT, &DAVE, &ALICE, unit(20) * 50, true),);
        // Check entries from orml-tokens directly
        assert_eq!(free_balance(LEND_KINT, &DAVE), 0);
        assert_eq!(reserved_balance(LEND_KINT, &DAVE), unit(80) * 50);
        assert_eq!(free_balance(LEND_KINT, &ALICE), unit(20) * 50);

        // DAVE Deposit KINT = 100 - 20 = 80
        // DAVE Borrow KINT = 0 + 50 - 40 = 10
        // DAVE liquidity KINT = 80 * 0.5 - 10 = 30
        assert_eq!(
            Loans::exchange_rate(KINT)
                .saturating_mul_int(Loans::account_deposits(Loans::lend_token_id(KINT).unwrap(), DAVE)),
            unit(80)
        );
        // DAVE Borrow 31 KINT should cause InsufficientLiquidity
        assert_noop!(
            Loans::borrow(RuntimeOrigin::signed(DAVE), KINT, unit(31)),
            Error::<Test>::InsufficientLiquidity
        );
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(DAVE), KINT, unit(30)));

        // Assert ALICE Supply KINT 20
        assert_eq!(
            Loans::exchange_rate(KINT).saturating_mul_int(Tokens::balance(Loans::lend_token_id(KINT).unwrap(), &ALICE)),
            unit(20)
        );
        // ALICE Redeem 20 KINT should be succeeded
        // Also means that transfer lend_token succeed
        assert_ok!(Loans::redeem_allowed(KINT, &ALICE, unit(20) * 50,));
    })
}
