# BTC Parachain

This repository is hosted on GitLab: [https://gitlab.com/interlay/btc-parachain](https://gitlab.com/interlay/btc-parachain) with a mirror on GitHub.

*This project is currently under active development*. 

## Overview

This is a proof of concept implementation of a BTC Parachain to bring Bitcoin into the Polkadot universe.
It allows the creation of **PolkaBTC**, a fungible token that represents Bitcoin in the Polkadot ecosystem.
PolkaBTC is backed by Bitcoin 1:1 and allows redeeming of the equivalent amount of Bitcoins by relying on a collateralized third-party.

![overview](https://interlay.gitlab.io/polkabtc-spec/_images/overview.png "BTC Parachain Overview")

The project uses the concept of [Cryptocurrency-backed Assets](https://xclaim.io) to lock Bitcoin on the Bitcoin blockchain and issue BTC-backed tokens on the BTC Parachain.
The implementation is based on the [BTC Parachain specification](https://interlay.gitlab.io/polkabtc-spec/).


## Substrate chain

The Substrate runtime can configuration is in the [parachain](./parachain) folder.

## Pallets and crates

The Substrate runtime makes use of various custom pallets and crates that are found in the [crates](./crates) folder.

### Development Progess

- [bitcoin](crates/bitcoin): [Beta] Library for Bitcoin type, parsing and verification functions.
- [btc-relay](crates/btc-relay): [Beta] Stateful SPV client for Bitcoin. Stores Bitcoin main chain, tracks forks, verifies Merkle proofs and validates specific transaction formats. 
- [btc-core](crates/btc-core):  [Beta] Error types used in BTC-Relay and Bitcoin.
- [priority-map](crates/priority-map): [WIP] Priority queue based on a mapping. Used to efficiently track ongoing forks and handle re-orgs.
- [exchange-rate-oracle](crates/exchange-rate-oracle): [Beta] Exchange rate oracle. Integration with external provider pending.
- [security](crates/security): [WIP] Security module, handling BTC Parachain status changes (error handling), Staked Relayers.
- [xclaim-core](crates/xclaim-core): [WIP] Error types used in the XCLAIM component (Issue, Redeem, Replace, Vault Registry, Collateral, etc.).
- [treasury](crates/treasury): [WIP] Exposes functions related to handling of the PolkaBTC currency (mint, transfer, lock, burn)
- [collateral](crates/collateral) [WIP] Handles locking, releasing and slashing of collateral (e.g. DOT). 
- see [specification](https://interlay.gitlab.io/polkabtc-spec/index.html) for outstanding modules.


## Contributions

If you would like to contribute, please file an issue on GitLab or reach out to us.

- [Telegram](https://t.me/joinchat/G9FaYhNbJK9v-6DN3IyhJw)
- [Riot](https://matrix.to/#/!nZablWWaicZyVTWyZk:matrix.org?via=matrix.org)

We are [hiring](https://www.interlay.io/careers/)!

## Copyright and License

(C) Copyright 2020 [Interlay](https://www.interlay.io) Ltd

BTC-Parachain is currently licensed under the terms of the Apache License (Version 2.0). See LICENSE


## Acknowledgements

This project is supported by a Web3 Foundation grant. 

[<img src="/docs/web3_foundation_grants_badge_black.png" width="500">](/docs/web3_foundation_grants_badge_black.png)


