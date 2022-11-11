//! The tests for normal voting functionality.

use super::*;

#[test]
fn overvoting_should_fail() {
    new_test_ext().execute_with(|| {
        let r = begin_referendum();
        assert_noop!(
            Democracy::vote(RuntimeOrigin::signed(1), r, aye(2)),
            Error::<Test>::InsufficientFunds
        );
    });
}

#[test]
fn single_proposal_should_work() {
    new_test_ext().execute_with(|| {
        System::set_block_number(0);
        assert_ok!(propose_set_balance_and_note(1, 2, 1));
        let r = 0;
        assert!(Democracy::referendum_info(r).is_none());

        // start of 2 => next referendum scheduled.
        fast_forward_to(2);
        assert_ok!(Democracy::vote(RuntimeOrigin::signed(1), r, aye(1)));

        assert_eq!(Democracy::referendum_count(), 1);
        assert_eq!(
            Democracy::referendum_status(0),
            Ok(ReferendumStatus {
                end: 4,
                proposal_hash: set_balance_proposal_hash_and_note(2),
                threshold: VoteThreshold::SuperMajorityAgainst,
                delay: 2,
                tally: Tally {
                    ayes: 10,
                    nays: 0,
                    turnout: 10
                },
            })
        );

        fast_forward_to(3);

        // referendum still running
        assert_ok!(Democracy::referendum_status(0));

        // referendum runs during 2 and 3, ends @ start of 4.
        fast_forward_to(4);

        assert_noop!(Democracy::referendum_status(0), Error::<Test>::ReferendumInvalid);
        assert!(pallet_scheduler::Agenda::<Test>::get(6)[0].is_some());

        // referendum passes and wait another two blocks for enactment.
        fast_forward_to(6);

        assert_eq!(Balances::free_balance(42), 2);
    });
}

#[test]
fn controversial_voting_should_work() {
    new_test_ext().execute_with(|| {
        let r = Democracy::inject_referendum(
            2,
            set_balance_proposal_hash_and_note(2),
            VoteThreshold::SuperMajorityApprove,
            0,
        );

        assert_ok!(Democracy::vote(RuntimeOrigin::signed(1), r, aye(1)));
        assert_ok!(Democracy::vote(RuntimeOrigin::signed(2), r, nay(2)));
        assert_ok!(Democracy::vote(RuntimeOrigin::signed(3), r, nay(3)));
        assert_ok!(Democracy::vote(RuntimeOrigin::signed(4), r, aye(4)));
        assert_ok!(Democracy::vote(RuntimeOrigin::signed(5), r, nay(5)));
        assert_ok!(Democracy::vote(RuntimeOrigin::signed(6), r, aye(6)));

        assert_eq!(
            tally(r),
            Tally {
                ayes: 110,
                nays: 100,
                turnout: 210
            }
        );

        next_block();
        next_block();

        assert_eq!(Balances::free_balance(42), 2);
    });
}

#[test]
fn controversial_low_turnout_voting_should_work() {
    new_test_ext().execute_with(|| {
        let r = Democracy::inject_referendum(
            2,
            set_balance_proposal_hash_and_note(2),
            VoteThreshold::SuperMajorityApprove,
            0,
        );
        assert_ok!(Democracy::vote(RuntimeOrigin::signed(5), r, nay(5)));
        assert_ok!(Democracy::vote(RuntimeOrigin::signed(6), r, aye(6)));

        assert_eq!(
            tally(r),
            Tally {
                ayes: 60,
                nays: 50,
                turnout: 110
            }
        );

        next_block();
        next_block();

        assert_eq!(Balances::free_balance(42), 0);
    });
}

#[test]
fn passing_low_turnout_voting_should_work() {
    new_test_ext().execute_with(|| {
        assert_eq!(Balances::free_balance(42), 0);
        assert_eq!(Balances::total_issuance(), 210);

        let r = Democracy::inject_referendum(
            2,
            set_balance_proposal_hash_and_note(2),
            VoteThreshold::SuperMajorityApprove,
            0,
        );
        assert_ok!(Democracy::vote(RuntimeOrigin::signed(4), r, aye(4)));
        assert_ok!(Democracy::vote(RuntimeOrigin::signed(5), r, nay(5)));
        assert_ok!(Democracy::vote(RuntimeOrigin::signed(6), r, aye(6)));
        assert_eq!(
            tally(r),
            Tally {
                ayes: 100,
                nays: 50,
                turnout: 150
            }
        );

        next_block();
        next_block();
        assert_eq!(Balances::free_balance(42), 2);
    });
}
