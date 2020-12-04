# Crates

Here you will find the different crates we are using to build the trustless Bitcoin bridge.

## Libraries

- [bitcoin](./bitcoin): [Alpha] Library for Bitcoin type, parsing and verification functions.

## Pallets

Pallets are modules that are integrated directly into a Substrate runtime. You can learn more about [FRAME and the pallet system here](https://substrate.dev/docs/en/conceptual/runtime/frame).

- [btc-relay](./btc-relay): [Alpha] Stateful SPV client for Bitcoin. Stores Bitcoin main chain, tracks forks, verifies Merkle proofs and validates specific transaction formats.
- [collateral](./collateral) [Alpha] Handles locking, releasing and slashing of collateral (e.g. DOT).
- [exchange-rate-oracle](./exchange-rate-oracle): [Alpha] Exchange rate oracle. Integration with external provider pending.
- [issue](./issue): [Alpha] Handles issuing of PolkaBTC.
- [redeem](./redeem) [Alpha] Handles redeeming of PolkaBTC for BTC on Bitcoin.
- [replace](./replace) [Alpha] Handles replacing vaults.
- [security](./security): [Alpha] Security module, handling BTC Parachain status changes (error handling).
- [staked-relayers](./staked-relayers): [Alpha] Handles registration and stake of Staked Relayers, as well as voting on Parachain status changes.
- [treasury](./treasury): [Alpha] Exposes functions related to handling of the PolkaBTC currency (mint, lock, burn)
- [vault-registry](./vault-registry): [Alpha] Handles registration, collateral and liquidation of Vaults.
