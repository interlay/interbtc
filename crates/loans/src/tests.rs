// Copyright 2022 Interlay.
// This file is part of Interlay.

// Copyright 2021 Parallel Finance Developer.
// This file is part of Parallel Finance.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

mod edge_cases;
mod interest_rate;
mod lend_tokens;
mod liquidate_borrow;
mod market;

use currency::{Amount, CurrencyConversion};
use frame_support::{assert_noop, assert_ok};

use mocktopus::mocking::Mockable;
use sp_runtime::{
    traits::{CheckedDiv, One, Saturating},
    FixedU128, Permill,
};

use primitives::{
    CurrencyId::Token, DOT as DOT_CURRENCY, IBTC as IBTC_CURRENCY, INTR as INTR_CURRENCY, KBTC as KBTC_CURRENCY,
    KINT as KINT_CURRENCY, KSM as KSM_CURRENCY,
};

use crate::{
    mock::*,
    tests::lend_tokens::{free_balance, reserved_balance},
};

const DOT: CurrencyId = Token(DOT_CURRENCY);
const KSM: CurrencyId = Token(KSM_CURRENCY);
const KBTC: CurrencyId = Token(KBTC_CURRENCY);
const IBTC: CurrencyId = Token(IBTC_CURRENCY);
const KINT: CurrencyId = Token(KINT_CURRENCY);
const INTR: CurrencyId = Token(INTR_CURRENCY);

#[test]
fn init_minting_ok() {
    new_test_ext().execute_with(|| {
        assert_eq!(Tokens::balance(KSM, &ALICE), unit(1000, KSM));
        assert_eq!(Tokens::balance(DOT, &ALICE), unit(100000, DOT));
        assert_eq!(Tokens::balance(KBTC, &ALICE), unit(10000000, KBTC));
        assert_eq!(Tokens::balance(KSM, &BOB), unit(1000, KSM));
        assert_eq!(Tokens::balance(DOT, &BOB), unit(100000, DOT));
    });
}

#[test]
fn init_markets_ok() {
    new_test_ext().execute_with(|| {
        assert_eq!(Loans::market(KSM).unwrap().state, MarketState::Active);
        assert_eq!(Loans::market(DOT).unwrap().state, MarketState::Active);
        assert_eq!(Loans::market(KBTC).unwrap().state, MarketState::Active);
        assert_eq!(BorrowIndex::<Test>::get(KINT), Rate::one());
        assert_eq!(BorrowIndex::<Test>::get(KSM), Rate::one());
        assert_eq!(BorrowIndex::<Test>::get(DOT), Rate::one());
        assert_eq!(BorrowIndex::<Test>::get(KBTC), Rate::one());

        assert_eq!(ExchangeRate::<Test>::get(KINT), Rate::saturating_from_rational(2, 100));
        assert_eq!(ExchangeRate::<Test>::get(KSM), Rate::saturating_from_rational(2, 100));
        assert_eq!(ExchangeRate::<Test>::get(DOT), Rate::saturating_from_rational(2, 100));
        assert_eq!(ExchangeRate::<Test>::get(KBTC), Rate::saturating_from_rational(2, 100));
    });
}

#[test]
fn loans_native_token_works() {
    new_test_ext().execute_with(|| {
        assert_eq!(Tokens::balance(KINT, &DAVE), unit(1000, KINT));
        assert_eq!(Loans::market(KINT).unwrap().state, MarketState::Active);
        assert_eq!(BorrowIndex::<Test>::get(KINT), Rate::one());
        assert_eq!(ExchangeRate::<Test>::get(KINT), Rate::saturating_from_rational(2, 100));
        assert_ok!(Loans::mint(RuntimeOrigin::signed(DAVE), KINT, unit(1000, KINT)));

        // Redeem 1001 KINT should cause InsufficientDeposit
        assert_noop!(
            Loans::redeem_allowed(KINT, &DAVE, unit(50050, KINT)),
            Error::<Test>::InsufficientDeposit
        );
        // Redeem 1000 KINT is ok
        assert_ok!(Loans::redeem_allowed(KINT, &DAVE, unit(50000, KINT),));

        assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(DAVE), KINT));
        assert_eq!(Loans::free_lend_tokens(KINT, &DAVE).unwrap().is_zero(), true);

        // Borrow 500 KINT will reduce 500 KINT liquidity for collateral_factor is 50%
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(DAVE), KINT, unit(500, KINT)));
        // Repay 400 KINT
        assert_ok!(Loans::repay_borrow(RuntimeOrigin::signed(DAVE), KINT, unit(400, KINT)));

        // KINT collateral: deposit = 1000
        // KINT borrow balance: borrow - repay = 500 - 400 = 100
        // KINT: cash - deposit + borrow - repay = 1000 - 1000 + 500 - 400 = 100
        assert_eq!(
            Loans::exchange_rate(KINT)
                .saturating_mul_int(Loans::account_deposits(Loans::lend_token_id(KINT).unwrap(), DAVE)),
            unit(1000, KINT)
        );
        let borrow_snapshot = Loans::account_borrows(KINT, DAVE);
        assert_eq!(borrow_snapshot.principal, unit(100, KINT));
        assert_eq!(borrow_snapshot.borrow_index, Loans::borrow_index(KINT));
        assert_eq!(Tokens::balance(KINT, &DAVE), unit(100, KINT),);
    })
}

#[test]
fn mint_works() {
    new_test_ext().execute_with(|| {
        // Deposit 10000 DOT
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), DOT, unit(10000, DOT)));

        // DOT collateral: deposit = 10000
        // DOT: cash - deposit = 100000 - 10000 = 90000
        assert_eq!(
            Loans::exchange_rate(DOT).saturating_mul_int(Loans::free_lend_tokens(DOT, &ALICE).unwrap().amount()),
            unit(10000, DOT)
        );
        assert_eq!(Tokens::balance(DOT, &ALICE), unit(90000, DOT),);
        assert_eq!(Tokens::balance(DOT, &Loans::account_id()), unit(10000, DOT),);
    })
}

#[test]
fn mint_must_return_err_when_overflows_occur() {
    new_test_ext().execute_with(|| {
        Loans::force_update_market(
            RuntimeOrigin::root(),
            DOT,
            Market {
                supply_cap: u128::MAX,
                ..ACTIVE_MARKET_MOCK
            },
        )
        .unwrap();
        // MAX_DEPOSIT = u128::MAX * exchangeRate
        const OVERFLOW_DEPOSIT: u128 = u128::MAX / 50 + 1;

        // Verify token balance first
        assert_noop!(
            Loans::mint(RuntimeOrigin::signed(CHARLIE), DOT, OVERFLOW_DEPOSIT),
            ArithmeticError::Underflow
        );

        // Deposit OVERFLOW_DEPOSIT DOT for CHARLIE
        assert_ok!(Tokens::set_balance(
            RuntimeOrigin::root(),
            CHARLIE,
            DOT,
            OVERFLOW_DEPOSIT,
            0
        ));

        // Amount is too large, OVERFLOW_DEPOSIT / 0.0X == Overflow
        // Underflow is used here redeem could also be 0
        assert_noop!(
            Loans::mint(RuntimeOrigin::signed(CHARLIE), DOT, OVERFLOW_DEPOSIT),
            ArithmeticError::Underflow
        );

        // Assume a misconfiguration occurs
        MinExchangeRate::<Test>::put(Rate::zero());
        // There is no cash in the market. To compute how many lend_tokens this first deposit
        // should mint, the `MinExchangeRate` is used, which has been misconfigured and
        // set to zero (default value for the type). The extrinsic should fail.
        assert_noop!(
            Loans::mint(RuntimeOrigin::signed(CHARLIE), DOT, 100),
            ArithmeticError::Underflow
        );
    })
}

#[test]
fn supply_cap_below_current_volume() {
    new_test_ext().execute_with(|| {
        // Deposit 200 DOT as collateral
        assert_ok!(Loans::mint(RuntimeOrigin::signed(BOB), DOT, 200));
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), KSM, 200));
        assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(ALICE), KSM));
        assert_eq!(Loans::free_lend_tokens(KSM, &ALICE).unwrap().is_zero(), true);

        let new_supply_cap = 10;
        assert_ok!(Loans::force_update_market(
            RuntimeOrigin::root(),
            DOT,
            Market {
                supply_cap: new_supply_cap,
                ..ACTIVE_MARKET_MOCK
            },
        ));
        // Minting anything would exceed the cap
        assert_noop!(
            Loans::mint(RuntimeOrigin::signed(ALICE), DOT, 10),
            Error::<Test>::SupplyCapacityExceeded
        );
        // Can redeem, even if the resulting deposit is still
        // above the new cap
        assert_ok!(Loans::withdraw_all_collateral(RuntimeOrigin::signed(ALICE), KSM));
        assert_eq!(Loans::reserved_lend_tokens(KSM, &ALICE).unwrap().is_zero(), true);
        assert_ok!(Loans::redeem(RuntimeOrigin::signed(ALICE), KSM, 10));

        // Cannot mint back the amount that has just been redeemed
        assert_noop!(
            Loans::mint(RuntimeOrigin::signed(ALICE), DOT, 10),
            Error::<Test>::SupplyCapacityExceeded
        );

        let lend_tokens = Loans::free_lend_tokens(KSM, &ALICE).unwrap();

        // Redeem enough to be below the cap
        assert_ok!(Loans::redeem(
            RuntimeOrigin::signed(ALICE),
            KSM,
            Loans::recompute_underlying_amount(&lend_tokens).unwrap().amount() - (new_supply_cap / 2)
        ));

        // Can now supply
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), KSM, new_supply_cap / 2));
    })
}

#[test]
fn redeem_allowed_works() {
    new_test_ext().execute_with(|| {
        // Prepare: Bob Deposit 200 DOT
        assert_ok!(Loans::mint(RuntimeOrigin::signed(BOB), DOT, 200));

        // Deposit 200 KSM as collateral
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), KSM, 200));
        // Redeem 201 KSM should cause InsufficientDeposit
        assert_noop!(
            Loans::redeem_allowed(KSM, &ALICE, 10050),
            Error::<Test>::InsufficientDeposit
        );
        // Redeem 1 DOT should cause InsufficientDeposit
        assert_noop!(
            Loans::redeem_allowed(DOT, &ALICE, 50),
            Error::<Test>::InsufficientDeposit
        );
        // Redeem 200 KSM is ok
        assert_ok!(Loans::redeem_allowed(KSM, &ALICE, 10000));

        assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(ALICE), KSM));
        assert_eq!(Loans::free_lend_tokens(KSM, &ALICE).unwrap().is_zero(), true);
        // Borrow 50 DOT will reduce 100 KSM liquidity for collateral_factor is 50%
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), DOT, 50));
        // Redeeming is not allowed because the user has no `free` lend tokens
        assert_noop!(
            Loans::redeem_allowed(KSM, &ALICE, 5050),
            Error::<Test>::LockedTokensCannotBeRedeemed
        );
        // However, the redeem extrinsic withdraws collateral first, if the account has
        // enough liquidity.
        // The `withdraw` call in `redeem` should cause InsufficientLiquidity for 101 KSM
        assert_noop!(
            Loans::redeem(RuntimeOrigin::signed(ALICE), KSM, 101),
            Error::<Test>::InsufficientLiquidity
        );
        // Redeeming 100 KSM is ok because the withdrawal succeds (the user has enough backing liquidity)
        assert_ok!(Loans::redeem(RuntimeOrigin::signed(ALICE), KSM, 100));
    })
}

#[test]
fn redeem_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), DOT, unit(10000, DOT)));
        assert_ok!(Loans::redeem(RuntimeOrigin::signed(ALICE), DOT, unit(2000, DOT)));

        // DOT collateral: deposit - redeem = 10000 - 2000 = 8000
        // DOT: cash - deposit + redeem = 100000 - 10000 + 2000 = 92000
        assert_eq!(
            Loans::exchange_rate(DOT).saturating_mul_int(Tokens::balance(Loans::lend_token_id(DOT).unwrap(), &ALICE)),
            unit(8000, DOT)
        );
        assert_eq!(Tokens::balance(DOT, &ALICE), unit(92000, DOT),);
    })
}

#[test]
fn zero_amount_extrinsics_fail() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Loans::add_reward(RuntimeOrigin::signed(ALICE), unit(0, DOT)),
            Error::<Test>::InvalidAmount
        );
        assert_noop!(
            Loans::mint(RuntimeOrigin::signed(ALICE), DOT, unit(0, DOT)),
            Error::<Test>::InvalidAmount
        );
        assert_noop!(
            Loans::redeem(RuntimeOrigin::signed(ALICE), DOT, unit(0, DOT)),
            Error::<Test>::InvalidAmount
        );
        assert_noop!(
            Loans::redeem_all(RuntimeOrigin::signed(ALICE), DOT),
            Error::<Test>::InvalidAmount
        );
        assert_noop!(
            Loans::borrow(RuntimeOrigin::signed(ALICE), DOT, unit(0, DOT)),
            Error::<Test>::InvalidAmount
        );
        assert_noop!(
            Loans::repay_borrow(RuntimeOrigin::signed(ALICE), DOT, unit(0, DOT)),
            Error::<Test>::InvalidAmount
        );
        assert_noop!(
            Loans::repay_borrow_all(RuntimeOrigin::signed(ALICE), DOT),
            Error::<Test>::InvalidAmount
        );
        assert_noop!(
            Loans::deposit_all_collateral(RuntimeOrigin::signed(ALICE), DOT),
            Error::<Test>::DepositAllCollateralFailed
        );
        assert_noop!(
            Loans::withdraw_all_collateral(RuntimeOrigin::signed(ALICE), DOT),
            Error::<Test>::WithdrawAllCollateralFailed
        );
        assert_noop!(
            Loans::liquidate_borrow(RuntimeOrigin::signed(ALICE), BOB, DOT, unit(0, DOT), IBTC),
            Error::<Test>::InvalidAmount
        );
        assert_noop!(
            Loans::add_reserves(RuntimeOrigin::root(), ALICE, DOT, unit(0, DOT)),
            Error::<Test>::InvalidAmount
        );
        assert_noop!(
            Loans::reduce_reserves(RuntimeOrigin::root(), ALICE, DOT, unit(0, DOT)),
            Error::<Test>::InvalidAmount
        );
        assert_noop!(
            Loans::reduce_incentive_reserves(RuntimeOrigin::root(), ALICE, DOT, unit(0, DOT)),
            Error::<Test>::InvalidAmount
        );
    })
}

#[test]
fn redeem_fails_when_insufficient_liquidity() {
    new_test_ext().execute_with(|| {
        // Prepare: Bob Deposit 200 DOT
        assert_ok!(Loans::mint(RuntimeOrigin::signed(BOB), DOT, 200));

        // Deposit 200 KSM as collateral
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), KSM, 200));

        assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(ALICE), KSM));
        assert_eq!(Loans::free_lend_tokens(KSM, &ALICE).unwrap().is_zero(), true);
        // Borrow 50 DOT will reduce 100 KSM liquidity for collateral_factor is 50%
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), DOT, 50));

        assert_noop!(
            Loans::redeem(RuntimeOrigin::signed(BOB), DOT, 151),
            Error::<Test>::InsufficientCash
        );
    })
}

#[test]
fn redeem_fails_when_would_use_reserved_balanace() {
    new_test_ext().execute_with(|| {
        // Prepare: Bob Deposit 200 DOT
        assert_ok!(Loans::mint(RuntimeOrigin::signed(BOB), DOT, 200));

        // Deposit 200 KSM as collateral
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), KSM, 200));

        assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(ALICE), KSM));
        // Borrow 50 DOT will reduce 100 KSM liquidity for collateral_factor is 50%
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), DOT, 50));
        assert_ok!(Loans::add_reserves(RuntimeOrigin::root(), ALICE, DOT, 50));

        assert_noop!(
            Loans::redeem(RuntimeOrigin::signed(BOB), DOT, 151),
            Error::<Test>::InsufficientCash
        );
    })
}

#[test]
fn redeem_must_return_err_when_overflows_occur() {
    new_test_ext().execute_with(|| {
        // Amount is too large, max_value / 0.0X == Overflow
        // Underflow is used here redeem could also be 0
        assert_noop!(
            Loans::redeem(RuntimeOrigin::signed(ALICE), DOT, u128::MAX),
            ArithmeticError::Underflow,
        );

        // Assume a misconfiguration occurs
        MinExchangeRate::<Test>::put(Rate::zero());
        assert_noop!(
            Loans::redeem(RuntimeOrigin::signed(ALICE), DOT, 100),
            ArithmeticError::Underflow
        );
    })
}

#[test]
fn redeem_all_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), DOT, unit(100, DOT)));
        assert_ok!(Loans::redeem_all(RuntimeOrigin::signed(ALICE), DOT));
        assert_eq!(Loans::free_lend_tokens(DOT, &ALICE).unwrap().is_zero(), true);
        assert_eq!(Loans::reserved_lend_tokens(DOT, &ALICE).unwrap().is_zero(), true);

        // DOT: cash - deposit + redeem = 100000 - 100 + 100 = 100000
        // DOT collateral: deposit - redeem = 100 - 100 = 0
        assert_eq!(
            Loans::exchange_rate(DOT)
                .saturating_mul_int(Loans::account_deposits(Loans::lend_token_id(DOT).unwrap(), ALICE)),
            0,
        );
        assert_eq!(Tokens::balance(DOT, &ALICE), unit(100000, DOT),);
        assert!(!AccountDeposits::<Test>::contains_key(DOT, &ALICE));
        assert_eq!(Tokens::balance(LEND_DOT, &ALICE), 0);
    })
}

#[test]
fn borrow_allowed_works() {
    new_test_ext().execute_with(|| {
        // Deposit 200 DOT as collateral
        assert_ok!(Loans::mint(RuntimeOrigin::signed(BOB), DOT, 200));
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), KSM, 200));
        assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(ALICE), KSM));
        // Borrow 101 DOT should cause InsufficientLiquidity
        assert_noop!(
            Loans::borrow_allowed(DOT, &ALICE, 101),
            Error::<Test>::InsufficientLiquidity
        );
        // Borrow 100 DOT is ok
        assert_ok!(Loans::borrow_allowed(DOT, &ALICE, 100));

        // Set borrow limit to 10
        assert_ok!(Loans::force_update_market(
            RuntimeOrigin::root(),
            DOT,
            Market {
                borrow_cap: 10,
                ..ACTIVE_MARKET_MOCK
            },
        ));
        // Borrow 10 DOT is ok
        assert_ok!(Loans::borrow_allowed(DOT, &ALICE, 10));
        // Borrow 11 DOT should cause BorrowLimitExceeded
        assert_noop!(
            Loans::borrow_allowed(DOT, &ALICE, 11),
            Error::<Test>::BorrowCapacityExceeded
        );
    })
}

#[test]
fn borrow_cap_below_current_volume() {
    new_test_ext().execute_with(|| {
        // Deposit 200 DOT as collateral
        assert_ok!(Loans::mint(RuntimeOrigin::signed(BOB), DOT, 200));
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), KSM, 200));
        assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(ALICE), KSM));
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), DOT, 50));

        let new_borrow_cap = 10;
        assert_ok!(Loans::force_update_market(
            RuntimeOrigin::root(),
            DOT,
            Market {
                borrow_cap: new_borrow_cap,
                ..ACTIVE_MARKET_MOCK
            },
        ));
        // Borrowing anything would exceed the cap
        assert_noop!(
            Loans::borrow_allowed(DOT, &ALICE, 10),
            Error::<Test>::BorrowCapacityExceeded
        );
        // Can repay the borrow, even if the resulting loan is still
        // above the new cap
        assert_ok!(Loans::repay_borrow(RuntimeOrigin::signed(ALICE), DOT, 10));

        // Cannot borrow back the amount that has just been repaid
        assert_noop!(
            Loans::borrow_allowed(DOT, &ALICE, 10),
            Error::<Test>::BorrowCapacityExceeded
        );

        let outstanding = Loans::current_borrow_balance(&ALICE, DOT).unwrap();

        // Repay enough to be below the cap
        assert_ok!(Loans::repay_borrow(
            RuntimeOrigin::signed(ALICE),
            DOT,
            outstanding - (new_borrow_cap / 2)
        ));

        // Can now borrow
        assert_ok!(Loans::borrow_allowed(DOT, &ALICE, new_borrow_cap / 2),);
    })
}

#[test]
fn get_account_liquidity_works() {
    new_test_ext().execute_with(|| {
        Loans::mint(RuntimeOrigin::signed(ALICE), IBTC, unit(200, IBTC)).unwrap();
        Loans::deposit_all_collateral(RuntimeOrigin::signed(ALICE), IBTC).unwrap();

        assert_eq!(
            Loans::get_account_liquidity(&ALICE).unwrap().liquidity().amount(),
            unit(100, IBTC)
        );
    })
}

#[test]
fn get_account_liquidation_threshold_liquidity_works() {
    new_test_ext().execute_with(|| {
        Loans::mint(RuntimeOrigin::signed(BOB), DOT, unit(20000, DOT)).unwrap();
        Loans::mint(RuntimeOrigin::signed(BOB), KSM, unit(200, KSM)).unwrap();

        Loans::mint(RuntimeOrigin::signed(ALICE), IBTC, unit(2000000, IBTC)).unwrap();
        Loans::deposit_all_collateral(RuntimeOrigin::signed(ALICE), IBTC).unwrap();

        Loans::mint(RuntimeOrigin::signed(ALICE), KBTC, unit(2000000, KBTC)).unwrap();
        Loans::deposit_all_collateral(RuntimeOrigin::signed(ALICE), KBTC).unwrap();

        Loans::borrow(RuntimeOrigin::signed(ALICE), KSM, unit(100, KSM)).unwrap();
        Loans::borrow(RuntimeOrigin::signed(ALICE), DOT, unit(10000, DOT)).unwrap();

        assert_eq!(
            Loans::get_account_liquidation_threshold_liquidity(&ALICE)
                .unwrap()
                .liquidity()
                .amount(),
            unit(200000, IBTC)
        );

        CurrencyConvert::convert.mock_safe(with_price(Some((KSM, 2.into()))));
        let account_liquidity = Loans::get_account_liquidation_threshold_liquidity(&ALICE).unwrap();
        assert_eq!(account_liquidity.liquidity().amount(), unit(0, IBTC));
        assert_eq!(account_liquidity.shortfall().amount(), unit(800000, IBTC));
    })
}

#[test]
fn borrow_works() {
    new_test_ext().execute_with(|| {
        // Deposit 20000 DOT as collateral
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), DOT, unit(20000, DOT)));
        assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(ALICE), DOT));
        // Borrow 10000 DOT
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), DOT, unit(10000, DOT)));

        // DOT collateral: deposit = 20000
        // DOT borrow balance: borrow = 10000
        // DOT: cash - deposit + borrow = 100000 - 20000 + 10000 = 90000
        assert_eq!(
            Loans::exchange_rate(DOT)
                .saturating_mul_int(Loans::account_deposits(Loans::lend_token_id(DOT).unwrap(), ALICE)),
            unit(20000, DOT)
        );
        let borrow_snapshot = Loans::account_borrows(DOT, ALICE);
        assert_eq!(borrow_snapshot.principal, unit(1000000, IBTC));
        assert_eq!(borrow_snapshot.borrow_index, Loans::borrow_index(DOT));
        assert_eq!(Tokens::balance(DOT, &ALICE), unit(90000, DOT),);
    })
}

#[test]
fn repay_borrow_works() {
    new_test_ext().execute_with(|| {
        // Deposit 20000 DOT as collateral
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), DOT, unit(20000, DOT)));
        assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(ALICE), DOT));
        // Borrow 10000 DOT
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), DOT, unit(10000, DOT)));
        // Repay 3000 DOT
        assert_ok!(Loans::repay_borrow(RuntimeOrigin::signed(ALICE), DOT, unit(3000, DOT)));

        // DOT collateral: deposit = 20000
        // DOT borrow balance: borrow - repay = 10000 - 3000 = 7000
        // DOT: cash - deposit + borrow - repay = 100000 - 20000 + 10000 - 3000 = 87000
        assert_eq!(
            Loans::exchange_rate(DOT)
                .saturating_mul_int(Loans::account_deposits(Loans::lend_token_id(DOT).unwrap(), ALICE)),
            unit(20000, DOT)
        );
        let borrow_snapshot = Loans::account_borrows(DOT, ALICE);
        assert_eq!(borrow_snapshot.principal, unit(700000, IBTC));
        assert_eq!(borrow_snapshot.borrow_index, Loans::borrow_index(DOT));
        assert_eq!(Tokens::balance(DOT, &ALICE), unit(8700000, IBTC),);
    })
}

#[test]
fn repay_borrow_all_works() {
    new_test_ext().execute_with(|| {
        // Bob deposits 200 KSM
        assert_ok!(Loans::mint(RuntimeOrigin::signed(BOB), KSM, unit(200, KSM)));
        // Alice deposit 20000 DOT as collateral
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), DOT, unit(20000, DOT)));
        assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(ALICE), DOT));
        // Alice borrow 50 KSM
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), KSM, unit(50, KSM)));

        // Alice repay all borrow balance
        assert_ok!(Loans::repay_borrow_all(RuntimeOrigin::signed(ALICE), KSM));

        // DOT: cash - deposit +  = 100000 - 20000 = 80000
        // DOT collateral: deposit = 20000
        // KSM: cash + borrow - repay = 1000 + 50 - 50 = 1000
        // KSM borrow balance: borrow - repay = 50 - 50 = 0
        assert_eq!(Tokens::balance(DOT, &ALICE), unit(80000, DOT),);
        assert_eq!(
            Loans::exchange_rate(DOT)
                .saturating_mul_int(Loans::account_deposits(Loans::lend_token_id(DOT).unwrap(), ALICE)),
            unit(20000, DOT)
        );
        let borrow_snapshot = Loans::account_borrows(KSM, ALICE);
        assert_eq!(borrow_snapshot.principal, 0);
        assert_eq!(borrow_snapshot.borrow_index, Loans::borrow_index(KSM));
    })
}

#[test]
fn collateral_asset_works() {
    new_test_ext().execute_with(|| {
        // No collateral assets
        // Deposit 200 DOT as collateral
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), DOT, 200));
        let lend_token_id = Loans::lend_token_id(DOT).unwrap();
        // No lend_tokens deposited as collateral
        assert_eq!(
            Loans::account_deposits(lend_token_id, ALICE).eq(&reserved_balance(lend_token_id, &ALICE)),
            true
        );
        assert_eq!(free_balance(lend_token_id, &ALICE), 200 * 50);
        assert_eq!(reserved_balance(lend_token_id, &ALICE), 0);

        assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(ALICE), DOT));
        // Non-zero lend_tokens deposited as collateral
        assert_eq!(
            Loans::account_deposits(lend_token_id, ALICE).eq(&reserved_balance(lend_token_id, &ALICE)),
            true
        );
        assert_eq!(free_balance(lend_token_id, &ALICE), 0);
        assert_eq!(reserved_balance(lend_token_id, &ALICE), 200 * 50);

        // Borrow 100 DOT base on the collateral of 200 DOT
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), DOT, 100));
        assert_noop!(
            Loans::withdraw_all_collateral(RuntimeOrigin::signed(ALICE), DOT),
            Error::<Test>::InsufficientLiquidity
        );
        // Repay all the borrows
        assert_ok!(Loans::repay_borrow_all(RuntimeOrigin::signed(ALICE), DOT));
        assert_ok!(Loans::withdraw_all_collateral(RuntimeOrigin::signed(ALICE), DOT));
        assert_eq!(
            Loans::account_deposits(lend_token_id, ALICE),
            reserved_balance(lend_token_id, &ALICE)
        );
        assert_eq!(free_balance(lend_token_id, &ALICE), 200 * 50);
        assert_eq!(reserved_balance(lend_token_id, &ALICE), 0);
    })
}

#[test]
fn total_collateral_value_works() {
    new_test_ext().execute_with(|| {
        // Mock the price for DOT = 1, KSM = 1
        let collateral_factor = Rate::saturating_from_rational(50, 100);
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), DOT, unit(100, DOT)));
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), KSM, unit(200, KSM)));
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), KBTC, unit(300, KBTC)));
        assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(ALICE), DOT));
        assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(ALICE), KSM));
        let fixed_u128_collateral = FixedU128::checked_from_integer(unit(10000, IBTC) + unit(2000000, IBTC)).unwrap();
        assert_eq!(
            Loans::total_collateral_value(&ALICE)
                .unwrap()
                .to_unsigned_fixed_point()
                .unwrap(),
            collateral_factor.saturating_mul(fixed_u128_collateral)
        );
    })
}

#[test]
fn add_reserves_works() {
    new_test_ext().execute_with(|| {
        // Add 10000 DOT reserves
        assert_ok!(Loans::add_reserves(RuntimeOrigin::root(), ALICE, DOT, unit(10000, DOT)));

        assert_eq!(Loans::total_reserves(DOT), unit(10000, DOT));
        assert_eq!(Tokens::balance(DOT, &Loans::account_id()), unit(10000, DOT),);
        assert_eq!(Tokens::balance(DOT, &ALICE), unit(90000, DOT),);
    })
}

#[test]
fn reduce_reserves_works() {
    new_test_ext().execute_with(|| {
        // Add 10000 DOT reserves
        assert_ok!(Loans::add_reserves(RuntimeOrigin::root(), ALICE, DOT, unit(10000, DOT)));

        // Reduce 2000 DOT reserves
        assert_ok!(Loans::reduce_reserves(RuntimeOrigin::root(), ALICE, DOT, unit(2000, DOT)));

        assert_eq!(Loans::total_reserves(DOT), unit(8000, DOT));
        assert_eq!(Tokens::balance(DOT, &Loans::account_id()), unit(8000, DOT),);
        assert_eq!(Tokens::balance(DOT, &ALICE), unit(92000, DOT),);
    })
}

#[test]
fn total_reserves_are_updated_on_withdrawal() {
    new_test_ext().execute_with(|| {
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), DOT, unit(10000, DOT)));
        assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(ALICE), DOT));
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), DOT, unit(1000, DOT)));

        let blocks_to_run = 1000;
        _run_to_block(blocks_to_run);
        Loans::accrue_interest(DOT).unwrap();
        let intermediary_total_reserves = Loans::total_reserves(DOT);
        _run_to_block(2 * blocks_to_run);

        // Should be able to withdraw the entire total_reserve accumulated so far
        assert_ok!(Loans::reduce_reserves(
            RuntimeOrigin::root(),
            ALICE,
            DOT,
            // the actual total reserve is greater than this because of accumulated interest
            2 * intermediary_total_reserves
        ));
        // Leftover
        assert_eq!(Loans::total_reserves(DOT), 8610);
    })
}

#[test]
fn total_reserves_are_updated_on_deposit() {
    new_test_ext().execute_with(|| {
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), DOT, unit(100, DOT)));
        assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(ALICE), DOT));
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), DOT, unit(10, DOT)));

        let blocks_to_run = 1000;
        _run_to_block(blocks_to_run);
        Loans::accrue_interest(DOT).unwrap();
        let intermediary_total_reserves = Loans::total_reserves(DOT);
        _run_to_block(2 * blocks_to_run);

        assert_ok!(Loans::add_reserves(RuntimeOrigin::root(), ALICE, DOT, 1,));

        // The stored total reserve must be up-to-date
        assert!(
            Loans::total_reserves(DOT) > 2 * intermediary_total_reserves + 1,
            "interest was not accrued when reserves were added"
        );
    })
}

#[test]
fn reduce_reserve_reduce_amount_must_be_less_than_total_reserves() {
    new_test_ext().execute_with(|| {
        assert_ok!(Loans::add_reserves(RuntimeOrigin::root(), ALICE, DOT, unit(100, DOT)));
        assert_noop!(
            Loans::reduce_reserves(RuntimeOrigin::root(), ALICE, DOT, unit(200, DOT)),
            Error::<Test>::InsufficientReserves
        );
    })
}

#[test]
fn ratio_and_rate_works() {
    new_test_ext().execute_with(|| {
        // Permill to FixedU128
        let ratio = Permill::from_percent(50);
        let rate: FixedU128 = ratio.into();
        assert_eq!(rate, FixedU128::saturating_from_rational(1, 2));

        // Permill  (one = 1_000_000)
        let permill = Permill::from_percent(50);
        assert_eq!(permill.mul_floor(100_u128), 50_u128);

        // FixedU128 (one = 1_000_000_000_000_000_000_000)
        let value1 = FixedU128::saturating_from_integer(100);
        let value2 = FixedU128::saturating_from_integer(10);
        assert_eq!(
            value1.checked_mul(&value2),
            Some(FixedU128::saturating_from_integer(1000))
        );
        assert_eq!(
            value1.checked_div(&value2),
            Some(FixedU128::saturating_from_integer(10))
        );
        assert_eq!(
            value1.saturating_mul(permill.into()),
            FixedU128::saturating_from_integer(50)
        );

        let value1 = FixedU128::saturating_from_rational(9, 10);
        let value2 = 10_u128;
        let value3 = FixedU128::saturating_from_integer(10_u128);
        assert_eq!(value1.reciprocal(), Some(FixedU128::saturating_from_rational(10, 9)));
        // u128 div FixedU128
        assert_eq!(
            FixedU128::saturating_from_integer(value2).checked_div(&value1),
            Some(FixedU128::saturating_from_rational(100, 9))
        );

        // FixedU128 div u128
        assert_eq!(value1.reciprocal().and_then(|r| r.checked_mul_int(value2)), Some(11));
        assert_eq!(
            FixedU128::from_inner(17_777_777_777_777_777_777).checked_div_int(value2),
            Some(1)
        );
        // FixedU128 mul u128
        assert_eq!(
            FixedU128::from_inner(17_777_777_777_777_777_777).checked_mul_int(value2),
            Some(177)
        );

        // reciprocal
        assert_eq!(
            FixedU128::saturating_from_integer(value2).checked_div(&value1),
            Some(FixedU128::saturating_from_rational(100, 9))
        );
        assert_eq!(
            value1
                .reciprocal()
                .and_then(|r| r.checked_mul(&FixedU128::saturating_from_integer(value2))),
            Some(FixedU128::from_inner(11_111_111_111_111_111_110))
        );
        assert_eq!(
            FixedU128::saturating_from_integer(value2)
                .checked_mul(&value3)
                .and_then(|v| v.checked_div(&value1)),
            Some(FixedU128::saturating_from_rational(1000, 9))
        );
        assert_eq!(
            FixedU128::saturating_from_integer(value2)
                .checked_div(&value1)
                .and_then(|v| v.checked_mul(&value3)),
            Some(FixedU128::from_inner(111_111_111_111_111_111_110))
        );

        // FixedU128 div Permill
        let value1 = Permill::from_percent(30);
        let value2 = Permill::from_percent(40);
        let value3 = FixedU128::saturating_from_integer(10);
        assert_eq!(
            value3.checked_div(&value1.into()),
            Some(FixedU128::saturating_from_rational(100, 3)) // 10/0.3
        );

        // u128 div Permill
        assert_eq!(value1.saturating_reciprocal_mul_floor(5_u128), 16); // (1/0.3) * 5 = 16.66666666..
        assert_eq!(value1.saturating_reciprocal_mul_floor(5_u128), 16); // (1/0.3) * 5 = 16.66666666..
        assert_eq!(value2.saturating_reciprocal_mul_floor(5_u128), 12); // (1/0.4) * 5 = 12.5

        // Permill * u128
        let value1 = Permill::from_percent(34);
        let value2 = Permill::from_percent(36);
        let value3 = Permill::from_percent(30);
        let value4 = Permill::from_percent(20);
        assert_eq!(value1 * 10_u64, 3); // 0.34 * 10
        assert_eq!(value2 * 10_u64, 4); // 0.36 * 10
        assert_eq!(value3 * 5_u64, 1); // 0.3 * 5
        assert_eq!(value4 * 8_u64, 2); // 0.2 * 8
        assert_eq!(value4.mul_floor(8_u64), 1); // 0.2 mul_floor 8
    })
}

#[test]
fn update_exchange_rate_works() {
    new_test_ext().execute_with(|| {
        // Initialize value of exchange rate is 0.02
        assert_eq!(Loans::exchange_rate(DOT), Rate::saturating_from_rational(2, 100));

        assert_ok!(Loans::accrue_interest(DOT));
        assert_eq!(
            Loans::exchange_rate_stored(DOT).unwrap(),
            Rate::saturating_from_rational(2, 100)
        );

        // exchange_rate = total_cash + total_borrows - total_reverse / total_supply
        // total_cash = 10, total_supply = 500
        // exchange_rate = 10 + 5 - 1 / 500
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), DOT, unit(10, DOT)));
        TotalBorrows::<Test>::insert(DOT, unit(5, DOT));
        TotalReserves::<Test>::insert(DOT, unit(1, DOT));
        assert_eq!(
            Loans::exchange_rate_stored(DOT).unwrap(),
            Rate::saturating_from_rational(14, 500)
        );
    })
}

#[test]
fn current_borrow_balance_works() {
    new_test_ext().execute_with(|| {
        // snapshot.principal = 0
        AccountBorrows::<Test>::insert(
            DOT,
            ALICE,
            BorrowSnapshot {
                principal: 0,
                borrow_index: Rate::one(),
            },
        );
        assert_eq!(Loans::current_borrow_balance(&ALICE, DOT).unwrap(), 0);

        // snapshot.borrow_index = 0
        AccountBorrows::<Test>::insert(
            DOT,
            ALICE,
            BorrowSnapshot {
                principal: 100,
                borrow_index: Rate::zero(),
            },
        );
        assert_eq!(Loans::current_borrow_balance(&ALICE, DOT).unwrap(), 0);

        // borrow_index = 1.2, snapshot.borrow_index = 1, snapshot.principal = 100
        BorrowIndex::<Test>::insert(DOT, Rate::saturating_from_rational(12, 10));
        AccountBorrows::<Test>::insert(
            DOT,
            ALICE,
            BorrowSnapshot {
                principal: 100,
                borrow_index: Rate::one(),
            },
        );
        assert_eq!(Loans::current_borrow_balance(&ALICE, DOT).unwrap(), 120);
    })
}

#[test]
fn calc_collateral_amount_works() {
    let exchange_rate = Rate::saturating_from_rational(3, 10);
    assert_eq!(Loans::calc_collateral_amount(1000, exchange_rate).unwrap(), 3333);
    assert_eq!(
        Loans::calc_collateral_amount(u128::MAX, exchange_rate),
        Err(DispatchError::Arithmetic(ArithmeticError::Underflow))
    );

    // relative test: prevent_the_exchange_rate_attack
    let exchange_rate = Rate::saturating_from_rational(30000, 1);
    assert_eq!(Loans::calc_collateral_amount(10000, exchange_rate).unwrap(), 0);
}

#[test]
fn ensure_enough_cash_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(Tokens::set_balance(
            RuntimeOrigin::root(),
            Loans::account_id(),
            KSM,
            unit(1000, KSM),
            0
        ));
        assert_ok!(Loans::ensure_enough_cash(KSM, unit(1000, KSM)));
        TotalReserves::<Test>::insert(KSM, unit(10, KSM));
        assert_noop!(
            Loans::ensure_enough_cash(KSM, unit(1000, KSM)),
            Error::<Test>::InsufficientCash,
        );
        assert_ok!(Loans::ensure_enough_cash(KSM, unit(990, KSM)));
        // Borrows don't count as cash
        TotalBorrows::<Test>::insert(KSM, unit(20, KSM));
        assert_noop!(
            Loans::ensure_enough_cash(KSM, unit(1000, KSM)),
            Error::<Test>::InsufficientCash
        );
    })
}

#[test]
fn ensure_valid_exchange_rate_works() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Loans::ensure_valid_exchange_rate(FixedU128::saturating_from_rational(1, 100)),
            Error::<Test>::InvalidExchangeRate
        );
        assert_ok!(Loans::ensure_valid_exchange_rate(FixedU128::saturating_from_rational(
            2, 100
        )));
        assert_ok!(Loans::ensure_valid_exchange_rate(FixedU128::saturating_from_rational(
            3, 100
        )));
        assert_ok!(Loans::ensure_valid_exchange_rate(FixedU128::saturating_from_rational(
            99, 100
        )));
        assert_noop!(
            Loans::ensure_valid_exchange_rate(Rate::one()),
            Error::<Test>::InvalidExchangeRate,
        );
        assert_noop!(
            Loans::ensure_valid_exchange_rate(Rate::saturating_from_rational(101, 100)),
            Error::<Test>::InvalidExchangeRate,
        );
    })
}

#[test]
fn update_market_reward_speed_works() {
    new_test_ext().execute_with(|| {
        assert_eq!(Loans::reward_supply_speed(DOT), 0);
        assert_eq!(Loans::reward_borrow_speed(DOT), 0);

        assert_ok!(Loans::update_market_reward_speed(
            RuntimeOrigin::root(),
            DOT,
            Some(unit(1, INTR)),
            Some(unit(2, INTR)),
        ));
        assert_eq!(Loans::reward_supply_speed(DOT), unit(1, INTR));
        assert_eq!(Loans::reward_borrow_speed(DOT), unit(2, INTR));

        assert_ok!(Loans::update_market_reward_speed(
            RuntimeOrigin::root(),
            DOT,
            Some(unit(2, INTR)),
            Some(0),
        ));
        assert_eq!(Loans::reward_supply_speed(DOT), unit(2, INTR));
        assert_eq!(Loans::reward_borrow_speed(DOT), unit(0, INTR));

        assert_ok!(Loans::update_market_reward_speed(
            RuntimeOrigin::root(),
            DOT,
            Some(0),
            Some(0)
        ));
        assert_eq!(Loans::reward_supply_speed(DOT), unit(0, INTR));
        assert_eq!(Loans::reward_borrow_speed(DOT), unit(0, INTR));
    })
}

#[test]
fn reward_calculation_one_player_in_multi_markets_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), DOT, unit(10000, DOT)));
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), KSM, unit(100, KSM)));
        assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(ALICE), DOT));
        assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(ALICE), KSM));
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), DOT, unit(1000, DOT)));
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), KSM, unit(10, KSM)));

        _run_to_block(10);
        assert_ok!(Loans::update_market_reward_speed(
            RuntimeOrigin::root(),
            DOT,
            Some(unit(1, INTR)),
            Some(unit(2, INTR)),
        ));

        // check status
        let supply_state = Loans::reward_supply_state(DOT);
        assert_eq!(supply_state.block, 10);
        assert_eq!(Loans::reward_supplier_index(DOT, ALICE), 0);
        let borrow_state = Loans::reward_borrow_state(DOT);
        assert_eq!(borrow_state.block, 10);
        assert_eq!(Loans::reward_borrower_index(DOT, ALICE), 0);
        // DOT supply:100   DOT supply reward: 0
        // DOT borrow:10    DOT borrow reward: 0
        // KSM supply:100   KSM supply reward: 0
        // KSM borrow:10    KSM borrow reward: 0
        assert_eq!(Loans::reward_accrued(ALICE), 0);

        _run_to_block(20);
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), DOT, unit(10000, DOT)));
        assert_ok!(Loans::update_market_reward_speed(
            RuntimeOrigin::root(),
            KSM,
            Some(unit(1, INTR)),
            Some(unit(1, INTR)),
        ));

        // check status
        let supply_state = Loans::reward_supply_state(DOT);
        assert_eq!(supply_state.block, 20);
        let borrow_state = Loans::reward_borrow_state(DOT);
        assert_eq!(borrow_state.block, 10);
        // DOT supply:200   DOT supply reward: 10
        // DOT borrow:10    DOT borrow reward: 0
        // KSM supply:100   KSM supply reward: 0
        // KSM borrow:10    KSM borrow reward: 0
        // borrow reward not accrued
        assert_eq!(Loans::reward_accrued(ALICE), unit(10, INTR));

        _run_to_block(30);
        assert_ok!(Loans::update_market_reward_speed(
            RuntimeOrigin::root(),
            DOT,
            Some(0),
            Some(0)
        ));
        assert_ok!(Loans::redeem(RuntimeOrigin::signed(ALICE), DOT, unit(10000, DOT)));
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), DOT, unit(1000, DOT)));
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), KSM, unit(100, KSM)));
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), KSM, unit(10, KSM)));

        let supply_state = Loans::reward_supply_state(DOT);
        assert_eq!(supply_state.block, 30);
        let borrow_state = Loans::reward_borrow_state(DOT);
        assert_eq!(borrow_state.block, 30);
        // DOT supply:100   DOT supply reward: 20
        // DOT borrow:20    DOT borrow reward: 40
        // KSM supply:200   KSM supply reward: 10
        // KSM borrow:20    KSM borrow reward: 10
        assert_eq!(almost_equal(Loans::reward_accrued(ALICE), unit(80, INTR), INTR), true);

        _run_to_block(40);
        assert_ok!(Loans::update_market_reward_speed(
            RuntimeOrigin::root(),
            KSM,
            Some(0),
            Some(0)
        ));
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), DOT, unit(10000, DOT)));
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), DOT, unit(1000, DOT)));
        assert_ok!(Loans::redeem(RuntimeOrigin::signed(ALICE), KSM, unit(100, KSM)));
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), KSM, unit(10, KSM)));

        let supply_state = Loans::reward_supply_state(DOT);
        assert_eq!(supply_state.block, 40);
        let borrow_state = Loans::reward_borrow_state(DOT);
        assert_eq!(borrow_state.block, 40);
        // DOT supply:200   DOT supply reward: 20
        // DOT borrow:30    DOT borrow reward: 40
        // KSM supply:100   KSM supply reward: 20
        // KSM borrow:30    KSM borrow reward: 20
        assert_eq!(almost_equal(Loans::reward_accrued(ALICE), unit(100, INTR), INTR), true,);

        _run_to_block(50);
        assert_ok!(Loans::update_market_reward_speed(
            RuntimeOrigin::root(),
            DOT,
            Some(unit(1, INTR)),
            Some(unit(1, INTR)),
        ));
        assert_ok!(Loans::redeem(RuntimeOrigin::signed(ALICE), DOT, unit(10000, DOT)));
        assert_ok!(Loans::repay_borrow_all(RuntimeOrigin::signed(ALICE), DOT));
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), KSM, unit(100, KSM)));
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), KSM, unit(10, KSM)));

        let supply_state = Loans::reward_supply_state(DOT);
        assert_eq!(supply_state.block, 50);
        let borrow_state = Loans::reward_borrow_state(DOT);
        assert_eq!(borrow_state.block, 50);
        // DOT supply:100   DOT supply reward: 20
        // DOT borrow:0     DOT borrow reward: 40
        // KSM supply:200   KSM supply reward: 20
        // KSM borrow:40    KSM borrow reward: 20
        assert_eq!(almost_equal(Loans::reward_accrued(ALICE), unit(100, INTR), INTR), true,);

        _run_to_block(60);
        assert_ok!(Loans::update_market_reward_speed(
            RuntimeOrigin::root(),
            KSM,
            Some(unit(1, INTR)),
            Some(unit(1, INTR)),
        ));
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), DOT, unit(10000, DOT)));
        assert_ok!(Loans::redeem(RuntimeOrigin::signed(ALICE), KSM, unit(100, KSM)));
        assert_ok!(Loans::repay_borrow_all(RuntimeOrigin::signed(ALICE), KSM));

        let supply_state = Loans::reward_supply_state(DOT);
        assert_eq!(supply_state.block, 60);
        let borrow_state = Loans::reward_borrow_state(DOT);
        assert_eq!(borrow_state.block, 50);
        // DOT supply:200   DOT supply reward: 30
        // DOT borrow:0     DOT borrow reward: 40
        // KSM supply:100   KSM supply reward: 20
        // KSM borrow:0     KSM borrow reward: 20
        assert_eq!(almost_equal(Loans::reward_accrued(ALICE), unit(110, INTR), INTR), true,);

        _run_to_block(70);
        assert_ok!(Loans::update_market_reward_speed(
            RuntimeOrigin::root(),
            DOT,
            Some(0),
            Some(0)
        ));
        assert_ok!(Loans::update_market_reward_speed(
            RuntimeOrigin::root(),
            KSM,
            Some(0),
            Some(0)
        ));
        assert_ok!(Loans::redeem(RuntimeOrigin::signed(ALICE), DOT, unit(10000, DOT)));
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), KSM, unit(100, KSM)));

        let supply_state = Loans::reward_supply_state(DOT);
        assert_eq!(supply_state.block, 70);
        let borrow_state = Loans::reward_borrow_state(DOT);
        assert_eq!(borrow_state.block, 70);
        // DOT supply:500   DOT supply reward: 40
        // DOT borrow:0     DOT borrow reward: 40
        // KSM supply:600   KSM supply reward: 30
        // KSM borrow:0     KSM borrow reward: 20
        assert_eq!(almost_equal(Loans::reward_accrued(ALICE), unit(130, INTR), INTR), true);

        _run_to_block(80);
        assert_ok!(Loans::add_reward(RuntimeOrigin::signed(DAVE), unit(200, INTR)));
        assert_ok!(Loans::claim_reward(RuntimeOrigin::signed(ALICE)));
        assert_eq!(Tokens::balance(INTR, &DAVE), unit(99800, INTR));
        assert_eq!(almost_equal(Tokens::balance(INTR, &ALICE), unit(130, INTR), INTR), true);
        assert_eq!(
            almost_equal(Tokens::balance(INTR, &Loans::reward_account_id()), unit(70, INTR), INTR),
            true
        );
        assert_ok!(Loans::update_market_reward_speed(
            RuntimeOrigin::root(),
            DOT,
            Some(unit(1, INTR)),
            Some(0),
        ));

        // DOT supply:500   DOT supply reward: 50
        // DOT borrow:0     DOT borrow reward: 40
        // KSM supply:600   KSM supply reward: 30
        // KSM borrow:0     KSM borrow reward: 20
        _run_to_block(90);
        assert_ok!(Loans::claim_reward(RuntimeOrigin::signed(ALICE)));
        assert_eq!(almost_equal(Tokens::balance(INTR, &ALICE), unit(140, INTR), INTR), true);
    })
}

#[test]
fn reward_calculation_multi_player_in_one_market_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), DOT, unit(10, DOT)));
        assert_ok!(Loans::mint(RuntimeOrigin::signed(BOB), DOT, unit(10, DOT)));
        assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(ALICE), DOT));
        assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(BOB), DOT));

        _run_to_block(10);
        assert_ok!(Loans::update_market_reward_speed(
            RuntimeOrigin::root(),
            DOT,
            Some(unit(1, INTR)),
            Some(unit(1, INTR)),
        ));
        // Alice supply:10     supply reward: 0
        // Alice borrow:0       borrow reward: 0
        // BOB supply:10       supply reward: 0
        // BOB borrow:0         borrow reward: 0
        assert_eq!(Loans::reward_accrued(ALICE), 0);
        assert_eq!(Loans::reward_accrued(BOB), 0);

        _run_to_block(20);
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), DOT, unit(70, DOT)));
        assert_ok!(Loans::mint(RuntimeOrigin::signed(BOB), DOT, unit(10, DOT)));
        // Alice supply:80     supply reward: 5
        // Alice borrow:0       borrow reward: 0
        // BOB supply:20       supply reward: 5
        // BOB borrow:10        borrow reward: 0
        assert_eq!(Loans::reward_accrued(ALICE), unit(5, INTR));
        assert_eq!(Loans::reward_accrued(BOB), unit(5, INTR));

        _run_to_block(30);
        assert_ok!(Loans::redeem(RuntimeOrigin::signed(ALICE), DOT, unit(70, DOT)));
        assert_ok!(Loans::redeem(RuntimeOrigin::signed(BOB), DOT, unit(10, DOT)));
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), DOT, unit(1, DOT)));
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(BOB), DOT, unit(1, DOT)));
        // Alice supply:10     supply reward: 13
        // Alice borrow:1      borrow reward: 0
        // BOB supply:10       supply reward: 7
        // BOB borrow:1        borrow reward: 0
        assert_eq!(Loans::reward_accrued(ALICE), unit(13, INTR));
        assert_eq!(Loans::reward_accrued(BOB), unit(7, INTR));

        _run_to_block(40);
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), DOT, unit(10, DOT)));
        assert_ok!(Loans::mint(RuntimeOrigin::signed(BOB), DOT, unit(10, DOT)));
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), DOT, unit(1, DOT)));
        assert_ok!(Loans::repay_borrow_all(RuntimeOrigin::signed(BOB), DOT));
        // Alice supply:20     supply reward: 18
        // Alice borrow:2      borrow reward: 5
        // BOB supply:20       supply reward: 12
        // BOB borrow:0        borrow reward: 5
        assert_eq!(almost_equal(Loans::reward_accrued(ALICE), unit(23, INTR), INTR), true);
        assert_eq!(almost_equal(Loans::reward_accrued(BOB), unit(17, INTR), INTR), true);

        _run_to_block(50);
        assert_ok!(Loans::redeem(RuntimeOrigin::signed(ALICE), DOT, unit(10, DOT)));
        assert_ok!(Loans::redeem_all(RuntimeOrigin::signed(BOB), DOT));
        assert_eq!(Loans::free_lend_tokens(DOT, &BOB).unwrap().is_zero(), true);
        assert_eq!(Loans::reserved_lend_tokens(DOT, &BOB).unwrap().is_zero(), true);
        assert_ok!(Loans::repay_borrow_all(RuntimeOrigin::signed(ALICE), DOT));
        // No loans to repay
        assert_noop!(
            Loans::repay_borrow_all(RuntimeOrigin::signed(BOB), DOT),
            Error::<Test>::InvalidAmount
        );
        // Alice supply:10     supply reward: 23
        // Alice borrow:0      borrow reward: 15
        // BOB supply:0       supply reward: 17
        // BOB borrow:0        borrow reward: 5
        assert_eq!(almost_equal(Loans::reward_accrued(ALICE), unit(38, INTR), INTR), true);
        assert_eq!(almost_equal(Loans::reward_accrued(BOB), unit(22, INTR), INTR), true);

        _run_to_block(60);
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), DOT, unit(10, DOT)));
        // There is no locked collateral left, so withdrawing will fail
        assert_noop!(
            Loans::withdraw_all_collateral(RuntimeOrigin::signed(BOB), DOT),
            Error::<Test>::WithdrawAllCollateralFailed
        );
        // There is also no free collateral left, so redeeming will fail
        assert_noop!(
            Loans::redeem_all(RuntimeOrigin::signed(BOB), DOT),
            Error::<Test>::InvalidAmount
        );
        assert_noop!(
            Loans::repay_borrow_all(RuntimeOrigin::signed(ALICE), DOT),
            Error::<Test>::InvalidAmount
        );
        assert_noop!(
            Loans::repay_borrow_all(RuntimeOrigin::signed(BOB), DOT),
            Error::<Test>::InvalidAmount
        );
        // Alice supply:10     supply reward: 33
        // Alice borrow:0      borrow reward: 15
        // BOB supply:0       supply reward: 17
        // BOB borrow:0        borrow reward: 5
        assert_eq!(almost_equal(Loans::reward_accrued(ALICE), unit(48, INTR), INTR), true);
        assert_eq!(almost_equal(Loans::reward_accrued(BOB), unit(22, INTR), INTR), true);

        _run_to_block(70);
        assert_ok!(Loans::add_reward(RuntimeOrigin::signed(DAVE), unit(200, INTR)));
        assert_ok!(Loans::claim_reward_for_market(RuntimeOrigin::signed(ALICE), DOT));
        assert_ok!(Loans::claim_reward_for_market(RuntimeOrigin::signed(BOB), DOT));
        assert_eq!(Tokens::balance(INTR, &DAVE), unit(99800, INTR));
        assert_eq!(almost_equal(Tokens::balance(INTR, &ALICE), unit(58, INTR), INTR), true);
        assert_eq!(almost_equal(Tokens::balance(INTR, &BOB), unit(22, INTR), INTR), true);
        assert_eq!(
            almost_equal(Tokens::balance(INTR, &Loans::reward_account_id()), unit(120, INTR), INTR),
            true
        );
    })
}

#[test]
fn reward_calculation_after_liquidate_borrow_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), DOT, unit(20000, DOT)));
        assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(ALICE), DOT));
        assert_ok!(Loans::mint(RuntimeOrigin::signed(BOB), KSM, unit(500, KSM)));
        assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(BOB), KSM));
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), KSM, unit(50, KSM)));
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(BOB), KSM, unit(75, KSM)));

        _run_to_block(10);
        assert_ok!(Loans::update_market_reward_speed(
            RuntimeOrigin::root(),
            DOT,
            Some(unit(1, INTR)),
            Some(unit(1, INTR)),
        ));
        assert_ok!(Loans::update_market_reward_speed(
            RuntimeOrigin::root(),
            KSM,
            Some(unit(1, INTR)),
            Some(unit(1, INTR)),
        ));

        _run_to_block(20);
        assert_ok!(Loans::update_reward_supply_index(DOT));
        assert_ok!(Loans::distribute_supplier_reward(DOT, &ALICE));
        assert_ok!(Loans::distribute_supplier_reward(DOT, &BOB));
        assert_ok!(Loans::update_reward_borrow_index(DOT));
        assert_ok!(Loans::distribute_borrower_reward(DOT, &ALICE));
        assert_ok!(Loans::distribute_borrower_reward(DOT, &BOB));

        assert_ok!(Loans::update_reward_supply_index(KSM));
        assert_ok!(Loans::distribute_supplier_reward(KSM, &ALICE));
        assert_ok!(Loans::distribute_supplier_reward(KSM, &BOB));
        assert_ok!(Loans::update_reward_borrow_index(KSM));
        assert_ok!(Loans::distribute_borrower_reward(KSM, &ALICE));
        assert_ok!(Loans::distribute_borrower_reward(KSM, &BOB));

        assert_eq!(almost_equal(Loans::reward_accrued(ALICE), unit(14, INTR), INTR), true);
        assert_eq!(almost_equal(Loans::reward_accrued(BOB), unit(16, INTR), INTR), true);

        CurrencyConvert::convert.mock_safe(with_price(Some((KSM, 2.into()))));
        // since we set liquidate_threshold more than collateral_factor,with KSM price as 2 alice not shortfall yet.
        // so we can not liquidate_borrow here
        assert_noop!(
            Loans::liquidate_borrow(RuntimeOrigin::signed(BOB), ALICE, KSM, unit(25, DOT), DOT),
            Error::<Test>::InsufficientShortfall
        );
        // then we change KSM price = 3 to make alice shortfall
        // incentive = repay KSM value * 1.1 = (25 * 3) * 1.1 = 82.5
        // Alice DOT Deposit: 200 - 82.5 = 117.5
        // Alice KSM Borrow: 50 - 25 = 25
        // Bob DOT Deposit: 75 + 75*0.07 = 80.25
        // Bob KSM Deposit: 500
        // Bob KSM Borrow: 75
        // incentive_reward_account DOT Deposit: 75*0.03 = 2.25
        CurrencyConvert::convert.mock_safe(with_price(Some((KSM, 3.into()))));
        assert_ok!(Loans::liquidate_borrow(
            RuntimeOrigin::signed(BOB),
            ALICE,
            KSM,
            unit(25, KSM),
            DOT
        ));

        _run_to_block(30);
        assert_ok!(Loans::update_reward_supply_index(DOT));
        assert_ok!(Loans::distribute_supplier_reward(DOT, &ALICE));
        assert_ok!(Loans::distribute_supplier_reward(DOT, &BOB));
        assert_ok!(Loans::update_reward_borrow_index(DOT));
        assert_ok!(Loans::distribute_borrower_reward(DOT, &ALICE));
        assert_ok!(Loans::distribute_borrower_reward(DOT, &BOB));

        assert_ok!(Loans::update_reward_supply_index(KSM));
        assert_ok!(Loans::distribute_supplier_reward(KSM, &ALICE));
        assert_ok!(Loans::distribute_supplier_reward(KSM, &BOB));
        assert_ok!(Loans::update_reward_borrow_index(KSM));
        assert_ok!(Loans::distribute_borrower_reward(KSM, &ALICE));
        assert_ok!(Loans::distribute_borrower_reward(KSM, &BOB));
        assert_ok!(Loans::distribute_supplier_reward(
            DOT,
            &Loans::incentive_reward_account_id(),
        ));

        assert_eq!(almost_equal(Loans::reward_accrued(ALICE), milli_unit(22375, INTR), INTR), true);
        assert_eq!(almost_equal(Loans::reward_accrued(BOB), micro_unit(37512500, INTR), INTR), true);
        assert_eq!(
            almost_equal(
                Loans::reward_accrued(Loans::incentive_reward_account_id()),
                micro_unit(112500, INTR), INTR
            ),
            true,
        );
    })
}

#[test]
fn redeeming_full_amount_leaves_no_leftover() {
    new_test_ext().execute_with(|| {
        let amount_to_mint = 1235;
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), DOT, amount_to_mint));
        assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(ALICE), DOT));
        assert_ok!(Loans::redeem(RuntimeOrigin::signed(ALICE), DOT, amount_to_mint));

        assert_eq!(Loans::free_lend_tokens(DOT, &ALICE).unwrap().is_zero(), true);
        assert_eq!(Loans::reserved_lend_tokens(DOT, &ALICE).unwrap().is_zero(), true);
    })
}

