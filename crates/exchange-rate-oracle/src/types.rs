use crate::Config;
use codec::{Decode, Encode};
use frame_support::traits::Currency;
use sp_arithmetic::FixedPointNumber;

pub(crate) type Collateral<T> = <<T as currency::Config<currency::Collateral>>::Currency as Currency<
    <T as frame_system::Config>::AccountId,
>>::Balance;

pub(crate) type Wrapped<T> =
    <<T as currency::Config<currency::Wrapped>>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

pub(crate) type UnsignedFixedPoint<T> = <T as Config>::UnsignedFixedPoint;

pub(crate) type Inner<T> = <<T as Config>::UnsignedFixedPoint as FixedPointNumber>::Inner;

/// Storage version.
#[derive(Encode, Decode, Eq, PartialEq)]
pub enum Version {
    /// Initial version.
    V0,
    /// BtcAddress type with script format.
    V1,
}

#[derive(Encode, Decode, Default, Eq, PartialEq, Debug)]
pub struct BtcTxFeesPerByte {
    /// The estimated Satoshis per bytes to get included in the next block (~10 min)
    pub fast: u32,
    /// The estimated Satoshis per bytes to get included in the next 3 blocks (~half hour)
    pub half: u32,
    /// The estimated Satoshis per bytes to get included in the next 6 blocks (~hour)
    pub hour: u32,
}
