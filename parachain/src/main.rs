//! Substrate Node Template CLI library.
#![warn(missing_docs)]

mod chain_spec;
#[macro_use]
mod cli;
mod command;
mod service;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

fn main() -> sc_cli::Result<()> {
    command::run()
}
