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
        assert_ok!(Democracy::second(Origin::signed(2), 0, u32::MAX));
        assert_ok!(Democracy::second(Origin::signed(5), 0, u32::MAX));
        assert_ok!(Democracy::second(Origin::signed(5), 0, u32::MAX));
        assert_ok!(Democracy::second(Origin::signed(5), 0, u32::MAX));
        assert_eq!(Balances::free_balance(1), 5);
        assert_eq!(Balances::free_balance(2), 15);
        assert_eq!(Balances::free_balance(5), 35);
    });
}

#[test]
fn deposit_for_proposals_should_be_returned() {
    new_test_ext().execute_with(|| {
        assert_ok!(propose_set_balance_and_note(1, 2, 5));
        assert_ok!(Democracy::second(Origin::signed(2), 0, u32::MAX));
        assert_ok!(Democracy::second(Origin::signed(5), 0, u32::MAX));
        assert_ok!(Democracy::second(Origin::signed(5), 0, u32::MAX));
        assert_ok!(Democracy::second(Origin::signed(5), 0, u32::MAX));
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
            Democracy::second(Origin::signed(1), 0, u32::MAX),
            BalancesError::<Test, _>::InsufficientBalance
        );
    });
}

#[test]
fn invalid_seconds_upper_bound_should_not_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(propose_set_balance_and_note(1, 2, 5));
        assert_noop!(
            Democracy::second(Origin::signed(2), 0, 0),
            Error::<Test>::WrongUpperBound
        );
    });
}

#[test]
fn runners_up_should_come_after() {
    new_test_ext().execute_with(|| {
        System::set_block_number(0);
        assert_ok!(propose_set_balance_and_note(1, 2, 2));
        assert_ok!(propose_set_balance_and_note(1, 4, 4));
        assert_ok!(propose_set_balance_and_note(1, 3, 3));
        fast_forward_to(2);
        assert_ok!(Democracy::vote(Origin::signed(1), 0, aye(1)));
        fast_forward_to(4);
        assert_ok!(Democracy::vote(Origin::signed(1), 1, aye(1)));
        fast_forward_to(6);
        assert_ok!(Democracy::vote(Origin::signed(1), 2, aye(1)));
    });
}

#[test]
fn propose_imminent_should_work() {
    new_test_ext().execute_with(|| {
        System::set_block_number(0);
        assert_noop!(
            Democracy::propose_imminent(
                Origin::signed(5),
                Box::new(Call::Balances(pallet_balances::Call::transfer { dest: 42, value: 100 }))
            ),
            Error::<Test>::NotImminent
        );
        assert_ok!(Democracy::propose_imminent(
            Origin::signed(5),
            Box::new(Call::Balances(pallet_balances::Call::set_balance {
                who: 42,
                new_free: 100,
                new_reserved: 0,
            }))
        ));
        assert_eq!(
            Democracy::referendum_status(0),
            Ok(ReferendumStatus {
                end: VotingPeriod::get(),
                proposal_hash: set_balance_proposal_hash_and_note(100),
                threshold: VoteThreshold::SuperMajorityAgainst,
                delay: EnactmentPeriod::get(),
                tally: Tally {
                    ayes: 0,
                    nays: 0,
                    turnout: 0
                },
            })
        );
    });
}
