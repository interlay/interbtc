use crate::Config;
use codec::{Decode, Encode};
use frame_support::traits::Currency;
use sp_arithmetic::FixedPointNumber;

pub enum VaultEvent<Wrapped, Collateral> {
    RedeemFailure,
    ExecuteIssue(Wrapped),
    Deposit(Collateral),
    Withdraw(Collateral),
    SubmitIssueProof,
    Refund,
    Liquidate,
}

#[derive(Encode, Decode, Clone, PartialEq, Debug)]
pub enum RelayerEvent {
    StoreBlock,
    TheftReport,
}

pub(crate) type Collateral<T> = <<T as currency::Config<currency::Collateral>>::Currency as Currency<
    <T as frame_system::Config>::AccountId,
>>::Balance;

pub(crate) type Wrapped<T> =
    <<T as currency::Config<currency::Wrapped>>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

pub(crate) type SignedFixedPoint<T> = <T as Config>::SignedFixedPoint;

pub(crate) type Inner<T> = <<T as Config>::SignedFixedPoint as FixedPointNumber>::Inner;
