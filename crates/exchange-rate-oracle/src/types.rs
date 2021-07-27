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
