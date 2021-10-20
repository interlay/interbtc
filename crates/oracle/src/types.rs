use codec::{Decode, Encode};
use scale_info::TypeInfo;

pub(crate) type Collateral<T> = <T as currency::Config>::Balance;

pub(crate) type Wrapped<T> = <T as currency::Config>::Balance;

pub type UnsignedFixedPoint<T> = <T as currency::Config>::UnsignedFixedPoint;

/// Storage version.
#[derive(Encode, Decode, Eq, PartialEq, TypeInfo)]
pub enum Version {
    /// Initial version.
    V0,
    /// BtcAddress type with script format.
    V1,
}
