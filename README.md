<p align="center">
  <a href="https://github.com/interlay/btc-parachain">
    <img src="/docs/img/polkaBtc.png">
  </a>

  <h2 align="center">BTC-Parachain</h2>

  <p align="center">
    A trust-minimized bridge from Bitcoin to Polkadot.
    <br />
    <a href="https://interlay.gitlab.io/polkabtc-spec/"><strong>Explore the specification »</strong></a>
    <br />
    <br />
    <a href="https://github.com/interlay/btc-parachain/issues">Report Bug</a>
    ·
    <a href="https://github.com/interlay/btc-parachain/issues">Request Feature</a>
  </p>
</p>

This repository is hosted on GitHub: [https://github.com/interlay/btc-parachain](https://github.com/interlay/btc-parachain) with a mirror on [GitLab](https://gitlab.com/interlay/btc-parachain) and [radicle](rad:git:hnrkxrw3axafn8n5fwo8pspjgtbt6jj6qe6mo).

This project is currently under active development.

## Table of Contents

- [About the Project](#about-the-project)
  - [Built With](#built-with)
- [Roadmap](#roadmap)
- [Getting Started](#getting-started)
  - [Prerequisites](#prerequisites)
  - [Installation](#installation)
- [Contributing](#contributing)
- [License](#license)
- [Contact](#contact)
- [Acknowledgements](#acknowledgements)

## About the Project

This is a proof of concept implementation of a BTC Parachain to bring Bitcoin into the Polkadot universe.
It allows the creation of **PolkaBTC**, a fungible token that represents Bitcoin in the Polkadot ecosystem.
PolkaBTC is backed by Bitcoin 1:1 and allows redeeming of the equivalent amount of Bitcoins by relying on a collateralized third-party.

The project uses the concept of [Cryptocurrency-backed Assets](https://xclaim.io) to lock Bitcoin on the Bitcoin blockchain and issue BTC-backed tokens on the BTC Parachain.
The implementation is based on the [BTC Parachain specification](https://interlay.gitlab.io/polkabtc-spec/).

### Built with

The BTC-Parachain is built with:

- [Rust](https://www.rust-lang.org/)
- [Substrate](https://substrate.dev/)

    <img src="https://interlay.gitlab.io/polkabtc-spec/_images/overview.png" alt="Logo" width="500">

## Roadmap

- **Alpha** - November 2020
- **Beta** - February 2021
- **Rococo** - Feburary 2021
- **Kusama** - TBD
- **Polkadot** - TBD

### Development Progess

The Substrate runtime makes use of various custom pallets that are found in the [crates](./crates) folder.

- [bitcoin](crates/bitcoin): Library for Bitcoin type, parsing and verification functions.
- [btc-relay](crates/btc-relay): Stateful SPV client for Bitcoin. Stores Bitcoin main chain, tracks forks, verifies Merkle proofs and validates specific transaction formats.
- [currency](crates/currency) Handles currencies used as backing collateral (e.g. DOT/KSM) and issued tokens (e.g. PolkaBTC).
- [exchange-rate-oracle](crates/exchange-rate-oracle): Exchange rate oracle. Integration with external provider pending.
- [fee](crates/fee): Participant reward calculation and distribution.
- [issue](crates/issue): Handles issuing of PolkaBTC for BTC on Bitcoin.
- [redeem](crates/redeem) Handles redeeming of PolkaBTC for BTC on Bitcoin.
- [refund](crates/refund) Handles refunds for when a vault receives more BTC than it can cover.
- [replace](crates/replace) Handles replacing vaults.
- [security](crates/security): Handles BTC Parachain status and error changes.
- [sla](crates/sla): Participant scoring for reward & slashing calculations.
- [staked-relayers](crates/staked-relayers): Handles registration and stake of Staked Relayers, as well as voting on Parachain status changes.
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

For more detailed development instructions [see here](./parachain/README.md).

[override file]: https://rust-lang.github.io/rustup/overrides.html#the-toolchain-file

### Testing

```
cargo test
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
cargo run --release -- --dev
```

Clear the database using the `purge-chain` command.

```shell
cargo run --release -- purge-chain --dev
```

To disable all btc-relay block inclusion checks, use the special `dev-no-btc` chain spec.
This is useful for testing without the overhead of running a block relayer.

```shell
cargo run --release -- --alice --chain dev-no-btc --rpc-cors all --validator --force-authoring --tmp
```

Additional CLI usage options are available and may be shown by running `cargo run -- --help`.

### Rococo

By default, the `btc-parachain` builds in standalone mode with the `aura-grandpa` feature.

To build with "parachain" support use the `cumulus-polkadot` feature:

```shell
cargo build --manifest-path parachain/Cargo.toml --release --no-default-features --features cumulus-polkadot
```

To connect with a local relay-chain follow [these instructions](docs/rococo.md).

#### Test Coverage

Test coverage reports available under [docs/testcoverage.html](https://github.com/interlay/btc-parachain/blob/master/docs/testcoverage.html)

### Substrate Chain Configuration

The Substrate runtime configuration is in the [parachain](./parachain) folder.

### Javascript / Typescript

When interacting via polkadot{.js} you will need to use our [custom types](https://github.com/interlay/polkabtc-types). Please also checkout [polkabtc-js](https://github.com/interlay/polkabtc-js) for a more complete (strongly-typed) library with [bitcoinjs-lib](https://github.com/bitcoinjs/bitcoinjs-lib) integration.

## Contributing

If you would like to contribute, please file an issue on GitLab or reach out to us.

- [Discord](https://discord.gg/C8tjMbgVXh)
- [Telegram](https://t.me/joinchat/G9FaYhNbJK9v-6DN3IyhJw)
- [Riot](https://matrix.to/#/!nZablWWaicZyVTWyZk:matrix.org?via=matrix.org)

We are [hiring](https://www.interlay.io/careers/)!

## License

(C) Copyright 2020 [Interlay](https://www.interlay.io) Ltd

BTC-Parachain is currently licensed under the terms of the Apache License (Version 2.0). See LICENSE

## Contact

Website: [Interlay.io](https://www.interlay.io)

Twitter: [@interlayHQ](https://twitter.com/InterlayHQ)

Email: contact@interlay.io

## Acknowledgements

This project is supported by a [Web3 Foundation grant](https://web3.foundation/grants/).

We would also like to thank the following teams for their continuous support:

- [Parity Technologies](https://www.parity.io/)

<p align="center">
  <a href="https://web3.foundation/grants/">
    <img src="/docs/img/web3GrantsBadge.png">
  </a>
</p>
