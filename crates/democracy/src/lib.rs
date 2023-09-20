//! # Democracy Pallet
//!
//! - [`Config`]
//! - [`Call`]
//!
//! ## Overview
//!
//! The democracy pallet handles the administration of general stakeholder voting.
//!
//! Proposals made by the community are added to a queue before they become a referendum.
//!
//! Every launch period - a length defined in the runtime - this pallet will launch a
//! referendum from the proposal queue. Any token holder in the system can vote on this.
//!
//! ### Terminology
//!
//! - **Enactment Period:** The period between a proposal being approved and enacted.
//! - **Vote:** A value that can either be in approval ("Aye") or rejection ("Nay") of a particular referendum.
//! - **Proposal:** A submission to the chain that represents an action that a proposer (either an
//! account or an external origin) suggests that the system adopt.
//! - **Referendum:** A proposal that is in the process of being voted on for either acceptance or rejection as a change
//!   to the system.
//!
//! ### Adaptive Quorum Biasing
//!
//! A _referendum_ can be either simple majority-carries in which 50%+1 of the
//! votes decide the outcome or _adaptive quorum biased_. Adaptive quorum biasing
//! makes the threshold for passing or rejecting a referendum higher or lower
//! depending on how the referendum was originally proposed. There are two types of
//! adaptive quorum biasing: 1) _positive turnout bias_ makes a referendum
//! require a super-majority to pass that decreases as turnout increases and
//! 2) _negative turnout bias_ makes a referendum require a super-majority to
//! reject that decreases as turnout increases. Another way to think about the
//! quorum biasing is that _positive bias_ referendums will be rejected by
//! default and _negative bias_ referendums get passed by default.
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! #### Public
//!
//! These calls can be made from any externally held account capable of creating
//! a signed extrinsic.
//!
//! Basic actions:
//! - `propose` - Submits a sensitive action, represented as a hash. Requires a deposit.
//! - `second` - Signals agreement with a proposal, moves it higher on the proposal queue, and requires a matching
//!   deposit to the original.
//! - `vote` - Votes in a referendum, either the vote is "Aye" to enact the proposal or "Nay" to keep the status quo.
//! - `remove_vote` - Cancel a previous vote, this must be done by the voter before the vote ends.
//!
//! #### Fast Track Origin
//!
//! This call can only be made by the `FastTrackOrigin`.
//!
//! - `table_proposal` - Upgrades a proposal to a referendum with the normal `VotingPeriod`.
//! - `fast_track` - Upgrades a proposal to a referendum with the `FastTrackVotingPeriod`.
//! - `fast_track_referendum` - Schedules an active referendum to end in `FastTrackVotingPeriod` blocks.
//!
//! #### Root
//!
//! - `cancel_referendum` - Removes a referendum.
//! - `clear_public_proposals` - Removes all public proposals.
//! - `cancel_proposal` - Removes a proposal.

#![deny(warnings)]
#![recursion_limit = "256"]
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
    ensure,
    traits::{
        schedule::{v3::Named as ScheduleNamed, DispatchTime},
        Bounded, Currency, Get, LockIdentifier, QueryPreimage, ReservableCurrency, StorePreimage, UnixTime,
    },
    transactional,
    weights::Weight,
};
use frame_system::pallet_prelude::BlockNumberFor;
use sp_runtime::{
    traits::{One, Saturating, Zero},
    ArithmeticError, DispatchError, DispatchResult,
};
use sp_std::prelude::*;

mod types;
mod vote_threshold;

pub use pallet::*;
pub use types::{ReferendumInfo, ReferendumStatus, Tally, Vote, Voting};
pub use vote_threshold::{Approved, VoteThreshold};

mod default_weights;
pub use default_weights::WeightInfo;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod migrations;

const DEMOCRACY_ID: LockIdentifier = *b"democrac";

/// A proposal index.
pub type PropIndex = u32;

/// A referendum index.
pub type ReferendumIndex = u32;

type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
pub type CallOf<T> = <T as frame_system::Config>::RuntimeCall;
pub type BoundedCallOf<T> = Bounded<CallOf<T>>;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use core::num::TryFromIntError;
    use frame_support::{
        pallet_prelude::*,
        traits::{EnsureOrigin, ExistenceRequirement},
    };
    use frame_system::{ensure_root, ensure_signed, pallet_prelude::*};
    use sp_runtime::DispatchResult;

    /// The current storage version.
    const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config + Sized {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// The Scheduler.
        type Scheduler: ScheduleNamed<BlockNumberFor<Self>, CallOf<Self>, Self::PalletsOrigin>;

        /// The Preimage provider.
        type Preimages: QueryPreimage + StorePreimage;

        /// Currency type for this pallet.
        type Currency: ReservableCurrency<Self::AccountId>;

        /// The period between a proposal being approved and enacted.
        ///
        /// It should generally be a little more than the unstake period to ensure that
        /// voting stakers have an opportunity to remove themselves from the system in the case
        /// where they are on the losing side of a vote.
        #[pallet::constant]
        type EnactmentPeriod: Get<BlockNumberFor<Self>>;

        /// How often (in blocks) to check for new votes.
        #[pallet::constant]
        type VotingPeriod: Get<BlockNumberFor<Self>>;

        /// Minimum voting period allowed for a fast-track referendum.
        #[pallet::constant]
        type FastTrackVotingPeriod: Get<BlockNumberFor<Self>>;

        /// The minimum amount to be used as a deposit for a public referendum proposal.
        #[pallet::constant]
        type MinimumDeposit: Get<BalanceOf<Self>>;

        /// The maximum number of votes for an account.
        ///
        /// Also used to compute weight, an overly big value can
        /// lead to extrinsic with very big weight.
        #[pallet::constant]
        type MaxVotes: Get<u32>;

        /// The maximum number of public proposals that can exist at any time.
        #[pallet::constant]
        type MaxProposals: Get<u32>;

        /// The maximum number of deposits a public proposal may have at any time.
        #[pallet::constant]
        type MaxDeposits: Get<u32>;

        /// Origin from which the next majority-carries (or more permissive) referendum may be
        /// tabled to vote according to the `FastTrackVotingPeriod` asynchronously in a similar
        /// manner to the emergency origin. It retains its threshold method.
        type FastTrackOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Overarching type of all pallets origins.
        type PalletsOrigin: From<frame_system::RawOrigin<Self::AccountId>>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;

        /// Unix time
        type UnixTime: UnixTime;

        /// Period from previous launch timestamp.
        type LaunchPeriod: Get<u64>;

        /// Account from which is transferred in `spend_from_treasury`.
        type TreasuryAccount: Get<Self::AccountId>;

        /// Currency used in `spend_from_treasury`.
        type TreasuryCurrency: Currency<Self::AccountId, Balance = BalanceOf<Self>>;
    }

    /// The number of (public) proposals that have been made so far.
    #[pallet::storage]
    #[pallet::getter(fn public_prop_count)]
    pub type PublicPropCount<T> = StorageValue<_, PropIndex, ValueQuery>;

    /// The public proposals. Unsorted. The second item is the proposal.
    #[pallet::storage]
    #[pallet::getter(fn public_props)]
    pub type PublicProps<T: Config> =
        StorageValue<_, BoundedVec<(PropIndex, BoundedCallOf<T>, T::AccountId), T::MaxProposals>, ValueQuery>;

    /// Those who have locked a deposit.
    ///
    /// TWOX-NOTE: Safe, as increasing integer keys are safe.
    #[pallet::storage]
    #[pallet::getter(fn deposit_of)]
    pub type DepositOf<T: Config> =
        StorageMap<_, Twox64Concat, PropIndex, (BoundedVec<T::AccountId, T::MaxDeposits>, BalanceOf<T>)>;

    /// The next free referendum index, aka the number of referenda started so far.
    #[pallet::storage]
    #[pallet::getter(fn referendum_count)]
    pub type ReferendumCount<T> = StorageValue<_, ReferendumIndex, ValueQuery>;

    /// The lowest referendum index representing an unbaked referendum. Equal to
    /// `ReferendumCount` if there isn't a unbaked referendum.
    #[pallet::storage]
    #[pallet::getter(fn lowest_unbaked)]
    pub type LowestUnbaked<T> = StorageValue<_, ReferendumIndex, ValueQuery>;

    /// Information concerning any given referendum.
    ///
    /// TWOX-NOTE: SAFE as indexes are not under an attacker’s control.
    #[pallet::storage]
    #[pallet::getter(fn referendum_info)]
    pub type ReferendumInfoOf<T: Config> =
        StorageMap<_, Twox64Concat, ReferendumIndex, ReferendumInfo<BlockNumberFor<T>, BoundedCallOf<T>, BalanceOf<T>>>;

    /// All votes for a particular voter. We store the balance for the number of votes that we
    /// have recorded.
    ///
    /// TWOX-NOTE: SAFE as `AccountId`s are crypto hashes anyway.
    #[pallet::storage]
    pub type VotingOf<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, Voting<BalanceOf<T>, T::MaxVotes>, ValueQuery>;

    #[pallet::storage]
    pub type NextLaunchTimestamp<T: Config> = StorageValue<_, u64, ValueQuery>;

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        _phantom: sp_std::marker::PhantomData<T>,
        next_launch_timestamp: u64,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            PublicPropCount::<T>::put(0 as PropIndex);
            ReferendumCount::<T>::put(0 as ReferendumIndex);
            LowestUnbaked::<T>::put(0 as ReferendumIndex);
            NextLaunchTimestamp::<T>::put(self.next_launch_timestamp);
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A motion has been proposed by a public account.
        Proposed {
            proposal_index: PropIndex,
            deposit: BalanceOf<T>,
        },
        /// A public proposal has been tabled for referendum vote.
        Tabled {
            proposal_index: PropIndex,
            deposit: BalanceOf<T>,
        },
        /// A referendum has begun.
        Started {
            ref_index: ReferendumIndex,
            threshold: VoteThreshold,
        },
        /// A proposal has been fast tracked.
        FastTrack { ref_index: ReferendumIndex },
        /// A referendum has been fast tracked.
        FastTrackReferendum { ref_index: ReferendumIndex },
        /// A proposal has been approved by referendum.
        Passed { ref_index: ReferendumIndex },
        /// A proposal has been rejected by referendum.
        NotPassed { ref_index: ReferendumIndex },
        /// A referendum has been cancelled.
        Cancelled { ref_index: ReferendumIndex },
        /// A proposal has been cancelled.
        CancelledProposal { prop_index: PropIndex },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Value too low
        ValueLow,
        /// Proposal does not exist
        ProposalMissing,
        /// Preimage does not exist
        PreimageMissing,
        /// Not imminent
        NotImminent,
        /// Too early
        TooEarly,
        /// Imminent
        Imminent,
        /// Vote given for invalid referendum
        ReferendumInvalid,
        /// Fast tracking failed, because the referendum is
        /// ending sooner than the fast track voting period.
        ReferendumFastTrackFailed,
        /// No proposals waiting
        NoneWaiting,
        /// The given account did not make this proposal.
        NotProposer,
        /// The given account did not vote on the referendum.
        NotVoter,
        /// Too high a balance was provided that the account cannot afford.
        InsufficientFunds,
        /// Invalid upper bound.
        WrongUpperBound,
        /// Maximum number of votes reached.
        MaxVotesReached,
        /// Maximum number of items reached.
        TooMany,
        /// Unable to convert value.
        TryIntoIntError,
    }

    impl<T> From<TryFromIntError> for Error<T> {
        fn from(_err: TryFromIntError) -> Self {
            Error::<T>::TryIntoIntError
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        /// Weight: see `begin_block`
        fn on_initialize(n: BlockNumberFor<T>) -> Weight {
            Self::begin_block(n).unwrap_or_else(|e| {
                sp_runtime::print(e);
                Weight::from_parts(0 as u64, 0u64)
            })
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Propose a sensitive action to be taken.
        ///
        /// The dispatch origin of this call must be _Signed_ and the sender must
        /// have funds to cover the deposit.
        ///
        /// - `proposal_hash`: The hash of the proposal preimage.
        /// - `value`: The amount of deposit (must be at least `MinimumDeposit`).
        ///
        /// Emits `Proposed`.
        ///
        /// Weight: `O(p)`
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::propose())]
        pub fn propose(
            origin: OriginFor<T>,
            proposal: BoundedCallOf<T>,
            #[pallet::compact] value: BalanceOf<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(value >= T::MinimumDeposit::get(), Error::<T>::ValueLow);

            let index = Self::public_prop_count();
            let real_prop_count = PublicProps::<T>::decode_len().unwrap_or(0) as u32;
            let max_proposals = T::MaxProposals::get();
            ensure!(real_prop_count < max_proposals, Error::<T>::TooMany);

            ensure!(T::Preimages::have(&proposal), Error::<T>::PreimageMissing);
            // Actually `hold` the proposal now to make sure it is not removed.
            // This will be reversed by the Scheduler pallet once it is executed
            // which assumes that we will already have placed a `hold` on it.
            T::Preimages::hold(&proposal);

            T::Currency::reserve(&who, value)?;
            PublicPropCount::<T>::put(index + 1);

            let depositors = BoundedVec::<_, T::MaxDeposits>::truncate_from(vec![who.clone()]);
            DepositOf::<T>::insert(index, (depositors, value));

            PublicProps::<T>::try_append((index, proposal, who)).map_err(|_| Error::<T>::TooMany)?;

            Self::deposit_event(Event::<T>::Proposed {
                proposal_index: index,
                deposit: value,
            });
            Ok(())
        }

        /// Signals agreement with a particular proposal.
        ///
        /// The dispatch origin of this call must be _Signed_ and the sender
        /// must have funds to cover the deposit, equal to the original deposit.
        ///
        /// - `proposal`: The index of the proposal to second.
        /// - `seconds_upper_bound`: an upper bound on the current number of seconds on this proposal. Extrinsic is
        ///   weighted according to this value with no refund.
        ///
        /// Weight: `O(S)` where S is the number of seconds a proposal already has.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::second())]
        pub fn second(origin: OriginFor<T>, #[pallet::compact] proposal: PropIndex) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let seconds = Self::len_of_deposit_of(proposal).ok_or_else(|| Error::<T>::ProposalMissing)?;
            ensure!(seconds < T::MaxDeposits::get(), Error::<T>::TooMany);
            let mut deposit = Self::deposit_of(proposal).ok_or(Error::<T>::ProposalMissing)?;
            T::Currency::reserve(&who, deposit.1)?;
            deposit.0.try_push(who.clone()).map_err(|_| Error::<T>::TooMany)?;
            <DepositOf<T>>::insert(proposal, deposit);
            Ok(())
        }

        /// Vote in a referendum. If `vote.is_aye()`, the vote is to enact the proposal;
        /// otherwise it is a vote to keep the status quo.
        ///
        /// The dispatch origin of this call must be _Signed_.
        ///
        /// - `ref_index`: The index of the referendum to vote for.
        /// - `vote`: The vote configuration.
        ///
        /// Weight: `O(R)` where R is the number of referendums the voter has voted on.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::vote_new().max(T::WeightInfo::vote_existing()))]
        pub fn vote(
            origin: OriginFor<T>,
            #[pallet::compact] ref_index: ReferendumIndex,
            vote: Vote<BalanceOf<T>>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Self::try_vote(&who, ref_index, vote)
        }

        /// Schedule a proposal to be tabled immediately with the `FastTrackVotingPeriod`.
        ///
        /// The dispatch of this call must be `FastTrackOrigin`.
        ///
        /// - `prop_index`: The index of the current external proposal.
        /// - `delay`: The number of blocks to wait after approval before execution.
        ///
        /// Emits `Started` and `FastTrack`.
        ///
        /// Weight: `O(1)`
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::fast_track())]
        pub fn fast_track(
            origin: OriginFor<T>,
            #[pallet::compact] prop_index: PropIndex,
            delay: BlockNumberFor<T>,
        ) -> DispatchResult {
            T::FastTrackOrigin::ensure_origin(origin)?;
            Self::fast_track_with_voting_period(prop_index, delay, T::FastTrackVotingPeriod::get())
        }

        /// Same as `fast_track` but with the default `VotingPeriod`.
        ///
        /// The dispatch of this call must be `FastTrackOrigin`.
        ///
        /// - `prop_index`: The index of the proposal.
        /// - `delay`: The number of blocks to wait after approval before execution.
        ///
        /// Emits `Started` and `FastTrack`.
        ///
        /// Weight: `O(1)`
        #[pallet::call_index(4)]
        // same complexity as `fast_track` so no need to benchmark separately
        #[pallet::weight(T::WeightInfo::fast_track())]
        pub fn table_proposal(
            origin: OriginFor<T>,
            #[pallet::compact] prop_index: PropIndex,
            delay: BlockNumberFor<T>,
        ) -> DispatchResult {
            T::FastTrackOrigin::ensure_origin(origin)?;
            Self::fast_track_with_voting_period(prop_index, delay, T::VotingPeriod::get())
        }

        /// Reduces the voting period of an existing referendum.
        ///
        /// The dispatch of this call must be `FastTrackOrigin`.
        ///
        /// - `ref_index`: The index of the referendum.
        ///
        /// Emits `FastTrackReferendum`.
        ///
        /// Weight: `O(1)`
        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::fast_track_referendum())]
        pub fn fast_track_referendum(origin: OriginFor<T>, #[pallet::compact] ref_index: PropIndex) -> DispatchResult {
            T::FastTrackOrigin::ensure_origin(origin)?;
            let mut status = Self::referendum_status(ref_index)?;
            let now = <frame_system::Pallet<T>>::block_number();
            let voting_period = T::FastTrackVotingPeriod::get();
            let end_block = now.saturating_add(voting_period);
            ensure!(status.end > end_block, Error::<T>::ReferendumFastTrackFailed);
            status.end = end_block;

            ReferendumInfoOf::<T>::insert(ref_index, ReferendumInfo::Ongoing(status));
            Self::deposit_event(Event::<T>::FastTrackReferendum { ref_index });
            Ok(())
        }

        /// Remove a referendum.
        ///
        /// The dispatch origin of this call must be _Root_.
        ///
        /// - `ref_index`: The index of the referendum to cancel.
        ///
        /// # Weight: `O(1)`.
        #[pallet::call_index(6)]
        #[pallet::weight(T::WeightInfo::cancel_referendum())]
        pub fn cancel_referendum(
            origin: OriginFor<T>,
            #[pallet::compact] ref_index: ReferendumIndex,
        ) -> DispatchResult {
            ensure_root(origin)?;
            Self::internal_cancel_referendum(ref_index);
            Ok(())
        }

        /// Clears all public proposals.
        ///
        /// The dispatch origin of this call must be _Root_.
        ///
        /// Weight: `O(1)`.
        #[pallet::call_index(8)]
        #[pallet::weight(T::WeightInfo::clear_public_proposals())]
        pub fn clear_public_proposals(origin: OriginFor<T>) -> DispatchResult {
            ensure_root(origin)?;
            <PublicProps<T>>::kill();
            Ok(())
        }

        /// Remove a proposal.
        ///
        /// - `prop_index`: The index of the proposal to cancel.
        ///
        /// Weight: `O(p)` where `p = PublicProps::<T>::decode_len()`
        #[pallet::call_index(9)]
        #[pallet::weight(T::WeightInfo::cancel_proposal())]
        #[transactional]
        pub fn cancel_proposal(origin: OriginFor<T>, #[pallet::compact] prop_index: PropIndex) -> DispatchResult {
            let who = ensure_signed(origin.clone())
                .map(Some)
                .or_else(|_| ensure_root(origin).map(|_| None))?;

            PublicProps::<T>::try_mutate(|props| {
                if let Some(i) = props.iter().position(|p| p.0 == prop_index) {
                    let (_, proposal, proposer) = props.remove(i);
                    if let Some(account_id) = who {
                        ensure!(proposer == account_id, Error::<T>::NotProposer);
                    }
                    // since we placed a `hold` on propose
                    // we should now unrequest the data
                    T::Preimages::drop(&proposal);
                    Ok(())
                } else {
                    Err(Error::<T>::ProposalMissing)
                }
            })?;
            if let Some((whos, amount)) = DepositOf::<T>::take(prop_index) {
                for who in whos.into_iter() {
                    T::Currency::unreserve(&who, amount);
                }
            }
            Self::deposit_event(Event::<T>::CancelledProposal { prop_index });

            Ok(())
        }

        /// Remove a vote for an ongoing referendum.
        ///
        /// The dispatch origin of this call must be _Signed_, and the signer must have a vote
        /// registered for referendum `index`.
        ///
        /// - `index`: The index of referendum of the vote to be removed.
        ///
        /// Weight: `O(R + log R)` where R is the number of referenda that `target` has voted on.
        ///   Weight is calculated for the maximum number of vote.
        #[pallet::call_index(13)]
        #[pallet::weight(T::WeightInfo::remove_vote(T::MaxVotes::get()))]
        pub fn remove_vote(origin: OriginFor<T>, index: ReferendumIndex) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Self::try_remove_vote(&who, index)
        }

        #[pallet::call_index(14)]
        #[pallet::weight(T::WeightInfo::spend_from_treasury())]
        pub fn spend_from_treasury(
            origin: OriginFor<T>,
            #[pallet::compact] value: BalanceOf<T>,
            beneficiary: T::AccountId,
        ) -> DispatchResult {
            ensure_root(origin)?;
            T::TreasuryCurrency::transfer(
                &T::TreasuryAccount::get(),
                &beneficiary,
                value,
                ExistenceRequirement::AllowDeath,
            )
        }
    }
}

pub trait EncodeInto: Encode {
    fn encode_into<T: AsMut<[u8]> + Default>(&self) -> T {
        let mut t = T::default();
        self.using_encoded(|data| {
            if data.len() <= t.as_mut().len() {
                t.as_mut()[0..data.len()].copy_from_slice(data);
            } else {
                // encoded self is too big to fit into a T. hash it and use the first bytes of that
                // instead.
                let hash = sp_io::hashing::blake2_256(data);
                let l = t.as_mut().len().min(hash.len());
                t.as_mut()[0..l].copy_from_slice(&hash[0..l]);
            }
        });
        t
    }
}
impl<T: Encode> EncodeInto for T {}

impl<T: Config> Pallet<T> {
    // exposed immutables.

    /// Get the amount locked in support of `proposal`; `None` if proposal isn't a valid proposal
    /// index.
    pub fn backing_for(proposal: PropIndex) -> Option<BalanceOf<T>> {
        Self::deposit_of(proposal).map(|(l, d)| d.saturating_mul((l.len() as u32).into()))
    }

    /// Get all referenda ready for tally at block `n`.
    pub fn maturing_referenda_at(
        n: BlockNumberFor<T>,
    ) -> Vec<(
        ReferendumIndex,
        ReferendumStatus<BlockNumberFor<T>, BoundedCallOf<T>, BalanceOf<T>>,
    )> {
        let next = Self::lowest_unbaked();
        let last = Self::referendum_count();
        Self::maturing_referenda_at_inner(n, next..last)
    }

    fn maturing_referenda_at_inner(
        n: BlockNumberFor<T>,
        range: core::ops::Range<PropIndex>,
    ) -> Vec<(
        ReferendumIndex,
        ReferendumStatus<BlockNumberFor<T>, BoundedCallOf<T>, BalanceOf<T>>,
    )> {
        range
            .into_iter()
            .map(|i| (i, Self::referendum_info(i)))
            .filter_map(|(i, maybe_info)| match maybe_info {
                Some(ReferendumInfo::Ongoing(status)) => Some((i, status)),
                _ => None,
            })
            .filter(|(_, status)| status.end == n)
            .collect()
    }

    // Exposed mutables.

    /// Start a referendum.
    pub fn internal_start_referendum(
        proposal: BoundedCallOf<T>,
        threshold: VoteThreshold,
        delay: BlockNumberFor<T>,
    ) -> ReferendumIndex {
        <Pallet<T>>::inject_referendum(
            <frame_system::Pallet<T>>::block_number().saturating_add(T::VotingPeriod::get()),
            proposal,
            threshold,
            delay,
        )
    }

    /// Remove a referendum.
    pub fn internal_cancel_referendum(ref_index: ReferendumIndex) {
        Self::deposit_event(Event::<T>::Cancelled { ref_index });
        if let Some(ReferendumInfo::Ongoing(status)) = ReferendumInfoOf::<T>::take(ref_index) {
            // unrequest the data since the scheduler
            // did not execute the call
            T::Preimages::drop(&status.proposal);
        }
    }

    // private.

    fn fast_track_with_voting_period(
        prop_index: PropIndex,
        delay: BlockNumberFor<T>,
        voting_period: BlockNumberFor<T>,
    ) -> DispatchResult {
        let mut public_props = Self::public_props();
        let (winner_index, _) = public_props
            .iter()
            .enumerate()
            .find(|(_, (i, ..))| *i == prop_index)
            .ok_or(Error::<T>::ProposalMissing)?;

        let (_, proposal_hash, _) = public_props.swap_remove(winner_index);
        <PublicProps<T>>::put(public_props);

        if let Some((depositors, deposit)) = <DepositOf<T>>::take(prop_index) {
            // refund depositors
            for d in &depositors {
                T::Currency::unreserve(d, deposit);
            }
        }

        let now = <frame_system::Pallet<T>>::block_number();
        let ref_index = Self::inject_referendum(
            now.saturating_add(voting_period),
            proposal_hash,
            VoteThreshold::SuperMajorityApprove,
            delay,
        );
        Self::deposit_event(Event::<T>::FastTrack { ref_index });
        Ok(())
    }

    /// Ok if the given referendum is active, Err otherwise
    fn ensure_ongoing(
        r: ReferendumInfo<BlockNumberFor<T>, BoundedCallOf<T>, BalanceOf<T>>,
    ) -> Result<ReferendumStatus<BlockNumberFor<T>, BoundedCallOf<T>, BalanceOf<T>>, DispatchError> {
        match r {
            ReferendumInfo::Ongoing(s) => Ok(s),
            _ => Err(Error::<T>::ReferendumInvalid.into()),
        }
    }

    fn referendum_status(
        ref_index: ReferendumIndex,
    ) -> Result<ReferendumStatus<BlockNumberFor<T>, BoundedCallOf<T>, BalanceOf<T>>, DispatchError> {
        let info = ReferendumInfoOf::<T>::get(ref_index).ok_or(Error::<T>::ReferendumInvalid)?;
        Self::ensure_ongoing(info)
    }

    /// Actually enact a vote, if legit.
    fn try_vote(who: &T::AccountId, ref_index: ReferendumIndex, vote: Vote<BalanceOf<T>>) -> DispatchResult {
        let mut status = Self::referendum_status(ref_index)?;
        ensure!(
            vote.balance <= T::Currency::free_balance(who),
            Error::<T>::InsufficientFunds
        );
        VotingOf::<T>::try_mutate(who, |voting| -> DispatchResult {
            let Voting { ref mut votes, .. } = voting;

            match votes.binary_search_by_key(&ref_index, |i| i.0) {
                Ok(i) => {
                    // Shouldn't be possible to fail, but we handle it gracefully.
                    status.tally.remove(votes[i].1).ok_or(ArithmeticError::Underflow)?;
                    votes[i].1 = vote;
                }
                Err(i) => {
                    votes
                        .try_insert(i, (ref_index, vote))
                        .map_err(|_| Error::<T>::MaxVotesReached)?;
                }
            }
            // Shouldn't be possible to fail, but we handle it gracefully.
            status.tally.add(vote).ok_or(ArithmeticError::Overflow)?;

            Ok(())
        })?;
        ReferendumInfoOf::<T>::insert(ref_index, ReferendumInfo::Ongoing(status));
        Ok(())
    }

    /// Remove the account's vote for the given referendum.
    fn try_remove_vote(who: &T::AccountId, ref_index: ReferendumIndex) -> DispatchResult {
        let info = ReferendumInfoOf::<T>::get(ref_index);
        VotingOf::<T>::try_mutate(who, |voting| -> DispatchResult {
            let Voting { ref mut votes, .. } = voting;

            let i = votes
                .binary_search_by_key(&ref_index, |i| i.0)
                .map_err(|_| Error::<T>::NotVoter)?;
            match info {
                Some(ReferendumInfo::Ongoing(mut status)) => {
                    // Shouldn't be possible to fail, but we handle it gracefully.
                    status.tally.remove(votes[i].1).ok_or(ArithmeticError::Underflow)?;
                    ReferendumInfoOf::<T>::insert(ref_index, ReferendumInfo::Ongoing(status));
                }
                Some(ReferendumInfo::Finished { .. }) => {}
                None => {} // Referendum was cancelled.
            }
            votes.remove(i);

            Ok(())
        })?;
        Ok(())
    }

    /// Start a referendum
    fn inject_referendum(
        end: BlockNumberFor<T>,
        proposal: BoundedCallOf<T>,
        threshold: VoteThreshold,
        delay: BlockNumberFor<T>,
    ) -> ReferendumIndex {
        let ref_index = Self::referendum_count();
        ReferendumCount::<T>::put(ref_index + 1);
        let status = ReferendumStatus {
            end,
            proposal,
            threshold,
            delay,
            tally: Default::default(),
        };
        let item = ReferendumInfo::Ongoing(status);
        <ReferendumInfoOf<T>>::insert(ref_index, item);
        Self::deposit_event(Event::<T>::Started { ref_index, threshold });
        ref_index
    }

    /// Table the next waiting proposal for a vote.
    fn launch_next(now: BlockNumberFor<T>) -> DispatchResult {
        Self::launch_public(now).map_err(|_| Error::<T>::NoneWaiting.into())
    }

    /// Table the waiting public proposal with the highest backing for a vote.
    fn launch_public(now: BlockNumberFor<T>) -> DispatchResult {
        let mut public_props = Self::public_props();
        if let Some((winner_index, _)) = public_props.iter().enumerate().max_by_key(
            // defensive only: All current public proposals have an amount locked
            |x| Self::backing_for((x.1).0).unwrap_or_else(Zero::zero),
        ) {
            let (prop_index, proposal, _) = public_props.swap_remove(winner_index);
            <PublicProps<T>>::put(public_props);

            if let Some((depositors, deposit)) = <DepositOf<T>>::take(prop_index) {
                // refund depositors
                for d in &depositors {
                    T::Currency::unreserve(d, deposit);
                }
                Self::deposit_event(Event::<T>::Tabled {
                    proposal_index: prop_index,
                    deposit,
                });
                Self::inject_referendum(
                    now.saturating_add(T::VotingPeriod::get()),
                    proposal,
                    VoteThreshold::SuperMajorityApprove,
                    T::EnactmentPeriod::get(),
                );
            }
            Ok(())
        } else {
            Err(Error::<T>::NoneWaiting)?
        }
    }

    fn bake_referendum(
        now: BlockNumberFor<T>,
        index: ReferendumIndex,
        status: ReferendumStatus<BlockNumberFor<T>, BoundedCallOf<T>, BalanceOf<T>>,
    ) -> Result<bool, DispatchError> {
        let total_issuance = T::Currency::total_issuance();
        let approved = status.threshold.approved(status.tally, total_issuance);

        if approved {
            Self::deposit_event(Event::<T>::Passed { ref_index: index });

            // Earliest it can be scheduled for is next block.
            let when = now.saturating_add(status.delay.max(One::one()));
            if T::Scheduler::schedule_named(
                (DEMOCRACY_ID, index).encode_into(),
                DispatchTime::At(when),
                None,
                63,
                frame_system::RawOrigin::Root.into(),
                status.proposal,
            )
            .is_err()
            {
                frame_support::print("LOGIC ERROR: bake_referendum/schedule_named failed");
            }
        } else {
            // scheduler will not drop the call data
            // so we should unrequest that here
            T::Preimages::drop(&status.proposal);
            Self::deposit_event(Event::<T>::NotPassed { ref_index: index });
        }

        Ok(approved)
    }

    /// Current era is ending; we should finish up any proposals.
    ///
    ///
    /// # <weight>
    /// If a referendum is launched or maturing, this will take full block weight if queue is not
    /// empty. Otherwise:
    /// - Complexity: `O(R)` where `R` is the number of unbaked referenda.
    /// - Db reads: `PublicProps`, `account`, `ReferendumCount`, `LowestUnbaked`
    /// - Db writes: `PublicProps`, `account`, `ReferendumCount`, `DepositOf`, `ReferendumInfoOf`
    /// - Db reads per R: `DepositOf`, `ReferendumInfoOf`
    /// # </weight>
    fn begin_block(now: BlockNumberFor<T>) -> Result<Weight, DispatchError> {
        let max_block_weight = T::BlockWeights::get().max_block;
        let mut weight = Weight::from_parts(0 as u64, 0u64);

        let next = Self::lowest_unbaked();
        let last = Self::referendum_count();
        let r = last.saturating_sub(next);

        // pick out another public referendum if it's time.
        let current_time = T::UnixTime::now();
        if Self::should_launch(current_time.as_secs()) {
            // Errors come from the queue being empty. If the queue is not empty, it will take
            // full block weight.
            if Self::launch_next(now).is_ok() {
                weight = max_block_weight;
                // try to launch another one. We ignore the result since weight can't increase beyond max_block_weight
                let _ = Self::launch_next(now);
            } else {
                weight = weight.saturating_add(T::WeightInfo::on_initialize_base_with_launch_period(r));
            }
        } else {
            weight = weight.saturating_add(T::WeightInfo::on_initialize_base(r));
        }

        // tally up votes for any expiring referenda.
        for (index, info) in Self::maturing_referenda_at_inner(now, next..last).into_iter() {
            let approved = Self::bake_referendum(now, index, info)?;
            ReferendumInfoOf::<T>::insert(index, ReferendumInfo::Finished { end: now, approved });
            weight = max_block_weight;
        }

        Ok(weight)
    }

    /// determine whether or not a new referendum should be launched. This will return true
    /// once every week.
    fn should_launch(now: u64) -> bool {
        if now < NextLaunchTimestamp::<T>::get() {
            return false;
        }

        // update storage
        NextLaunchTimestamp::<T>::mutate(|next_launch_timestamp| {
            // period is number of seconds - e.g. to next week (mon 9am)
            let launch_period = T::LaunchPeriod::get();
            next_launch_timestamp.saturating_accrue(
                (now.saturating_sub(*next_launch_timestamp)
                    .saturating_div(launch_period)
                    .saturating_add(One::one()))
                .saturating_mul(launch_period),
            );
        });

        true
    }

    /// Reads the length of account in DepositOf without getting the complete value in the runtime.
    ///
    /// Return 0 if no deposit for this proposal.
    fn len_of_deposit_of(proposal: PropIndex) -> Option<u32> {
        // DepositOf first tuple element is a vec, decoding its len is equivalent to decode a
        // `Compact<u32>`.
        decode_compact_u32_at(&<DepositOf<T>>::hashed_key_for(proposal))
    }
}

/// Decode `Compact<u32>` from the trie at given key.
fn decode_compact_u32_at(key: &[u8]) -> Option<u32> {
    // `Compact<u32>` takes at most 5 bytes.
    let mut buf = [0u8; 5];
    let bytes = sp_io::storage::read(&key, &mut buf, 0)?;
    // The value may be smaller than 5 bytes.
    let mut input = &buf[0..buf.len().min(bytes as usize)];
    match codec::Compact::<u32>::decode(&mut input) {
        Ok(c) => Some(c.0),
        Err(_) => {
            sp_runtime::print("Failed to decode compact u32 at:");
            sp_runtime::print(key);
            None
        }
    }
}
