//! The tests for the public proposal queue.

use super::*;

#[test]
fn backing_for_should_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(propose_set_balance_and_note(1, 2, 2));
        assert_ok!(propose_set_balance_and_note(1, 4, 4));
        assert_ok!(propose_set_balance_and_note(1, 3, 3));
        assert_eq!(Democracy::backing_for(0), Some(2));
        assert_eq!(Democracy::backing_for(1), Some(4));
        assert_eq!(Democracy::backing_for(2), Some(3));
    });
}

#[test]
fn deposit_for_proposals_should_be_taken() {
    new_test_ext().execute_with(|| {
        assert_ok!(propose_set_balance_and_note(1, 2, 5));
        assert_ok!(Democracy::second(RuntimeOrigin::signed(2), 0, u32::MAX));
        assert_ok!(Democracy::second(RuntimeOrigin::signed(5), 0, u32::MAX));
        assert_ok!(Democracy::second(RuntimeOrigin::signed(5), 0, u32::MAX));
        assert_ok!(Democracy::second(RuntimeOrigin::signed(5), 0, u32::MAX));
        assert_eq!(Balances::free_balance(1), 5);
        assert_eq!(Balances::free_balance(2), 15);
        assert_eq!(Balances::free_balance(5), 35);
    });
}

#[test]
fn only_author_should_cancel_proposal() {
    new_test_ext().execute_with(|| {
        assert_ok!(propose_set_balance_and_note(1, 2, 5));
        assert_noop!(
            Democracy::cancel_proposal(RuntimeOrigin::signed(2), 0),
            Error::<Test>::NotProposer
        );
        assert_ok!(Democracy::cancel_proposal(RuntimeOrigin::signed(1), 0));
        assert_noop!(
            Democracy::cancel_proposal(RuntimeOrigin::signed(1), 0),
            Error::<Test>::ProposalMissing
        );
        // deposit is returned if cancelled
        assert_eq!(Balances::free_balance(1), 10);
    });
}

#[test]
fn root_can_cancel_any_proposal() {
    new_test_ext().execute_with(|| {
        assert_ok!(propose_set_balance_and_note(1, 2, 5));
        assert_ok!(Democracy::cancel_proposal(RuntimeOrigin::root(), 0));
        assert_noop!(
            Democracy::cancel_proposal(RuntimeOrigin::root(), 0),
            Error::<Test>::ProposalMissing
        );
    });
}

#[test]
fn deposit_for_proposals_should_be_returned() {
    new_test_ext().execute_with(|| {
        assert_ok!(propose_set_balance_and_note(1, 2, 5));
        assert_ok!(Democracy::second(RuntimeOrigin::signed(2), 0, u32::MAX));
        assert_ok!(Democracy::second(RuntimeOrigin::signed(5), 0, u32::MAX));
        assert_ok!(Democracy::second(RuntimeOrigin::signed(5), 0, u32::MAX));
        assert_ok!(Democracy::second(RuntimeOrigin::signed(5), 0, u32::MAX));
        fast_forward_to(3);
        assert_eq!(Balances::free_balance(1), 10);
        assert_eq!(Balances::free_balance(2), 20);
        assert_eq!(Balances::free_balance(5), 50);
    });
}

#[test]
fn proposal_with_deposit_below_minimum_should_not_work() {
    new_test_ext().execute_with(|| {
        assert_noop!(propose_set_balance(1, 2, 0), Error::<Test>::ValueLow);
    });
}

#[test]
fn poor_proposer_should_not_work() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            propose_set_balance(1, 2, 11),
            BalancesError::<Test, _>::InsufficientBalance
        );
    });
}

#[test]
fn poor_seconder_should_not_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(propose_set_balance_and_note(2, 2, 11));
        assert_noop!(
            Democracy::second(RuntimeOrigin::signed(1), 0, u32::MAX),
            BalancesError::<Test, _>::InsufficientBalance
        );
    });
}

#[test]
fn invalid_seconds_upper_bound_should_not_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(propose_set_balance_and_note(1, 2, 5));
        assert_noop!(
            Democracy::second(RuntimeOrigin::signed(2), 0, 0),
            Error::<Test>::WrongUpperBound
        );
    });
}

#[test]
fn runners_up_should_come_after() {
    new_test_ext().execute_with(|| {
        System::set_block_number(0);
        // make 6 proposals and check that 2 get launched each period launching period
        // (which is every 2 blocks in these tests)

        assert_ok!(propose_set_balance_and_note(1, 2, 2));
        assert_ok!(propose_set_balance_and_note(1, 4, 4));
        assert_ok!(propose_set_balance_and_note(1, 3, 3));
        assert_ok!(propose_set_balance_and_note(2, 5, 3));
        assert_ok!(propose_set_balance_and_note(2, 6, 3));
        assert_ok!(propose_set_balance_and_note(2, 8, 3));

        // sanity check: nothing launched yet
        assert!(Democracy::vote(RuntimeOrigin::signed(1), 0, aye(1)).is_err());

        fast_forward_to(1);
        assert_ok!(Democracy::vote(RuntimeOrigin::signed(1), 0, aye(1)));
        assert_ok!(Democracy::vote(RuntimeOrigin::signed(1), 1, aye(1)));
        assert!(Democracy::vote(RuntimeOrigin::signed(1), 2, aye(1)).is_err()); // third one not yet launched

        // sanity check: on next block nothing should get launched
        fast_forward_to(2);
        assert!(Democracy::vote(RuntimeOrigin::signed(1), 2, aye(1)).is_err());

        fast_forward_to(3);
        assert_ok!(Democracy::vote(RuntimeOrigin::signed(1), 2, aye(1)));
        assert_ok!(Democracy::vote(RuntimeOrigin::signed(1), 3, aye(1)));
        assert!(Democracy::vote(RuntimeOrigin::signed(1), 4, aye(1)).is_err()); // fifth one not yet launched

        fast_forward_to(5);
        assert_ok!(Democracy::vote(RuntimeOrigin::signed(1), 4, aye(1)));
        assert_ok!(Democracy::vote(RuntimeOrigin::signed(1), 5, aye(1)));
    });
}
