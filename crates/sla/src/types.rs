use crate::Config;
use sp_arithmetic::FixedPointNumber;

pub enum VaultEvent<PolkaBTC> {
    RedeemFailure,
    ExecutedIssue(PolkaBTC),
    SubmittedIssueProof,
    Refunded,
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

pub(crate) type Inner<T> = <<T as Config>::SignedFixedPoint as FixedPointNumber>::Inner;
