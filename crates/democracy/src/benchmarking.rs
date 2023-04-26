//! Democracy pallet benchmarking.

use super::*;

use frame_benchmarking::{account, benchmarks, whitelist_account};
use frame_support::traits::{Currency, EnsureOrigin, Get, Hash as PreimageHash, OnInitialize};
use frame_system::RawOrigin;
use sp_core::H256;
use sp_runtime::traits::Bounded;

use crate::Pallet as Democracy;

const SEED: u32 = 0;

fn funded_account<T: Config>(name: &'static str, index: u32) -> T::AccountId {
    let caller: T::AccountId = account(name, index, SEED);
    T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
    caller
}

fn make_proposal<T: Config>(n: u32) -> BoundedCallOf<T> {
    let call: CallOf<T> = frame_system::Call::remark { remark: n.encode() }.into();
    <T as Config>::Preimages::bound(call).unwrap()
}

fn add_proposal<T: Config>(n: u32) -> Result<H256, &'static str> {
    let other = funded_account::<T>("proposer", n);
    let value = T::MinimumDeposit::get();
    let proposal = make_proposal::<T>(n);
    Democracy::<T>::propose(RawOrigin::Signed(other).into(), proposal.clone(), value)?;
    Ok(proposal.hash())
}

fn add_referendum<T: Config>(n: u32) -> (ReferendumIndex, H256, PreimageHash) {
    let vote_threshold = VoteThreshold::SimpleMajority;
    let proposal = make_proposal::<T>(n);
    let hash = proposal.hash();
    let index = Democracy::<T>::inject_referendum(T::VotingPeriod::get(), proposal, vote_threshold, 0u32.into());
    let preimage_hash = note_preimage::<T>();
    (index, hash, preimage_hash)
}

// note a new preimage.
fn note_preimage<T: Config>() -> PreimageHash {
    use core::sync::atomic::{AtomicU8, Ordering};
    use sp_std::borrow::Cow;
    // note a new preimage on every function invoke.
    static COUNTER: AtomicU8 = AtomicU8::new(0);
    let data = Cow::from(vec![COUNTER.fetch_add(1, Ordering::Relaxed)]);
    let hash = <T as Config>::Preimages::note(data).unwrap();
    hash
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
        let proposal = make_proposal::<T>(0);
        let value = T::MinimumDeposit::get();
        whitelist_account!(caller);
    }: _(RawOrigin::Signed(caller), proposal, value.into())
    verify {
        assert_eq!(Democracy::<T>::public_props().len(), p as usize, "Proposals not created.");
    }

    second {
        let caller = funded_account::<T>("caller", 0);
        add_proposal::<T>(0)?;

        // Create s existing "seconds"
        // we must reserve one deposit for the `proposal` and one for our benchmarked `second` call.
        for i in 0 .. T::MaxDeposits::get() - 2 {
            let seconder = funded_account::<T>("seconder", i);
            Democracy::<T>::second(RawOrigin::Signed(seconder).into(), 0)?;
        }

        let deposits = Democracy::<T>::deposit_of(0).ok_or("Proposal not created")?;
        assert_eq!(deposits.0.len(), (T::MaxDeposits::get() - 1) as usize, "Seconds not recorded");
        whitelist_account!(caller);
    }: _(RawOrigin::Signed(caller), 0)
    verify {
        let deposits = Democracy::<T>::deposit_of(0).ok_or("Proposal not created")?;
        assert_eq!(deposits.0.len(), (T::MaxDeposits::get()) as usize, "`second` benchmark did not work");
    }

    vote_new {
        let caller = funded_account::<T>("caller", 0);
        let account_vote = account_vote::<T>(100u32.into());

        // We need to create existing direct votes
        for i in 0 .. T::MaxVotes::get() - 1 {
            let ref_index = add_referendum::<T>(i).0;
            Democracy::<T>::vote(RawOrigin::Signed(caller.clone()).into(), ref_index, account_vote)?;
        }
        let Voting { votes, .. } = VotingOf::<T>::get(&caller);
        assert_eq!(votes.len(), (T::MaxVotes::get() - 1) as usize, "Votes were not recorded.");

        let ref_index = add_referendum::<T>(T::MaxVotes::get() - 1).0;
        whitelist_account!(caller);
    }: vote(RawOrigin::Signed(caller.clone()), ref_index, account_vote)
    verify {
        let Voting { votes, .. } = VotingOf::<T>::get(&caller);
        assert_eq!(votes.len(), T::MaxVotes::get() as usize, "Vote was not recorded.");
    }

    vote_existing {
        let caller = funded_account::<T>("caller", 0);
        let account_vote = account_vote::<T>(100u32.into());

        // We need to create existing direct votes
        for i in 0..T::MaxVotes::get() {
            let ref_index = add_referendum::<T>(i).0;
            Democracy::<T>::vote(RawOrigin::Signed(caller.clone()).into(), ref_index, account_vote)?;
        }
        let Voting { votes, .. } = VotingOf::<T>::get(&caller);
        assert_eq!(votes.len(), T::MaxVotes::get() as usize, "Votes were not recorded.");

        // Change vote from aye to nay
        let new_vote = Vote { aye: false, balance: 1000u32.into() };
        let ref_index = Democracy::<T>::referendum_count() - 1;

        // This tests when a user changes a vote
        whitelist_account!(caller);
    }: vote(RawOrigin::Signed(caller.clone()), ref_index, new_vote)
    verify {
        let Voting { votes, .. } = VotingOf::<T>::get(&caller);
        assert_eq!(votes.len(), T::MaxVotes::get() as usize, "Vote was incorrectly added");
        let referendum_info = Democracy::<T>::referendum_info(ref_index)
            .ok_or("referendum doesn't exist")?;
        let tally =  match referendum_info {
            ReferendumInfo::Ongoing(r) => r.tally,
            _ => return Err("referendum not ongoing".into()),
        };
        assert_eq!(tally.nays, 1000u32.into(), "changed vote was not recorded");
    }

    fast_track {
        let origin_fast_track = T::FastTrackOrigin::try_successful_origin().unwrap();
        let proposal_hash = add_proposal::<T>(0)?;
        let prop_index = PublicProps::<T>::get()
            .iter()
            .find(|p| p.1.hash() == proposal_hash)
            .map(|p| p.0)
            .unwrap();
        let delay = 0u32;
        let ref_count_before = Democracy::<T>::referendum_count();
    }: _<T::RuntimeOrigin>(origin_fast_track, prop_index, delay.into())
    verify {
        assert_eq!(
            Democracy::<T>::referendum_count(),
            ref_count_before + 1,
            "referendum not created"
        );
    }

    cancel_referendum {
        let (ref_index, _, _) = add_referendum::<T>(0);
    }: _(RawOrigin::Root, ref_index)

    // This measures the path of `launch_next` public. Not currently used as we simply
    // assume the weight is `MaxBlockWeight` when executing.
    #[extra]
    on_initialize_public {
        let r in 0 .. (T::MaxVotes::get() - 1);

        for i in 0..r {
            add_referendum::<T>(i);
        }

        assert_eq!(Democracy::<T>::referendum_count(), r, "referenda not created");

        // Launch public
        assert!(add_proposal::<T>(r).is_ok(), "proposal not created");

        let block_number = T::VotingPeriod::get();

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
        let r in 0 .. (T::MaxVotes::get() - 1);

        for i in 0..r {
            add_referendum::<T>(i);
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
        let r in 0 .. (T::MaxVotes::get() - 1);

        for i in 0..r {
            add_referendum::<T>(i);
        }

        for (key, mut info) in ReferendumInfoOf::<T>::iter() {
            if let ReferendumInfo::Ongoing(ref mut status) = info {
                status.end += 100u32.into();
            }
            ReferendumInfoOf::<T>::insert(key, info);
        }

        assert_eq!(Democracy::<T>::referendum_count(), r, "referenda not created");
        assert_eq!(Democracy::<T>::lowest_unbaked(), 0, "invalid referenda init");

        let block_number = T::VotingPeriod::get();

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

    remove_vote {
        let r in 1 .. T::MaxVotes::get();

        let caller = funded_account::<T>("caller", 0);
        let account_vote = account_vote::<T>(100u32.into());

        for i in 0 .. r {
            let ref_index = add_referendum::<T>(i).0;
            Democracy::<T>::vote(RawOrigin::Signed(caller.clone()).into(), ref_index, account_vote)?;
        }

        let Voting { votes, .. } = VotingOf::<T>::get(&caller);
        assert_eq!(votes.len(), r as usize, "Votes not created");

        let ref_index = r - 1;
        whitelist_account!(caller);
    }: _(RawOrigin::Signed(caller.clone()), ref_index)
    verify {
        let Voting { votes, .. } = VotingOf::<T>::get(&caller);
        assert_eq!(votes.len(), (r - 1) as usize, "Vote was not removed");
    }

    spend_from_treasury {
        let beneficiary: T::AccountId = account("beneficiary", 0, 0);
        T::Currency::make_free_balance_be(&T::TreasuryAccount::get(), 100u32.into());
        let value = 100u32.into();
    }: _(RawOrigin::Root, value, beneficiary.clone())
    verify {
        assert_eq!(T::TreasuryCurrency::free_balance(&beneficiary), 100u32.into());
    }

    impl_benchmark_test_suite!(
        Democracy,
        crate::tests::new_test_ext(),
        crate::tests::Test
    );
}
