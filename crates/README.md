# Crates

Here you will find the different crates we are using to build the Bitcoin bridge.

## Pallets

Pallets are modules that are integrated directly into a Substrate runtime. You can learn more about [FRAME and the pallet system here](https://substrate.dev/docs/en/conceptual/runtime/frame).
We are writing our own pallets to allow trustless issue and redeem of Bitcoin on our Substrate chain.

- BTC-Relay: A stateful SPV client for Bitcoin [btc-relay](./btc-relay)
- Security: Handling failure cases in the BTC Parachain [security](./security)

## Crates

We are also adding crates for helpers and libraries that can be used from within our pallets, but don't represent a runtime pallet.

- Bitcoin: Bitcoin type, parsing and verification functions [bitcoin](./bitcoin)
- BTC-Core: Error types used in BTC-Relay and Bitcoin [btc-core](./btc-core) 
- Priority-map: a WIP for a priority queue based on a mapping [priority-map](./priority-map)
