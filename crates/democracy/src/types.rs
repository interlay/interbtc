//! Miscellaneous additional datatypes.

use crate::{AccountVote, Conviction, Vote, VoteThreshold};
use codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_runtime::{
    traits::{Bounded, CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, Saturating, Zero},
    RuntimeDebug,
};

/// Info regarding an ongoing referendum.
#[derive(Encode, Decode, Default, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct Tally<Balance> {
    /// The number of aye votes, expressed in terms of post-conviction lock-vote.
    pub ayes: Balance,
    /// The number of nay votes, expressed in terms of post-conviction lock-vote.
    pub nays: Balance,
    /// The amount of funds currently expressing its opinion. Pre-conviction.
    pub turnout: Balance,
}

impl<Balance: From<u8> + Zero + Copy + CheckedAdd + CheckedSub + CheckedMul + CheckedDiv + Bounded + Saturating>
    Tally<Balance>
{
    /// Create a new tally.
    pub fn new(vote: Vote, balance: Balance) -> Self {
        let votes = vote.conviction.votes(balance);
        Self {
            ayes: if vote.aye { votes } else { Zero::zero() },
            nays: if vote.aye { Zero::zero() } else { votes },
            turnout: balance,
        }
    }

    /// Add an account's vote into the tally.
    pub fn add(&mut self, vote: AccountVote<Balance>) -> Option<()> {
        match vote {
            AccountVote::Standard { vote, balance } => {
                let votes = vote.conviction.votes(balance);
                self.turnout = self.turnout.checked_add(&balance)?;
                match vote.aye {
                    true => self.ayes = self.ayes.checked_add(&votes)?,
                    false => self.nays = self.nays.checked_add(&votes)?,
                }
            }
            AccountVote::Split { aye, nay } => {
                let aye_votes = Conviction::None.votes(aye);
                let nay_votes = Conviction::None.votes(nay);
                self.turnout = self.turnout.checked_add(&aye)?.checked_add(&nay)?;
                self.ayes = self.ayes.checked_add(&aye_votes)?;
                self.nays = self.nays.checked_add(&nay_votes)?;
            }
        }
        Some(())
    }

    /// Remove an account's vote from the tally.
    pub fn remove(&mut self, vote: AccountVote<Balance>) -> Option<()> {
        match vote {
            AccountVote::Standard { vote, balance } => {
                let votes = vote.conviction.votes(balance);
                self.turnout = self.turnout.checked_sub(&balance)?;
                match vote.aye {
                    true => self.ayes = self.ayes.checked_sub(&votes)?,
                    false => self.nays = self.nays.checked_sub(&votes)?,
                }
            }
            AccountVote::Split { aye, nay } => {
                let aye_votes = Conviction::None.votes(aye);
                let nay_votes = Conviction::None.votes(nay);
                self.turnout = self.turnout.checked_sub(&aye)?.checked_sub(&nay)?;
                self.ayes = self.ayes.checked_sub(&aye_votes)?;
                self.nays = self.nays.checked_sub(&nay_votes)?;
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

/// Whether an `unvote` operation is able to make actions that are not strictly always in the
/// interest of an account.
pub enum UnvoteScope {
    /// Permitted to do everything.
    Any,
    /// Permitted to do only the changes that do not need the owner's permission.
    OnlyExpired,
}
