use codec::{Decode, Encode};

pub(crate) type Collateral<T> = <T as currency::Config>::Balance;

pub(crate) type Wrapped<T> = <T as currency::Config>::Balance;

pub type UnsignedFixedPoint<T> = <T as currency::Config>::UnsignedFixedPoint;

/// Storage version.
#[derive(Encode, Decode, Eq, PartialEq)]
pub enum Version {
    /// Initial version.
    V0,
    /// BtcAddress type with script format.
    V1,
}

#[derive(Decode, Encode, PartialEq, Eq)]
pub(crate) enum OracleStatus {
    Uninitialized,
    Offline,
    Online,
}

impl Default for OracleStatus {
    fn default() -> Self {
        Self::Uninitialized
    }
}
