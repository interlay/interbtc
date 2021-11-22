//! The vote datatype.

use crate::ReferendumIndex;
use codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_runtime::{traits::Saturating, RuntimeDebug};
use sp_std::prelude::*;

/// A number of lock periods, plus a vote, one way or the other.
#[derive(Encode, Decode, Copy, Clone, Eq, PartialEq, Default, RuntimeDebug, TypeInfo)]
pub struct Vote {
    pub aye: bool,
}

/// A vote for a referendum of a particular account.
#[derive(Encode, Decode, Copy, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo)]
pub enum AccountVote<Balance> {
    /// A standard vote, one-way (approve or reject).
    Standard { vote: Vote, balance: Balance },
    /// A split vote with balances given for both ways, useful for
    /// parachains when voting.
    Split { aye: Balance, nay: Balance },
}

impl<Balance: Saturating> AccountVote<Balance> {
    /// Returns `Some` of the lock periods that the account is locked for, assuming that the
    /// referendum passed iff `approved` is `true`.
    pub fn locked_if(self, approved: bool) -> Option<Balance> {
        // winning side: can only be removed after the lock period ends.
        match self {
            AccountVote::Standard { vote, balance } if vote.aye == approved => Some(balance),
            _ => None,
        }
    }

    /// The total balance involved in this vote.
    pub fn balance(self) -> Balance {
        match self {
            AccountVote::Standard { balance, .. } => balance,
            AccountVote::Split { aye, nay } => aye.saturating_add(nay),
        }
    }

    /// Returns `Some` with whether the vote is an aye vote if it is standard, otherwise `None` if
    /// it is split.
    pub fn as_standard(self) -> Option<bool> {
        match self {
            AccountVote::Standard { vote, .. } => Some(vote.aye),
            _ => None,
        }
    }
}

/// The account is voting directly.
#[derive(Encode, Decode, Clone, Eq, PartialEq, Default, RuntimeDebug, TypeInfo)]
pub struct Voting<Balance> {
    /// The current votes of the account.
    pub(crate) votes: Vec<(ReferendumIndex, AccountVote<Balance>)>,
}
