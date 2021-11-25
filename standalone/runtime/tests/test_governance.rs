mod mock;
use crate::assert_eq;
use mock::*;

use democracy::{ReferendumIndex, Vote};
use frame_support::traits::{Currency, Hooks};
use pallet_collective::ProposalIndex;
use sp_core::{Encode, Hasher};
use sp_runtime::traits::BlakeTwo256;

type DemocracyCall = democracy::Call<Runtime>;
type DemocracyPallet = democracy::Pallet<Runtime>;
type DemocracyEvent = democracy::Event<Runtime>;

type SchedulerPallet = pallet_scheduler::Pallet<Runtime>;

type TechnicalCommitteeCall = pallet_collective::Call<Runtime, TechnicalCommitteeInstance>;
type TechnicalCommitteeEvent = pallet_collective::Event<Runtime, TechnicalCommitteeInstance>;

type TreasuryCall = pallet_treasury::Call<Runtime>;
type TreasuryPallet = pallet_treasury::Pallet<Runtime>;

const COLLATERAL_CURRENCY_ID: CurrencyId = CurrencyId::DOT;
const NATIVE_CURRENCY_ID: CurrencyId = CurrencyId::INTR;

const DEMOCRACY_VOTE_AMOUNT: u128 = 30_000_000;

fn test_with<R>(execute: impl Fn() -> R) {
    ExtBuilder::build().execute_with(|| {
        assert_ok!(Call::Tokens(TokensCall::set_balance {
            who: account_of(ALICE),
            currency_id: NATIVE_CURRENCY_ID,
            new_free: 5_000_000_000_000,
            new_reserved: 0,
        })
        .dispatch(root()));

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

fn set_balance_proposal_hash(who: AccountId, value: u128) -> H256 {
    BlakeTwo256::hash(&set_balance_proposal(who, value)[..])
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
        .expect("external referendum was not started")
}

fn assert_democracy_passed_event(index: ReferendumIndex) {
    SystemPallet::events()
        .iter()
        .rev()
        .find(|record| matches!(record.event, Event::Democracy(DemocracyEvent::Passed(i)) if i == index))
        .expect("external referendum was not passed");
}

fn assert_technical_committee_proposal_event() -> (ProposalIndex, H256) {
    SystemPallet::events()
        .iter()
        .rev()
        .find_map(|record| {
            if let Event::TechnicalCommittee(TechnicalCommitteeEvent::Proposed(_, index, hash, _)) = record.event {
                Some((index, hash))
            } else {
                None
            }
        })
        .expect("proposal event not found")
}

fn get_total_locked(account_id: AccountId) -> u128 {
    TokensPallet::locks(&account_id, NATIVE_CURRENCY_ID)
        .iter()
        .map(|balance_lock| balance_lock.amount)
        .reduce(|accum, item| accum + item)
        .unwrap_or_default()
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

fn setup_proposal(amount_to_set: u128) {
    assert_ok!(Call::Democracy(DemocracyCall::note_preimage {
        encoded_proposal: set_balance_proposal(account_of(EVE), amount_to_set)
    })
    .dispatch(origin_of(account_of(ALICE))));

    assert_ok!(Call::Democracy(DemocracyCall::propose {
        proposal_hash: set_balance_proposal_hash(account_of(EVE), amount_to_set),
        value: <Runtime as democracy::Config>::MinimumDeposit::get(),
    })
    .dispatch(origin_of(account_of(ALICE))));
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

// #[test]
// fn integration_test_governance() {
//     test_with(|| {
//         let amount_to_set = 1000;
//         setup_proposal(amount_to_set);
//         let (start_height, index) = launch_and_approve_referendum();

//         // simulate end of voting period
//         let end_height = start_height + <Runtime as democracy::Config>::VotingPeriod::get();
//         DemocracyPallet::on_initialize(end_height);
//         assert_democracy_passed_event(index);

//         // simulate end of enactment period
//         let act_height = end_height + <Runtime as democracy::Config>::EnactmentPeriod::get();
//         SchedulerPallet::on_initialize(act_height);

//         // balance is now set to amount above
//         assert_eq!(CollateralCurrency::total_balance(&account_of(EVE)), amount_to_set);
//     });
// }

// #[test]
// fn integration_test_governance_technical_committee() {
//     test_with(|| {
//         let amount_to_set = 1000;
//         setup_proposal(amount_to_set);

//         // create motion to fast-track simple-majority referendum
//         assert_ok!(Call::TechnicalCommittee(TechnicalCommitteeCall::propose {
//             threshold: 2, // member count
//             proposal: Box::new(Call::Democracy(DemocracyCall::fast_track {
//                 proposal_hash: set_balance_proposal_hash(account_of(EVE), 1000),
//                 voting_period: <Runtime as democracy::Config>::FastTrackVotingPeriod::get(),
//                 delay: <Runtime as democracy::Config>::EnactmentPeriod::get()
//             })),
//             length_bound: 100000 // length bound
//         })
//         .dispatch(origin_of(account_of(ALICE))));

//         // unanimous committee approves motion
//         let (index, hash) = assert_technical_committee_proposal_event();
//         assert_ok!(Call::TechnicalCommittee(TechnicalCommitteeCall::vote {
//             proposal: hash,
//             index: index,
//             approve: true
//         })
//         .dispatch(origin_of(account_of(ALICE))));
//         assert_ok!(Call::TechnicalCommittee(TechnicalCommitteeCall::vote {
//             proposal: hash,
//             index: index,
//             approve: true
//         })
//         .dispatch(origin_of(account_of(BOB))));

//         // vote is approved, should dispatch to democracy
//         assert_ok!(Call::TechnicalCommittee(TechnicalCommitteeCall::close {
//             proposal_hash: hash,
//             index: index,
//             proposal_weight_bound: 10000000000, // weight bound
//             length_bound: 100000                // length bound
//         })
//         .dispatch(origin_of(account_of(ALICE))));

//         let (_, index) = launch_and_approve_referendum();
//         let start_height = SystemPallet::block_number();

//         // simulate end of voting period
//         let end_height = start_height + <Runtime as democracy::Config>::FastTrackVotingPeriod::get();
//         DemocracyPallet::on_initialize(end_height);
//         assert_democracy_passed_event(index);
//     });
// }

// #[test]
// fn integration_test_governance_treasury() {
//     test_with(|| {
//         let balance_before = NativeCurrency::total_balance(&account_of(BOB));

//         // fund treasury
//         let amount_to_fund = 10000;
//         assert_ok!(Call::Tokens(TokensCall::set_balance {
//             who: TreasuryPallet::account_id(),
//             currency_id: NATIVE_CURRENCY_ID,
//             new_free: amount_to_fund,
//             new_reserved: 0,
//         })
//         .dispatch(root()));
//         assert_eq!(TreasuryPallet::pot(), amount_to_fund);

//         // proposals should increase by 1
//         assert_eq!(TreasuryPallet::proposal_count(), 0);
//         assert_ok!(Call::Treasury(TreasuryCall::propose_spend {
//             value: amount_to_fund,
//             beneficiary: account_of(BOB)
//         })
//         .dispatch(origin_of(account_of(ALICE))));
//         assert_eq!(TreasuryPallet::proposal_count(), 1);

//         // create motion to approve treasury spend
//         propose_and_approve_motion(Box::new(Call::Treasury(TreasuryCall::approve_proposal {
//             proposal_id: 0,
//         })));

//         // bob should receive funds
//         TreasuryPallet::spend_funds();
//         assert_eq!(
//             balance_before + amount_to_fund,
//             NativeCurrency::total_balance(&account_of(BOB))
//         )
//     });
// }
