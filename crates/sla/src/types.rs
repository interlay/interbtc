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
