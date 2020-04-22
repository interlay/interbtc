#![cfg_attr(not(feature = "std"), no_std)]
#![deny(warnings)]
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

    /// use only for errors which means something
    ///  going very wrong and which do not match any other error
    RuntimeError,
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
            Error::RuntimeError => "Runtime error",
        }
    }
}

impl ToString for Error {
    fn to_string(&self) -> String {
        String::from(self.message())
    }
}

impl sp_std::convert::From<Error> for DispatchError {
    fn from(error: Error) -> Self {
        DispatchError::Module {
            // FIXME: this should be set to the module returning the error
            // It should be super easy to do if Substrate has an "after request"
            // kind of middleware but not sure if it does
            index: 0,
            error: error as u8,
            message: Some(error.message()),
        }
    }
}
