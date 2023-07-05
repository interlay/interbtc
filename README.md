<p align="center">
  <h2 align="center">interBTC</h2>

  <p align="center">
    A modular and programmable layer for Bitcoin and the multi-chain ecosystem.
    <br />
    <a href="https://docs.interlay.io/"><strong>Explore the docs »</strong></a>
    <br />
    <br />
    <a href="https://github.com/interlay/interbtc/issues">Report Bug</a>
    ·
    <a href="https://github.com/interlay/interbtc/issues">Request Feature</a>
  </p>
</p>

## Table of Contents

- [About the Project](#about-the-project)
- [Getting Started](#getting-started)
  - [Prerequisites](#prerequisites)
  - [Installation](#installation)
- [Contributing](#contributing)
- [License](#license)
- [Contact](#contact)
- [Acknowledgements](#acknowledgements)

## About the Project

The interBTC project is a modular and programmable layer to bring Bitcoin to the multi-chain ecosystem. It includes:

- A collateralized and premissionless Bitcoin bridge based on [XCLAIM](https://www.xclaim.io/)
- A DeFi hub with a Rust-native Uniswap v2-style AMM, Curve v1-style AMM, and a Compound v2-style money market.
- [EVM-compatible smart contracts and blocks](https://github.com/paritytech/frontier).
- Bridges to other smart contract chains.
- A [Rust smart contract layer](https://use.ink/) to interact with everything above.

### Built with

The interBTC project is built with:

- [Rust](https://www.rust-lang.org/)
- [Substrate](https://substrate.dev/)

### Structure

#### Runtime

The Substrate runtime configuration is in the [parachain](./parachain) folder.

- [Interlay](parachain/runtime/interlay/): The Interlay runtime configuration.
- [Kintsugi](parachain/runtime/kintsugi/): The Kintsugi canary network runtime configuration.
- [Common](parachain/runtime/common/): Common runtime configuration for Interlay and Kintsugi.

Test networks are build from the mainnet runtimes and have no dedicated runtimes.

#### Crates

The chain makes use of various custom pallets that are found in the [crates](./crates) folder.

- [annuity](crates/annuity): Block rewards for stake-to-vote and vaults.
- [bitcoin](crates/bitcoin): Library for Bitcoin type, parsing and verification functions.
- [btc-relay](crates/btc-relay): Stateful SPV client for Bitcoin. Stores Bitcoin main chain, tracks forks, verifies Merkle proofs and validates specific transaction formats.
- [clients-info](crates/clients-info): Stores current and future [interbtc-client](https://github.com/interlay/interbtc-clients) software releases.
- [collator-selection](crates/collator-selection/): Decentralized sequencers (collators) for the chain.
- [currency](crates/currency) Handles currencies (e.g. DOT/KSM/IBTC).
- [democracy](crates/democracy): Optimistic governance fork of `pallet-democracy`.
- [dex-general](crates/dex-general/): Uniswap v2-style AMM implementation.
- [dex-stable](crates/dex-stable/): Curve v1-style AMM implementation.
- [dex-swap-router](crates/dex-swap-router/): Swap router for the AMMs.
- [escrow](crates/escrow): Rust implementation of Curve's Voting Escrow contract.
- [farming](crates/farming): Farming module for AMM liquidity mining.
- [fee](crates/fee): Participant reward calculation and distribution.
- [issue](crates/issue): Handles issuing of interBTC for BTC on Bitcoin.
- [loans](crates/loans): Compound v2-style money market implementation.
- [multi-transaction-payment](crates/multi-transaction-payment/): Pay assets other than the native one for transaction fees.
- [nomination](crates/nomination): Interface for vault nomination.
- [oracle](crates/oracle): Trusted providers use this to set exchange rates and Bitcoin fee estimates.
- [redeem](crates/redeem): Handles redeeming of interBTC for BTC on Bitcoin.
- [replace](crates/replace): Handles replacing vaults.
- [reward](crates/reward): Scalable reward distribution.
- [security](crates/security): Handles status and error changes.
- [staking](crates/staking): Core logic for vault nomination and slashing.
- [supply](crates/supply): Token minting and inflation.
- [tx-pause](crates/tx-pause): Handles pausing of transactions.
- [vault-registry](crates/vault-registry): Handles registration, collateral and liquidation of vaults.

## Getting started

### Prerequisites

```
curl https://sh.rustup.rs -sSf | sh
```

Please also install the following dependencies:

- `cmake`
- `clang` (>=10.0.0)
- `clang-dev`
- `libc6-dev`
- `libssl-dev`

### Installation

Building requires a specific rust toolchain and nightly compiler version. The
requirements are specified in the [./rust-toolchain.toml](./rust-toolchain.toml)
[override file][].

Running `rustup show` from the root directory of this repo should be enough to
set up the toolchain and you can inspect the output to verify that it matches
the version specified in the override file.

To build, run:

```
cargo build
```

For more detailed development instructions [see here](./docs/README.md).

[override file]: https://rust-lang.github.io/rustup/overrides.html#the-toolchain-file

### Testing

```
cargo test --features runtime-benchmarks
```

To run with coverage, using [cargo-cov](https://github.com/kennytm/cov):

```
cargo install cargo-cov

# clean up previous coverage result
cargo cov clean

# test the code
cargo cov test

# open the coverage report
cargo cov report --open
```


### Running

To run a local development node, use the `dev` chain spec.

```shell
cargo run --release --bin interbtc-parachain -- --dev
```

To connect with a local relay-chain follow [these instructions](docs/rococo.md).

#### Test Coverage

Test coverage reports available under [docs/testcoverage.html](https://github.com/interlay/interbtc/blob/master/docs/testcoverage.html)

### Javascript / Typescript

Either use the [polkadot.js API](https://polkadot.js.org/docs/api) or checkout [interbtc-api](https://github.com/interlay/interbtc-api) for a TypeScript SDK.

## Contributing

If you would like to contribute, please file an issue on GitHub or reach out to us.

- [Discord](https://discord.gg/interlay)

## License

interBTC is licensed under the terms of the Apache License (Version 2.0). See LICENSE

## Contact

Linktree: [Linktree](https://linktr.ee/interlay)

Website: [interlay.io](https://www.interlay.io)

Twitter: [@interlayHQ](https://twitter.com/InterlayHQ)

Discord: [Discord](https://discord.gg/interlay)

Telegram: [Telegram](https://t.me/joinchat/G9FaYhNbJK9v-6DN3IyhJw)

## Acknowledgements

This project is supported by a [Web3 Foundation grant](https://web3.foundation/grants/) and the [Substrate Builders Program](https://substrate.io/ecosystem/substrate-builders-program/).
