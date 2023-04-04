//! The tests for cancelation functionality.

use super::*;

#[test]
fn cancel_referendum_should_work() {
    new_test_ext().execute_with(|| {
        let r = Democracy::inject_referendum(2, set_balance_proposal(2), VoteThreshold::SuperMajorityApprove, 0);
        assert_ok!(Democracy::vote(RuntimeOrigin::signed(1), r, aye(1)));
        assert_ok!(Democracy::cancel_referendum(RuntimeOrigin::root(), r.into()));

        next_block();
        next_block();

        assert_eq!(Balances::free_balance(42), 0);
    });
}

#[test]
fn only_author_should_cancel_proposal() {
    new_test_ext().execute_with(|| {
        assert_ok!(propose_set_balance(1, 2, 5));
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
        assert_ok!(propose_set_balance(1, 2, 5));
        assert_ok!(Democracy::cancel_proposal(RuntimeOrigin::root(), 0));
        assert_noop!(
            Democracy::cancel_proposal(RuntimeOrigin::root(), 0),
            Error::<Test>::ProposalMissing
        );
    });
}
