//! Democracy pallet benchmarking.

use super::*;

use frame_benchmarking::{account, benchmarks, whitelist_account};
use frame_support::{
    codec::Decode,
    traits::{schedule::DispatchTime, Currency, Get, OnInitialize, UnfilteredDispatchable},
};
use frame_system::{Pallet as System, RawOrigin};
use sp_runtime::traits::{BadOrigin, Bounded, One};

use crate::Pallet as Democracy;

const SEED: u32 = 0;
const MAX_REFERENDUMS: u32 = 99;
const MAX_SECONDERS: u32 = 100;
const MAX_BYTES: u32 = 16_384;

fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
    frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

fn funded_account<T: Config>(name: &'static str, index: u32) -> T::AccountId {
    let caller: T::AccountId = account(name, index, SEED);
    T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
    caller
}

fn add_proposal<T: Config>(n: u32) -> Result<PropIndex, &'static str> {
    let other = funded_account::<T>("proposer", n);
    let value = T::MinimumDeposit::get();
    let proposal_hash: T::Hash = T::Hashing::hash_of(&n);
    let prop_index: PropIndex = PublicPropCount::<T>::get();

    Democracy::<T>::propose(RawOrigin::Signed(other).into(), proposal_hash, value.into())?;

    Ok(prop_index)
}

fn add_referendum<T: Config>(n: u32) -> Result<ReferendumIndex, &'static str> {
    let proposal_hash: T::Hash = T::Hashing::hash_of(&n);
    let vote_threshold = VoteThreshold::SimpleMajority;

    Democracy::<T>::inject_referendum(T::LaunchPeriod::get(), proposal_hash, vote_threshold, 0u32.into());
    let referendum_index: ReferendumIndex = ReferendumCount::<T>::get() - 1;
    T::Scheduler::schedule_named(
        (DEMOCRACY_ID, referendum_index).encode(),
        DispatchTime::At(2u32.into()),
        None,
        63,
        frame_system::RawOrigin::Root.into(),
        Call::enact_proposal {
            proposal_hash,
            index: referendum_index,
        }
        .into(),
    )
    .map_err(|_| "failed to schedule named")?;
    Ok(referendum_index)
}

fn account_vote<T: Config>(b: BalanceOf<T>) -> Vote<BalanceOf<T>> {
    Vote { aye: true, balance: b }
}

benchmarks! {
    propose {
        let p = T::MaxProposals::get();

        for i in 0 .. (p - 1) {
            add_proposal::<T>(i)?;
        }

        let caller = funded_account::<T>("caller", 0);
        let proposal_hash: T::Hash = T::Hashing::hash_of(&0);
        let value = T::MinimumDeposit::get();
        whitelist_account!(caller);
    }: _(RawOrigin::Signed(caller), proposal_hash, value.into())
    verify {
        assert_eq!(Democracy::<T>::public_props().len(), p as usize, "Proposals not created.");
    }

    second {
        let s in 0 .. MAX_SECONDERS;

        let caller = funded_account::<T>("caller", 0);
        let proposal_hash = add_proposal::<T>(s)?;

        // Create s existing "seconds"
        for i in 0 .. s {
            let seconder = funded_account::<T>("seconder", i);
            Democracy::<T>::second(RawOrigin::Signed(seconder).into(), 0, u32::MAX)?;
        }

        let deposits = Democracy::<T>::deposit_of(0).ok_or("Proposal not created")?;
        assert_eq!(deposits.0.len(), (s + 1) as usize, "Seconds not recorded");
        whitelist_account!(caller);
    }: _(RawOrigin::Signed(caller), 0, u32::MAX)
    verify {
        let deposits = Democracy::<T>::deposit_of(0).ok_or("Proposal not created")?;
        assert_eq!(deposits.0.len(), (s + 2) as usize, "`second` benchmark did not work");
    }

    vote_new {
        let r in 1 .. MAX_REFERENDUMS;

        let caller = funded_account::<T>("caller", 0);
        let account_vote = account_vote::<T>(100u32.into());

        // We need to create existing direct votes
        for i in 0 .. r {
            let ref_idx = add_referendum::<T>(i)?;
            Democracy::<T>::vote(RawOrigin::Signed(caller.clone()).into(), ref_idx, account_vote.clone())?;
        }
        let Voting { votes, .. } = VotingOf::<T>::get(&caller);
        assert_eq!(votes.len(), r as usize, "Votes were not recorded.");

        let referendum_index = add_referendum::<T>(r)?;
        whitelist_account!(caller);
    }: vote(RawOrigin::Signed(caller.clone()), referendum_index, account_vote)
    verify {
        let Voting { votes, .. } = VotingOf::<T>::get(&caller);
        assert_eq!(votes.len(), (r + 1) as usize, "Vote was not recorded.");
    }

    vote_existing {
        let r in 1 .. MAX_REFERENDUMS;

        let caller = funded_account::<T>("caller", 0);
        let account_vote = account_vote::<T>(100u32.into());

        // We need to create existing direct votes
        for i in 0 ..=r {
            let ref_idx = add_referendum::<T>(i)?;
            Democracy::<T>::vote(RawOrigin::Signed(caller.clone()).into(), ref_idx, account_vote.clone())?;
        }
        let Voting { votes, .. } = VotingOf::<T>::get(&caller);
        assert_eq!(votes.len(), (r + 1) as usize, "Votes were not recorded.");

        // Change vote from aye to nay
        let new_vote = Vote { aye: false, balance: 1000u32.into() };
        let referendum_index = Democracy::<T>::referendum_count() - 1;

        // This tests when a user changes a vote
        whitelist_account!(caller);
    }: vote(RawOrigin::Signed(caller.clone()), referendum_index, new_vote)
    verify {
        let Voting { votes, .. } = VotingOf::<T>::get(&caller);
        assert_eq!(votes.len(), (r + 1) as usize, "Vote was incorrectly added");
        let referendum_info = Democracy::<T>::referendum_info(referendum_index)
            .ok_or("referendum doesn't exist")?;
        let tally =  match referendum_info {
            ReferendumInfo::Ongoing(r) => r.tally,
            _ => return Err("referendum not ongoing".into()),
        };
        assert_eq!(tally.nays, 1000u32.into(), "changed vote was not recorded");
    }

    // TODO: successful_origin only available in runtime-benchmarks
    // fast_track {
    //     let prop_index = add_proposal::<T>(0)?;

    //     let origin_fast_track = T::FastTrackOrigin::successful_origin();
    //     let voting_period = T::FastTrackVotingPeriod::get();
    //     let delay = 0u32;
    // }: _<T::Origin>(origin_fast_track, prop_index, delay.into())
    // verify {
    //     assert_eq!(Democracy::<T>::referendum_count(), 1, "referendum not created")
    // }

    cancel_referendum {
        let referendum_index = add_referendum::<T>(0)?;
    }: _(RawOrigin::Root, referendum_index)

    cancel_queued {
        let r in 1 .. MAX_REFERENDUMS;

        for i in 0..r {
            add_referendum::<T>(i)?; // This add one element in the scheduler
        }

        let referendum_index = add_referendum::<T>(r)?;
    }: _(RawOrigin::Root, referendum_index)

    // This measures the path of `launch_next` public. Not currently used as we simply
    // assume the weight is `MaxBlockWeight` when executing.
    #[extra]
    on_initialize_public {
        let r in 1 .. MAX_REFERENDUMS;

        for i in 0..r {
            add_referendum::<T>(i)?;
        }

        assert_eq!(Democracy::<T>::referendum_count(), r, "referenda not created");

        // Launch public
        assert!(add_proposal::<T>(r).is_ok(), "proposal not created");

        let block_number = T::LaunchPeriod::get();

    }: { Democracy::<T>::on_initialize(block_number) }
    verify {
        // One extra because of next public
        assert_eq!(Democracy::<T>::referendum_count(), r + 1, "proposal not accepted");

        // All should be finished
        for i in 0 .. r {
            if let Some(value) = ReferendumInfoOf::<T>::get(i) {
                match value {
                    ReferendumInfo::Finished { .. } => (),
                    ReferendumInfo::Ongoing(_) => return Err("Referendum was not finished".into()),
                }
            }
        }
    }

    // No launch no maturing referenda.
    on_initialize_base {
        let r in 1 .. MAX_REFERENDUMS;

        for i in 0..r {
            add_referendum::<T>(i)?;
        }

        for (key, mut info) in ReferendumInfoOf::<T>::iter() {
            if let ReferendumInfo::Ongoing(ref mut status) = info {
                status.end += 100u32.into();
            }
            ReferendumInfoOf::<T>::insert(key, info);
        }

        assert_eq!(Democracy::<T>::referendum_count(), r, "referenda not created");
        assert_eq!(Democracy::<T>::lowest_unbaked(), 0, "invalid referenda init");

    }: { Democracy::<T>::on_initialize(1u32.into()) }
    verify {
        // All should be on going
        for i in 0 .. r {
            if let Some(value) = ReferendumInfoOf::<T>::get(i) {
                match value {
                    ReferendumInfo::Finished { .. } => return Err("Referendum has been finished".into()),
                    ReferendumInfo::Ongoing(_) => (),
                }
            }
        }
    }

    on_initialize_base_with_launch_period {
        let r in 1 .. MAX_REFERENDUMS;

        for i in 0..r {
            add_referendum::<T>(i)?;
        }

        for (key, mut info) in ReferendumInfoOf::<T>::iter() {
            if let ReferendumInfo::Ongoing(ref mut status) = info {
                status.end += 100u32.into();
            }
            ReferendumInfoOf::<T>::insert(key, info);
        }

        assert_eq!(Democracy::<T>::referendum_count(), r, "referenda not created");
        assert_eq!(Democracy::<T>::lowest_unbaked(), 0, "invalid referenda init");

        let block_number = T::LaunchPeriod::get();

    }: { Democracy::<T>::on_initialize(block_number) }
    verify {
        // All should be on going
        for i in 0 .. r {
            if let Some(value) = ReferendumInfoOf::<T>::get(i) {
                match value {
                    ReferendumInfo::Finished { .. } => return Err("Referendum has been finished".into()),
                    ReferendumInfo::Ongoing(_) => (),
                }
            }
        }
    }

    clear_public_proposals {
        add_proposal::<T>(0)?;

    }: _(RawOrigin::Root)

    note_preimage {
        // Num of bytes in encoded proposal
        let b in 0 .. MAX_BYTES;

        let caller = funded_account::<T>("caller", 0);
        let encoded_proposal = vec![1; b as usize];
        whitelist_account!(caller);
    }: _(RawOrigin::Signed(caller), encoded_proposal.clone())
    verify {
        let proposal_hash = T::Hashing::hash(&encoded_proposal[..]);
        match Preimages::<T>::get(proposal_hash) {
            Some(PreimageStatus::Available { .. }) => (),
            _ => return Err("preimage not available".into())
        }
    }

    note_imminent_preimage {
        // Num of bytes in encoded proposal
        let b in 0 .. MAX_BYTES;

        // d + 1 to include the one we are testing
        let encoded_proposal = vec![1; b as usize];
        let proposal_hash = T::Hashing::hash(&encoded_proposal[..]);
        let block_number = T::BlockNumber::one();
        Preimages::<T>::insert(&proposal_hash, PreimageStatus::Missing(block_number));

        let caller = funded_account::<T>("caller", 0);
        let encoded_proposal = vec![1; b as usize];
        whitelist_account!(caller);
    }: _(RawOrigin::Signed(caller), encoded_proposal.clone())
    verify {
        let proposal_hash = T::Hashing::hash(&encoded_proposal[..]);
        match Preimages::<T>::get(proposal_hash) {
            Some(PreimageStatus::Available { .. }) => (),
            _ => return Err("preimage not available".into())
        }
    }

    reap_preimage {
        // Num of bytes in encoded proposal
        let b in 0 .. MAX_BYTES;

        let encoded_proposal = vec![1; b as usize];
        let proposal_hash = T::Hashing::hash(&encoded_proposal[..]);

        let submitter = funded_account::<T>("submitter", b);
        Democracy::<T>::note_preimage(RawOrigin::Signed(submitter.clone()).into(), encoded_proposal.clone())?;

        // We need to set this otherwise we get `Early` error.
        let block_number = T::VotingPeriod::get() + T::EnactmentPeriod::get() + T::BlockNumber::one();
        System::<T>::set_block_number(block_number.into());

        assert!(Preimages::<T>::contains_key(proposal_hash));

        let caller = funded_account::<T>("caller", 0);
        whitelist_account!(caller);
    }: _(RawOrigin::Signed(caller), proposal_hash.clone(), u32::MAX)
    verify {
        let proposal_hash = T::Hashing::hash(&encoded_proposal[..]);
        assert!(!Preimages::<T>::contains_key(proposal_hash));
    }

    remove_vote {
        let r in 1 .. MAX_REFERENDUMS;

        let caller = funded_account::<T>("caller", 0);
        let account_vote = account_vote::<T>(100u32.into());

        for i in 0 .. r {
            let ref_idx = add_referendum::<T>(i)?;
            Democracy::<T>::vote(RawOrigin::Signed(caller.clone()).into(), ref_idx, account_vote.clone())?;
        }

        let Voting { votes, .. } = VotingOf::<T>::get(&caller);
        assert_eq!(votes.len(), r as usize, "Votes not created");

        let referendum_index = r - 1;
        whitelist_account!(caller);
    }: _(RawOrigin::Signed(caller.clone()), referendum_index)
    verify {
        let Voting { votes, .. } = VotingOf::<T>::get(&caller);
        assert_eq!(votes.len(), (r - 1) as usize, "Vote was not removed");
    }

    #[extra]
    enact_proposal_execute {
        // Num of bytes in encoded proposal
        let b in 0 .. MAX_BYTES;

        let proposer = funded_account::<T>("proposer", 0);
        let raw_call = Call::note_preimage { encoded_proposal: vec![1; b as usize] };
        let generic_call: T::Proposal = raw_call.into();
        let encoded_proposal = generic_call.encode();
        let proposal_hash = T::Hashing::hash(&encoded_proposal[..]);
        Democracy::<T>::note_preimage(RawOrigin::Signed(proposer).into(), encoded_proposal)?;

        match Preimages::<T>::get(proposal_hash) {
            Some(PreimageStatus::Available { .. }) => (),
            _ => return Err("preimage not available".into())
        }
    }: enact_proposal(RawOrigin::Root, proposal_hash, 0)
    verify {
        // Fails due to mismatched origin
        assert_last_event::<T>(Event::<T>::Executed(0, Err(BadOrigin.into())).into());
    }

    #[extra]
    enact_proposal_slash {
        // Num of bytes in encoded proposal
        let b in 0 .. MAX_BYTES;

        let proposer = funded_account::<T>("proposer", 0);
        // Random invalid bytes
        let encoded_proposal = vec![200; b as usize];
        let proposal_hash = T::Hashing::hash(&encoded_proposal[..]);
        Democracy::<T>::note_preimage(RawOrigin::Signed(proposer).into(), encoded_proposal)?;

        match Preimages::<T>::get(proposal_hash) {
            Some(PreimageStatus::Available { .. }) => (),
            _ => return Err("preimage not available".into())
        }
        let origin = RawOrigin::Root.into();
        let call = Call::<T>::enact_proposal { proposal_hash, index: 0 }.encode();
    }: {
        assert_eq!(
            <Call<T> as Decode>::decode(&mut &*call)
                .expect("call is encoded above, encoding must be correct")
                .dispatch_bypass_filter(origin),
            Err(Error::<T>::PreimageInvalid.into())
        );
    }

    impl_benchmark_test_suite!(
        Democracy,
        crate::tests::new_test_ext(),
        crate::tests::Test
    );
}
