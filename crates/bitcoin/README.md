# Bitcoin library

Library handling BTC-Relay and Bitcoin specific data types and provides parsing and verification functionality.

- `address.rs`: Bitcoin address types
- `error.rs`: Associated module errors
- `formatter.rs`: Type serialization
- `merkle.rs`: Verification of merkle proofs
- `parser.rs`: Type deserialization
- `types.rs`: BTC-Relay / Bitcoin data model
- `utils.rs`: Bitcoin-specific util functions

## Installation

Run `cargo build` from the root folder of this directory.

## Testing

Run `cargo test` from the root folder of this directory.

## Integration

To add this library to your crate, simply include the following in your crate's `Cargo.toml` file:

```TOML
[dependencies.bitcoin]
default_features = false
git = '../creates/bitcoin'
```

Update your crate's `std` feature to include this pallet:

```TOML
std = [
    # --snip--
    'bitcoin/std',
]
```

## Reference Docs

You can view the reference docs for this pallet by running:

```
cargo doc --open
```
