//! Miscellaneous additional datatypes.

use crate::{AccountVote, Vote, VoteThreshold};
use codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_runtime::{
    traits::{Bounded, CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, Saturating, Zero},
    RuntimeDebug,
};

/// Info regarding an ongoing referendum.
#[derive(Encode, Decode, Default, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
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
    /// Create a new tally.
    pub fn new(vote: Vote, balance: Balance) -> Self {
        Self {
            ayes: if vote.aye { balance } else { Zero::zero() },
            nays: if vote.aye { Zero::zero() } else { balance },
            turnout: balance,
        }
    }

    /// Add an account's vote into the tally.
    pub fn add(&mut self, vote: AccountVote<Balance>) -> Option<()> {
        match vote {
            AccountVote::Standard { vote, balance } => {
                self.turnout = self.turnout.checked_add(&balance)?;
                match vote.aye {
                    true => self.ayes = self.ayes.checked_add(&balance)?,
                    false => self.nays = self.nays.checked_add(&balance)?,
                }
            }
            AccountVote::Split { aye, nay } => {
                self.turnout = self.turnout.checked_add(&aye)?.checked_add(&nay)?;
                self.ayes = self.ayes.checked_add(&aye)?;
                self.nays = self.nays.checked_add(&nay)?;
            }
        }
        Some(())
    }

    /// Remove an account's vote from the tally.
    pub fn remove(&mut self, vote: AccountVote<Balance>) -> Option<()> {
        match vote {
            AccountVote::Standard { vote, balance } => {
                self.turnout = self.turnout.checked_sub(&balance)?;
                match vote.aye {
                    true => self.ayes = self.ayes.checked_sub(&balance)?,
                    false => self.nays = self.nays.checked_sub(&balance)?,
                }
            }
            AccountVote::Split { aye, nay } => {
                self.turnout = self.turnout.checked_sub(&aye)?.checked_sub(&nay)?;
                self.ayes = self.ayes.checked_sub(&aye)?;
                self.nays = self.nays.checked_sub(&nay)?;
            }
        }
        Some(())
    }
}

/// Info regarding an ongoing referendum.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct ReferendumStatus<BlockNumber, Hash, Balance> {
    /// When voting on this referendum will end.
    pub end: BlockNumber,
    /// The hash of the proposal being voted on.
    pub proposal_hash: Hash,
    /// The thresholding mechanism to determine whether it passed.
    pub threshold: VoteThreshold,
    /// The delay (in blocks) to wait after a successful referendum before deploying.
    pub delay: BlockNumber,
    /// The current tally of votes in this referendum.
    pub tally: Tally<Balance>,
}

/// Info regarding a referendum, present or past.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub enum ReferendumInfo<BlockNumber, Hash, Balance> {
    /// Referendum is happening, the arg is the block number at which it will end.
    Ongoing(ReferendumStatus<BlockNumber, Hash, Balance>),
    /// Referendum finished at `end`, and has been `approved` or rejected.
    Finished { approved: bool, end: BlockNumber },
}

impl<BlockNumber, Hash, Balance: Default> ReferendumInfo<BlockNumber, Hash, Balance> {
    /// Create a new instance.
    pub fn new(end: BlockNumber, proposal_hash: Hash, threshold: VoteThreshold, delay: BlockNumber) -> Self {
        let s = ReferendumStatus {
            end,
            proposal_hash,
            threshold,
            delay,
            tally: Tally::default(),
        };
        ReferendumInfo::Ongoing(s)
    }
}
