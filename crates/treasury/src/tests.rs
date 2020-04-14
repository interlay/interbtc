/// Tests for Treasury
use crate::mock::*;
use crate::RawEvent;
use frame_support::{assert_err, assert_ok};

// use mocktopus::mocking::*;
/// Total supply
#[test]
fn test_total_supply_correct() {
    run_test(|| {
        // initial supply
        let desired_total_supply = ALICE_BALANCE + BOB_BALANCE;
        let total_supply = Treasury::get_total_supply();

        assert_eq!(desired_total_supply, total_supply);
    })
}

/// Transfer
#[test]
fn test_transfer_succeeds() {
    run_test(|| {
        let sender = Origin::signed(ALICE);
        let receiver = BOB;
        let amount: Balance = 3;

        let init_balance_alice = Treasury::get_balance_from_account(ALICE);
        let init_balance_bob = Treasury::get_balance_from_account(BOB);
        let init_total_supply = Treasury::get_total_supply();

        assert_ok!(Treasury::transfer(sender, receiver, amount));
        let transfer_event = TestEvent::test_events(RawEvent::Transfer(ALICE, BOB, amount));

        assert!(System::events().iter().any(|a| a.event == transfer_event));

        let balance_alice = Treasury::get_balance_from_account(ALICE);
        let balance_bob = Treasury::get_balance_from_account(BOB);
        let total_supply = Treasury::get_total_supply();

        assert_eq!(balance_alice, init_balance_alice - amount);
        assert_eq!(balance_bob, init_balance_bob + amount);
        assert_eq!(total_supply, init_total_supply);
    })
}

#[test]
fn test_transfer_fails() {
    run_test(|| {
        let sender = Origin::signed(ALICE);
        let receiver = BOB;
        let amount = ALICE_BALANCE + 10;

        let init_balance_alice = Treasury::get_balance_from_account(ALICE);
        let init_balance_bob = Treasury::get_balance_from_account(BOB);
        let init_total_supply = Treasury::get_total_supply();

        assert_err!(
            Treasury::transfer(sender, receiver, amount),
            Error::InsufficientFunds
        );

        let balance_alice = Treasury::get_balance_from_account(ALICE);
        let balance_bob = Treasury::get_balance_from_account(BOB);
        let total_supply = Treasury::get_total_supply();

        assert_eq!(balance_alice, init_balance_alice);
        assert_eq!(balance_bob, init_balance_bob);
        assert_eq!(total_supply, init_total_supply);
    })
}

/// Mint
#[test]
fn test_mint_succeeds() {
    run_test(|| {
        let requester = ALICE;
        let amount: Balance = 5;

        let init_balance_alice = Treasury::get_balance_from_account(ALICE);
        let init_total_supply = Treasury::get_total_supply();

        Treasury::mint(requester, amount);
        let mint_event = TestEvent::test_events(RawEvent::Mint(ALICE, amount));

        assert!(System::events().iter().any(|a| a.event == mint_event));

        let balance_alice = Treasury::get_balance_from_account(ALICE);
        let total_supply = Treasury::get_total_supply();

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

        let init_balance = Treasury::get_balance_from_account(ALICE);
        let init_locked_balance = Treasury::get_locked_balance_from_account(ALICE);
        let init_total_supply = Treasury::get_total_supply();

        assert_ok!(Treasury::lock(redeemer, amount));
        let lock_event = TestEvent::test_events(RawEvent::Lock(ALICE, amount));

        assert!(System::events().iter().any(|a| a.event == lock_event));

        let balance = Treasury::get_balance_from_account(ALICE);
        let locked_balance = Treasury::get_locked_balance_from_account(ALICE);
        let total_supply = Treasury::get_total_supply();

        assert_eq!(balance, init_balance - amount);
        assert_eq!(locked_balance, init_locked_balance + amount);
        assert_eq!(total_supply, init_total_supply);
    })
}

#[test]
fn test_lock_fails() {
    run_test(|| {
        let redeemer = ALICE;
        let amount = ALICE_BALANCE + 5;

        let init_balance = Treasury::get_balance_from_account(ALICE);
        let init_locked_balance = Treasury::get_locked_balance_from_account(ALICE);
        let init_total_supply = Treasury::get_total_supply();

        assert_err!(Treasury::lock(redeemer, amount), Error::InsufficientFunds);

        let balance = Treasury::get_balance_from_account(ALICE);
        let locked_balance = Treasury::get_locked_balance_from_account(ALICE);
        let total_supply = Treasury::get_total_supply();

        assert_eq!(balance, init_balance);
        assert_eq!(locked_balance, init_locked_balance);
        assert_eq!(total_supply, init_total_supply);
    })
}

/// Burn
#[test]
fn test_burn_succeeds() {
    run_test(|| {
        let redeemer = ALICE;
        let amount = ALICE_BALANCE;

        let init_balance = Treasury::get_balance_from_account(ALICE);
        let init_locked_balance = Treasury::get_locked_balance_from_account(ALICE);
        let init_total_supply = Treasury::get_total_supply();

        assert_ok!(Treasury::lock(redeemer, amount));
        assert_ok!(Treasury::burn(redeemer, amount));
        let burn_event = TestEvent::test_events(RawEvent::Burn(ALICE, amount));

        assert!(System::events().iter().any(|a| a.event == burn_event));

        let balance = Treasury::get_balance_from_account(ALICE);
        let locked_balance = Treasury::get_locked_balance_from_account(ALICE);
        let total_supply = Treasury::get_total_supply();

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

        let init_balance = Treasury::get_balance_from_account(ALICE);
        let init_locked_balance = Treasury::get_locked_balance_from_account(ALICE);
        let init_total_supply = Treasury::get_total_supply();

        assert_err!(
            Treasury::burn(redeemer, amount),
            Error::InsufficientLockedFunds
        );

        let balance = Treasury::get_balance_from_account(ALICE);
        let locked_balance = Treasury::get_locked_balance_from_account(ALICE);
        let total_supply = Treasury::get_total_supply();

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

        let init_balance = Treasury::get_balance_from_account(ALICE);
        let init_locked_balance = Treasury::get_locked_balance_from_account(ALICE);
        let init_total_supply = Treasury::get_total_supply();

        assert_ok!(Treasury::lock(redeemer, amount));
        assert_ok!(Treasury::burn(redeemer, burn_amount));
        let burn_event = TestEvent::test_events(RawEvent::Burn(ALICE, burn_amount));

        assert!(System::events().iter().any(|a| a.event == burn_event));

        let balance = Treasury::get_balance_from_account(ALICE);
        let locked_balance = Treasury::get_locked_balance_from_account(ALICE);
        let total_supply = Treasury::get_total_supply();

        assert_eq!(balance, init_balance - amount); // balance is locked
                                                    // part of the balance is still locked
        assert_eq!(locked_balance, init_locked_balance + (amount - burn_amount));
        assert_eq!(total_supply, init_total_supply - burn_amount);
    })
}
