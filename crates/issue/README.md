# Issue

## Benchmarks

We utilize ECDH to generate new vault deposit addresses on `request_issue`, here are the weights
taken from benchmarks while using different secp256k1 libraries:

```
paritytech/libsecp256k1 (interpreted): 9_396_222_0005
paritytech/libsecp256k1 (compiled): 329_857_000

RustCrypto/elliptic-curves/k256 (interpreted): 9_706_541_000
RustCrypto/elliptic-curves/k256 (compiled): 354_545_000

bitcoin-core/secp256k1 (interpreted): 13_916_855_000
bitcoin-core/secp256k1 (compiled): 452_088_000
```