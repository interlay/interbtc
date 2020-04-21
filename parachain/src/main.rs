//! Substrate Node Template CLI library.
#![warn(missing_docs)]

mod chain_spec;
#[macro_use]
mod service;
mod cli;
mod command;

fn main() -> sc_cli::Result<()> {
    let version = sc_cli::VersionInfo {
        name: "Substrate Node",
        commit: env!("VERGEN_SHA_SHORT"),
        version: env!("CARGO_PKG_VERSION"),
        executable_name: "btc-parachain",
        author: "Interlay Ltd",
        description: "BTC Parachain connects Bitcoin and Polkadot",
        support_url: "interlay.io",
        copyright_start_year: 2017,
    };

    command::run(version)
}
