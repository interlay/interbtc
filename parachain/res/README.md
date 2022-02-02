# Parachain Runtimes

This folder includes all parachain runtimes based on the interBTC bridge code.

## Usage

The `kintsugi.json` file serves as the chain specification used together with the `--chain` parameter when starting a [collator](https://docs.interlay.io/#/collator/guide) or [full-node](https://docs.interlay.io/#/full-node/guide).

## Development

**Note**: The commands below refer to the `kintsugi` chain. Adjust for the desired chain specification.

**Warning!** Create a chain specification only with tagged releases of the parachain!

```shell
docker run interlayhq/interbtc:interbtc-parachain-1.0.2 interbtc-parachain build-spec-with-id --chain kusama --disable-default-bootnode --raw > parachain/res/kintsugi.json
```

The raw chain spec contains all the same information, but it contains the encoded storage keys that the node will use to reference the data in its local storage.
Distributing a raw spec ensures that each node will store the data at the proper storage keys.

**Note**: Because Rust -> Wasm optimized builds aren't reproducible, each person will get a slightly different Wasm blob which will break consensus if each participant generates the file themselves. For the curious, learn more about this issue in [this blog post](https://dev.to/gnunicorn/hunting-down-a-non-determinism-bug-in-our-rust-wasm-build-4fk1).

## Parachain Ids

- **Kintsugi**: `2092`
