use crate::Config;
use codec::{Decode, Encode};
use sp_arithmetic::FixedPointNumber;

pub enum VaultEvent<Issuing> {
    RedeemFailure,
    ExecutedIssue(Issuing),
    SubmittedIssueProof,
    Refunded,
}

#[derive(Encode, Decode, Clone, PartialEq, Debug)]
pub enum RelayerEvent {
    BlockSubmission,
    DuplicateBlockSubmission,
    CorrectTheftReport,
}

pub(crate) type Inner<T> = <<T as Config>::SignedFixedPoint as FixedPointNumber>::Inner;
