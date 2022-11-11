/// Tests for Escrow
use crate::mock::*;
use crate::{Limits, Point};
use frame_support::{
    assert_err, assert_ok,
    traits::{Currency, ReservableCurrency},
};
use sp_runtime::traits::Identity;

fn create_lock(origin: AccountId, amount: Balance, end_time: BlockNumber) {
    <Balances as Currency<AccountId>>::make_free_balance_be(&origin, amount);
    assert_ok!(Escrow::create_lock(RuntimeOrigin::signed(origin), amount, end_time));
}

fn extend_lock(origin: AccountId, amount: Balance) {
    let free_balance = <Balances as Currency<AccountId>>::free_balance(&origin);
    <Balances as Currency<AccountId>>::make_free_balance_be(&origin, free_balance + amount);
    assert_ok!(Escrow::increase_amount(RuntimeOrigin::signed(origin), amount));
}

#[test]
fn should_lock_and_degrade_power() {
    run_test(|| {
        let start_time = System::block_number();
        let max_time = start_time + MaxPeriod::get();
        let end_time = max_time;
        let amount = 1000;
        let slope = amount / max_time;
        let bias = slope * (end_time - start_time);

        create_lock(ALICE, amount, end_time);

        for current_time in [0, 50, 100] {
            let balance = bias - (slope * (current_time - start_time));
            assert_eq!(Escrow::balance_at(&ALICE, Some(current_time)), balance);
        }
    })
}

#[test]
fn should_withdraw_after_expiry() {
    run_test(|| {
        let end_time = MaxPeriod::get();
        let amount = 1000;

        create_lock(ALICE, amount, end_time);
        assert_eq!(Escrow::balance_at(&ALICE, Some(end_time)), 0);

        // cannot withdraw before expiry
        assert_err!(
            Escrow::withdraw(RuntimeOrigin::signed(ALICE)),
            TestError::LockNotExpired
        );

        // advance system and remove lock
        System::set_block_number(end_time);
        assert_ok!(Escrow::withdraw(RuntimeOrigin::signed(ALICE)));
    })
}

#[test]
fn should_increase_amount_locked() {
    run_test(|| {
        let amount = 1000;
        let start_time = System::block_number();
        let max_time = MaxPeriod::get();

        // lock MUST exist first
        assert_err!(
            Escrow::increase_amount(RuntimeOrigin::signed(ALICE), amount),
            TestError::LockNotFound
        );

        create_lock(ALICE, amount, max_time);

        extend_lock(ALICE, amount);

        assert_eq!(Escrow::balance_at(&ALICE, Some(start_time)), amount + amount);
    })
}

#[test]
fn should_increase_unlock_height() {
    run_test(|| {
        let amount = 1000;
        let max_time = MaxPeriod::get();

        // lock MUST exist first
        assert_err!(
            Escrow::increase_unlock_height(RuntimeOrigin::signed(ALICE), amount),
            TestError::LockHasExpired
        );

        create_lock(ALICE, amount, max_time);

        let half_time = max_time / 2;
        System::set_block_number(half_time);
        assert_eq!(Escrow::balance_at(&ALICE, Some(half_time)), amount / 2);

        assert_ok!(Escrow::increase_unlock_height(
            RuntimeOrigin::signed(ALICE),
            half_time + max_time
        ));

        assert_eq!(Escrow::balance_at(&ALICE, Some(half_time)), amount);
    })
}

#[test]
fn should_calculate_total_supply_and_locked() {
    run_test(|| {
        let end_time_1 = 100;
        let amount_1 = 1000;

        create_lock(ALICE, amount_1, end_time_1);

        let end_time_2 = 140;
        let amount_2 = 2000;

        let current_time = 40;
        System::set_block_number(current_time);
        create_lock(BOB, amount_2, end_time_2);

        assert_eq!(Escrow::balance_at(&ALICE, None), 600);
        assert_eq!(Escrow::balance_at(&BOB, None), 2000);
        assert_eq!(Escrow::total_supply(None), 2600);
    })
}

#[test]
fn should_calculate_total_supply_and_locked_after_withdraw() {
    run_test(|| {
        let end_time_1 = 100;
        let amount_1 = 1000;

        create_lock(ALICE, amount_1, end_time_1);

        let end_time_2 = 140;
        let amount_2 = 2000;

        let current_time = 40;
        System::set_block_number(current_time);
        create_lock(BOB, amount_2, end_time_2);

        System::set_block_number(end_time_1);
        assert_eq!(Escrow::balance_at(&ALICE, None), 0);
        assert_ok!(Escrow::withdraw(RuntimeOrigin::signed(ALICE)));

        assert_eq!(Escrow::balance_at(&BOB, None), 800);
        assert_eq!(Escrow::total_supply(None), 800);
    })
}

#[test]
fn should_create_lock_and_reserve() {
    run_test(|| {
        let end_time = MaxPeriod::get();
        let free_balance = 900;
        let reserved_balance = 100;
        let total_balance = free_balance + reserved_balance;

        create_lock(ALICE, total_balance, end_time);
        assert_eq!(Escrow::free_balance(&ALICE), total_balance);
        assert_ok!(Escrow::reserve(&ALICE, reserved_balance));
        assert_eq!(Escrow::free_balance(&ALICE), free_balance);
        assert_eq!(Escrow::total_balance(&ALICE), total_balance);
    })
}

#[test]
fn should_reverse_escrow_free_balance() {
    run_test(|| {
        let free_balance: u32 = 1000;

        let start_time = 0;
        let end_time = 100;

        for (current_time, balance) in [(0, 0), (50, 500), (100, 1000)] {
            let point = Point::new::<Identity>(free_balance, start_time, end_time, end_time);
            assert_eq!(point.reverse_balance_at::<Identity>(end_time, current_time), balance);
        }
    })
}

fn limit_account(who: AccountId, amount: Balance, start: BlockNumber, end: BlockNumber) {
    <Balances as Currency<AccountId>>::make_free_balance_be(&who, amount);
    <Limits<Test>>::insert(&who, (start, end));
}

#[test]
fn should_get_free_balance() {
    run_test(|| {
        limit_account(ALICE, 1000, 0, 100);
        assert_eq!(Escrow::get_free_balance(&ALICE), 0);
        System::set_block_number(100);
        assert_eq!(Escrow::get_free_balance(&ALICE), 1000);
        <Balances as Currency<AccountId>>::make_free_balance_be(&BOB, 2000);
        assert_eq!(Escrow::get_free_balance(&BOB), 2000);
    })
}

#[test]
fn should_not_allow_amount_smaller_than_max_period() {
    run_test(|| {
        let end_time = MaxPeriod::get();
        let amount = end_time / 2;

        <Balances as Currency<AccountId>>::make_free_balance_be(&ALICE, amount);
        assert_err!(
            Escrow::create_lock(RuntimeOrigin::signed(ALICE), amount, end_time),
            TestError::LockAmountTooLow
        );
    })
}

#[test]
fn deposit_below_max_height_truncates_to_zero() {
    run_test(|| {
        let free_balance: u128 = 4_838_200;
        assert_eq!(
            Point::new::<Identity>(free_balance, 0, 4_838_400, 4_838_400).balance_at::<Identity>(0),
            0
        );
    })
}
