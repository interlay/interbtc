use codec::{Decode, Encode};
pub use primitives::issue::{IssueRequest, IssueRequestStatus};

/// Storage version.
#[derive(Encode, Decode, Eq, PartialEq)]
pub enum Version {
    /// Initial version.
    V0,
    /// BtcAddress type with script format.
    V1,
    /// IssueRequestStatus
    V2,
    /// ActiveBlockNumber, btc_height
    V3,
}

pub(crate) type BalanceOf<T> = <T as vault_registry::Config>::Balance;

pub(crate) type Collateral<T> = BalanceOf<T>;

pub(crate) type Wrapped<T> = BalanceOf<T>;
