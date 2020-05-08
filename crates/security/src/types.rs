use codec::{Decode, Encode};
use sp_std::cmp::Ord;
use sp_std::fmt::Debug;

/// Enum indicating the status of the BTC Parachain.
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub enum StatusCode {
    /// BTC Parachain is fully operational.
    Running = 0,
    /// An error has occurred. See Errors for more details.
    Error = 1,
    /// BTC Parachain operation has been fully suspended.
    Shutdown = 2,
}

impl Default for StatusCode {
    fn default() -> Self {
        StatusCode::Running
    }
}

/// Enum specifying errors which lead to the Error status, tacked in Errors
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, Ord, PartialOrd)]
pub enum ErrorCode {
    /// No error. Used as default value
    None = 0,
    /// Missing transactional data for a block header submitted to BTC-Relay
    NoDataBTCRelay = 1,
    /// Invalid transaction was detected in a block header submitted to BTC-Relay
    InvalidBTCRelay = 2,
    /// The exchangeRateOracle experienced a liveness failure (no up-to-date exchange rate available)
    OracleOffline = 3,
    /// At least one Vault is being liquidated. Redeem requests paid out partially in collateral (DOT).
    Liquidation = 4,
}

impl Default for ErrorCode {
    fn default() -> Self {
        ErrorCode::None
    }
}

#[macro_export]
macro_rules! error_set {
    () => { BTreeSet::<ErrorCode>::new() };
    ($($x:expr),*) => {
        {
            let mut set = BTreeSet::<ErrorCode>::new();
            $(
                set.insert($x);
            )*
            set
        }
    };
}
