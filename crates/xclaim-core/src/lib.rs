#![cfg_attr(not(feature = "std"), no_std)]
#![deny(warnings)]
use btc_core::Error as BTCError;
use codec::alloc::string::{String, ToString};
use frame_support::dispatch::DispatchError;
use sp_std::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Error {
    MissingExchangeRate,
    InvalidOracleSource,
    InsufficientFunds,
    InsufficientLockedFunds,
    InsufficientCollateralAvailable,

    VaultNotFound,
    VaultBanned,
    InsufficientCollateral,
    ExceedingVaultLimit,
    IssueIdNotFound,
    CommitPeriodExpired,
    UnauthorizedUser,
    TimeNotExpired,
    IssueCompleted,

    /// use only for errors which means something
    /// going very wrong and which do not match any other error
    RuntimeError,

    /// issue, redeem and replace elicit errors from both cores
    BitcoinError(BTCError),
}

impl Error {
    pub fn message(self) -> &'static str {
        match self {
            Error::MissingExchangeRate => "Exchange rate not set",
            Error::InvalidOracleSource => "Invalid oracle account",
            Error::InsufficientFunds => {
                "The balance of this account is insufficient to complete the transaction."
            }
            Error::InsufficientLockedFunds => {
                "The locked token balance of this account is insufficient to burn the tokens."
            }
            Error::InsufficientCollateralAvailable => {
                "The sender’s collateral balance is below the requested amount."
            }

            Error::VaultNotFound => "There exists no Vault with the given account id",
            Error::VaultBanned => "The selected Vault has been temporarily banned",
            Error::InsufficientCollateral => "User provided collateral below limit",
            Error::ExceedingVaultLimit => "The requested Vault has not locked enough collateral",
            Error::IssueIdNotFound => "Requested issue id not found",
            Error::CommitPeriodExpired => "Time to issue PolkaBTC expired",
            Error::UnauthorizedUser => "Unauthorized: Caller must be associated user",
            Error::TimeNotExpired => "Time to issue PolkaBTC not yet expired",
            Error::IssueCompleted => "Issue completed and cannot be cancelled",

            Error::RuntimeError => "Runtime error",

            Error::BitcoinError(e) => e.message(),
        }
    }
}

impl ToString for Error {
    fn to_string(&self) -> String {
        String::from(self.message())
    }
}

impl From<BTCError> for Error {
    fn from(error: BTCError) -> Self {
        Error::BitcoinError(error)
    }
}

// Note: with btc-code, error here is a non-primitive type
// meaning we can cannot convert with `as`
impl From<Error> for u8 {
    fn from(error: Error) -> Self {
        match error {
            Error::MissingExchangeRate => 0,
            Error::InvalidOracleSource => 1,
            Error::InsufficientFunds => 2,
            Error::InsufficientLockedFunds => 3,
            Error::InsufficientCollateralAvailable => 4,
            Error::VaultNotFound => 5,
            Error::VaultBanned => 6,
            Error::InsufficientCollateral => 7,
            Error::ExceedingVaultLimit => 8,
            Error::IssueIdNotFound => 9,
            Error::CommitPeriodExpired => 10,
            Error::UnauthorizedUser => 11,
            Error::TimeNotExpired => 12,
            Error::IssueCompleted => 13,
            Error::RuntimeError => 14,
            Error::BitcoinError(_) => 15,
        }
    }
}

impl From<Error> for DispatchError {
    fn from(error: Error) -> Self {
        DispatchError::Module {
            // FIXME: this should be set to the module returning the error
            // It should be super easy to do if Substrate has an "after request"
            // kind of middleware but not sure if it does
            index: 0,
            error: u8::from(error),
            message: Some(error.message()),
        }
    }
}
