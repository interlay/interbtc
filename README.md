<p align="center">
  <a href="https://gitlab.com/interlay/btc-parachain">
    <img src="/docs/polka_btc.png">
  </a>

  <h2 align="center">BTC-Parachain</h2>

  <p align="center">
    A trust-minimized bridge from Bitcoin to Polkadot.
    <br />
    <a href="https://interlay.gitlab.io/polkabtc-spec/"><strong>Explore the specification »</strong></a>
    <br />
    <br />
    <a href="https://gitlab.com/interlay/btc-parachain/-/issues">Report Bug</a>
    ·
    <a href="https://gitlab.com/interlay/btc-parachain/-/issues">Request Feature</a>
  </p>
</p>

This repository is hosted on GitLab: [https://gitlab.com/interlay/btc-parachain](https://gitlab.com/interlay/btc-parachain) with a mirror on GitHub.

_This project is currently under active development_.

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

The Substrate runtime makes use of various custom pallets and crates that are found in the [crates](./crates) folder.

**Development status**: Proof-of-concept - Aplha release

### Development Progess

- [bitcoin](crates/bitcoin): [Alpha] Library for Bitcoin type, parsing and verification functions.
- [btc-relay](crates/btc-relay): [Alpha] Stateful SPV client for Bitcoin. Stores Bitcoin main chain, tracks forks, verifies Merkle proofs and validates specific transaction formats.
- [collateral](crates/collateral) [Alpha] Handles locking, releasing and slashing of collateral (e.g. DOT).
- [exchange-rate-oracle](crates/exchange-rate-oracle): [Alpha] Exchange rate oracle. Integration with external provider pending.
- [issue](crates/issue): [Alpha] Handles issuing of PolkaBTC.
- [redeem](crates/redeem) [Alpha] Handles redeeming of PolkaBTC for BTC on Bitcoin.
- [replace](crates/replace) [Alpha] Handles replacing vaults.
- [security](crates/security): [Alpha] Security module, handling BTC Parachain status changes (error handling).
- [staked-relayers](crates/staked-relayers): [Alpha] Handles registration and stake of Staked Relayers, as well as voting on Parachain status changes.
- [treasury](crates/treasury): [Alpha] Exposes functions related to handling of the PolkaBTC currency (mint, transfer, lock, burn)
- [vault-registry](crate/vault-registry): [Alpha] Handles registration, collateral and liquidation of Vaults.
- [x-core](crates/xclaim-core): [Alpha] Error types and other shared types/functions used across BTC-Parachain components.

## Getting started

### Prerequesites

- rustup

```
curl https://sh.rustup.rs -sSf | sh
```

### Installation

Building requires `nightly`. Run the following commands to set up:

```
rustup toolchain install nightly-2020-10-01
rustup default nightly-2020-10-01
rustup component add rustfmt
rustup component add rls
rustup toolchain install nightly
rustup target add wasm32-unknown-unknown --toolchain nightly-2020-10-01
```

To build, run

```
cargo build
```

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
cargo run --release -- --chain dev-no-btc
```

#### Test coverage

Test coverage reports available under [docs/testcoverage.html](https://gitlab.com/interlay/btc-parachain/-/blob/dev/docs/testcoverage.html)

### Substrate Chain Configuration

The Substrate runtime configuration is in the [parachain](./parachain) folder.

#### Custom Types

```json
{
  "H256Le": "Hash",
  "DOT": "Balance"
}
```

## Contributing

If you would like to contribute, please file an issue on GitLab or reach out to us.

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
    <img src="/docs/web3_foundation_grants_badge_black.png">
  </a>
</p>
