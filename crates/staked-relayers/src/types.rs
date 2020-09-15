use bitcoin::types::*;
use codec::{Decode, Encode};
use frame_support::traits::Currency;
use security::types::{ErrorCode, StatusCode};
use sp_std::cmp::Ord;
use sp_std::collections::btree_set::BTreeSet;
use sp_std::fmt::Debug;

pub(crate) type DOT<T> =
    <<T as collateral::Trait>::DOT as Currency<<T as frame_system::Trait>::AccountId>>::Balance;

pub(crate) type PolkaBTC<T> =
    <<T as treasury::Trait>::PolkaBTC as Currency<<T as frame_system::Trait>::AccountId>>::Balance;

/// Indicates the state of a proposed StatusUpdate.
#[derive(Encode, Decode, Clone, PartialEq, Debug)]
pub enum ProposalStatus {
    /// StatusUpdate is current under review and is being voted upon
    Pending = 0,
    /// StatusUpdate has been accepted
    Accepted = 1,
    /// StatusUpdate has been rejected
    Rejected = 2,
}

impl Default for ProposalStatus {
    fn default() -> Self {
        ProposalStatus::Pending
    }
}

/// ## Structs
/// Struct storing information on a proposed parachain status update
#[derive(Encode, Decode, Default, Clone, PartialEq, Debug)]
pub struct StatusUpdate<AccountId: Ord + Clone, BlockNumber, DOT> {
    /// New status of the BTC Parachain.
    pub new_status_code: StatusCode,
    /// Previous status of the BTC Parachain.
    pub old_status_code: StatusCode,
    /// If new_status_code is Error, specifies which error is to be added to Errors
    pub add_error: Option<ErrorCode>,
    /// Indicates which ErrorCode is to be removed from Errors (recovery).
    pub remove_error: Option<ErrorCode>,
    /// Parachain block number at which this status update was suggested.
    pub time: BlockNumber,
    /// Status of the proposed status update. See ProposalStatus.
    pub proposal_status: ProposalStatus,
    /// LE Block hash of the Bitcoin block where the error was detected, if related to BTC-Relay.
    pub btc_block_hash: Option<H256Le>,
    /// Origin of this proposal.
    pub proposer: AccountId,
    /// Deposit paid to submit this proposal.
    pub deposit: DOT,
    /// Bookkeeping for this proposal.
    pub tally: Tally<AccountId>,
}

/// Record keeping for yes and no votes. Based loosely on the
/// democracy pallet in FRAME with restricted functionality.
#[derive(Encode, Decode, Default, Clone, PartialEq, Debug)]
pub struct Tally<AccountId: Ord> {
    /// Set of accounts which have voted FOR this status update. This can be either Staked Relayers or the Governance Mechanism.
    pub(crate) aye: BTreeSet<AccountId>,
    /// Set of accounts which have voted AGAINST this status update. This can be either Staked Relayers or the Governance Mechanism.
    pub(crate) nay: BTreeSet<AccountId>,
}

impl<AccountId: Ord + Clone> Tally<AccountId> {
    /// Returns true if the majority of votes are in favour.
    pub(crate) fn is_approved(&self, total: u64, threshold: u64) -> bool {
        let n = self.aye.len() as u64;
        if n == total {
            return true;
        } else if (self.aye.len() as u64) * 100 / total > threshold {
            return true;
        }
        false
    }

    /// Returns true if the majority of votes are against.
    pub(crate) fn is_rejected(&self, total: u64, threshold: u64) -> bool {
        if (self.nay.len() as u64) * 100 / total > 100 - threshold {
            return true;
        }
        false
    }

    /// Checks if the account has already voted in this poll.
    pub(crate) fn contains(&self, id: &AccountId) -> bool {
        self.nay.contains(&id) || self.aye.contains(&id)
    }

    /// Casts a vote on the poll, returns true if successful.
    /// Returns false if the account has already voted.
    pub(crate) fn vote(&mut self, id: AccountId, approve: bool) -> bool {
        if self.contains(&id) {
            return false;
        } else if approve {
            self.aye.insert(id);
            return true;
        } else {
            self.nay.insert(id);
            return true;
        }
    }
}

/// Online staked relayers who are able to participate in votes.
#[derive(Encode, Decode, Default, Clone, PartialEq, Debug)]
pub struct ActiveStakedRelayer<DOT> {
    pub(crate) stake: DOT,
}

/// Reason for unavailability, chilled or maturing.
#[derive(Encode, Decode, Clone, PartialEq, Debug)]
pub enum StakedRelayerStatus<BlockNumber> {
    Unknown,
    Idle,                 // deregistered
    Bonding(BlockNumber), // (height + MaturityPeriod)
}

impl<BlockNumber> Default for StakedRelayerStatus<BlockNumber> {
    fn default() -> Self {
        StakedRelayerStatus::Unknown
    }
}

/// Offline staked relayers who are not able to participate in a vote.
#[derive(Encode, Decode, Default, Clone, PartialEq, Debug)]
pub struct InactiveStakedRelayer<BlockNumber, DOT> {
    pub(crate) stake: DOT,
    pub(crate) status: StakedRelayerStatus<BlockNumber>,
}
