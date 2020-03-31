# BTC Relay

Based on the BTC Relay specification [https://interlay.gitlab.io/polkabtc-spec/btcrelay-spec/index.html](https://interlay.gitlab.io/polkabtc-spec/btcrelay-spec/index.html).

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
git = '../creates/btc-relay'
```

and update your runtime's `std` feature to include this pallet:

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
impl BTCRelay::Trait for Runtime {
	type Event = Event;
}
```

and include it in your `construct_runtime!` macro:

```rust
BTCRelay: btc-relay::{Module, Call, Storage, Event},
```

## Reference Docs

You can view the reference docs for this pallet by running:

```
cargo doc --open
```

