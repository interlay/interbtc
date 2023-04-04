mod mock;
use crate::assert_eq;
use mock::*;

use codec::Encode;
use democracy::{PropIndex, ReferendumIndex, ReferendumInfo, ReferendumStatus, Tally, Vote, VoteThreshold};
use frame_support::{
    assert_err_ignore_postinfo,
    traits::{Bounded, BoundedInline, Currency, GetCallMetadata, Hooks, StorePreimage},
};
use orml_vesting::VestingSchedule;
use tx_pause::FullNameOf;

type PreimageCall = pallet_preimage::Call<Runtime>;
type PreimageEvent = pallet_preimage::Event<Runtime>;

type DemocracyCall = democracy::Call<Runtime>;
type DemocracyPallet = democracy::Pallet<Runtime>;
type DemocracyEvent = democracy::Event<Runtime>;
type DemocracyError = democracy::Error<Runtime>;

type TechnicalCommitteeCall = pallet_collective::Call<Runtime, TechnicalCommitteeInstance>;
type TechnicalCommitteeEvent = pallet_collective::Event<Runtime, TechnicalCommitteeInstance>;

type TreasuryCall = pallet_treasury::Call<Runtime>;
type TreasuryPallet = pallet_treasury::Pallet<Runtime>;

type VestingCall = orml_vesting::Call<Runtime>;

const INITIAL_VOTING_POWER: Balance = 5_000_000_000_000;

fn get_max_locked(account_id: AccountId) -> Balance {
    TokensPallet::locks(&account_id, DEFAULT_NATIVE_CURRENCY)
        .iter()
        .map(|balance_lock| balance_lock.amount)
        .max()
        .unwrap_or_default()
}

fn create_lock(account_id: AccountId, amount: Balance) {
    assert_ok!(RuntimeCall::Escrow(EscrowCall::create_lock {
        amount,
        unlock_height: <Runtime as escrow::Config>::MaxPeriod::get()
    })
    .dispatch(origin_of(account_id)));
}

fn set_free_balance(account: AccountId, amount: Balance) {
    assert_ok!(RuntimeCall::Tokens(TokensCall::set_balance {
        who: account,
        currency_id: DEFAULT_NATIVE_CURRENCY,
        new_free: amount,
        new_reserved: 0,
    })
    .dispatch(root()));
}

fn test_with<R>(execute: impl Fn() -> R) {
    ExtBuilder::build().execute_with(|| {
        set_free_balance(account_of(ALICE), 10_000_000_000_000);
        create_lock(account_of(ALICE), INITIAL_VOTING_POWER);
        execute()
    });
}

fn assert_democracy_proposed_event() -> PropIndex {
    SystemPallet::events()
        .iter()
        .rev()
        .find_map(|record| {
            if let RuntimeEvent::Democracy(DemocracyEvent::Proposed { proposal_index, .. }) = record.event {
                Some(proposal_index)
            } else {
                None
            }
        })
        .expect("nothing was proposed")
}

fn assert_preimage_noted_event() -> H256 {
    SystemPallet::events()
        .iter()
        .rev()
        .find_map(|record| {
            if let RuntimeEvent::Preimage(PreimageEvent::Noted { hash }) = record.event {
                Some(hash)
            } else {
                None
            }
        })
        .expect("nothing was proposed")
}

fn assert_democracy_started_event() -> ReferendumIndex {
    SystemPallet::events()
        .iter()
        .rev()
        .find_map(|record| {
            if let RuntimeEvent::Democracy(DemocracyEvent::Started { ref_index, .. }) = record.event {
                Some(ref_index)
            } else {
                None
            }
        })
        .expect("referendum was not started")
}

fn assert_democracy_passed_event(index: ReferendumIndex) {
    SystemPallet::events()
        .iter()
        .rev()
        .find(|record| matches!(record.event, RuntimeEvent::Democracy(DemocracyEvent::Passed { ref_index }) if ref_index == index))
        .expect("referendum was not passed");
}

fn assert_technical_committee_executed_event() {
    SystemPallet::events()
        .iter()
        .rev()
        .find(|record| {
            matches!(
                record.event,
                RuntimeEvent::TechnicalCommittee(TechnicalCommitteeEvent::Executed { result: Ok(()), .. })
            )
        })
        .expect("execution failed");
}

fn set_balance_proposal(who: AccountId, value: Balance) -> RuntimeCall {
    RuntimeCall::Tokens(TokensCall::set_balance {
        who: who,
        currency_id: DEFAULT_COLLATERAL_CURRENCY,
        new_free: value,
        new_reserved: 0,
    })
}

fn create_proposal_call(proposal: RuntimeCall, deposit_value: Balance) -> RuntimeCall {
    RuntimeCall::Democracy(DemocracyCall::propose {
        proposal: <Runtime as democracy::Config>::Preimages::bound(proposal).unwrap(),
        value: deposit_value,
    })
}

fn create_proposal(proposal: RuntimeCall) {
    assert_ok!(
        create_proposal_call(proposal, <Runtime as democracy::Config>::MinimumDeposit::get())
            .dispatch(origin_of(account_of(ALICE)))
    );
}

fn create_set_balance_proposal(amount_to_set: Balance) {
    create_proposal(set_balance_proposal(account_of(EVE), amount_to_set))
}

fn launch_and_approve_referendum() -> (BlockNumber, ReferendumIndex) {
    let start_height = <Runtime as democracy::Config>::VotingPeriod::get();
    DemocracyPallet::on_initialize(start_height);
    let index = assert_democracy_started_event();

    // vote overwhelmingly in favour
    assert_ok!(RuntimeCall::Democracy(DemocracyCall::vote {
        ref_index: index,
        vote: Vote {
            aye: true,
            balance: 30_000_000,
        }
    })
    .dispatch(origin_of(account_of(ALICE))));

    (start_height, index)
}

fn launch_and_execute_referendum() {
    let (start_height, index) = launch_and_approve_referendum();

    // simulate end of voting period
    let end_height = start_height + <Runtime as democracy::Config>::VotingPeriod::get();
    DemocracyPallet::on_initialize(end_height);
    assert_democracy_passed_event(index);

    // simulate end of enactment period
    let act_height = end_height + <Runtime as democracy::Config>::EnactmentPeriod::get();
    SchedulerPallet::on_initialize(act_height);
}

fn full_name(pallet_name_bytes: &[u8], maybe_call_name_bytes: Option<&[u8]>) -> FullNameOf<Runtime> {
    match maybe_call_name_bytes {
        Some(call_name_bytes) => <FullNameOf<Runtime>>::from((
            pallet_name_bytes.to_vec().try_into().unwrap(),
            Some(call_name_bytes.to_vec().try_into().unwrap()),
        )),
        None => <FullNameOf<Runtime>>::from((pallet_name_bytes.to_vec().try_into().unwrap(), None)),
    }
}

#[test]
fn can_pause_pallets() {
    test_with(|| {
        let call = RuntimeCall::Tokens(TokensCall::transfer {
            dest: account_of(ALICE),
            currency_id: DEFAULT_NATIVE_CURRENCY,
            amount: 123,
        });

        // sanity check: this call is allowed by default
        assert_ok!(call.clone().dispatch(origin_of(account_of(ALICE))));

        // pause calls to Tokens pallet..
        assert_ok!(RuntimeCall::TxPause(TxPauseCall::pause {
            full_name: full_name(b"Tokens", None)
        })
        .dispatch(root()));

        assert_noop!(call.dispatch(origin_of(account_of(ALICE))), SystemError::CallFiltered);
    })
}

#[test]
fn can_pause_functions() {
    test_with(|| {
        let call = RuntimeCall::Tokens(TokensCall::transfer {
            dest: account_of(ALICE),
            currency_id: DEFAULT_NATIVE_CURRENCY,
            amount: 123,
        });

        // sanity check: this call is allowed by default
        assert_ok!(call.clone().dispatch(origin_of(account_of(ALICE))));

        // pause call..
        assert_ok!(RuntimeCall::TxPause(TxPauseCall::pause {
            full_name: full_name(b"Tokens", Some(b"transfer"))
        })
        .dispatch(root()));

        assert_noop!(call.dispatch(origin_of(account_of(ALICE))), SystemError::CallFiltered);

        // other functions from the Tokens pallet should continue to function
        assert_ok!(RuntimeCall::Tokens(TokensCall::transfer_all {
            dest: account_of(ALICE),
            currency_id: DEFAULT_NATIVE_CURRENCY,
            keep_alive: false,
        })
        .dispatch(origin_of(account_of(ALICE))));
    })
}

#[test]
fn pause_works_on_calls_in_batch() {
    test_with(|| {
        let call = RuntimeCall::Utility(UtilityCall::batch_all {
            calls: vec![RuntimeCall::Tokens(TokensCall::transfer {
                dest: account_of(ALICE),
                currency_id: DEFAULT_NATIVE_CURRENCY,
                amount: 123,
            })],
        });

        // sanity check: this call is allowed by default
        assert_ok!(call.clone().dispatch(origin_of(account_of(ALICE))));

        // pause call..
        assert_ok!(RuntimeCall::TxPause(TxPauseCall::pause {
            full_name: full_name(b"Tokens", Some(b"transfer"))
        })
        .dispatch(root()));

        // we need assert_err_ignore_postinfo since the call does execute, it only bails when the
        // nested call inside the batch gets dispatched
        assert_err_ignore_postinfo!(call.dispatch(origin_of(account_of(ALICE))), SystemError::CallFiltered);
    })
}

#[test]
fn can_not_use_txpause_to_brick_parachain() {
    test_with(|| {
        let call = RuntimeCall::Tokens(TokensCall::transfer {
            dest: account_of(ALICE),
            currency_id: DEFAULT_NATIVE_CURRENCY,
            amount: 123,
        });

        // sanity check: this call is allowed by default
        assert_ok!(call.clone().dispatch(origin_of(account_of(ALICE))));

        // tx-pause pallet itself is unpausable..
        assert_noop!(
            RuntimeCall::TxPause(TxPauseCall::pause {
                full_name: full_name(b"TxPause", None)
            })
            .dispatch(root()),
            tx_pause::Error::<Runtime>::IsUnpausable
        );
        let pausable_pallets = RuntimeCall::get_module_names()
            .into_iter()
            .filter(|&&pallet| pallet != "TxPause");
        for pallet in pausable_pallets {
            assert_ok!(RuntimeCall::TxPause(TxPauseCall::pause {
                full_name: full_name(pallet.as_bytes(), None)
            })
            .dispatch(root()));
        }

        // sanity check: this call is now paused
        assert_noop!(
            call.clone().dispatch(origin_of(account_of(ALICE))),
            SystemError::CallFiltered
        );

        create_proposal(RuntimeCall::TxPause(TxPauseCall::unpause {
            full_name: full_name(b"Tokens", None),
        }));
        launch_and_execute_referendum();

        // verify that call is indeed unpaused
        assert_ok!(call.clone().dispatch(origin_of(account_of(ALICE))));
    })
}

#[test]
fn integration_test_governance() {
    test_with(|| {
        let amount_to_set = 1000;
        create_set_balance_proposal(amount_to_set);
        launch_and_execute_referendum();

        // balance is now set to amount above
        assert_eq!(CollateralCurrency::total_balance(&account_of(EVE)), amount_to_set);
    });
}

#[test]
fn integration_test_governance_inline_proposal() {
    test_with(|| {
        let amount_to_set = 1000;

        let call = set_balance_proposal(account_of(EVE), amount_to_set);
        assert_ok!(RuntimeCall::Democracy(DemocracyCall::propose {
            proposal: Bounded::Inline(BoundedInline::try_from(call.encode()).unwrap()),
            value: <Runtime as democracy::Config>::MinimumDeposit::get(),
        })
        .dispatch(origin_of(account_of(ALICE))));

        launch_and_execute_referendum();

        // balance is now set to amount above
        assert_eq!(CollateralCurrency::total_balance(&account_of(EVE)), amount_to_set);
    });
}

#[test]
fn integration_test_governance_preimage_proposal() {
    test_with(|| {
        let amount_to_set = 1000;

        let call = set_balance_proposal(account_of(EVE), amount_to_set);
        assert_ok!(
            RuntimeCall::Preimage(PreimageCall::note_preimage { bytes: call.encode() })
                .dispatch(origin_of(account_of(ALICE)))
        );

        assert_ok!(RuntimeCall::Democracy(DemocracyCall::propose {
            proposal: Bounded::Lookup {
                hash: assert_preimage_noted_event(),
                len: call.encode().len() as u32
            },
            value: <Runtime as democracy::Config>::MinimumDeposit::get(),
        })
        .dispatch(origin_of(account_of(ALICE))));

        launch_and_execute_referendum();

        // balance is now set to amount above
        assert_eq!(CollateralCurrency::total_balance(&account_of(EVE)), amount_to_set);
    });
}

#[test]
fn integration_test_governance_fast_track() {
    test_with(|| {
        let amount_to_set = 1000;
        create_set_balance_proposal(amount_to_set);

        // create motion to fast-track simple-majority referendum
        assert_ok!(RuntimeCall::TechnicalCommittee(TechnicalCommitteeCall::propose {
            threshold: 1, // member count
            proposal: Box::new(RuntimeCall::Democracy(DemocracyCall::fast_track {
                prop_index: assert_democracy_proposed_event(),
                delay: <Runtime as democracy::Config>::EnactmentPeriod::get()
            })),
            length_bound: 100000 // length bound
        })
        .dispatch(origin_of(account_of(ALICE))));
        // should be executed immediately with only one member
        assert_technical_committee_executed_event();

        let (_, index) = launch_and_approve_referendum();
        let start_height = SystemPallet::block_number();

        // simulate end of voting period
        let end_height = start_height + <Runtime as democracy::Config>::FastTrackVotingPeriod::get();
        DemocracyPallet::on_initialize(end_height);
        assert_democracy_passed_event(index);
    });
}

#[test]
fn integration_test_governance_treasury() {
    test_with(|| {
        let balance_before = NativeCurrency::total_balance(&account_of(BOB));

        // fund treasury
        let amount_to_fund = 10000;
        set_free_balance(TreasuryPallet::account_id(), amount_to_fund);
        assert_eq!(TreasuryPallet::pot(), amount_to_fund);

        // proposals should increase by 1
        assert_eq!(TreasuryPallet::proposal_count(), 0);
        assert_ok!(RuntimeCall::Treasury(TreasuryCall::propose_spend {
            value: amount_to_fund,
            beneficiary: account_of(BOB)
        })
        .dispatch(origin_of(account_of(ALICE))));
        assert_eq!(TreasuryPallet::proposal_count(), 1);

        // create proposal to approve treasury spend
        create_proposal(RuntimeCall::Treasury(TreasuryCall::approve_proposal { proposal_id: 0 }));
        launch_and_execute_referendum();

        // bob should receive funds
        TreasuryPallet::spend_funds();
        assert_eq!(
            balance_before + amount_to_fund,
            NativeCurrency::total_balance(&account_of(BOB))
        )
    });
}

#[test]
fn integration_test_vested_escrow() {
    test_with(|| {
        // need free balance first to lock
        let vesting_amount = 10_000_000_000_000;
        set_free_balance(account_of(BOB), vesting_amount);

        // create vesting schedule to lock amount
        let vesting_schedule = VestingSchedule {
            start: 0,
            period: 10,
            period_count: 100,
            per_period: vesting_amount / 100,
        };
        assert_eq!(vesting_schedule.total_amount(), Some(vesting_amount));
        assert_ok!(RuntimeCall::Vesting(VestingCall::update_vesting_schedules {
            who: account_of(BOB),
            vesting_schedules: vec![vesting_schedule]
        })
        .dispatch(root()));
        assert_eq!(get_max_locked(account_of(BOB)), vesting_amount);

        // re-lock vested balance in escrow
        create_lock(account_of(BOB), vesting_amount);
        assert_eq!(get_max_locked(account_of(BOB)), vesting_amount);
    });
}

#[test]
fn integration_test_governance_voter_can_change_vote() {
    test_with(|| {
        let amount_to_set = 1000;
        create_set_balance_proposal(amount_to_set);

        let start_height = <Runtime as democracy::Config>::VotingPeriod::get();
        DemocracyPallet::on_initialize(start_height);
        let index = assert_democracy_started_event();

        assert_ok!(RuntimeCall::Democracy(DemocracyCall::vote {
            ref_index: index,
            vote: Vote {
                aye: true,
                balance: 30_000_000,
            }
        })
        .dispatch(origin_of(account_of(ALICE))));
        assert!(
            matches!(DemocracyPallet::referendum_info(index), Some(ReferendumInfo::Ongoing(status)) if status.tally.ayes == 30_000_000)
        );

        // can decrease vote amount
        assert_ok!(RuntimeCall::Democracy(DemocracyCall::vote {
            ref_index: index,
            vote: Vote {
                aye: true,
                balance: 20_000_000,
            }
        })
        .dispatch(origin_of(account_of(ALICE))));
        assert!(
            matches!(DemocracyPallet::referendum_info(index), Some(ReferendumInfo::Ongoing(status)) if status.tally.ayes == 20_000_000)
        );

        // can increase vote amount
        assert_ok!(RuntimeCall::Democracy(DemocracyCall::vote {
            ref_index: index,
            vote: Vote {
                aye: true,
                balance: 40_000_000,
            }
        })
        .dispatch(origin_of(account_of(ALICE))));
        assert!(
            matches!(DemocracyPallet::referendum_info(index), Some(ReferendumInfo::Ongoing(status)) if status.tally.ayes == 40_000_000)
        );

        // can change the vote direction
        assert_ok!(RuntimeCall::Democracy(DemocracyCall::vote {
            ref_index: index,
            vote: Vote {
                aye: false,
                balance: 20_000_000,
            }
        })
        .dispatch(origin_of(account_of(ALICE))));
        assert!(
            matches!(DemocracyPallet::referendum_info(index), Some(ReferendumInfo::Ongoing(status)) if status.tally.ayes == 0)
        );
        assert!(
            matches!(DemocracyPallet::referendum_info(index), Some(ReferendumInfo::Ongoing(status)) if status.tally.nays == 20_000_000)
        );
    });
}

#[test]
fn integration_test_fast_track_referendum() {
    test_with(|| {
        let amount_to_set = 1000;
        create_set_balance_proposal(amount_to_set);

        let start_height = <Runtime as democracy::Config>::VotingPeriod::get();
        DemocracyPallet::on_initialize(start_height);
        let index = assert_democracy_started_event();

        // create motion to fast-track simple-majority referendum
        assert_ok!(RuntimeCall::TechnicalCommittee(TechnicalCommitteeCall::propose {
            threshold: 1, // member count
            proposal: Box::new(RuntimeCall::Democracy(DemocracyCall::fast_track_referendum {
                ref_index: index,
            })),
            length_bound: 100000 // length bound
        })
        .dispatch(origin_of(account_of(ALICE))));
        // should be executed immediately with only one member
        assert_technical_committee_executed_event();

        let now = SystemPallet::block_number();
        let fast_track_voting_period = <Runtime as democracy::Config>::FastTrackVotingPeriod::get();
        let end = now + fast_track_voting_period;
        assert_eq!(
            DemocracyPallet::referendum_info(index),
            Some(ReferendumInfo::Ongoing(ReferendumStatus {
                end,
                proposal: <Runtime as democracy::Config>::Preimages::bound(set_balance_proposal(
                    account_of(EVE),
                    amount_to_set
                ))
                .unwrap(),
                threshold: VoteThreshold::SuperMajorityApprove,
                delay: <Runtime as democracy::Config>::EnactmentPeriod::get(),
                tally: Tally {
                    ayes: 0,
                    nays: 0,
                    turnout: 0
                },
            }))
        );
    });
}

#[test]
fn integration_test_governance_voter_can_change_vote_with_limited_funds() {
    test_with(|| {
        let amount_to_set = 1000;
        create_set_balance_proposal(amount_to_set);

        let start_height = <Runtime as democracy::Config>::VotingPeriod::get();
        DemocracyPallet::on_initialize(start_height);
        let index = assert_democracy_started_event();

        let max_period = <Runtime as escrow::Config>::MaxPeriod::get() as u128;
        let expected_voting_power = INITIAL_VOTING_POWER - INITIAL_VOTING_POWER % max_period;

        set_free_balance(account_of(BOB), expected_voting_power);

        let start = <Runtime as escrow::Config>::Span::get();
        SystemPallet::set_block_number(start);

        let lock_duration = <Runtime as escrow::Config>::MaxPeriod::get();
        assert_ok!(RuntimeCall::Escrow(EscrowCall::create_lock {
            amount: expected_voting_power - max_period,
            unlock_height: start + lock_duration
        })
        .dispatch(origin_of(account_of(BOB))));

        assert_ok!(RuntimeCall::Democracy(DemocracyCall::vote {
            ref_index: index,
            vote: Vote {
                aye: true,
                balance: expected_voting_power - max_period,
            }
        })
        .dispatch(origin_of(account_of(BOB))));

        assert_ok!(RuntimeCall::Escrow(EscrowCall::increase_amount { amount: max_period })
            .dispatch(origin_of(account_of(BOB))));

        assert_ok!(RuntimeCall::Democracy(DemocracyCall::vote {
            ref_index: index,
            vote: Vote {
                aye: true,
                balance: expected_voting_power,
            }
        })
        .dispatch(origin_of(account_of(BOB))));
    })
}
#[test]
fn integration_test_create_lock_half_max_period() {
    ExtBuilder::build().execute_with(|| {
        set_free_balance(account_of(ALICE), 10_000_000_000_000);
        let max_period = <Runtime as escrow::Config>::MaxPeriod::get() as u128;

        let start = <Runtime as escrow::Config>::Span::get();
        SystemPallet::set_block_number(start);

        let lock_duration = <Runtime as escrow::Config>::MaxPeriod::get() / 2;

        assert_ok!(RuntimeCall::Escrow(EscrowCall::create_lock {
            amount: INITIAL_VOTING_POWER,
            unlock_height: start + lock_duration
        })
        .dispatch(origin_of(account_of(ALICE))));

        // initial voting power is rounded down to a multiple of max_period. We get only 50% of voting power for locking
        // half time
        let expected_voting_power = (INITIAL_VOTING_POWER - INITIAL_VOTING_POWER % max_period) / 2;
        assert_eq!(
            <Runtime as democracy::Config>::Currency::total_issuance(),
            expected_voting_power
        );

        SystemPallet::set_block_number(start + lock_duration / 2);
        assert_eq!(
            <Runtime as democracy::Config>::Currency::total_issuance(),
            expected_voting_power / 2
        );

        SystemPallet::set_block_number(start + lock_duration);
        assert_eq!(<Runtime as democracy::Config>::Currency::total_issuance(), 0);
    })
}

#[test]
fn integration_test_create_lock_halfway_span() {
    ExtBuilder::build().execute_with(|| {
        set_free_balance(account_of(ALICE), 10_000_000_000_000);

        let span = <Runtime as escrow::Config>::Span::get() as u128;
        let max_period = <Runtime as escrow::Config>::MaxPeriod::get() as u128;
        let num_spans = max_period / span;

        let start = <Runtime as escrow::Config>::Span::get() / 2;
        SystemPallet::set_block_number(start);

        assert_ok!(RuntimeCall::Escrow(EscrowCall::create_lock {
            amount: INITIAL_VOTING_POWER,
            unlock_height: <Runtime as escrow::Config>::MaxPeriod::get() + start
        })
        .dispatch(origin_of(account_of(ALICE))));

        // initial voting power is rounded down to a multiple of max_period
        let initial_voting_power = INITIAL_VOTING_POWER - INITIAL_VOTING_POWER % max_period;

        // we are locking for a period of (max_period - span / 2), since our unlock_height got rounded down.
        // We have to correct for the half-span worth of locking power that we lose out on
        let expected_voting_power = initial_voting_power - initial_voting_power / (2 * num_spans);
        assert_eq!(
            <Runtime as democracy::Config>::Currency::total_issuance(),
            expected_voting_power
        );

        SystemPallet::set_block_number(<Runtime as escrow::Config>::MaxPeriod::get() / 2);
        assert_eq!(
            <Runtime as democracy::Config>::Currency::total_issuance(),
            initial_voting_power / 2
        );
    })
}

#[test]
fn integration_test_vote_exceeds_total_voting_power() {
    ExtBuilder::build().execute_with(|| {
        set_free_balance(account_of(ALICE), 10_000_000_000_000_000_000_000);

        // we choose a referendum height that is both on a SPAN and VotingPeriod boundary
        let referendum_height =
            <Runtime as democracy::Config>::VotingPeriod::get() * <Runtime as escrow::Config>::Span::get();
        let start_height = referendum_height - <Runtime as democracy::Config>::VotingPeriod::get();
        let end_height = start_height + <Runtime as democracy::Config>::VotingPeriod::get();

        SystemPallet::set_block_number(start_height);
        assert_ok!(RuntimeCall::Escrow(EscrowCall::create_lock {
            amount: 10_000_000_000_000_000_000_000,
            unlock_height: end_height
        })
        .dispatch(origin_of(account_of(ALICE))));

        create_set_balance_proposal(1000);
        DemocracyPallet::on_initialize(start_height);
        let index = assert_democracy_started_event();

        // vote in favour
        assert_ok!(RuntimeCall::Democracy(DemocracyCall::vote {
            ref_index: index,
            vote: Vote {
                aye: true,
                balance: 1000,
            }
        })
        .dispatch(origin_of(account_of(ALICE))));

        // simulate end of voting period
        SystemPallet::set_block_number(end_height);
        DemocracyPallet::on_initialize(end_height);

        // total voting power should have decayed to zero
        assert_eq!(<Runtime as democracy::Config>::Currency::total_issuance(), 0);
        // but vote passed due to the vote in favour
        assert_democracy_passed_event(index);
    });
}

#[test]
fn integration_test_proposing_and_voting_only_possible_with_staked_tokens() {
    ExtBuilder::build().execute_with(|| {
        let minimum_proposal_value = <Runtime as democracy::Config>::MinimumDeposit::get();
        let start_height = <Runtime as democracy::Config>::VotingPeriod::get();

        // making a proposal to increase Eve's balance without having tokens staked fails
        let amount_to_fund = 100_000;

        let proposal = create_proposal_call(
            set_balance_proposal(account_of(EVE), amount_to_fund),
            minimum_proposal_value,
        );
        assert_noop!(
            proposal.clone().dispatch(origin_of(account_of(BOB))),
            EscrowError::InsufficientFunds
        );

        // Create free balance for Bob, Carol, and Dave
        set_free_balance(account_of(BOB), 10 * minimum_proposal_value);
        set_free_balance(account_of(CAROL), 10 * minimum_proposal_value);
        set_free_balance(account_of(DAVE), 10 * minimum_proposal_value);

        // Bob stakes 50% of tokens and proposes again
        create_lock(account_of(BOB), 5 * minimum_proposal_value);
        assert_ok!(proposal.dispatch(origin_of(account_of(BOB))));

        // Carol fails to second the proposal without having tokens staked
        let prop_index = assert_democracy_proposed_event();
        assert_noop!(
            RuntimeCall::Democracy(DemocracyCall::second { proposal: prop_index })
                .dispatch(origin_of(account_of(CAROL))),
            EscrowError::InsufficientFunds
        );

        // Carol succeeds to second the proposal with staking tokens beforehand
        create_lock(account_of(CAROL), 5 * minimum_proposal_value);
        assert_ok!(RuntimeCall::Democracy(DemocracyCall::second { proposal: prop_index })
            .dispatch(origin_of(account_of(CAROL))));

        // Proceed proposal to a referendum
        DemocracyPallet::on_initialize(start_height);
        let ref_index = assert_democracy_started_event();

        // Dave cannot vote since no tokens are staked
        assert_noop!(
            RuntimeCall::Democracy(DemocracyCall::vote {
                ref_index,
                vote: Vote {
                    aye: true,
                    balance: 5 * minimum_proposal_value,
                }
            })
            .dispatch(origin_of(account_of(DAVE))),
            DemocracyError::InsufficientFunds
        );

        // Bob votes aye
        assert_ok!(RuntimeCall::Democracy(DemocracyCall::vote {
            ref_index,
            vote: Vote {
                aye: true,
                balance: 3 * minimum_proposal_value,
            }
        })
        .dispatch(origin_of(account_of(BOB))));

        // simulate end of voting period
        let end_height = start_height + <Runtime as democracy::Config>::VotingPeriod::get();
        DemocracyPallet::on_initialize(end_height);
        assert_democracy_passed_event(ref_index);

        // simulate end of enactment period
        let act_height = end_height + <Runtime as democracy::Config>::EnactmentPeriod::get();
        SchedulerPallet::on_initialize(act_height);

        // Eve should receive funds
        TreasuryPallet::spend_funds();
        assert_eq!(amount_to_fund, CollateralCurrency::total_balance(&account_of(EVE)))
    });
}

fn get_free_vkint(account: AccountId) -> Balance {
    <Runtime as democracy::Config>::Currency::free_balance(&account)
}

#[test]
fn integration_test_proposal_vkint_gets_released_on_regular_launch() {
    test_with(|| {
        let minimum_proposal_value = <Runtime as democracy::Config>::MinimumDeposit::get();
        assert!(minimum_proposal_value > 0); // sanity check - the test would be useless otherwise

        set_free_balance(account_of(CAROL), 10 * minimum_proposal_value);
        create_lock(account_of(CAROL), 5 * minimum_proposal_value);

        let start_vkint_alice = get_free_vkint(account_of(ALICE));
        let start_vkint_carol = get_free_vkint(account_of(CAROL));

        // making a proposal to increase Eve's balance without having tokens staked fails
        assert_ok!(
            create_proposal_call(set_balance_proposal(account_of(EVE), 100_000), minimum_proposal_value)
                .dispatch(origin_of(account_of(ALICE)))
        );

        // alice should have locked some vkint
        assert_eq!(
            get_free_vkint(account_of(ALICE)),
            start_vkint_alice - minimum_proposal_value
        );

        assert_ok!(RuntimeCall::Democracy(DemocracyCall::second { proposal: 0 }).dispatch(origin_of(account_of(CAROL))));

        // now both alice and carol should have locked some vkint
        assert_eq!(
            get_free_vkint(account_of(ALICE)),
            start_vkint_alice - minimum_proposal_value
        );
        assert_eq!(
            get_free_vkint(account_of(CAROL)),
            start_vkint_carol - minimum_proposal_value
        );

        DemocracyPallet::on_initialize(<Runtime as democracy::Config>::VotingPeriod::get());

        // now that it's no longer a proposal, the deposit should be released
        assert_eq!(get_free_vkint(account_of(ALICE)), start_vkint_alice);
        assert_eq!(get_free_vkint(account_of(CAROL)), start_vkint_carol);
    });
}

#[test]
fn integration_test_proposal_vkint_gets_released_on_fast_track() {
    test_with(|| {
        let minimum_proposal_value = <Runtime as democracy::Config>::MinimumDeposit::get();
        assert!(minimum_proposal_value > 0); // sanity check - the test would be useless otherwise

        let start_vkint_alice = get_free_vkint(account_of(ALICE));

        // making a proposal to increase Eve's balance without having tokens staked fails
        assert_ok!(
            create_proposal_call(set_balance_proposal(account_of(EVE), 100_000), minimum_proposal_value)
                .dispatch(origin_of(account_of(ALICE)))
        );

        // alice should have locked some vkint
        assert_eq!(
            get_free_vkint(account_of(ALICE)),
            start_vkint_alice - minimum_proposal_value
        );

        assert_ok!(RuntimeCall::TechnicalCommittee(TechnicalCommitteeCall::propose {
            threshold: 1, // member count
            proposal: Box::new(RuntimeCall::Democracy(DemocracyCall::fast_track {
                prop_index: assert_democracy_proposed_event(),
                delay: <Runtime as democracy::Config>::EnactmentPeriod::get()
            })),
            length_bound: 100000 // length bound
        })
        .dispatch(origin_of(account_of(ALICE))));

        // now that it's no longer a proposal, the deposit should be released
        assert_eq!(get_free_vkint(account_of(ALICE)), start_vkint_alice);
    });
}

fn set_block_number(block_number: u32) {
    SystemPallet::set_block_number(block_number);
    DemocracyPallet::on_initialize(block_number);
}

#[test]
fn integration_test_limiting_voting_power_works() {
    let lock_time = <Runtime as escrow::Config>::MaxPeriod::get();
    let kint_amount = <Runtime as escrow::Config>::MaxPeriod::get() as u128 * 1000;
    let limit_start = 500;
    let limit_end = 1500;
    let limit_period = limit_end - limit_start;

    let setup = || {
        set_free_balance(account_of(BOB), kint_amount);

        assert_ok!(RuntimeCall::Escrow(EscrowCall::set_account_limit {
            who: account_of(BOB),
            start: 500,
            end: 1500,
        })
        .dispatch(root()));

        set_block_number(1);
        assert_eq!(get_free_vkint(account_of(BOB)), 0);
    };

    let assert_minting_limit = |amount| {
        assert_noop!(
            RuntimeCall::Escrow(EscrowCall::create_lock {
                amount: amount + 1,
                unlock_height: lock_time
            })
            .dispatch(origin_of(account_of(BOB))),
            EscrowError::InsufficientFunds
        );

        assert_ok!(RuntimeCall::Escrow(EscrowCall::create_lock {
            amount,
            unlock_height: lock_time
        })
        .dispatch(origin_of(account_of(BOB))));
    };

    let test_minting_limit_at = |block, limit| {
        test_with(|| {
            setup();
            set_block_number(block);
            assert_minting_limit(limit);
        });
    };

    test_minting_limit_at(limit_start + limit_period / 4, kint_amount / 4);
    test_minting_limit_at(limit_start + limit_period / 2, kint_amount / 2);
    test_minting_limit_at(limit_start + limit_period, kint_amount);
    test_minting_limit_at(limit_start + limit_period * 2, kint_amount);
}

#[test]
fn test_sudo_is_disabled_if_key_is_none() {
    test_with(|| {
        // first a sanity check: sudo works if key is set
        assert_ok!(RuntimeCall::Sudo(SudoCall::sudo {
            call: Box::new(RuntimeCall::Security(SecurityCall::set_parachain_status {
                status_code: StatusCode::Error,
            })),
        })
        .dispatch(origin_of(account_of(ALICE))),);

        use frame_support::storage::migration::take_storage_value;
        assert!(!pallet_sudo::Pallet::<Runtime>::key().is_none());
        take_storage_value::<AccountId>(b"Sudo", b"Key", &[]);
        assert!(pallet_sudo::Pallet::<Runtime>::key().is_none());

        // assert that sudo does not work when key is none
        assert_noop!(
            RuntimeCall::Sudo(SudoCall::sudo {
                call: Box::new(RuntimeCall::Security(SecurityCall::set_parachain_status {
                    status_code: StatusCode::Error,
                })),
            })
            .dispatch(origin_of(account_of(ALICE))),
            SudoError::RequireSudo
        );
    })
}
