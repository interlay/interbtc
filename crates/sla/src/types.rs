use crate::Trait;
use sp_arithmetic::FixedPointNumber;

pub enum VaultEvent<PolkaBTC> {
    RedeemFailure,
    ExecutedIssue(PolkaBTC),
    SubmittedIssueProof,
}

pub enum RelayerEvent {
    BlockSubmission,
    CorrectNoDataVoteOrReport,
    CorrectInvalidVoteOrReport,
    CorrectLiquidationReport,
    CorrectTheftReport,
    CorrectOracleOfflineReport,
    FalseNoDataVoteOrReport,
    FalseInvalidVoteOrReport,
    IgnoredVote,
}

pub(crate) type Inner<T> = <<T as Trait>::SignedFixedPoint as FixedPointNumber>::Inner;
