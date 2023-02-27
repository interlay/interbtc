//! The tests for fast-tracking functionality.

use super::*;

#[test]
fn fast_track_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(0);
        let prop_index = 0;
        assert_noop!(
            Democracy::fast_track(RuntimeOrigin::signed(5), prop_index, 2),
            Error::<Test>::ProposalMissing
        );
        assert_ok!(propose_set_balance(3, 2, 2));
        assert_noop!(
            Democracy::fast_track(RuntimeOrigin::signed(1), prop_index, 2),
            BadOrigin
        );
        assert_ok!(Democracy::fast_track(RuntimeOrigin::signed(5), prop_index, 0));
        assert_eq!(
            Democracy::referendum_status(0),
            Ok(ReferendumStatus {
                end: 2,
                proposal: set_balance_proposal(2),
                threshold: VoteThreshold::SuperMajorityAgainst,
                delay: 0,
                tally: Tally {
                    ayes: 0,
                    nays: 0,
                    turnout: 0
                },
            })
        );
    });
}

#[test]
fn fast_track_referendum_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(0);
        let fast_track_voting_period = <tests::Test as Config>::FastTrackVotingPeriod::get();
        let ref_index = Democracy::inject_referendum(
            fast_track_voting_period * 2,
            set_balance_proposal(2),
            VoteThreshold::SuperMajorityAgainst,
            0,
        );

        assert_noop!(
            Democracy::fast_track_referendum(RuntimeOrigin::signed(1), ref_index),
            BadOrigin
        );
        assert_ok!(Democracy::fast_track_referendum(RuntimeOrigin::signed(5), ref_index));

        let start_height = System::block_number();
        let end_height = start_height + fast_track_voting_period;
        assert_eq!(
            Democracy::referendum_status(ref_index),
            Ok(ReferendumStatus {
                end: end_height,
                proposal: set_balance_proposal(2),
                threshold: VoteThreshold::SuperMajorityAgainst,
                delay: 0,
                tally: Tally {
                    ayes: 0,
                    nays: 0,
                    turnout: 0
                },
            })
        );
    });
}

#[test]
fn fast_track_referendum_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(0);
        let fast_track_voting_period = <tests::Test as Config>::FastTrackVotingPeriod::get();
        let ref_index = Democracy::inject_referendum(
            fast_track_voting_period - 1,
            set_balance_proposal(2),
            VoteThreshold::SuperMajorityAgainst,
            0,
        );

        // Fails because the referendum ends too soon
        assert_noop!(
            Democracy::fast_track_referendum(RuntimeOrigin::signed(5), ref_index),
            Error::<Test>::ReferendumFastTrackFailed
        );
    });
}
