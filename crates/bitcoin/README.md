# Bitcoin library

Library handling BTC-Relay and Bitcoin specific data types and provides parsing and verficaition functionality.

* types.rs: BTC-Relay / Bitcoin data model
* parser.rs: Parsing Bitcoin block headers and transactions
* merkle.rs: Verification of Merkle Proofs
* utils.rs: Bitcoin-specific util functions

## Installation

Run `cargo build` from the root folder of this directory.

## Testing

Run `cargo test` from the root folder of this directory.


## Integration into Runtimes 

### Runtime `Cargo.toml`

To add this pallet to your runtime, simply include the following to your runtime's `Cargo.toml` file:

```TOML
[dependencies.bitcoin]
default_features = false
git = '../creates/bitcoin'
```

and update your runtime's `std` feature to include this pallet:

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

