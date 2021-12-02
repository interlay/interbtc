# Vault Registry

Based on the Vault Registry [specification](https://spec.interlay.io/spec/vault-registry.html).

## Installation

Run `cargo build` from the root folder of this directory.

## Testing

Run `cargo test` from the root folder of this directory.

## Runtime Integration

### Runtime `Cargo.toml`

To add this pallet to your runtime, simply include the following to your runtime's `Cargo.toml` file:

```TOML
[dependencies.btc-relay]
default_features = false
git = '../creates/vault-registry'
```

Update your runtime's `std` feature to include this pallet:

```TOML
std = [
    # --snip--
    'vault-registry/std',
]
```

### Runtime `lib.rs`

You should implement it's trait like so:

```rust
/// Used for test_module
impl vault_registry::Config for Runtime {
    type Event = Event;
    type WeightInfo = ();
}
```

and include it in your `construct_runtime!` macro:

```rust
VaultRegistry: vault_registry::{Module, Call, Config<T>, Storage, Event<T>},
```

## Reference Docs

You can view the reference docs for this pallet by running:

```
cargo doc --open
```
