Performance Analysis
====================

Contrary to permissionless blockchains, such as Ethereum, Polkadot\'s
Parachains can easily implement the cryptographic primitives of the
verified blockchains, instead of relying on pre-compiled smart contracts
or manual and costly implementation of primitives. In the case of
Bitcoin, the BTC Parachain can provide native support for the SHA256 and
RIPEMD-160 hash functions, as well as for ECDSA using the
[secp256k1](https://en.bitcoin.it/wiki/Secp256k1) curve.

Consequently, storage resembles the main cost factor of BTC-Relay on
Polkadot.

Estimation of Storage Costs
---------------------------

BTC-Relay only stores Bitcoin block headers. Transactions are not stored
directly in the relay \-- this responsibility lies with other components
or applications interacting with BTC-Relay.

The size of the necessary storage allocation hence grows linear with the
length of the Bitcoin blockchain (tracked in BTC-Relay) \--
specifically, the block headers stored in `BlockHeaders` which are
referenced in `Chains` or in an entry of `Forks`.

Recall, for each block header, BTC-Relay merely stores:

-   the 32 byte `blockHash`
-   4 byte `blockHeight` (twice for better referencing, so 8 bytes in
    total)
-   the 32 byte `merkleRoot`
-   the 4 byte `timestamp` (u32, wrapped in
    [DateTime](https://substrate.dev/rustdocs/v1.0/chrono/struct.DateTime.html)
    )
-   and the 32 byte `target` (u256 integer)

That is, in total 108 bytes per submitted Bitcoin block header (fork or
main chain block).

For example, if we were to sync BTC-Relay from the genesis block all the
way to block height **612450**, the storage requirements would amount to
around **66 MB** \-- an arguably negligible number. At the current rate
and under this configuration, we would reach 100 MB in about 10 years.

::: {.note}
::: {.title}
Note
:::

Fork submissions take up additional storage space, depending om the
length of the tracked fork. Compared to the (already negligible) size of
the main chain block headers, this overhead is negligible. Furthermore,
fork entries are deleted when a chain reorganization occurs, while old
entries (with sufficient confirmations) can be subject to pruning.
:::

BTC-Relay Optimizations
-----------------------

### Pruning

Optionally, to further reduce storage requirements (e.g., in case more
data is to be stored per block in the future), *pruning* of `Chains` and
`BlockHeaders` can be introduced. While the storage overhead for Bitcoin
itself may be acceptable, Polkadot is expected to connect to numerous
blockchains and tracking the entire blockchain history for each could
unnecessarily bloat Parachains (even more so, if Parachains are
non-exclusive to specific blockchains).

With pruning activated, `Chains` would be implemented as a FIFO queue,
where sufficiently old block headers are removed from `BlockHeaders`
(and the references from `Chains` and `Forks` accordingly). The pruning
depth can be set to e.g. 10 000 blocks. There is no need to store more
block headers, as verification of transactions contained in older blocks
can still be performed by requiring users to *re-spend*. More detailed
analysis of the spending behavior in Bitcoin, i.e., UTXOs of which age
are spent most frequently and at which \"depth\" the spending behavior
declines, can be considered to optimize the cost reduction.

::: {.warning}
::: {.title}
Warning
:::

If pruning is implemented for `BlockHeaders` and `Chains` as performance
optimization, it is important to make sure there are no `Forks` entries
left which reference pruned blocks.
:::

### Batch Submissions

Currently, BTC-Relay supports submissions of a single Bitcoin block
header per transaction.

To reduce network load on the Parachain, multiple block header
submissions can be batched into a single transaction. Note: the
improvement in terms of data broadcast to the Parachain depends on the
fixed costs per Parachain transaction (if Parachain transactions are
considered a negligible cost, batching may be unnecessary).

The potential improvement can especially be useful for blockchains with
higher block generation rates than Bitcoin\'s 1 block / 10 minutes, as
in the case of Ethereum.

### Outlook on Sub-Linear Verification in Bitcoin

Recently, so called \"sub-linear\" light clients were proposed for
Bitcoin, which use random sampling of blocks to deter malicious actors
from tricking light clients into accepting an invalid chain.

We refer the reader to the [Superblock
NiPoPoW](https://eprint.iacr.org/2017/963.pdf) and the
[FlyClient](https://eprint.iacr.org/2019/226.pdf) papers for more
details.

As of this writing, both techniques require soft fork modifications to
Bitcoin, if to be deployed in a secure and useful manner. The design of
BTC-Relay as specified in this document (split into storage,
verification, parser, etc. components) thereby allows for introduction
of additional verification methods, without major modifications to the
architecture.
