#![warn(missing_docs)]

//! This crate is part of the `bitcoin-spv` project.
//!
//! This work is produced and copyrighted by Summa, and released under
//! the terms of the LGPLv3 license.
//!
//! It contains a collection of Rust functions and structs for working with
//! Bitcoin data structures. Basically, these tools help you parse, inspect,
//! and authenticate Bitcoin transactions.
//!
//! *It is extremely easy to write insecure code using these libraries. We do
//! not recommend a specific security model. Any SPV verification involves
//! complex security assumptions. Please seek external review for your design
//! before building with these libraries.*

/// `btcspv` provides basic Bitcoin transaction and header parsing, as well as
/// utility functions like merkle verification and difficulty adjustment
/// calculation.
pub mod btcspv;

/// `validatespv` provides higher-levels of abstraction for evaluating
/// SPV proofs, transactions, and headers.
pub mod validatespv;

/// `types` exposes useful structs for headers and SPV proofs, and provides
/// (de)serialization for these structs. It implements a standard JSON format
/// that is compatible with all other `bitcoin-spv` implementations.
pub mod types;

/// `utils` contains utility functions for working with bytestrings, including
/// hex encoding and decoding.
pub mod utils;
