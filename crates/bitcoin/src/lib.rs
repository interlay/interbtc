#![cfg_attr(test, deny(warnings))]
#![cfg_attr(test, feature(proc_macro_hygiene))]

#[cfg(test)]
extern crate mocktopus;
#[cfg(test)]
use mocktopus::macros::mockable;

pub mod merkle;

pub mod types;

#[cfg_attr(test, mockable)]
pub mod parser;

pub mod utils;
