use crate::Config;
use codec::{Decode, Encode};
use sp_arithmetic::FixedPointNumber;

pub enum VaultEvent<PolkaBTC> {
    RedeemFailure,
    ExecutedIssue(PolkaBTC),
    SubmittedIssueProof,
    Refunded,
}

#[derive(Encode, Decode, Clone, PartialEq, Debug)]
pub enum RelayerEvent {
    BlockSubmission,
    DuplicateBlockSubmission,
    CorrectNoDataVoteOrReport,
    CorrectInvalidVoteOrReport,
    CorrectLiquidationReport,
    CorrectTheftReport,
    CorrectOracleOfflineReport,
    FalseNoDataVoteOrReport,
    FalseInvalidVoteOrReport,
    IgnoredVote,
}

pub(crate) type Inner<T> = <<T as Config>::SignedFixedPoint as FixedPointNumber>::Inner;
