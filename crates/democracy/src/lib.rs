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
//! - `unvote` - Cancel a previous vote, this must be done by the voter before the vote ends.
//!
//! Administration actions that can be done to any account:
//! - `reap_vote` - Remove some account's expired votes.
//!
//! Preimage actions:
//! - `note_preimage` - Registers the preimage for an upcoming proposal, requires a deposit that is returned once the
//!   proposal is enacted.
//! - `note_imminent_preimage` - Registers the preimage for an upcoming proposal. Does not require a deposit, but the
//!   proposal must be in the dispatch queue.
//! - `reap_preimage` - Removes the preimage for an expired proposal. Will only work under the condition that it's the
//!   same account that noted it and after the voting period, OR it's a different account after the enactment period.
//!
//! #### Fast Track Origin
//!
//! This call can only be made by the `FastTrackOrigin`.
//!
//! - `fast_track` - Schedules the current externally proposed proposal that is "majority-carries" to become a
//!   referendum immediately.
//! - `fast_track_referendum` - Schedules an active referendum to end in `FastTrackVotingPeriod`
//!  blocks.
//!
//! #### Root
//!
//! - `cancel_referendum` - Removes a referendum.
//! - `cancel_queued` - Cancels a proposal that is queued for enactment.
//! - `clear_public_proposal` - Removes all public proposals.

#![deny(warnings)]
#![recursion_limit = "256"]
#![cfg_attr(not(feature = "std"), no_std)]

use core::time::Duration;

use chrono::Days;
use codec::{Decode, DecodeLimit, Encode, Input, MaxEncodedLen};
use frame_support::{
    ensure,
    traits::{
        schedule::{DispatchTime, Named as ScheduleNamed},
        BalanceStatus, Currency, Get, LockIdentifier, OnUnbalanced, ReservableCurrency, UnfilteredDispatchable,
        UnixTime,
    },
    transactional,
    weights::Weight,
};
use scale_info::TypeInfo;
use sp_runtime::{
    traits::{Hash, Saturating, Zero},
    ArithmeticError, DispatchError, DispatchResult, RuntimeDebug,
};
use sp_std::prelude::*;

mod types;
mod vote_threshold;
pub mod weights;
pub use pallet::*;
pub use types::{ReferendumInfo, ReferendumStatus, Tally, Vote, Voting};
pub use vote_threshold::{Approved, VoteThreshold};
pub use weights::WeightInfo;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

const DEMOCRACY_ID: LockIdentifier = *b"democrac";

/// A proposal index.
pub type PropIndex = u32;

/// A referendum index.
pub type ReferendumIndex = u32;

type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
type NegativeImbalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::NegativeImbalance;

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum PreimageStatus<AccountId, Balance, BlockNumber> {
    /// The preimage is imminently needed at the argument.
    Missing(BlockNumber),
    /// The preimage is available.
    Available {
        data: Vec<u8>,
        provider: AccountId,
        deposit: Balance,
        since: BlockNumber,
        /// None if it's not imminent.
        expiry: Option<BlockNumber>,
    },
}

impl<AccountId, Balance, BlockNumber> PreimageStatus<AccountId, Balance, BlockNumber> {
    fn to_missing_expiry(self) -> Option<BlockNumber> {
        match self {
            PreimageStatus::Missing(expiry) => Some(expiry),
            _ => None,
        }
    }
}

// A value placed in storage that represents the current version of the Democracy storage.
// This value is used by the `on_runtime_upgrade` logic to determine whether we run
// storage migration logic.
#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
enum Releases {
    V1,
}

#[frame_support::pallet]
pub mod pallet {
    use core::num::TryFromIntError;

    use super::*;
    use frame_support::{
        dispatch::{DispatchClass, DispatchResultWithPostInfo, Pays},
        pallet_prelude::*,
        traits::EnsureOrigin,
        Parameter,
    };
    use frame_system::{ensure_root, ensure_signed, pallet_prelude::*};
    use sp_runtime::DispatchResult;

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config + Sized {
        type Proposal: DecodeLimit
            + Parameter
            + UnfilteredDispatchable<RuntimeOrigin = Self::RuntimeOrigin>
            + From<Call<Self>>;
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Currency type for this pallet.
        type Currency: ReservableCurrency<Self::AccountId>;

        /// The period between a proposal being approved and enacted.
        ///
        /// It should generally be a little more than the unstake period to ensure that
        /// voting stakers have an opportunity to remove themselves from the system in the case
        /// where they are on the losing side of a vote.
        #[pallet::constant]
        type EnactmentPeriod: Get<Self::BlockNumber>;

        /// How often (in blocks) to check for new votes.
        #[pallet::constant]
        type VotingPeriod: Get<Self::BlockNumber>;

        /// The minimum amount to be used as a deposit for a public referendum proposal.
        #[pallet::constant]
        type MinimumDeposit: Get<BalanceOf<Self>>;

        /// Origin from which the next majority-carries (or more permissive) referendum may be
        /// tabled to vote according to the `FastTrackVotingPeriod` asynchronously in a similar
        /// manner to the emergency origin. It retains its threshold method.
        type FastTrackOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Minimum voting period allowed for a fast-track referendum.
        #[pallet::constant]
        type FastTrackVotingPeriod: Get<Self::BlockNumber>;

        /// The amount of balance that must be deposited per byte of preimage stored.
        #[pallet::constant]
        type PreimageByteDeposit: Get<BalanceOf<Self>>;

        /// Handler for the unbalanced reduction when slashing a preimage deposit.
        type Slash: OnUnbalanced<NegativeImbalanceOf<Self>>;

        /// The Scheduler.
        type Scheduler: ScheduleNamed<Self::BlockNumber, Self::Proposal, Self::PalletsOrigin>;

        /// Overarching type of all pallets origins.
        type PalletsOrigin: From<frame_system::RawOrigin<Self::AccountId>>;

        /// The maximum number of votes for an account.
        ///
        /// Also used to compute weight, an overly big value can
        /// lead to extrinsic with very big weight.
        #[pallet::constant]
        type MaxVotes: Get<u32>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;

        /// The maximum number of public proposals that can exist at any time.
        #[pallet::constant]
        type MaxProposals: Get<u32>;

        /// Unix time
        type UnixTime: UnixTime;

        /// Duration
        type Moment: TryInto<i64>;

        type LaunchOffsetMillis: Get<Self::Moment>;
    }

    /// The number of (public) proposals that have been made so far.
    #[pallet::storage]
    #[pallet::getter(fn public_prop_count)]
    pub type PublicPropCount<T> = StorageValue<_, PropIndex, ValueQuery>;

    /// The public proposals. Unsorted. The second item is the proposal's hash.
    #[pallet::storage]
    #[pallet::getter(fn public_props)]
    pub type PublicProps<T: Config> = StorageValue<_, Vec<(PropIndex, T::Hash, T::AccountId)>, ValueQuery>;

    /// Those who have locked a deposit.
    ///
    /// TWOX-NOTE: Safe, as increasing integer keys are safe.
    #[pallet::storage]
    #[pallet::getter(fn deposit_of)]
    pub type DepositOf<T: Config> = StorageMap<_, Twox64Concat, PropIndex, (Vec<T::AccountId>, BalanceOf<T>)>;

    /// Map of hashes to the proposal preimage, along with who registered it and their deposit.
    /// The block number is the block at which it was deposited.
    // TODO: Refactor Preimages into its own pallet.
    // https://github.com/paritytech/substrate/issues/5322
    #[pallet::storage]
    pub type Preimages<T: Config> =
        StorageMap<_, Identity, T::Hash, PreimageStatus<T::AccountId, BalanceOf<T>, T::BlockNumber>>;

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
        StorageMap<_, Twox64Concat, ReferendumIndex, ReferendumInfo<T::BlockNumber, T::Hash, BalanceOf<T>>>;

    /// All votes for a particular voter. We store the balance for the number of votes that we
    /// have recorded.
    ///
    /// TWOX-NOTE: SAFE as `AccountId`s are crypto hashes anyway.
    #[pallet::storage]
    pub type VotingOf<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, Voting<BalanceOf<T>>, ValueQuery>;

    /// Storage version of the pallet.
    ///
    /// New networks start with last version.
    #[pallet::storage]
    pub(crate) type StorageVersion<T> = StorageValue<_, Releases>;

    #[pallet::storage]
    pub type NextLaunchTimestamp<T: Config> = StorageValue<_, u64, ValueQuery>;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        _phantom: sp_std::marker::PhantomData<T>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            GenesisConfig {
                _phantom: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            PublicPropCount::<T>::put(0 as PropIndex);
            ReferendumCount::<T>::put(0 as ReferendumIndex);
            LowestUnbaked::<T>::put(0 as ReferendumIndex);
            StorageVersion::<T>::put(Releases::V1);
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A motion has been proposed by a public account. \[proposal_index, deposit\]
        Proposed(PropIndex, BalanceOf<T>),
        /// A public proposal has been tabled for referendum vote. \[proposal_index, deposit,
        /// depositors\]
        Tabled(PropIndex, BalanceOf<T>, Vec<T::AccountId>),
        /// A referendum has begun. \[ref_index, threshold\]
        Started(ReferendumIndex, VoteThreshold),
        /// A proposal has been fast tracked. \[ref_index\]
        FastTrack(ReferendumIndex),
        /// A referendum has been fast tracked. \[ref_index\]
        FastTrackReferendum(ReferendumIndex),
        /// A proposal has been approved by referendum. \[ref_index\]
        Passed(ReferendumIndex),
        /// A proposal has been rejected by referendum. \[ref_index\]
        NotPassed(ReferendumIndex),
        /// A referendum has been cancelled. \[ref_index\]
        Cancelled(ReferendumIndex),
        /// A proposal has been enacted. \[ref_index, result\]
        Executed(ReferendumIndex, DispatchResult),
        /// A proposal's preimage was noted, and the deposit taken. \[proposal_hash, who, deposit\]
        PreimageNoted(T::Hash, T::AccountId, BalanceOf<T>),
        /// A proposal preimage was removed and used (the deposit was returned).
        /// \[proposal_hash, provider, deposit\]
        PreimageUsed(T::Hash, T::AccountId, BalanceOf<T>),
        /// A proposal could not be executed because its preimage was invalid.
        /// \[proposal_hash, ref_index\]
        PreimageInvalid(T::Hash, ReferendumIndex),
        /// A proposal could not be executed because its preimage was missing.
        /// \[proposal_hash, ref_index\]
        PreimageMissing(T::Hash, ReferendumIndex),
        /// A registered preimage was removed and the deposit collected by the reaper.
        /// \[proposal_hash, provider, deposit, reaper\]
        PreimageReaped(T::Hash, T::AccountId, BalanceOf<T>, T::AccountId),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Value too low
        ValueLow,
        /// Proposal does not exist
        ProposalMissing,
        /// Proposal already made
        DuplicateProposal,
        /// Preimage already noted
        DuplicatePreimage,
        /// Not imminent
        NotImminent,
        /// Too early
        TooEarly,
        /// Imminent
        Imminent,
        /// Preimage not found
        PreimageMissing,
        /// Vote given for invalid referendum
        ReferendumInvalid,
        /// Fast tracking failed, because the referendum is
        /// ending sooner than the fast track voting period.
        ReferendumFastTrackFailed,
        /// Invalid preimage
        PreimageInvalid,
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
        /// Maximum number of proposals reached.
        TooManyProposals,
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
        fn on_initialize(n: T::BlockNumber) -> Weight {
            Self::begin_block(n).unwrap_or_else(|e| {
                sp_runtime::print(e);
                Weight::from_ref_time(0 as u64)
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
            proposal_hash: T::Hash,
            #[pallet::compact] value: BalanceOf<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(value >= T::MinimumDeposit::get(), Error::<T>::ValueLow);

            let index = Self::public_prop_count();
            let real_prop_count = PublicProps::<T>::decode_len().unwrap_or(0) as u32;
            let max_proposals = T::MaxProposals::get();
            ensure!(real_prop_count < max_proposals, Error::<T>::TooManyProposals);

            T::Currency::reserve(&who, value)?;
            PublicPropCount::<T>::put(index + 1);
            <DepositOf<T>>::insert(index, (&[&who][..], value));

            <PublicProps<T>>::append((index, proposal_hash, who));

            Self::deposit_event(Event::<T>::Proposed(index, value));
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
        #[pallet::weight(T::WeightInfo::second(*seconds_upper_bound))]
        pub fn second(
            origin: OriginFor<T>,
            #[pallet::compact] proposal: PropIndex,
            #[pallet::compact] seconds_upper_bound: u32,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let seconds = Self::len_of_deposit_of(proposal).ok_or_else(|| Error::<T>::ProposalMissing)?;
            ensure!(seconds <= seconds_upper_bound, Error::<T>::WrongUpperBound);
            let mut deposit = Self::deposit_of(proposal).ok_or(Error::<T>::ProposalMissing)?;
            T::Currency::reserve(&who, deposit.1)?;
            deposit.0.push(who);
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
        #[pallet::weight(
			T::WeightInfo::vote_new(T::MaxVotes::get())
				.max(T::WeightInfo::vote_existing(T::MaxVotes::get()))
		)]
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
            delay: T::BlockNumber,
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
        #[pallet::weight(T::WeightInfo::fast_track())]
        pub fn fast_track_default(
            origin: OriginFor<T>,
            #[pallet::compact] prop_index: PropIndex,
            delay: T::BlockNumber,
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
            Self::deposit_event(Event::<T>::FastTrackReferendum(ref_index));
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

        /// Cancel a proposal queued for enactment.
        ///
        /// The dispatch origin of this call must be _Root_.
        ///
        /// - `which`: The index of the referendum to cancel.
        ///
        /// Weight: `O(D)` where `D` is the items in the dispatch queue. Weighted as `D = 10`.
        #[pallet::call_index(7)]
        #[pallet::weight((T::WeightInfo::cancel_queued(10), DispatchClass::Operational))]
        pub fn cancel_queued(origin: OriginFor<T>, which: ReferendumIndex) -> DispatchResult {
            ensure_root(origin)?;
            T::Scheduler::cancel_named((DEMOCRACY_ID, which).encode()).map_err(|_| Error::<T>::ProposalMissing)?;
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
        #[pallet::weight(T::WeightInfo::cancel_proposal(T::MaxProposals::get()))]
        #[transactional]
        pub fn cancel_proposal(origin: OriginFor<T>, #[pallet::compact] prop_index: PropIndex) -> DispatchResult {
            let who = ensure_signed(origin.clone())
                .map(Some)
                .or_else(|_| ensure_root(origin).map(|_| None))?;

            PublicProps::<T>::try_mutate(|props| {
                if let Some(i) = props.iter().position(|p| p.0 == prop_index) {
                    let (_, _, proposer) = props.remove(i);
                    if let Some(account_id) = who {
                        ensure!(proposer == account_id, Error::<T>::NotProposer);
                    }
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

            Ok(())
        }

        /// Register the preimage for an upcoming proposal. This doesn't require the proposal to be
        /// in the dispatch queue but does require a deposit, returned once enacted.
        ///
        /// The dispatch origin of this call must be _Signed_.
        ///
        /// - `encoded_proposal`: The preimage of a proposal.
        ///
        /// Emits `PreimageNoted`.
        ///
        /// Weight: `O(E)` with E size of `encoded_proposal` (protected by a required deposit).
        #[pallet::call_index(10)]
        #[pallet::weight(T::WeightInfo::note_preimage(encoded_proposal.len() as u32))]
        pub fn note_preimage(origin: OriginFor<T>, encoded_proposal: Vec<u8>) -> DispatchResult {
            Self::note_preimage_inner(ensure_signed(origin)?, encoded_proposal)?;
            Ok(())
        }

        /// Register the preimage for an upcoming proposal. This requires the proposal to be
        /// in the dispatch queue. No deposit is needed. When this call is successful, i.e.
        /// the preimage has not been uploaded before and matches some imminent proposal,
        /// no fee is paid.
        ///
        /// The dispatch origin of this call must be _Signed_.
        ///
        /// - `encoded_proposal`: The preimage of a proposal.
        ///
        /// Emits `PreimageNoted`.
        ///
        /// Weight: `O(E)` with E size of `encoded_proposal` (protected by a required deposit).
        #[pallet::call_index(11)]
        #[pallet::weight(T::WeightInfo::note_imminent_preimage(encoded_proposal.len() as u32))]
        pub fn note_imminent_preimage(origin: OriginFor<T>, encoded_proposal: Vec<u8>) -> DispatchResultWithPostInfo {
            Self::note_imminent_preimage_inner(ensure_signed(origin)?, encoded_proposal)?;
            // We check that this preimage was not uploaded before in
            // `note_imminent_preimage_inner`, thus this call can only be successful once. If
            // successful, user does not pay a fee.
            Ok(Pays::No.into())
        }

        /// Remove an expired proposal preimage and collect the deposit.
        ///
        /// The dispatch origin of this call must be _Signed_.
        ///
        /// - `proposal_hash`: The preimage hash of a proposal.
        /// - `proposal_length_upper_bound`: an upper bound on length of the proposal. Extrinsic is weighted according
        ///   to this value with no refund.
        ///
        /// This will only work after `VotingPeriod` blocks from the time that the preimage was
        /// noted, if it's the same account doing it. If it's a different account, then it'll only
        /// work an additional `EnactmentPeriod` later.
        ///
        /// Emits `PreimageReaped`.
        ///
        /// Weight: `O(D)` where D is length of proposal.
        #[pallet::call_index(12)]
        #[pallet::weight(T::WeightInfo::reap_preimage(*proposal_len_upper_bound))]
        pub fn reap_preimage(
            origin: OriginFor<T>,
            proposal_hash: T::Hash,
            #[pallet::compact] proposal_len_upper_bound: u32,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            ensure!(
                Self::pre_image_data_len(proposal_hash)? <= proposal_len_upper_bound,
                Error::<T>::WrongUpperBound,
            );

            let (provider, deposit, since, expiry) = <Preimages<T>>::get(&proposal_hash)
                .and_then(|m| match m {
                    PreimageStatus::Available {
                        provider,
                        deposit,
                        since,
                        expiry,
                        ..
                    } => Some((provider, deposit, since, expiry)),
                    _ => None,
                })
                .ok_or(Error::<T>::PreimageMissing)?;

            let now = <frame_system::Pallet<T>>::block_number();
            let (voting, enactment) = (T::VotingPeriod::get(), T::EnactmentPeriod::get());
            let additional = if who == provider { Zero::zero() } else { enactment };
            ensure!(
                now >= since.saturating_add(voting).saturating_add(additional),
                Error::<T>::TooEarly
            );
            ensure!(expiry.map_or(true, |e| now > e), Error::<T>::Imminent);

            let res = T::Currency::repatriate_reserved(&provider, &who, deposit, BalanceStatus::Free);
            debug_assert!(res.is_ok());
            <Preimages<T>>::remove(&proposal_hash);
            Self::deposit_event(Event::<T>::PreimageReaped(proposal_hash, provider, deposit, who));
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

        /// Enact a proposal from a referendum. For now we just make the weight be the maximum.
        #[pallet::call_index(14)]
        #[pallet::weight(1000)]
        pub fn enact_proposal(origin: OriginFor<T>, proposal_hash: T::Hash, index: ReferendumIndex) -> DispatchResult {
            ensure_root(origin)?;
            Self::do_enact_proposal(proposal_hash, index)
        }
    }
}

impl<T: Config> Pallet<T> {
    // exposed immutables.

    /// Get the amount locked in support of `proposal`; `None` if proposal isn't a valid proposal
    /// index.
    pub fn backing_for(proposal: PropIndex) -> Option<BalanceOf<T>> {
        Self::deposit_of(proposal).map(|(l, d)| d.saturating_mul((l.len() as u32).into()))
    }

    /// Get all referenda ready for tally at block `n`.
    pub fn maturing_referenda_at(
        n: T::BlockNumber,
    ) -> Vec<(ReferendumIndex, ReferendumStatus<T::BlockNumber, T::Hash, BalanceOf<T>>)> {
        let next = Self::lowest_unbaked();
        let last = Self::referendum_count();
        Self::maturing_referenda_at_inner(n, next..last)
    }

    fn maturing_referenda_at_inner(
        n: T::BlockNumber,
        range: core::ops::Range<PropIndex>,
    ) -> Vec<(ReferendumIndex, ReferendumStatus<T::BlockNumber, T::Hash, BalanceOf<T>>)> {
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
        proposal_hash: T::Hash,
        threshold: VoteThreshold,
        delay: T::BlockNumber,
    ) -> ReferendumIndex {
        <Pallet<T>>::inject_referendum(
            <frame_system::Pallet<T>>::block_number().saturating_add(T::VotingPeriod::get()),
            proposal_hash,
            threshold,
            delay,
        )
    }

    /// Remove a referendum.
    pub fn internal_cancel_referendum(ref_index: ReferendumIndex) {
        Self::deposit_event(Event::<T>::Cancelled(ref_index));
        ReferendumInfoOf::<T>::remove(ref_index);
    }

    // private.

    fn fast_track_with_voting_period(
        prop_index: PropIndex,
        delay: T::BlockNumber,
        voting_period: T::BlockNumber,
    ) -> DispatchResult {
        let mut public_props = <PublicProps<T>>::get();
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
            VoteThreshold::SuperMajorityAgainst,
            delay,
        );
        Self::deposit_event(Event::<T>::FastTrack(ref_index));
        Ok(())
    }

    /// Ok if the given referendum is active, Err otherwise
    fn ensure_ongoing(
        r: ReferendumInfo<T::BlockNumber, T::Hash, BalanceOf<T>>,
    ) -> Result<ReferendumStatus<T::BlockNumber, T::Hash, BalanceOf<T>>, DispatchError> {
        match r {
            ReferendumInfo::Ongoing(s) => Ok(s),
            _ => Err(Error::<T>::ReferendumInvalid.into()),
        }
    }

    fn referendum_status(
        ref_index: ReferendumIndex,
    ) -> Result<ReferendumStatus<T::BlockNumber, T::Hash, BalanceOf<T>>, DispatchError> {
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
                    ensure!(votes.len() as u32 <= T::MaxVotes::get(), Error::<T>::MaxVotesReached);
                    votes.insert(i, (ref_index, vote));
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
        end: T::BlockNumber,
        proposal_hash: T::Hash,
        threshold: VoteThreshold,
        delay: T::BlockNumber,
    ) -> ReferendumIndex {
        let ref_index = Self::referendum_count();
        ReferendumCount::<T>::put(ref_index + 1);
        let status = ReferendumStatus {
            end,
            proposal_hash,
            threshold,
            delay,
            tally: Default::default(),
        };
        let item = ReferendumInfo::Ongoing(status);
        <ReferendumInfoOf<T>>::insert(ref_index, item);
        Self::deposit_event(Event::<T>::Started(ref_index, threshold));
        ref_index
    }

    /// Table the next waiting proposal for a vote.
    fn launch_next(now: T::BlockNumber) -> DispatchResult {
        Self::launch_public(now).map_err(|_| Error::<T>::NoneWaiting.into())
    }

    /// Table the waiting public proposal with the highest backing for a vote.
    fn launch_public(now: T::BlockNumber) -> DispatchResult {
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
                Self::deposit_event(Event::<T>::Tabled(prop_index, deposit, depositors));
                Self::inject_referendum(
                    now.saturating_add(T::VotingPeriod::get()),
                    proposal,
                    VoteThreshold::SuperMajorityAgainst,
                    T::EnactmentPeriod::get(),
                );
            }
            Ok(())
        } else {
            Err(Error::<T>::NoneWaiting)?
        }
    }

    fn do_enact_proposal(proposal_hash: T::Hash, index: ReferendumIndex) -> DispatchResult {
        let preimage = <Preimages<T>>::take(&proposal_hash);
        if let Some(PreimageStatus::Available {
            data,
            provider,
            deposit,
            ..
        }) = preimage
        {
            if let Ok(proposal) = T::Proposal::decode_with_depth_limit(sp_api::MAX_EXTRINSIC_DEPTH, &mut &data[..]) {
                let err_amount = T::Currency::unreserve(&provider, deposit);
                debug_assert!(err_amount.is_zero());
                Self::deposit_event(Event::<T>::PreimageUsed(proposal_hash, provider, deposit));

                let res = proposal
                    .dispatch_bypass_filter(frame_system::RawOrigin::Root.into())
                    .map(|_| ())
                    .map_err(|e| e.error);
                Self::deposit_event(Event::<T>::Executed(index, res));

                Ok(())
            } else {
                T::Slash::on_unbalanced(T::Currency::slash_reserved(&provider, deposit).0);
                Self::deposit_event(Event::<T>::PreimageInvalid(proposal_hash, index));
                Err(Error::<T>::PreimageInvalid.into())
            }
        } else {
            Self::deposit_event(Event::<T>::PreimageMissing(proposal_hash, index));
            Err(Error::<T>::PreimageMissing.into())
        }
    }

    fn bake_referendum(
        now: T::BlockNumber,
        index: ReferendumIndex,
        status: ReferendumStatus<T::BlockNumber, T::Hash, BalanceOf<T>>,
    ) -> Result<bool, DispatchError> {
        // TODO: dynamically calculate votes from escrow
        let total_issuance = T::Currency::total_issuance();
        let approved = status.threshold.approved(status.tally, total_issuance);

        if approved {
            Self::deposit_event(Event::<T>::Passed(index));
            if status.delay.is_zero() {
                let _ = Self::do_enact_proposal(status.proposal_hash, index);
            } else {
                let when = now.saturating_add(status.delay);
                // Note that we need the preimage now.
                Preimages::<T>::mutate_exists(&status.proposal_hash, |maybe_pre| match *maybe_pre {
                    Some(PreimageStatus::Available { ref mut expiry, .. }) => *expiry = Some(when),
                    ref mut a => *a = Some(PreimageStatus::Missing(when)),
                });

                if T::Scheduler::schedule_named(
                    (DEMOCRACY_ID, index).encode(),
                    DispatchTime::At(when),
                    None,
                    63,
                    frame_system::RawOrigin::Root.into(),
                    Call::enact_proposal {
                        proposal_hash: status.proposal_hash,
                        index,
                    }
                    .into(),
                )
                .is_err()
                {
                    frame_support::print("LOGIC ERROR: bake_referendum/schedule_named failed");
                }
            }
        } else {
            Self::deposit_event(Event::<T>::NotPassed(index));
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
    fn begin_block(now: T::BlockNumber) -> Result<Weight, DispatchError> {
        let max_block_weight = T::BlockWeights::get().max_block;
        let mut weight = Weight::from_ref_time(0 as u64);

        let next = Self::lowest_unbaked();
        let last = Self::referendum_count();
        let r = last.saturating_sub(next);

        // pick out another public referendum if it's time.
        let current_time = T::UnixTime::now();
        if Self::should_launch(current_time)? {
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
    fn should_launch(now: Duration) -> Result<bool, DispatchError> {
        if now.as_secs() < NextLaunchTimestamp::<T>::get() {
            return Ok(false);
        }

        // time to launch - calculate the date of next launch.

        // convert to format used by `chrono`
        let secs: i64 = now.as_secs().try_into().map_err(Error::<T>::from)?;
        let now =
            chrono::NaiveDateTime::from_timestamp_opt(secs, now.subsec_nanos()).ok_or(Error::<T>::TryIntoIntError)?;

        // calculate next week boundary
        let beginning_of_week = now.date().week(chrono::Weekday::Mon).first_day();
        let next_week = beginning_of_week
            .checked_add_days(Days::new(7))
            .ok_or(Error::<T>::TryIntoIntError)?
            .and_time(Default::default());

        let offset = T::LaunchOffsetMillis::get()
            .try_into()
            .map_err(|_| Error::<T>::TryIntoIntError)?;
        let next_launch = next_week
            .checked_add_signed(chrono::Duration::milliseconds(offset))
            .ok_or(ArithmeticError::Overflow)?;

        // update storage
        let next_timestamp: u64 = next_launch.timestamp().try_into().map_err(Error::<T>::from)?;
        NextLaunchTimestamp::<T>::set(next_timestamp);

        Ok(true)
    }

    /// Reads the length of account in DepositOf without getting the complete value in the runtime.
    ///
    /// Return 0 if no deposit for this proposal.
    fn len_of_deposit_of(proposal: PropIndex) -> Option<u32> {
        // DepositOf first tuple element is a vec, decoding its len is equivalent to decode a
        // `Compact<u32>`.
        decode_compact_u32_at(&<DepositOf<T>>::hashed_key_for(proposal))
    }

    /// Check that pre image exists and its value is variant `PreimageStatus::Missing`.
    ///
    /// This check is done without getting the complete value in the runtime to avoid copying a big
    /// value in the runtime.
    fn check_pre_image_is_missing(proposal_hash: T::Hash) -> DispatchResult {
        // To decode the enum variant we only need the first byte.
        let mut buf = [0u8; 1];
        let key = <Preimages<T>>::hashed_key_for(proposal_hash);
        let bytes = sp_io::storage::read(&key, &mut buf, 0).ok_or_else(|| Error::<T>::NotImminent)?;
        // The value may be smaller that 1 byte.
        let mut input = &buf[0..buf.len().min(bytes as usize)];

        match input.read_byte() {
            Ok(0) => Ok(()), // PreimageStatus::Missing is variant 0
            Ok(1) => Err(Error::<T>::DuplicatePreimage.into()),
            _ => {
                sp_runtime::print("Failed to decode `PreimageStatus` variant");
                Err(Error::<T>::NotImminent.into())
            }
        }
    }

    /// Check that pre image exists, its value is variant `PreimageStatus::Available` and decode
    /// the length of `data: Vec<u8>` fields.
    ///
    /// This check is done without getting the complete value in the runtime to avoid copying a big
    /// value in the runtime.
    ///
    /// If the pre image is missing variant or doesn't exist then the error `PreimageMissing` is
    /// returned.
    fn pre_image_data_len(proposal_hash: T::Hash) -> Result<u32, DispatchError> {
        // To decode the `data` field of Available variant we need:
        // * one byte for the variant
        // * at most 5 bytes to decode a `Compact<u32>`
        let mut buf = [0u8; 6];
        let key = <Preimages<T>>::hashed_key_for(proposal_hash);
        let bytes = sp_io::storage::read(&key, &mut buf, 0).ok_or_else(|| Error::<T>::PreimageMissing)?;
        // The value may be smaller that 6 bytes.
        let mut input = &buf[0..buf.len().min(bytes as usize)];

        match input.read_byte() {
            Ok(1) => (), // Check that input exists and is second variant.
            Ok(0) => return Err(Error::<T>::PreimageMissing.into()),
            _ => {
                sp_runtime::print("Failed to decode `PreimageStatus` variant");
                return Err(Error::<T>::PreimageMissing.into());
            }
        }

        // Decode the length of the vector.
        let len = codec::Compact::<u32>::decode(&mut input)
            .map_err(|_| {
                sp_runtime::print("Failed to decode `PreimageStatus` variant");
                DispatchError::from(Error::<T>::PreimageMissing)
            })?
            .0;

        Ok(len)
    }

    // See `note_preimage`
    fn note_preimage_inner(who: T::AccountId, encoded_proposal: Vec<u8>) -> DispatchResult {
        let proposal_hash = T::Hashing::hash(&encoded_proposal[..]);
        ensure!(
            !<Preimages<T>>::contains_key(&proposal_hash),
            Error::<T>::DuplicatePreimage
        );

        let deposit = <BalanceOf<T>>::from(encoded_proposal.len() as u32).saturating_mul(T::PreimageByteDeposit::get());
        T::Currency::reserve(&who, deposit)?;

        let now = <frame_system::Pallet<T>>::block_number();
        let a = PreimageStatus::Available {
            data: encoded_proposal,
            provider: who.clone(),
            deposit,
            since: now,
            expiry: None,
        };
        <Preimages<T>>::insert(proposal_hash, a);

        Self::deposit_event(Event::<T>::PreimageNoted(proposal_hash, who, deposit));

        Ok(())
    }

    // See `note_imminent_preimage`
    fn note_imminent_preimage_inner(who: T::AccountId, encoded_proposal: Vec<u8>) -> DispatchResult {
        let proposal_hash = T::Hashing::hash(&encoded_proposal[..]);
        Self::check_pre_image_is_missing(proposal_hash)?;
        let status = Preimages::<T>::get(&proposal_hash).ok_or(Error::<T>::NotImminent)?;
        let expiry = status.to_missing_expiry().ok_or(Error::<T>::DuplicatePreimage)?;

        let now = <frame_system::Pallet<T>>::block_number();
        let free = <BalanceOf<T>>::zero();
        let a = PreimageStatus::Available {
            data: encoded_proposal,
            provider: who.clone(),
            deposit: Zero::zero(),
            since: now,
            expiry: Some(expiry),
        };
        <Preimages<T>>::insert(proposal_hash, a);

        Self::deposit_event(Event::<T>::PreimageNoted(proposal_hash, who, free));

        Ok(())
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
