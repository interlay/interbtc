# Rococo - Local (Automated Setup)

Install [polkadot-launch](https://github.com/paritytech/polkadot-launch).

```
## Manual
git clone https://github.com/paritytech/polkadot-launch.git
cd polkadot-launch
yarn global add file:$(pwd)

## Automatic
yarn global add polkadot-launch
```

Compile and install [polkadot](https://github.com/paritytech/polkadot).

```shell
git clone git@github.com:paritytech/polkadot.git
cd polkadot
git checkout v0.9.11

cargo build --release
sudo cp ./target/release/polkadot /usr/local/bin/
```

Compile and install the [parachain](https://github.com/interlay/interbtc).

```shell
git clone git@github.com:interlay/interbtc.git
cd interbtc

cargo build --release --bin interbtc-parachain --features rococo-native
sudo cp ./target/release/interbtc-parachain /usr/local/bin/
```

Run polkadot-launch with [this config](./xcm-config.json).

```shell
polkadot-launch ./docs/xcm-config.json
```

# Rococo - Local (Manual Setup)

## Relay Chain

Compile and install polkadot as above.

```shell
# Generate chain spec
polkadot build-spec --chain rococo-local --disable-default-bootnode --raw > rococo-local.json

# Run 1st validator
polkadot --chain rococo-local.json --alice --tmp --discover-local

# Run 2nd validator
polkadot --chain rococo-local.json --bob --tmp --discover-local --port 30334
```

## Parachain

Compile and install the parachain as above.

```shell
# Export the chain spec
interbtc-parachain build-spec --chain rococo-local --disable-default-bootnode --raw > rococo-spec.json

# Export genesis state (using reserved paraid)
interbtc-parachain export-genesis-state --chain rococo-spec.json --parachain-id 2121 > genesis-state

# Export genesis wasm
interbtc-parachain export-genesis-wasm --chain rococo-spec.json > genesis-wasm

# Run parachain collator
interbtc-parachain \
    --alice \
    --collator \
    --force-authoring \
    --chain=rococo-spec.json \
    --parachain-id 2121 \
    --port 40335 \
    --ws-port 9946 \
    --discover-local \
    --tmp \
    --execution wasm \
    -- \
    --execution wasm \
    --chain rococo-local.json \
    --port 30335 \
    --discover-local
```

## Register

To register the parachain, you can use the [Polkadot JS Apps UI](https://polkadot.js.org/apps/#/?rpc=ws://localhost:9944).

![Initialize Parachain](./img/sudoScheduleParaInitialize.png)

Before sending messages between parachains you must first establish a channel.

![Establish Channel](./img/sudoEstablishHrmpChannel.png)

Add the [types](https://github.com/interlay/interbtc-types) to the developer settings if the app fails to decode any responses.


## Extend the lease period

By default the lease period of the parachain is a bit more than 1 day. For a longer running test network, we want to have longer lease periods otherwise the parachain is demoted to a parathread after 1 day.

Use sudo to extend the lease period:

![Extend Lease](./img/sudoExtendLease.png)
