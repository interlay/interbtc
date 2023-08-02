//! Miscellaneous additional datatypes.

use crate::{ReferendumIndex, VoteThreshold};
use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::traits::Get;
use scale_info::TypeInfo;
use sp_runtime::{
    traits::{Bounded, CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, Saturating, Zero},
    BoundedVec, RuntimeDebug,
};
use sp_std::prelude::*;

/// A standard vote, one-way (approve or reject).
#[derive(Encode, Decode, Copy, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct Vote<Balance> {
    pub aye: bool,
    pub balance: Balance,
}

/// The account is voting directly.
#[derive(Clone, Encode, Decode, Eq, MaxEncodedLen, PartialEq, RuntimeDebug, TypeInfo)]
#[codec(mel_bound(skip_type_params(MaxVotes)))]
#[scale_info(skip_type_params(MaxVotes))]
pub struct Voting<Balance, MaxVotes: Get<u32>> {
    /// The current votes of the account.
    pub(crate) votes: BoundedVec<(ReferendumIndex, Vote<Balance>), MaxVotes>,
}

impl<Balance: Default, MaxVotes: Get<u32>> Default for Voting<Balance, MaxVotes> {
    fn default() -> Self {
        Voting {
            votes: Default::default(),
        }
    }
}

/// Info regarding an ongoing referendum.
#[derive(Encode, Decode, Default, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct Tally<Balance> {
    /// The number of aye votes.
    pub ayes: Balance,
    /// The number of nay votes.
    pub nays: Balance,
    /// The amount of funds currently expressing its opinion.
    pub turnout: Balance,
}

impl<Balance: From<u8> + Zero + Copy + CheckedAdd + CheckedSub + CheckedMul + CheckedDiv + Bounded + Saturating>
    Tally<Balance>
{
    /// Add an account's vote into the tally.
    pub(crate) fn add(&mut self, vote: Vote<Balance>) -> Option<()> {
        self.turnout = self.turnout.checked_add(&vote.balance)?;
        match vote.aye {
            true => self.ayes = self.ayes.checked_add(&vote.balance)?,
            false => self.nays = self.nays.checked_add(&vote.balance)?,
        }

        Some(())
    }

    /// Remove an account's vote from the tally.
    pub(crate) fn remove(&mut self, vote: Vote<Balance>) -> Option<()> {
        self.turnout = self.turnout.checked_sub(&vote.balance)?;
        match vote.aye {
            true => self.ayes = self.ayes.checked_sub(&vote.balance)?,
            false => self.nays = self.nays.checked_sub(&vote.balance)?,
        }

        Some(())
    }
}

/// Info regarding an ongoing referendum.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct ReferendumStatus<BlockNumber, Proposal, Balance> {
    /// When voting on this referendum will end.
    pub end: BlockNumber,
    /// The proposal being voted on.
    pub proposal: Proposal,
    /// The thresholding mechanism to determine whether it passed.
    pub threshold: VoteThreshold,
    /// The delay (in blocks) to wait after a successful referendum before deploying.
    pub delay: BlockNumber,
    /// The current tally of votes in this referendum.
    pub tally: Tally<Balance>,
}

/// Info regarding a referendum, present or past.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum ReferendumInfo<BlockNumber, Proposal, Balance> {
    /// Referendum is happening, the arg is the block number at which it will end.
    Ongoing(ReferendumStatus<BlockNumber, Proposal, Balance>),
    /// Referendum finished at `end`, and has been `approved` or rejected.
    Finished { approved: bool, end: BlockNumber },
}
