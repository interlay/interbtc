use crate::Config;
use codec::{Decode, Encode};
use frame_support::traits::Currency;
use sp_arithmetic::FixedPointNumber;

pub enum VaultEvent<Issuing, Backing> {
    RedeemFailure,
    ExecuteIssue(Issuing),
    Deposit(Backing),
    Withdraw(Backing),
    SubmitIssueProof,
    Refund,
}

#[derive(Encode, Decode, Clone, PartialEq, Debug)]
pub enum RelayerEvent {
    StoreBlock,
    TheftReport,
}

pub(crate) type Backing<T> =
    <<T as currency::Config<currency::Backing>>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

pub(crate) type Issuing<T> =
    <<T as currency::Config<currency::Issuing>>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

pub(crate) type SignedFixedPoint<T> = <T as Config>::SignedFixedPoint;

pub(crate) type Inner<T> = <<T as Config>::SignedFixedPoint as FixedPointNumber>::Inner;
