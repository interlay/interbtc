use crate::Config;
use codec::{Decode, Encode};
use sp_arithmetic::FixedPointNumber;

pub enum VaultEvent<Balance> {
    RedeemFailure,
    ExecuteIssue(Balance),
    Deposit(Balance),
    Withdraw(Balance),
    SubmitIssueProof,
    Refund,
    Liquidate,
}

#[derive(Encode, Decode, Clone, PartialEq, Debug)]
pub enum RelayerEvent {
    StoreBlock,
    TheftReport,
}

pub(crate) type BalanceOf<T> = <T as Config>::Balance;

pub(crate) type SignedFixedPoint<T> = <T as Config>::SignedFixedPoint;

pub(crate) type Inner<T> = <<T as Config>::SignedFixedPoint as FixedPointNumber>::Inner;
