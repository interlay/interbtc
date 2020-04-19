/// Tests for Collateral
use crate::mock::*;
use crate::RawEvent;
use frame_support::{assert_err, assert_ok};

/// Total supply
#[test]
fn test_total_supply_correct() {
    run_test(|| {
        // initial supply
        let desired_total = ALICE_BALANCE + BOB_BALANCE;
        let total = Collateral::get_total_supply();

        assert_eq!(desired_total, total);
    })
}

/// Total collateral
#[test]
fn test_total_collateral_correct() {
    run_test(|| {
        // initial supply
        let desired_total_collateral = 0;
        let increase_amount: Balance = 5;
        let decrease_amount: Balance = 3;

        let total_collateral = Collateral::get_total_collateral();
        assert_eq!(desired_total_collateral, total_collateral);

        Collateral::increase_total_collateral(increase_amount);
        let increased_collateral = Collateral::get_total_collateral();
        assert_eq!(total_collateral + increase_amount, increased_collateral);

        Collateral::decrease_total_collateral(decrease_amount);
        let decreased_collateral = Collateral::get_total_collateral();
        assert_eq!(increased_collateral - decrease_amount, decreased_collateral);
    })
}

/// Lock collateral
#[test]
fn test_lock_collateral_succeeds() {
    run_test(|| {
        let sender = ALICE;
        let amount: Balance = 5;

        let init_collateral = Collateral::get_collateral_from_account(&ALICE);
        let init_total = Collateral::get_total_collateral();

        assert_ok!(Collateral::lock_collateral(&sender, amount));
        let lock_event = TestEvent::test_events(RawEvent::LockCollateral(ALICE, amount));

        assert!(System::events().iter().any(|a| a.event == lock_event));

        let collateral = Collateral::get_collateral_from_account(&ALICE);
        let total = Collateral::get_total_collateral();

        assert_eq!(collateral, init_collateral + amount);
        assert_eq!(total, init_total + amount);
    })
}

#[test]
fn test_lock_collateral_fails() {
    run_test(|| {
        let sender = ALICE;
        let amount = ALICE_BALANCE + 5;

        let init_collateral = Collateral::get_collateral_from_account(&ALICE);
        let init_total = Collateral::get_total_collateral();

        assert_err!(
            Collateral::lock_collateral(&sender, amount),
            Error::InsufficientFunds
        );

        let collateral = Collateral::get_collateral_from_account(&ALICE);
        let total = Collateral::get_total_collateral();

        assert_eq!(collateral, init_collateral);
        assert_eq!(total, init_total);
    })
}

/// Release collateral
#[test]
fn test_release_collateral_succeeds() {
    run_test(|| {
        let sender = ALICE;
        let amount = ALICE_BALANCE;

        assert_ok!(Collateral::lock_collateral(&sender, amount));

        let init_collateral = Collateral::get_collateral_from_account(&ALICE);
        let init_total = Collateral::get_total_collateral();

        assert_ok!(Collateral::release_collateral(sender, amount));
        let release_event = TestEvent::test_events(RawEvent::ReleaseCollateral(ALICE, amount));

        assert!(System::events().iter().any(|a| a.event == release_event));

        let collateral = Collateral::get_collateral_from_account(&ALICE);
        let total = Collateral::get_total_collateral();

        assert_eq!(collateral, init_collateral - amount);
        assert_eq!(total, init_total - amount);
    })
}

#[test]
fn test_release_collateral_fails() {
    run_test(|| {
        let sender = ALICE;
        let lock_amount = ALICE_BALANCE;

        let init_collateral = Collateral::get_collateral_from_account(&ALICE);
        let init_total = Collateral::get_total_collateral();

        assert_err!(
            Collateral::release_collateral(sender, lock_amount),
            Error::InsufficientCollateralAvailable
        );

        let collateral = Collateral::get_collateral_from_account(&ALICE);
        let total = Collateral::get_total_collateral();

        assert_eq!(collateral, init_collateral);
        assert_eq!(total, init_total);
    })
}

#[test]
fn test_release_collateral_partially_succeeds() {
    run_test(|| {
        let sender = ALICE;
        let amount = ALICE_BALANCE;
        let release_amount = ALICE_BALANCE - 10;

        assert_ok!(Collateral::lock_collateral(&sender, amount));

        let init_collateral = Collateral::get_collateral_from_account(&ALICE);
        let init_total = Collateral::get_total_collateral();

        assert_ok!(Collateral::release_collateral(sender, release_amount));
        let release_event =
            TestEvent::test_events(RawEvent::ReleaseCollateral(ALICE, release_amount));

        assert!(System::events().iter().any(|a| a.event == release_event));

        let collateral = Collateral::get_collateral_from_account(&ALICE);
        let total = Collateral::get_total_collateral();

        assert_eq!(collateral, init_collateral - release_amount);
        assert_eq!(total, init_total - release_amount);
    })
}

/// Slash collateral
#[test]
fn test_slash_collateral_succeeds() {
    run_test(|| {
        let sender = ALICE;
        let receiver = BOB;
        let amount = ALICE_BALANCE;

        assert_ok!(Collateral::lock_collateral(&sender, amount));

        let init_collateral_alice = Collateral::get_collateral_from_account(&ALICE);
        let init_collateral_bob = Collateral::get_collateral_from_account(&BOB);
        let init_total = Collateral::get_total_collateral();

        assert_ok!(Collateral::slash_collateral(sender, receiver, amount));
        let slash_event = TestEvent::test_events(RawEvent::SlashCollateral(ALICE, BOB, amount));

        assert!(System::events().iter().any(|a| a.event == slash_event));

        let collateral_alice = Collateral::get_collateral_from_account(&ALICE);
        let collateral_bob = Collateral::get_collateral_from_account(&BOB);
        let total = Collateral::get_total_collateral();

        assert_eq!(collateral_alice, init_collateral_alice - amount);
        assert_eq!(collateral_bob, init_collateral_bob + amount);
        assert_eq!(total, init_total);
    })
}

#[test]
fn test_slash_collateral_fails() {
    run_test(|| {
        let sender = ALICE;
        let receiver = BOB;
        let amount = ALICE_BALANCE;

        let init_collateral_alice = Collateral::get_collateral_from_account(&ALICE);
        let init_collateral_bob = Collateral::get_collateral_from_account(&BOB);
        let init_total = Collateral::get_total_collateral();

        assert_err!(
            Collateral::slash_collateral(sender, receiver, amount),
            Error::InsufficientCollateralAvailable
        );

        let collateral_alice = Collateral::get_collateral_from_account(&ALICE);
        let collateral_bob = Collateral::get_collateral_from_account(&BOB);
        let total = Collateral::get_total_collateral();

        assert_eq!(collateral_alice, init_collateral_alice);
        assert_eq!(collateral_bob, init_collateral_bob);
        assert_eq!(total, init_total);
    })
}

#[test]
fn test_slash_collateral_partially_succeeds() {
    run_test(|| {
        let sender = ALICE;
        let receiver = BOB;
        let amount = ALICE_BALANCE;
        let slash_amount = ALICE_BALANCE;

        assert_ok!(Collateral::lock_collateral(&sender, amount));

        let init_collateral_alice = Collateral::get_collateral_from_account(&ALICE);
        let init_collateral_bob = Collateral::get_collateral_from_account(&BOB);
        let init_total = Collateral::get_total_collateral();

        assert_ok!(Collateral::slash_collateral(sender, receiver, slash_amount));
        let slash_event =
            TestEvent::test_events(RawEvent::SlashCollateral(ALICE, BOB, slash_amount));

        assert!(System::events().iter().any(|a| a.event == slash_event));

        let collateral_alice = Collateral::get_collateral_from_account(&ALICE);
        let collateral_bob = Collateral::get_collateral_from_account(&BOB);
        let total = Collateral::get_total_collateral();

        assert_eq!(collateral_alice, init_collateral_alice - slash_amount);
        assert_eq!(collateral_bob, init_collateral_bob + slash_amount);
        assert_eq!(total, init_total);
    })
}
