# BTC Parachain

This repository is based on the [Substrate Node Template](https://github.com/substrate-developer-hub/substrate-node-template).

## Build

Install Rust:

```bash
curl https://sh.rustup.rs -sSf | sh
```

Initialize your Wasm Build environment:

```bash
./scripts/init.sh
```

Build WASM and native code:

```bash
cargo build --release
```

## Test

To download the recent 100 Bitcoin blocks run:

```bash
python ./scripts/fetch_bitcoin_data.py
```

Execute tests:

```bash
cargo test --release
```

## Run

### Single Node Development Chain

Purge any existing developer chain state:

```bash
./target/release/node-template purge-chain --dev
```

Start a development chain with:

```bash
./target/release/node-template --dev
```

Detailed logs may be shown by running the node with the following environment variables set: `RUST_LOG=debug RUST_BACKTRACE=1 cargo run -- --dev`.

### Multi-Node Local Testnet

If you want to see the multi-node consensus algorithm in action locally, then you can create a local testnet with two validator nodes for Alice and Bob, who are the initial authorities of the genesis chain that have been endowed with testnet units.

Optionally, give each node a name and expose them so they are listed on the Polkadot [telemetry site](https://telemetry.polkadot.io/#/Local%20Testnet).

You'll need two terminal windows open.

We'll start Alice's substrate node first on default TCP port 30333 with her chain database stored locally at `/tmp/alice`. The bootnode ID of her node is `QmRpheLN4JWdAnY7HGJfWFNbfkQCb6tFf4vvA6hgjMZKrR`, which is generated from the `--node-key` value that we specify below:

```bash
cargo run -- \
  --base-path /tmp/alice \
  --chain=local \
  --alice \
  --node-key 0000000000000000000000000000000000000000000000000000000000000001 \
  --telemetry-url ws://telemetry.polkadot.io:1024 \
  --validator
```

In the second terminal, we'll start Bob's substrate node on a different TCP port of 30334, and with his chain database stored locally at `/tmp/bob`. We'll specify a value for the `--bootnodes` option that will connect his node to Alice's bootnode ID on TCP port 30333:

```bash
cargo run -- \
  --base-path /tmp/bob \
  --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/QmRpheLN4JWdAnY7HGJfWFNbfkQCb6tFf4vvA6hgjMZKrR \
  --chain=local \
  --bob \
  --port 30334 \
  --telemetry-url ws://telemetry.polkadot.io:1024 \
  --validator
```

## Runtime Upgrades

1. Implement storage migration logic using the module's `on_runtime_upgrade` hook.
2. Bump the module's default storage `Version`.
3. Increment the runtime `spec_version`.
4. Compile the WASM runtime: `cargo build --release -p interbtc-runtime-parachain`.
5. Use the sudo module to wrap a call to system `setCode(code)`.

The WASM file can be found here:

```
./target/release/wbuild/interbtc-runtime-parachain/interbtc-runtime-parachain.compact.wasm
```

Additional instructions can be found here:

- https://substrate.dev/docs/en/knowledgebase/runtime/upgrades
- https://substrate.dev/docs/en/tutorials/upgrade-a-chain/sudo-upgrade

## Benchmarks

To run benchmarks for a particular module (e.g. `issue`):

```shell
cd ./parachain
cargo run --features runtime-benchmarks --release -- \
  benchmark \
  pallet \
  --chain dev \
  --execution=wasm \
  --wasm-execution=compiled \
  --pallet "issue" \
  --extrinsic "*" \
  --steps 100 \
  --repeat 10 \
  --output ../crates/issue/src/default_weights.rs \
  --template ../.deploy/weight-template.hbs
```

This will overwrite the default weights (i.e. in the example, `../crates/issue/src/default_weights.rs`).

## Code Coverage

To generate a code coverage report, install and run tarpaulin:

```shell
cargo install cargo-tarpaulin
cargo tarpaulin -v \
  --exclude-files '/test,/mock.rs,/mock/mod.rs,/default_weights.rs,/weights.rs,/ext.rs,/runtime-api/,/benchmarking.rs,parachain/*' \
  --out Html \
  --output-dir "./cov"
```
