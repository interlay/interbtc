/// Tests for Treasury 
use crate::mock::*;
use crate::RawEvent;
use frame_support::{assert_err, assert_ok};

// use mocktopus::mocking::*;

#[test]
fn test_transfer_succeeds() {
    run_test(|| {
        let sender = Origin::signed(ALICE);
        let receiver = BOB;
        let amount: Balance = 3;
        
        let init_balance_alice = Balances::free_balance(ALICE);
        let init_balance_bob = Balances::free_balance(BOB);

        assert_ok!(Treasury::transfer(sender, receiver, amount));
        let transfer_event = TestEvent::test_events(
            RawEvent::Transfer(ALICE, BOB, amount));
        
        assert!(System::events().iter().any(|a| a.event == transfer_event));
        
        let balance_alice = Balances::free_balance(ALICE);
        let balance_bob = Balances::free_balance(BOB);

        assert_eq!(balance_alice, init_balance_alice - amount);
        assert_eq!(balance_bob, init_balance_bob + amount);
    })
}

#[test]
fn test_transfer_fails() {
    run_test(|| {
        let sender = Origin::signed(ALICE);
        let receiver = BOB;
        let amount = ALICE_BALANCE + 10;
        
        let init_balance_alice = Balances::free_balance(ALICE);
        let init_balance_bob = Balances::free_balance(BOB);

        assert_err!(Treasury::transfer(sender, receiver, amount),
            Error::InsufficientFunds);
        
        let balance_alice = Balances::free_balance(ALICE);
        let balance_bob = Balances::free_balance(BOB);

        assert_eq!(balance_alice, init_balance_alice);
        assert_eq!(balance_bob, init_balance_bob);
    })
}

