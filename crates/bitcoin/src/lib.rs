//! # Bitcoin Library
//!
//! This crate provides low-level parsing and verification functionality for Bitcoin specific data structures.
//!
//! Unless otherwise stated, the primary source of truth for code contained herein is the
//! [Bitcoin Core repository](https://github.com/bitcoin/bitcoin), though implementation
//! details may vary.
//!
//! ## Overview
//!
//! This crate provides functions for:
//!
//! - (De)serialization of block headers, transactions and merkle proofs.
//! - Script (address) construction and parsing.
//! - Merkle proof construction and verification.
//! - Elliptic curve multiplication over Secp256k1.

#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
extern crate mocktopus;

mod error;
pub use error::Error;

pub mod merkle;

mod address;
pub use address::*;

mod script;
pub use script::Script;

pub mod types;

pub mod formatter;
pub mod parser;

pub mod utils;
