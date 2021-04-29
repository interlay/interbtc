/// Tests for Cuurency
use crate::mock::*;
use frame_support::{assert_err, assert_ok};

type Event = crate::Event<Test>;

/// Total supply
#[test]
fn test_total_supply_correct() {
    run_test(|| {
        // initial supply
        let desired_total = ALICE_BALANCE + BOB_BALANCE;
        let total = Currency::get_total_supply();

        assert_eq!(desired_total, total);
    })
}

/// Total locked
#[test]
fn test_total_locked_correct() {
    run_test(|| {
        // initial supply
        let desired_total_locked = 0;
        let increase_amount: Balance = 5;
        let decrease_amount: Balance = 3;

        let total_locked = Currency::get_total_locked();
        assert_eq!(desired_total_locked, total_locked);

        Currency::increase_total_locked(increase_amount);
        let increased_locked = Currency::get_total_locked();
        assert_eq!(total_locked + increase_amount, increased_locked);

        Currency::decrease_total_locked(decrease_amount);
        let decreased_locked = Currency::get_total_locked();
        assert_eq!(increased_locked - decrease_amount, decreased_locked);
    })
}

/// Mint
#[test]
fn test_mint_succeeds() {
    run_test(|| {
        let requester = ALICE;
        let amount: Balance = 5;

        let init_balance_alice = Currency::get_free_balance(&ALICE);
        let init_total_supply = Currency::get_total_supply();

        Currency::mint(requester, amount);
        let mint_event = TestEvent::currency(Event::Mint(ALICE, amount));

        assert!(System::events().iter().any(|a| a.event == mint_event));

        let balance_alice = Currency::get_free_balance(&ALICE);
        let total_supply = Currency::get_total_supply();

        assert_eq!(balance_alice, init_balance_alice + amount);
        assert_eq!(total_supply, init_total_supply + amount);
    })
}

/// Lock
#[test]
fn test_lock_succeeds() {
    run_test(|| {
        let redeemer = ALICE;
        let amount = ALICE_BALANCE;

        let init_balance = Currency::get_free_balance(&ALICE);
        let init_locked_balance = Currency::get_reserved_balance(&ALICE);
        let init_total_supply = Currency::get_total_supply();

        assert_ok!(Currency::lock(&redeemer, amount));
        let lock_event = TestEvent::currency(Event::Lock(ALICE, amount));

        assert!(System::events().iter().any(|a| a.event == lock_event));

        let balance = Currency::get_free_balance(&ALICE);
        let locked_balance = Currency::get_reserved_balance(&ALICE);
        let total_supply = Currency::get_total_supply();

        assert_eq!(balance, init_balance - amount);
        assert_eq!(locked_balance, init_locked_balance + amount);
        assert_eq!(total_supply, init_total_supply);
    })
}

#[test]
fn test_lock_fails() {
    run_test(|| {
        let sender = ALICE;
        let amount = ALICE_BALANCE + 5;

        let init_reserved = Currency::get_reserved_balance(&ALICE);
        let init_total = Currency::get_total_locked();

        assert_err!(Currency::lock(&sender, amount), TestError::InsufficientFreeBalance);

        let reserved = Currency::get_reserved_balance(&ALICE);
        let total = Currency::get_total_locked();

        assert_eq!(reserved, init_reserved);
        assert_eq!(total, init_total);
    })
}

/// Burn
#[test]
fn test_burn_succeeds() {
    run_test(|| {
        let redeemer = ALICE;
        let amount = ALICE_BALANCE;

        let init_balance = Currency::get_free_balance(&ALICE);
        let init_locked_balance = Currency::get_reserved_balance(&ALICE);
        let init_total_supply = Currency::get_total_supply();

        assert_ok!(Currency::lock(&redeemer, amount));
        assert_ok!(Currency::burn(&redeemer, amount));
        let burn_event = TestEvent::currency(Event::Burn(ALICE, amount));

        assert!(System::events().iter().any(|a| a.event == burn_event));

        let balance = Currency::get_free_balance(&ALICE);
        let locked_balance = Currency::get_reserved_balance(&ALICE);
        let total_supply = Currency::get_total_supply();

        assert_eq!(balance, init_balance - amount);
        assert_eq!(locked_balance, init_locked_balance);
        assert_eq!(total_supply, init_total_supply - amount);
    })
}

#[test]
fn test_burn_fails() {
    run_test(|| {
        let redeemer = ALICE;
        let amount = ALICE_BALANCE;

        let init_balance = Currency::get_free_balance(&ALICE);
        let init_locked_balance = Currency::get_reserved_balance(&ALICE);
        let init_total_supply = Currency::get_total_supply();

        assert_err!(
            Currency::burn(&redeemer, amount),
            TestError::InsufficientReservedBalance
        );

        let balance = Currency::get_free_balance(&ALICE);
        let locked_balance = Currency::get_reserved_balance(&ALICE);
        let total_supply = Currency::get_total_supply();

        assert_eq!(balance, init_balance);
        assert_eq!(locked_balance, init_locked_balance);
        assert_eq!(total_supply, init_total_supply);
    })
}

#[test]
fn test_burn_partially_succeeds() {
    run_test(|| {
        let redeemer = ALICE;
        let amount = ALICE_BALANCE;
        let burn_amount = amount - 10;

        let init_balance = Currency::get_free_balance(&ALICE);
        let init_locked_balance = Currency::get_reserved_balance(&ALICE);
        let init_total_supply = Currency::get_total_supply();

        assert_ok!(Currency::lock(&redeemer, amount));
        assert_ok!(Currency::burn(&redeemer, burn_amount));
        let burn_event = TestEvent::currency(Event::Burn(ALICE, burn_amount));

        assert!(System::events().iter().any(|a| a.event == burn_event));

        let balance = Currency::get_free_balance(&ALICE);
        let locked_balance = Currency::get_reserved_balance(&ALICE);
        let total_supply = Currency::get_total_supply();

        assert_eq!(balance, init_balance - amount); // balance is locked
                                                    // part of the balance is still locked
        assert_eq!(locked_balance, init_locked_balance + (amount - burn_amount));
        assert_eq!(total_supply, init_total_supply - burn_amount);
    })
}

/// Release
#[test]
fn test_release_succeeds() {
    run_test(|| {
        let sender = ALICE;
        let amount = ALICE_BALANCE;

        assert_ok!(Currency::lock(&sender, amount));

        let init_reserved = Currency::get_reserved_balance(&ALICE);
        let init_total = Currency::get_total_locked();

        assert_ok!(Currency::release(&sender, amount));
        let release_event = TestEvent::currency(Event::Release(ALICE, amount));

        assert!(System::events().iter().any(|a| a.event == release_event));

        let reserved = Currency::get_reserved_balance(&ALICE);
        let total = Currency::get_total_locked();

        assert_eq!(reserved, init_reserved - amount);
        assert_eq!(total, init_total - amount);
    })
}

#[test]
fn test_release_fails() {
    run_test(|| {
        let sender = ALICE;
        let lock_amount = ALICE_BALANCE;

        let init_reserved = Currency::get_reserved_balance(&ALICE);
        let init_total = Currency::get_total_locked();

        assert_err!(
            Currency::release(&sender, lock_amount),
            TestError::InsufficientReservedBalance
        );

        let reserved = Currency::get_reserved_balance(&ALICE);
        let total = Currency::get_total_locked();

        assert_eq!(reserved, init_reserved);
        assert_eq!(total, init_total);
    })
}

#[test]
fn test_release_partially_succeeds() {
    run_test(|| {
        let sender = ALICE;
        let amount = ALICE_BALANCE;
        let release_amount = ALICE_BALANCE - 10;

        assert_ok!(Currency::lock(&sender, amount));

        let init_reserved = Currency::get_reserved_balance(&ALICE);
        let init_total = Currency::get_total_locked();

        assert_ok!(Currency::release(&sender, release_amount));
        let release_event = TestEvent::currency(Event::Release(ALICE, release_amount));

        assert!(System::events().iter().any(|a| a.event == release_event));

        let reserved = Currency::get_reserved_balance(&ALICE);
        let total = Currency::get_total_locked();

        assert_eq!(reserved, init_reserved - release_amount);
        assert_eq!(total, init_total - release_amount);
    })
}

/// Slash
#[test]
fn test_slash_succeeds() {
    run_test(|| {
        let sender = ALICE;
        let receiver = BOB;
        let amount = ALICE_BALANCE;

        assert_ok!(Currency::lock(&sender, amount));

        let init_reserved_alice = Currency::get_reserved_balance(&ALICE);
        let init_reserved_bob = Currency::get_reserved_balance(&BOB);
        let init_total = Currency::get_total_locked();

        assert_ok!(Currency::slash(sender, receiver, amount));
        let slash_event = TestEvent::currency(Event::Slash(ALICE, BOB, amount));

        assert!(System::events().iter().any(|a| a.event == slash_event));

        let reserved_alice = Currency::get_reserved_balance(&ALICE);
        let reserved_bob = Currency::get_reserved_balance(&BOB);
        let total = Currency::get_total_locked();

        assert_eq!(reserved_alice, init_reserved_alice - amount);
        assert_eq!(reserved_bob, init_reserved_bob + amount);
        assert_eq!(total, init_total);
    })
}

#[test]
fn test_slash_fails() {
    run_test(|| {
        let sender = ALICE;
        let receiver = BOB;
        let amount = ALICE_BALANCE;

        let init_reserved_alice = Currency::get_reserved_balance(&ALICE);
        let init_reserved_bob = Currency::get_reserved_balance(&BOB);
        let init_total = Currency::get_total_locked();

        assert_err!(
            Currency::slash(sender, receiver, amount),
            TestError::InsufficientReservedBalance
        );

        let reserved_alice = Currency::get_reserved_balance(&ALICE);
        let reserved_bob = Currency::get_reserved_balance(&BOB);
        let total = Currency::get_total_locked();

        assert_eq!(reserved_alice, init_reserved_alice);
        assert_eq!(reserved_bob, init_reserved_bob);
        assert_eq!(total, init_total);
    })
}

#[test]
fn test_slash_partially_succeeds() {
    run_test(|| {
        let sender = ALICE;
        let receiver = BOB;
        let amount = ALICE_BALANCE;
        let slash_amount = ALICE_BALANCE;

        assert_ok!(Currency::lock(&sender, amount));

        let init_reserved_alice = Currency::get_reserved_balance(&ALICE);
        let init_reserved_bob = Currency::get_reserved_balance(&BOB);
        let init_total = Currency::get_total_locked();

        assert_ok!(Currency::slash(sender, receiver, slash_amount));
        let slash_event = TestEvent::currency(Event::Slash(ALICE, BOB, slash_amount));

        assert!(System::events().iter().any(|a| a.event == slash_event));

        let reserved_alice = Currency::get_reserved_balance(&ALICE);
        let reserved_bob = Currency::get_reserved_balance(&BOB);
        let total = Currency::get_total_locked();

        assert_eq!(reserved_alice, init_reserved_alice - slash_amount);
        assert_eq!(reserved_bob, init_reserved_bob + slash_amount);
        assert_eq!(total, init_total);
    })
}
