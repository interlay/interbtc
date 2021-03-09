use bitcoin::types::*;
use codec::{Decode, Encode};
use frame_support::traits::Currency;
use security::types::{ErrorCode, StatusCode};
use sp_arithmetic::traits::Saturating;
use sp_std::cmp::Ord;
use sp_std::collections::btree_set::BTreeSet;
use sp_std::fmt::Debug;
use sp_std::prelude::Vec;

pub(crate) type DOT<T> =
    <<T as collateral::Config>::DOT as Currency<<T as frame_system::Config>::AccountId>>::Balance;

pub(crate) type PolkaBTC<T> = <<T as treasury::Config>::PolkaBTC as Currency<
    <T as frame_system::Config>::AccountId,
>>::Balance;

pub type StatusUpdateId = u64;

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
pub struct StatusUpdate<AccountId: Ord + Clone, BlockNumber, DOT: Clone + PartialOrd + Saturating> {
    /// New status of the BTC Parachain.
    pub new_status_code: StatusCode,
    /// Previous status of the BTC Parachain.
    pub old_status_code: StatusCode,
    /// If new_status_code is Error, specifies which error is to be added to Errors
    pub add_error: Option<ErrorCode>,
    /// Indicates which ErrorCode is to be removed from Errors (recovery).
    pub remove_error: Option<ErrorCode>,
    /// Parachain block number at which this status update was suggested.
    pub start: BlockNumber,
    /// Parachain block number at which this status update will expire.
    pub end: BlockNumber,
    /// Status of the proposed status update. See ProposalStatus.
    pub proposal_status: ProposalStatus,
    /// LE Block hash of the Bitcoin block where the error was detected, if related to BTC-Relay.
    pub btc_block_hash: Option<H256Le>,
    /// Origin of this proposal.
    pub proposer: AccountId,
    /// Deposit paid to submit this proposal.
    pub deposit: DOT,
    /// Bookkeeping for this proposal.
    pub tally: Tally<AccountId, DOT>,
    /// Message providing more details on the change of status (detailed error message or recovery reason).
    pub message: Vec<u8>,
}

#[derive(Encode, Decode, Default, Clone, PartialEq, Debug)]
pub struct Votes<AccountId: Ord, Balance: Clone + Saturating> {
    pub(crate) accounts: BTreeSet<AccountId>,
    pub(crate) total_stake: Balance,
}

impl<AccountId: Ord, Balance: Clone + Saturating> Votes<AccountId, Balance> {
    pub(crate) fn contains(&self, id: &AccountId) -> bool {
        self.accounts.contains(id)
    }

    pub(crate) fn insert(&mut self, id: AccountId, stake: Balance) {
        self.accounts.insert(id);
        self.total_stake = self.total_stake.clone().saturating_add(stake);
    }
}

/// Record keeping for yes and no votes. Based loosely on the
/// democracy pallet in FRAME with restricted functionality.
#[derive(Encode, Decode, Default, Clone, PartialEq, Debug)]
pub struct Tally<AccountId: Ord, Balance: Clone + PartialOrd + Saturating> {
    /// Set of accounts which have voted FOR this status update. This can be either Staked Relayers or the Governance Mechanism.
    pub(crate) aye: Votes<AccountId, Balance>,
    /// Set of accounts which have voted AGAINST this status update. This can be either Staked Relayers or the Governance Mechanism.
    pub(crate) nay: Votes<AccountId, Balance>,
}

impl<AccountId: Ord + Clone, Balance: Clone + PartialOrd + Saturating> Tally<AccountId, Balance> {
    /// Returns true if the majority of votes are in favour.
    pub fn is_approved(&self) -> bool {
        self.aye.total_stake > self.nay.total_stake
    }

    /// Checks if the account has already voted in this poll.
    pub fn contains(&self, id: &AccountId) -> bool {
        self.nay.contains(&id) || self.aye.contains(&id)
    }

    /// Casts a vote on the poll, returns true if successful.
    /// Returns false if the account has already voted.
    pub(crate) fn vote(&mut self, id: AccountId, stake: Balance, approve: bool) -> bool {
        if self.contains(&id) {
            return false;
        } else if approve {
            self.aye.insert(id, stake);
            return true;
        } else {
            self.nay.insert(id, stake);
            return true;
        }
    }
}

/// Bonded participant which can suggest and vote on proposals.
#[derive(Encode, Decode, Default, Clone, PartialEq, Debug)]
pub struct StakedRelayer<Balance, BlockNumber> {
    // total stake for this participant
    pub stake: Balance,
    // the height at which the participant bonded
    pub height: BlockNumber,
}
