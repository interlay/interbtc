# BRC21

A POC implementation for the BRC-21 Token Standard

This is not production-ready code and is only meant to be used for experimentation.

## Basic Protocol

The basic protocol is as follows:

### Minting

1. Mint the locked tokens on Bitcoin via an inscription
2. Lock the underlying token in the smart contract and proof that the inscription locks the same amount of tokens

Indexers now accept the Bitcoin-minted BRC21 as minted

### Transfer

1. Transfer BRC21 just like a BRC20 on Bitcoin

### Redeeming

1. Redeem BRC21 on Bitcoin
2. Proof BRC21 redeem to this contract and unlock tokens

### Reference

See the full protocol at https://interlay-labs.gitbook.io/brc-21/

## Getting started

### Contracts

#### Build

```bash
cd brc21
cargo contract build
```

#### Test

From inside the `brc21` directory.

**Run unit tests**

```bash
cargo test
```

**Run integration tests**

```bash
cargo test --features e2e-tests
```

### Ordinals

Follow the instructions at [the Ordinals Project](https://docs.ordinals.com/guides/inscriptions.html#ordinal-inscription-guide).

Make sure that `bitcoind` and `ord` is in your `$PATH`.

#### Run Bitcoin regtest

Open a new terminal window and leave it running.

```bash
bitcoind -regtest -txindex -fallbackfee=0.0001
```

#### Inscriptions (through script)

Go to the `scripts` directory and run either `./mine-inscription mint` or `./mine-inscription redeem`:
```bash
cd scripts
./mine-inscription mint
```

The script needs the `BITCOIN_RPC_PASSWORD` and `BITCOIN_RPC_USER` environment variables set, and furthermore needs `ord`, `bitcoin-cli` and `modified-vault` to be in `$PATH`. The `modified-vault` can be obtained by compiling the vault binary from this branch: https://github.com/interlay/interbtc-clients/pull/510 and renaming it `modified-vault`.

The script will output a bunch of text, finishing with `full proof: <long-hex-string>`. When interacting with the contract through contracts-ui, this is the argument to pass to the `mint` or `redeem` functions. Make sure to prepend the string with `0x`.

#### Inscriptions (MANUAL)

**Create the wallet**

```bash
ord --regtest wallet create
```

Returns a new seed phrase like:

```bash
{
  "mnemonic": "error season truly unknown trouble letter fame subway host defense brief flavor",
  "passphrase": ""
}
```

**Receive funds**

```bash
ord --regtest wallet receive
```

Returns a new address:

```bash
{
  "address": "bcrt1pap5f6reexewxfu9fk522tcrczmqwklxv2rc7fvtlfs6mzp4vp3zqfeq6cf"
}
```

Mint funds to the address:

```bash
bitcoin-cli -regtest generatetoaddress 101 bcrt1pap5f6reexewxfu9fk522tcrczmqwklxv2rc7fvtlfs6mzp4vp3zqfeq6cf
```

You can see the transactions sent to the wallet with:

```bash
ord --regtest wallet transactions
```

By minting 101 transactions, the funds become immediately spendable.

**Create an inscription**

Create a file with a content like the below and store it in `mint.json`:

```json
{
    "p": "brc-21",
    "op": "mint",
    "tick": "INTR",
    "amt": "100",
    "src": "INTERLAY"
}
```

Inscribe the mint operation:

```bash
ord --regtest wallet inscribe --fee-rate 1 mint.json
```

This should return the `commit`, `inscription`, and `reveal`:

```json
{
  "commit": "f88e0f04afd7cb096d6ce40aed2d561a4364c99895e331c1639990a134daabf8",
  "inscription": "fd99046915eed3ede4ab92dc19456ea9f264b5068bba1b3fd36d4231253fd012i0",
  "reveal": "fd99046915eed3ede4ab92dc19456ea9f264b5068bba1b3fd36d4231253fd012",
  "fees": 315
}
```

Mint a couple of blocks to include the inscription:

```bash
bitcoin-cli -regtest generatetoaddress 5 bcrt1pap5f6reexewxfu9fk522tcrczmqwklxv2rc7fvtlfs6mzp4vp3zqfeq6cf
```

#### Inscription Explorer

Open a new terminal window and leave it running.

```bash
ord --regtest server
```

Open a browser and navigate to http://localhost:80

You should now see the inscription that was made.