use crate::Config;
use codec::{Decode, Encode};

pub(crate) type Collateral<T> = <T as Config>::Balance;

pub(crate) type Wrapped<T> = <T as Config>::Balance;

pub(crate) type UnsignedFixedPoint<T> = <T as Config>::UnsignedFixedPoint;

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
