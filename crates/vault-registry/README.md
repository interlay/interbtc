# Vault Registry

Based on the Vault Registry specification [https://interlay.gitlab.io/polkabtc-spec/spec/vaultregistry.html](https://interlay.gitlab.io/polkabtc-spec/spec/vaultregistry.html).

## Installation

Run `cargo build` from the root folder of this directory.

## Testing

Run `cargo test` from the root folder of this directory.


## Integration into Runtimes

### Runtime `Cargo.toml`

To add this pallet to your runtime, simply include the following to your runtime's `Cargo.toml` file:

```TOML
[dependencies.btc-relay]
default_features = false
git = '../creates/vault-registry'
```

and update your runtime's `std` feature to include this pallet:

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
impl VaultRegistry::Trait for Runtime {
	type Event = Event;
}
```

and include it in your `construct_runtime!` macro:

```rust
VaultRegistry: vault-registry::{Module, Call, Storage, Event},
```

## Reference Docs

You can view the reference docs for this pallet by running:

```
cargo doc --open
```
