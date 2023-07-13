# Smart Contracts

This directory contains example smart contracts. The contracts are written in Rust using the [ink! framework](https://use.ink/). ink! is an [Embedded Domain Specific Language (EDSL)](https://wiki.haskell.org/Embedded_domain_specific_language) that uses attribute macros within standard Rust to define smart contracts.

## Prerequisites

Install `cargo-contract`:

```bash
cargo install cargo-contract
```

## Usage

### Existing contracts

Change into the contracts directory:

```bash
cd hello_world
```

Build the contracts:

```bash
cargo contract build
```

Run the tests:

```bash
cargo test
```

### New contracts

Create a new contract:

```bash
cargo contract new <contract-name>
```

Write the contract in the generated `lib.rs` file. 

Build and run tests like above.

## Deploy

Deploy the contract to the local testnet:

```bash
cargo contract upload
```

## Interact

Interact with the contract on the local testnet:

```bash
cargo contract call
```

## Useful resources

- ink! documentation: https://use.ink/getting-started/setup
- hackathon template: https://github.com/scio-labs/inkathon
