#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
extern crate mocktopus;

mod error;
pub use error::Error;

pub mod merkle;

mod address;
pub use address::Address;

mod script;
pub use script::Script;

pub mod types;

pub mod formatter;
pub mod parser;

pub mod utils;
