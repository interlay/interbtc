use crate::{mock::*, tests::Loans, Markets};
use currency::Amount;
use frame_support::assert_ok;
use primitives::{CurrencyId::Token, Rate, Ratio, DOT, KSM, SECONDS_PER_YEAR};
use sp_runtime::{
    traits::{CheckedDiv, One, Saturating},
    FixedPointNumber,
};

#[test]
fn utilization_rate_works() {
    // 50% borrow
    assert_eq!(Loans::calc_utilization_ratio(1, 1, 0).unwrap(), Ratio::from_percent(50));
    assert_eq!(
        Loans::calc_utilization_ratio(100, 100, 0).unwrap(),
        Ratio::from_percent(50)
    );
    // no borrow
    assert_eq!(Loans::calc_utilization_ratio(1, 0, 0).unwrap(), Ratio::zero());
    // full borrow
    assert_eq!(
        Loans::calc_utilization_ratio(0, 1, 0).unwrap(),
        Ratio::from_percent(100)
    );
}

#[test]
fn interest_rate_model_works() {
    new_test_ext().execute_with(|| {
        let rate_decimal: u128 = 1_000_000_000_000_000_000;
        Tokens::set_balance(
            RuntimeOrigin::root(),
            ALICE,
            Token(DOT),
            million_unit(1000) - unit(1000),
            0,
        )
        .unwrap();
        // Deposit 200 DOT and borrow 100 DOT
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), Token(DOT), million_unit(200)));
        assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(ALICE), Token(DOT)));
        assert_ok!(Loans::borrow(
            RuntimeOrigin::signed(ALICE),
            Token(DOT),
            million_unit(100)
        ));

        let total_cash = million_unit(200) - million_unit(100);
        let total_supply = Loans::calc_collateral_amount(million_unit(200), Loans::exchange_rate(Token(DOT))).unwrap();
        assert_eq!(Loans::total_supply(Token(DOT)).unwrap(), total_supply);

        let borrow_snapshot = Loans::account_borrows(Token(DOT), ALICE);
        assert_eq!(borrow_snapshot.principal, million_unit(100));
        assert_eq!(borrow_snapshot.borrow_index, Rate::one());

        let base_rate = Rate::saturating_from_rational(2, 100);
        let jump_rate = Rate::saturating_from_rational(10, 100);
        // let full_rate = Rate::saturating_from_rational(32, 100);
        let jump_utilization = Ratio::from_percent(80);

        let mut borrow_index = Rate::one();
        let mut total_borrows = borrow_snapshot.principal;
        let mut total_reserves: u128 = 0;

        // Interest accrued from blocks 1 to 49
        for i in 1..49 {
            let delta_time = 6u128;
            TimestampPallet::set_timestamp(6000 * (i + 1));
            assert_ok!(Loans::accrue_interest(Token(DOT)));
            // utilizationRatio = totalBorrows / (totalCash + totalBorrows - totalReserves)
            let util_ratio = Ratio::from_rational(total_borrows, total_cash + total_borrows - total_reserves);
            assert_eq!(Loans::utilization_ratio(Token(DOT)), util_ratio);

            let borrow_rate = (jump_rate - base_rate) * util_ratio.into() / jump_utilization.into() + base_rate;
            let interest_accumulated: u128 = borrow_rate
                .saturating_mul_int(total_borrows)
                .saturating_mul(delta_time.into())
                .checked_div(SECONDS_PER_YEAR.into())
                .unwrap();
            total_borrows = interest_accumulated + total_borrows;
            assert_eq!(Loans::total_borrows(Token(DOT)), total_borrows);
            total_reserves = Markets::<Test>::get(&Token(DOT))
                .unwrap()
                .reserve_factor
                .mul_floor(interest_accumulated)
                + total_reserves;
            assert_eq!(Loans::total_reserves(Token(DOT)), total_reserves);

            // exchangeRate = (totalCash + totalBorrows - totalReserves) / totalSupply
            assert_eq!(
                Loans::exchange_rate(Token(DOT)).into_inner(),
                (total_cash + total_borrows - total_reserves) * rate_decimal / total_supply
            );
            let numerator = borrow_index
                .saturating_mul(borrow_rate)
                .saturating_mul(Rate::saturating_from_integer(delta_time))
                .checked_div(&Rate::saturating_from_integer(SECONDS_PER_YEAR))
                .unwrap();
            borrow_index = numerator + borrow_index;
            assert_eq!(Loans::borrow_index(Token(DOT)), borrow_index);
        }
        assert_eq!(total_borrows, 100000063926960646826);
        assert_eq!(total_reserves, 9589044097001);
        assert_eq!(borrow_index, Rate::from_inner(1000000639269606444));
        assert_eq!(Loans::exchange_rate(Token(DOT)), Rate::from_inner(20000005433791654));

        // Calculate borrow accrued interest
        let borrow_principal =
            (borrow_index / borrow_snapshot.borrow_index).saturating_mul_int(borrow_snapshot.principal);
        // TODO: Why subtract `million_unit(200)` here? Accruing interest doesn't fix this.
        let supply_interest = Loans::exchange_rate(Token(DOT)).saturating_mul_int(total_supply) - million_unit(200);
        assert_eq!(supply_interest, 54337916540000);
        assert_eq!(borrow_principal, 100000063926960644400);
        assert_eq!(total_borrows / 10000, borrow_principal / 10000);
        assert_eq!(
            (total_borrows - million_unit(100) - total_reserves) / 10000,
            supply_interest / 10000
        );
    })
}

#[test]
fn last_accrued_interest_time_should_be_update_correctly() {
    new_test_ext().execute_with(|| {
        assert_eq!(Loans::borrow_index(Token(DOT)), Rate::one());
        assert_eq!(Loans::last_accrued_interest_time(Token(DOT)), 0);
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), Token(DOT), unit(200)));
        assert_eq!(Loans::last_accrued_interest_time(Token(DOT)), 6);
        assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(ALICE), Token(DOT)));
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), Token(DOT), unit(100)));
        assert_eq!(Loans::borrow_index(Token(DOT)), Rate::one());
        TimestampPallet::set_timestamp(12000);
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), Token(DOT), unit(100)));
        assert_eq!(Loans::borrow_index(Token(DOT)), Rate::from_inner(1000000013318112633),);
    })
}

#[test]
fn accrue_interest_works_after_mint() {
    new_test_ext().execute_with(|| {
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), Token(DOT), unit(200)));
        assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(ALICE), Token(DOT)));
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), Token(DOT), unit(100)));
        assert_eq!(Loans::borrow_index(Token(DOT)), Rate::one());
        TimestampPallet::set_timestamp(12000);
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), Token(DOT), unit(100)));
        assert_eq!(Loans::borrow_index(Token(DOT)), Rate::from_inner(1000000013318112633),);
    })
}

#[test]
fn accrue_interest_works_after_borrow() {
    new_test_ext().execute_with(|| {
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), Token(DOT), unit(200)));
        assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(ALICE), Token(DOT)));
        assert_eq!(Loans::borrow_index(Token(DOT)), Rate::one());
        TimestampPallet::set_timestamp(12000);
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), Token(DOT), unit(100)));
        assert_eq!(Loans::borrow_index(Token(DOT)), Rate::from_inner(1000000003805175038),);
    })
}

#[test]
fn accrue_interest_works_after_redeem() {
    new_test_ext().execute_with(|| {
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), Token(DOT), unit(200)));
        assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(ALICE), Token(DOT)));
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), Token(DOT), unit(10)));
        assert_eq!(Loans::borrow_index(Token(DOT)), Rate::one());
        TimestampPallet::set_timestamp(12000);

        let amount_to_redeem = unit(10);
        assert_ok!(Loans::redeem(
            RuntimeOrigin::signed(ALICE),
            Token(DOT),
            amount_to_redeem
        ));
        assert_eq!(Loans::borrow_index(Token(DOT)), Rate::from_inner(1000000004756468797),);
        assert_eq!(
            Loans::exchange_rate(Token(DOT))
                .saturating_mul_int(Loans::account_deposits(Loans::lend_token_id(Token(DOT)).unwrap(), BOB)),
            0,
        );
        assert_eq!(Tokens::balance(Token(DOT), &ALICE), 819999999999999);
    })
}

#[test]
fn accrue_interest_works_after_redeem_all() {
    new_test_ext().execute_with(|| {
        assert_eq!(Tokens::balance(Token(DOT), &BOB), 1000000000000000);
        assert_ok!(Loans::mint(RuntimeOrigin::signed(BOB), Token(DOT), unit(20)));
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), Token(DOT), unit(200)));
        assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(ALICE), Token(DOT)));
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), Token(DOT), unit(10)));
        assert_eq!(Loans::borrow_index(Token(DOT)), Rate::one());
        assert_eq!(Tokens::balance(Token(DOT), &BOB), 980000000000000);
        TimestampPallet::set_timestamp(12000);
        assert_ok!(Loans::redeem_all(RuntimeOrigin::signed(BOB), Token(DOT)));
        assert_eq!(Loans::borrow_index(Token(DOT)), Rate::from_inner(1000000004669977168),);
        assert_eq!(
            Loans::exchange_rate(Token(DOT))
                .saturating_mul_int(Loans::account_deposits(Loans::lend_token_id(Token(DOT)).unwrap(), BOB)),
            0,
        );
        assert_eq!(Tokens::balance(Token(DOT), &BOB), 1000000000003608);
        assert_eq!(Loans::free_lend_tokens(Token(DOT), &BOB).unwrap().is_zero(), true);
        assert_eq!(Loans::reserved_lend_tokens(Token(DOT), &BOB).unwrap().is_zero(), true);
        assert!(!AccountDeposits::<Test>::contains_key(Token(DOT), &BOB))
    })
}

#[test]
fn accrue_interest_works_after_repay() {
    new_test_ext().execute_with(|| {
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), Token(DOT), unit(200)));
        assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(ALICE), Token(DOT)));
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), Token(DOT), unit(20)));
        assert_eq!(Loans::borrow_index(Token(DOT)), Rate::one());
        TimestampPallet::set_timestamp(12000);
        assert_ok!(Loans::repay_borrow(RuntimeOrigin::signed(ALICE), Token(DOT), unit(10)));
        assert_eq!(Loans::borrow_index(Token(DOT)), Rate::from_inner(1000000005707762557),);
    })
}

#[test]
fn accrue_interest_works_after_repay_all() {
    new_test_ext().execute_with(|| {
        assert_ok!(Loans::mint(RuntimeOrigin::signed(BOB), Token(KSM), unit(200)));
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), Token(DOT), unit(200)));
        assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(ALICE), Token(DOT)));
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), Token(KSM), unit(50)));
        assert_eq!(Loans::borrow_index(Token(KSM)), Rate::one());
        TimestampPallet::set_timestamp(12000);
        assert_ok!(Loans::repay_borrow_all(RuntimeOrigin::signed(ALICE), Token(KSM)));
        assert_eq!(Loans::borrow_index(Token(KSM)), Rate::from_inner(1000000008561643835),);
        assert_eq!(Tokens::balance(Token(KSM), &ALICE), 999999999571918);
        let borrow_snapshot = Loans::account_borrows(Token(KSM), ALICE);
        assert_eq!(borrow_snapshot.principal, 0);
        assert_eq!(borrow_snapshot.borrow_index, Loans::borrow_index(Token(KSM)));
    })
}

#[test]
fn accrue_interest_works_after_liquidate_borrow() {
    new_test_ext().execute_with(|| {
        // Bob deposits 200 KSM
        assert_ok!(Loans::mint(RuntimeOrigin::signed(BOB), Token(KSM), unit(200)));
        // Alice deposits 300 DOT as collateral
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), Token(DOT), unit(300)));
        assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(ALICE), Token(DOT)));
        // Alice borrows 100 KSM and 50 DOT
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), Token(KSM), unit(100)));
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), Token(DOT), unit(50)));
        assert_eq!(Loans::borrow_index(Token(DOT)), Rate::one());
        assert_eq!(Loans::borrow_index(Token(KSM)), Rate::one());
        TimestampPallet::set_timestamp(12000);
        // Adjust KSM price to make shortfall
        MockPriceFeeder::set_price(Token(KSM), 2.into());
        // BOB repay the KSM loan and get DOT callateral from ALICE
        assert_ok!(Loans::liquidate_borrow(
            RuntimeOrigin::signed(BOB),
            ALICE,
            Token(KSM),
            unit(50),
            Token(DOT)
        ));
        assert_eq!(Loans::borrow_index(Token(KSM)), Rate::from_inner(1000000013318112633),);
        assert_eq!(Loans::borrow_index(Token(DOT)), Rate::from_inner(1000000006976141552),);
    })
}

#[test]
fn accrue_interest_works_after_recompute_collateral_amount() {
    new_test_ext().execute_with(|| {
        assert_ok!(Loans::mint(RuntimeOrigin::signed(BOB), Token(KSM), unit(200)));
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), Token(DOT), unit(200)));
        assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(ALICE), Token(DOT)));
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), Token(KSM), unit(50)));
        assert_eq!(Loans::borrow_index(Token(KSM)), Rate::one());
        TimestampPallet::set_timestamp(12000);
        assert_ok!(Loans::recompute_collateral_amount(&Amount::<Test>::new(
            1234,
            Token(KSM)
        )));
        assert_eq!(Loans::borrow_index(Token(KSM)), Rate::from_inner(1000000008561643835),);
    })
}

#[test]
fn accrue_interest_works_after_recompute_underlying_amount() {
    new_test_ext().execute_with(|| {
        assert_ok!(Loans::mint(RuntimeOrigin::signed(BOB), Token(KSM), unit(200)));
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), Token(DOT), unit(200)));
        assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(ALICE), Token(DOT)));
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), Token(KSM), unit(50)));
        assert_eq!(Loans::borrow_index(Token(KSM)), Rate::one());
        TimestampPallet::set_timestamp(12000);
        assert_ok!(Loans::recompute_underlying_amount(
            &Loans::free_lend_tokens(Token(KSM), &ALICE).unwrap()
        ));
        assert_eq!(Loans::borrow_index(Token(KSM)), Rate::from_inner(1000000008561643835),);
    })
}

#[test]
fn different_markets_can_accrue_interest_in_one_block() {
    new_test_ext().execute_with(|| {
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), Token(DOT), unit(200)));
        assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(ALICE), Token(DOT)));
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), Token(KSM), unit(200)));
        assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(ALICE), Token(KSM)));
        assert_eq!(Loans::borrow_index(Token(DOT)), Rate::one());
        assert_eq!(Loans::borrow_index(Token(KSM)), Rate::one());
        TimestampPallet::set_timestamp(12000);
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), Token(DOT), unit(100)));
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), Token(KSM), unit(100)));
        assert_eq!(Loans::borrow_index(Token(DOT)), Rate::from_inner(1000000003805175038),);
        assert_eq!(Loans::borrow_index(Token(KSM)), Rate::from_inner(1000000003805175038),);
    })
}

#[test]
fn a_market_can_only_accrue_interest_once_in_a_block() {
    new_test_ext().execute_with(|| {
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), Token(DOT), unit(200)));
        assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(ALICE), Token(DOT)));
        assert_ok!(Loans::mint(RuntimeOrigin::signed(BOB), Token(DOT), unit(200)));
        assert_ok!(Loans::deposit_all_collateral(RuntimeOrigin::signed(BOB), Token(DOT)));
        assert_eq!(Loans::borrow_index(Token(DOT)), Rate::one());
        TimestampPallet::set_timestamp(12000);
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), Token(DOT), unit(100)));
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(BOB), Token(DOT), unit(100)));
        assert_eq!(Loans::borrow_index(Token(DOT)), Rate::from_inner(1000000003805175038),);
    })
}
