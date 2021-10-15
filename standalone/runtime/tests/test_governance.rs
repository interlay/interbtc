mod mock;
use crate::assert_eq;
use mock::*;

use frame_support::traits::{Currency, Hooks};
use pallet_democracy::{AccountVote, Conviction, Vote};
use sp_core::{Encode, Hasher};
use sp_runtime::traits::BlakeTwo256;

type DemocracyCall = pallet_democracy::Call<Runtime>;
type DemocracyPallet = pallet_democracy::Pallet<Runtime>;

type CouncilCall = pallet_collective::Call<Runtime, CouncilInstance>;
type CouncilEvent = pallet_collective::Event<Runtime, CouncilInstance>;

type SchedulerPallet = pallet_scheduler::Pallet<Runtime>;

type TechnicalCommitteeCall = pallet_collective::Call<Runtime, TechnicalCommitteeInstance>;
type TechnicalCommitteeEvent = pallet_collective::Event<Runtime, TechnicalCommitteeInstance>;

fn test_with<R>(execute: impl Fn() -> R) {
    ExtBuilder::build().execute_with(|| {
        assert_ok!(Call::Tokens(TokensCall::set_balance(
            account_of(ALICE),
            CurrencyId::INTR,
            5_000_000_000_000,
            0,
        ))
        .dispatch(root()));

        execute()
    });
}

fn set_balance_proposal(who: AccountId, value: u128) -> Vec<u8> {
    Call::Tokens(TokensCall::set_balance(who, CurrencyId::DOT, value, 0)).encode()
}

fn set_balance_proposal_hash(who: AccountId, value: u128) -> H256 {
    BlakeTwo256::hash(&set_balance_proposal(who, value)[..])
}

fn assert_council_proposal_event() -> (u32, H256) {
    let events = SystemModule::events();
    let record = events
        .iter()
        .rev()
        .find(|record| matches!(record.event, Event::Council(CouncilEvent::Proposed(_, _, _, _))));
    if let Event::Council(CouncilEvent::Proposed(_, index, hash, _)) = record.unwrap().event {
        (index, hash)
    } else {
        panic!("proposal event not found")
    }
}

fn assert_technical_committee_proposal_event() -> (u32, H256) {
    let events = SystemModule::events();
    let record = events.iter().rev().find(|record| {
        matches!(
            record.event,
            Event::TechnicalCommittee(TechnicalCommitteeEvent::Proposed(_, _, _, _))
        )
    });
    if let Event::TechnicalCommittee(TechnicalCommitteeEvent::Proposed(_, index, hash, _)) = record.unwrap().event {
        (index, hash)
    } else {
        panic!("proposal event not found")
    }
}

fn setup_council_proposal(amount_to_set: u128) {
    assert_ok!(Call::Democracy(DemocracyCall::note_preimage(set_balance_proposal(
        account_of(EVE),
        amount_to_set
    )))
    .dispatch(origin_of(account_of(ALICE))));

    // create motion to start simple-majority referendum
    assert_ok!(Call::Council(CouncilCall::propose(
        2, // member count
        Box::new(Call::Democracy(DemocracyCall::external_propose(
            set_balance_proposal_hash(account_of(EVE), 1000)
        ))),
        100000 // length bound
    ))
    .dispatch(origin_of(account_of(ALICE))));

    // unanimous council approves motion
    let (index, hash) = assert_council_proposal_event();
    assert_ok!(Call::Council(CouncilCall::vote(hash, index, true)).dispatch(origin_of(account_of(ALICE))));
    assert_ok!(Call::Council(CouncilCall::vote(hash, index, true)).dispatch(origin_of(account_of(BOB))));

    // vote is approved, should dispatch to democracy
    assert_ok!(Call::Council(CouncilCall::close(
        hash,
        index,
        10000000000, // weight bound
        100000       // length bound
    ))
    .dispatch(origin_of(account_of(ALICE))));
}

#[test]
fn integration_test_governance_council() {
    test_with(|| {
        let amount_to_set = 1000;
        setup_council_proposal(amount_to_set);

        // referenda should increase by 1 once launched
        assert_eq!(DemocracyPallet::referendum_count(), 0);
        let start_height = <Runtime as pallet_democracy::Config>::LaunchPeriod::get();
        DemocracyPallet::on_initialize(start_height);
        assert_eq!(DemocracyPallet::referendum_count(), 1);

        // vote overwhelmingly in favour
        assert_ok!(Call::Democracy(DemocracyCall::vote(
            0,
            AccountVote::Standard {
                vote: Vote {
                    aye: true,
                    conviction: Conviction::Locked1x
                },
                balance: 30_000_000
            }
        ))
        .dispatch(origin_of(account_of(ALICE))));

        // simulate end of voting period
        let end_height = start_height + <Runtime as pallet_democracy::Config>::VotingPeriod::get();
        DemocracyPallet::on_initialize(end_height);

        // simulate end of enactment period
        let act_height = end_height + <Runtime as pallet_democracy::Config>::EnactmentPeriod::get();
        SchedulerPallet::on_initialize(act_height);

        // balance is now set to amount above
        assert_eq!(CollateralPallet::total_balance(&account_of(EVE)), amount_to_set);
    });
}

#[test]
fn integration_test_governance_technical_committee() {
    test_with(|| {
        let amount_to_set = 1000;
        setup_council_proposal(amount_to_set);

        // create motion to fast-track simple-majority referendum
        assert_ok!(Call::TechnicalCommittee(TechnicalCommitteeCall::propose(
            2, // member count
            Box::new(Call::Democracy(DemocracyCall::fast_track(
                set_balance_proposal_hash(account_of(EVE), 1000),
                <Runtime as pallet_democracy::Config>::FastTrackVotingPeriod::get(),
                <Runtime as pallet_democracy::Config>::EnactmentPeriod::get()
            ))),
            100000 // length bound
        ))
        .dispatch(origin_of(account_of(ALICE))));

        // unanimous committee approves motion
        let (index, hash) = assert_technical_committee_proposal_event();
        assert_ok!(
            Call::TechnicalCommittee(TechnicalCommitteeCall::vote(hash, index, true))
                .dispatch(origin_of(account_of(ALICE)))
        );
        assert_ok!(
            Call::TechnicalCommittee(TechnicalCommitteeCall::vote(hash, index, true))
                .dispatch(origin_of(account_of(BOB)))
        );

        // vote is approved, should dispatch to democracy
        assert_ok!(Call::TechnicalCommittee(TechnicalCommitteeCall::close(
            hash,
            index,
            10000000000, // weight bound
            100000       // length bound
        ))
        .dispatch(origin_of(account_of(ALICE))));

        // referenda should increase by 1 once launched
        assert_eq!(DemocracyPallet::referendum_count(), 0);
        let start_height = <Runtime as pallet_democracy::Config>::LaunchPeriod::get();
        DemocracyPallet::on_initialize(start_height);
        assert_eq!(DemocracyPallet::referendum_count(), 1);
    });
}
