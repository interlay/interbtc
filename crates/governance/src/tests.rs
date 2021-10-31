/// Tests for Governance
use crate::mock::*;
use crate::{Config as GovernanceConfig, Event as GovernanceEvent};
use codec::Encode;
use frame_support::{assert_err, assert_ok, traits::Currency};
use sp_core::H256;

fn set_balance_proposal(who: AccountId, value: Balance) -> Vec<u8> {
    Call::Balances(pallet_balances::Call::set_balance {
        who,
        new_free: value,
        new_reserved: 0,
    })
    .encode()
}

fn assert_proposal_event() -> H256 {
    System::events()
        .iter()
        .rev()
        .find_map(|record| {
            if let TestEvent::Governance(GovernanceEvent::Proposed { proposal_hash, .. }) = record.event {
                Some(proposal_hash)
            } else {
                None
            }
        })
        .expect("proposal event not found")
}

fn setup_proposal() -> H256 {
    let proposal = set_balance_proposal(BOB, 100);
    <Balances as Currency<_>>::make_free_balance_be(&ALICE, proposal.len() as Balance);
    assert_ok!(Governance::create_proposal(Origin::signed(ALICE), proposal));
    assert_proposal_event()
}

#[test]
fn should_create_and_finalize_proposal() {
    run_test(|| {
        let proposal_hash = setup_proposal();

        // expire dispute period and execute
        System::set_block_number(System::block_number() + <Test as GovernanceConfig>::DisputePeriod::get());
        assert_ok!(Governance::finalize_proposal(Origin::signed(ALICE), proposal_hash));
        assert_eq!(<Balances as Currency<_>>::free_balance(&BOB), 100);
    })
}

#[test]
fn should_not_finalize_challenged_proposal() {
    run_test(|| {
        let proposal_hash = setup_proposal();

        // challenge and try execute
        assert_ok!(Governance::challenge_proposal(Origin::signed(ALICE), proposal_hash));
        assert_err!(
            Governance::finalize_proposal(Origin::signed(ALICE), proposal_hash),
            TestError::Challenged
        );
    })
}

#[test]
fn should_approve_and_execute_challenged_proposal() {
    run_test(|| {
        let proposal_hash = setup_proposal();

        // challenge and root approve
        assert_ok!(Governance::challenge_proposal(Origin::signed(ALICE), proposal_hash));
        assert_ok!(Governance::approve_proposal(Origin::root(), proposal_hash));
        assert_eq!(<Balances as Currency<_>>::free_balance(&BOB), 100);
    })
}

#[test]
fn should_reject_and_slash_challenged_proposal() {
    run_test(|| {
        let proposal_hash = setup_proposal();

        // challenge and root reject
        assert_ok!(Governance::challenge_proposal(Origin::signed(ALICE), proposal_hash));
        assert_ok!(Governance::reject_proposal(Origin::root(), proposal_hash));
        // set_balance was not successful
        assert_eq!(<Balances as Currency<_>>::free_balance(&BOB), 0);
        // reserved deposit was slashed
        assert_eq!(<Balances as Currency<_>>::total_balance(&ALICE), 0);
    })
}
