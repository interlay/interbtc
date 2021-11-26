mod mock;
use crate::assert_eq;
use mock::*;

use democracy::{PropIndex, ReferendumIndex, Vote};
use frame_support::traits::{Currency, Hooks};
use sp_core::{Encode, Hasher};
use sp_runtime::traits::BlakeTwo256;

type Balance = u128;

type DemocracyCall = democracy::Call<Runtime>;
type DemocracyPallet = democracy::Pallet<Runtime>;
type DemocracyEvent = democracy::Event<Runtime>;

type EscrowCall = escrow::Call<Runtime>;

type SchedulerPallet = pallet_scheduler::Pallet<Runtime>;

type TechnicalCommitteeCall = pallet_collective::Call<Runtime, TechnicalCommitteeInstance>;
type TechnicalCommitteeEvent = pallet_collective::Event<Runtime, TechnicalCommitteeInstance>;

type TreasuryCall = pallet_treasury::Call<Runtime>;
type TreasuryPallet = pallet_treasury::Pallet<Runtime>;

const COLLATERAL_CURRENCY_ID: CurrencyId = CurrencyId::DOT;
const NATIVE_CURRENCY_ID: CurrencyId = CurrencyId::INTR;

const DEMOCRACY_VOTE_AMOUNT: u128 = 30_000_000;

fn create_lock(account_id: AccountId, amount: Balance) {
    assert_ok!(Call::Escrow(EscrowCall::create_lock {
        amount,
        unlock_height: <Runtime as escrow::Config>::MaxPeriod::get()
    })
    .dispatch(origin_of(account_id)));
}

fn test_with<R>(execute: impl Fn() -> R) {
    ExtBuilder::build().execute_with(|| {
        assert_ok!(Call::Tokens(TokensCall::set_balance {
            who: account_of(ALICE),
            currency_id: NATIVE_CURRENCY_ID,
            new_free: 10_000_000_000_000,
            new_reserved: 0,
        })
        .dispatch(root()));
        create_lock(account_of(ALICE), 5_000_000_000_000);

        execute()
    });
}

fn set_balance_proposal(who: AccountId, value: u128) -> Vec<u8> {
    Call::Tokens(TokensCall::set_balance {
        who: who,
        currency_id: COLLATERAL_CURRENCY_ID,
        new_free: value,
        new_reserved: 0,
    })
    .encode()
}

fn assert_democracy_proposed_event() -> PropIndex {
    SystemPallet::events()
        .iter()
        .rev()
        .find_map(|record| {
            if let Event::Democracy(DemocracyEvent::Proposed(index, _)) = record.event {
                Some(index)
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
            if let Event::Democracy(DemocracyEvent::Started(index, _)) = record.event {
                Some(index)
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
        .find(|record| matches!(record.event, Event::Democracy(DemocracyEvent::Passed(i)) if i == index))
        .expect("referendum was not passed");
}

fn assert_technical_committee_executed_event() {
    SystemPallet::events()
        .iter()
        .rev()
        .find(|record| {
            matches!(
                record.event,
                Event::TechnicalCommittee(TechnicalCommitteeEvent::Executed(_, Ok(())))
            )
        })
        .expect("execution failed");
}

fn create_proposal(encoded_proposal: Vec<u8>) {
    let proposal_hash = BlakeTwo256::hash(&encoded_proposal[..]);

    assert_ok!(
        Call::Democracy(DemocracyCall::note_preimage { encoded_proposal }).dispatch(origin_of(account_of(ALICE)))
    );

    assert_ok!(Call::Democracy(DemocracyCall::propose {
        proposal_hash,
        value: <Runtime as democracy::Config>::MinimumDeposit::get(),
    })
    .dispatch(origin_of(account_of(ALICE))));
}

fn create_set_balance_proposal(amount_to_set: Balance) {
    create_proposal(set_balance_proposal(account_of(EVE), amount_to_set))
}

fn launch_and_approve_referendum() -> (BlockNumber, ReferendumIndex) {
    let start_height = <Runtime as democracy::Config>::LaunchPeriod::get();
    DemocracyPallet::on_initialize(start_height);
    let index = assert_democracy_started_event();

    // vote overwhelmingly in favour
    assert_ok!(Call::Democracy(DemocracyCall::vote {
        ref_index: index,
        vote: Vote {
            aye: true,
            balance: DEMOCRACY_VOTE_AMOUNT,
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

#[test]
fn can_recover_from_shutdown() {
    test_with(|| {
        // use sudo to set parachain status
        assert_ok!(Call::Sudo(SudoCall::sudo {
            call: Box::new(Call::Security(SecurityCall::set_parachain_status {
                status_code: StatusCode::Shutdown,
            })),
        })
        .dispatch(origin_of(account_of(ALICE))));

        // verify we cant execute normal calls
        assert_noop!(
            Call::Tokens(TokensCall::transfer {
                dest: account_of(ALICE),
                currency_id: NATIVE_CURRENCY_ID,
                amount: 123,
            })
            .dispatch(origin_of(account_of(ALICE))),
            DispatchError::BadOrigin
        );

        // use sudo to set parachain status back to running
        assert_ok!(Call::Sudo(SudoCall::sudo {
            call: Box::new(Call::Security(SecurityCall::set_parachain_status {
                status_code: StatusCode::Running,
            }))
        })
        .dispatch(origin_of(account_of(ALICE))));

        // verify that we can execute normal calls again
        assert_ok!(Call::Tokens(TokensCall::transfer {
            dest: account_of(ALICE),
            currency_id: NATIVE_CURRENCY_ID,
            amount: 123,
        })
        .dispatch(origin_of(account_of(ALICE))));
    });
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
fn integration_test_governance_fast_track() {
    test_with(|| {
        let amount_to_set = 1000;
        create_set_balance_proposal(amount_to_set);

        // create motion to fast-track simple-majority referendum
        assert_ok!(Call::TechnicalCommittee(TechnicalCommitteeCall::propose {
            threshold: 1, // member count
            proposal: Box::new(Call::Democracy(DemocracyCall::fast_track {
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
        assert_ok!(Call::Tokens(TokensCall::set_balance {
            who: TreasuryPallet::account_id(),
            currency_id: NATIVE_CURRENCY_ID,
            new_free: amount_to_fund,
            new_reserved: 0,
        })
        .dispatch(root()));
        assert_eq!(TreasuryPallet::pot(), amount_to_fund);

        // proposals should increase by 1
        assert_eq!(TreasuryPallet::proposal_count(), 0);
        assert_ok!(Call::Treasury(TreasuryCall::propose_spend {
            value: amount_to_fund,
            beneficiary: account_of(BOB)
        })
        .dispatch(origin_of(account_of(ALICE))));
        assert_eq!(TreasuryPallet::proposal_count(), 1);

        // create proposal to approve treasury spend
        create_proposal(Call::Treasury(TreasuryCall::approve_proposal { proposal_id: 0 }).encode());
        launch_and_execute_referendum();

        // bob should receive funds
        TreasuryPallet::spend_funds();
        assert_eq!(
            balance_before + amount_to_fund,
            NativeCurrency::total_balance(&account_of(BOB))
        )
    });
}
