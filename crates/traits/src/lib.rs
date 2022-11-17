#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::dispatch::DispatchError;
use num_bigint::{BigUint, ToBigUint};

pub mod loans;
pub use loans::*;

pub trait ConvertToBigUint {
    fn get_big_uint(&self) -> BigUint;
}

impl ConvertToBigUint for u128 {
    fn get_big_uint(&self) -> BigUint {
        self.to_biguint().unwrap()
    }
}

pub trait OracleApi<Amount, CurrencyId> {
    fn convert(amount: &Amount, to: CurrencyId) -> Result<Amount, DispatchError>;
}
