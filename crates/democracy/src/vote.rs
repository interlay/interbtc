//! The vote datatype.

use crate::{Conviction, ReferendumIndex};
use codec::{Decode, Encode, EncodeLike, Input, Output};
use scale_info::TypeInfo;
use sp_runtime::{
    traits::{Saturating, Zero},
    RuntimeDebug,
};
use sp_std::{convert::TryFrom, prelude::*, result::Result};

/// A number of lock periods, plus a vote, one way or the other.
#[derive(Copy, Clone, Eq, PartialEq, Default, RuntimeDebug)]
pub struct Vote {
    pub aye: bool,
    pub conviction: Conviction,
}

impl Encode for Vote {
    fn encode_to<T: Output + ?Sized>(&self, output: &mut T) {
        output.push_byte(u8::from(self.conviction) | if self.aye { 0b1000_0000 } else { 0 });
    }
}

impl EncodeLike for Vote {}

impl Decode for Vote {
    fn decode<I: Input>(input: &mut I) -> Result<Self, codec::Error> {
        let b = input.read_byte()?;
        Ok(Vote {
            aye: (b & 0b1000_0000) == 0b1000_0000,
            conviction: Conviction::try_from(b & 0b0111_1111).map_err(|_| codec::Error::from("Invalid conviction"))?,
        })
    }
}

impl TypeInfo for Vote {
    type Identity = Self;

    fn type_info() -> scale_info::Type {
        scale_info::Type::builder()
            .path(scale_info::Path::new("Vote", module_path!()))
            .composite(
                scale_info::build::Fields::unnamed()
                    .field(|f| f.ty::<u8>().docs(&["Raw vote byte, encodes aye + conviction"])),
            )
    }
}

/// A vote for a referendum of a particular account.
#[derive(Encode, Decode, Copy, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo)]
pub enum AccountVote<Balance> {
    /// A standard vote, one-way (approve or reject) with a given amount of conviction.
    Standard { vote: Vote, balance: Balance },
    /// A split vote with balances given for both ways, and with no conviction, useful for
    /// parachains when voting.
    Split { aye: Balance, nay: Balance },
}

impl<Balance: Saturating> AccountVote<Balance> {
    /// Returns `Some` of the lock periods that the account is locked for, assuming that the
    /// referendum passed iff `approved` is `true`.
    pub fn locked_if(self, approved: bool) -> Option<(u32, Balance)> {
        // winning side: can only be removed after the lock period ends.
        match self {
            AccountVote::Standard { vote, balance } if vote.aye == approved => {
                Some((vote.conviction.lock_periods(), balance))
            }
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

/// A "prior" lock, i.e. a lock for some now-forgotten reason.
#[derive(Encode, Decode, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, RuntimeDebug, TypeInfo)]
pub struct PriorLock<BlockNumber, Balance>(BlockNumber, Balance);

impl<BlockNumber: Ord + Copy + Zero, Balance: Ord + Copy + Zero> PriorLock<BlockNumber, Balance> {
    /// Accumulates an additional lock.
    pub fn accumulate(&mut self, until: BlockNumber, amount: Balance) {
        self.0 = self.0.max(until);
        self.1 = self.1.max(amount);
    }

    pub fn locked(&self) -> Balance {
        self.1
    }

    pub fn rejig(&mut self, now: BlockNumber) {
        if now >= self.0 {
            self.0 = Zero::zero();
            self.1 = Zero::zero();
        }
    }
}

/// The account is voting directly.
#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo)]
pub struct Voting<Balance, BlockNumber> {
    /// The current votes of the account.
    pub(crate) votes: Vec<(ReferendumIndex, AccountVote<Balance>)>,
    /// Any pre-existing locks from past voting activity.
    pub(crate) prior: PriorLock<BlockNumber, Balance>,
}

impl<Balance: Default, BlockNumber: Zero> Default for Voting<Balance, BlockNumber> {
    fn default() -> Self {
        Voting {
            votes: Vec::new(),
            prior: PriorLock(Zero::zero(), Default::default()),
        }
    }
}

impl<Balance: Saturating + Ord + Zero + Copy, BlockNumber: Ord + Copy + Zero> Voting<Balance, BlockNumber> {
    pub fn rejig(&mut self, now: BlockNumber) {
        self.prior.rejig(now);
    }

    /// The amount of this account's balance that much currently be locked due to voting.
    pub fn locked_balance(&self) -> Balance {
        let Voting { votes, prior, .. } = self;
        votes
            .iter()
            .map(|i| i.1.balance())
            .fold(prior.locked(), |a, i| a.max(i))
    }

    pub fn set_common(&mut self, p: PriorLock<BlockNumber, Balance>) {
        let Voting { ref mut prior, .. } = self;
        *prior = p;
    }
}
