# BTC Relay

Based on the BTC Relay [specification](https://spec.interlay.io/spec/btc-relay/index.html).

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
git = '../creates/btc-relay'
```

Update your runtime's `std` feature to include this pallet:

```TOML
std = [
    # --snip--
    'btc-relay/std',
]
```

### Runtime `lib.rs`

You should implement it's trait like so:

```rust
/// Used for test_module
impl btc_relay::Config for Runtime {
    type Event = Event;
    type WeightInfo = ();
}
```

and include it in your `construct_runtime!` macro:

```rust
BTCRelay: btc_relay::{Module, Call, Config<T>, Storage, Event<T>},
```

## Reference Docs

You can view the reference docs for this pallet by running:

```
cargo doc --open
```
