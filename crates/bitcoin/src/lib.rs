#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
extern crate mocktopus;
#[cfg(test)]
use mocktopus::macros::mockable;

pub mod merkle;

pub mod types;

pub mod formatter;
#[cfg_attr(test, mockable)]
pub mod parser;

pub mod utils;
