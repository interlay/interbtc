BTC-Relay {#btc_relay}
=========

Overview
--------

Below, we provide an overview of the BTC-Relay components - offering
references to the full specification contained in the rest of this
document.

![Overview of the BTC-Relay architecture. Bitcoin block headers are
submitted to the Verification Component, which interacts with the Utils,
Parser and Failure Handling components, as well as the Parachain
Storage.](../../figures/intro/btcrelay-architecture.png)

### Storage

This component stores the Bitcoin block headers and additional data
structures, necessary for operating BTC-Relay. See
`data-model`{.interpreted-text role="ref"} for more details.

### Verification

The Verification component offers functionality to verify Bitcoin block
headers and transaction inclusion proofs. See
`storage-verification`{.interpreted-text role="ref"} for the full
function specification.

In more detail, the verification component performs the operations of a
[Bitcoin SPV
client](https://bitcoin.org/en/operating-modes-guide#simplified-payment-verification-spv).
See [this paper (Appendix D)](https://eprint.iacr.org/2018/643.pdf) for
a more detailed and formal discussion on the necessary functionality.

-   *Difficulty Adjustment* - check and keep track of Bitcoin\'s
    difficulty adjustment mechanism, so as to be able to determine when
    the PoW difficulty target needs to be recomputed.
-   *PoW Verification* - check that, given a 80 byte Bitcoin block
    header and its block hash, (i) the block header is indeed the
    pre-image to the hash and (ii) the PoW hash matches the difficulty
    target specified in the block header.
-   *Chain Verification* - check that the block header references an
    existing block already stored in BTC-Relay.
-   *Main Chain Detection / Fork Handling* - when given two conflicting
    Bitcoin chains, determine the *main chain*, i.e., the chain with the
    most accumulated PoW (longest chain in Bitcoin, though under
    consideration of the difficulty adjustment mechanism).
-   *Transaction Inclusion Verification* - given a transaction, a
    reference to a block header, the transaction\'s index in that block
    and a Merkle tree path, determine whether the transaction is indeed
    included in the specified block header (which in turn must be
    already verified and stored in the Bitcoin main chain tracked by
    BTC-Relay).

An overview and explanation of the different classes of blockchain state
verification in the context of cross-chain communication, specifically
the difference between full validation of transactions and mere
verification of their inclusion in the underlying blockchain, can be
found [in this paper (Section
5)](https://eprint.iacr.org/2019/1128.pdf).

### Utils

The Utils component provides \"helper\" functions used by the Storage
and Verification components, such as the calculation of Bitcoin\'s
double SHA256 hash, or re-construction of Merkle trees. See
`utils`{.interpreted-text role="ref"} for the full function
specification.

### Parser

The Parser component offers functions to parse Bitcoin\'s block and
transaction data structures, e.g. extracting the Merkle tree root from a
block header or the OP\_RETURN field from a transaction output. See
`parser`{.interpreted-text role="ref"} for the full function
specification.

Specification
-------------

::: {.toctree maxdepth="1"}
data-model functions parser helpers events errors
:::
