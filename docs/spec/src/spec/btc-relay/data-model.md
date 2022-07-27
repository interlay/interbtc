Data Model
==========

The BTC-Relay, as opposed to Bitcoin SPV clients, only stores a subset
of information contained in block headers and does not store
transactions. Specifically, only data that is absolutely necessary to
perform correct verification of block headers and transaction inclusion
is stored.

Note that the structs used to represent bitcoin transactions and blocks
is slightly different from the `bitcoin-data-model`{.interpreted-text
role="ref"}. For example, no `tx_in count` is required, since this
information is implicitly stored in the vector of inputs.

Types
-----

### RawBlockHeader {#RawBlockHeader}

An 80 bytes long Bitcoin blockchain header, according to the format as
specified by the [Bitcoin
reference](https://developer.bitcoin.org/reference/block_chain.html).

Constants
---------

### DIFFICULTY\_ADJUSTMENT\_INTERVAL

The interval in number of blocks at which Bitcoin adjusts its difficulty
(approx. every 2 weeks = 2016 blocks).

### TARGET\_TIMESPAN

Expected duration of the different adjustment interval in seconds,
`1209600` seconds (two weeks) in the case of Bitcoin.

### TARGET\_TIMESPAN\_DIVISOR

Auxiliary constant used in Bitcoin\'s difficulty re-target mechanism.

### UNROUNDED\_MAX\_TARGET

The maximum difficulty target, $2^{224}-1$ in the case of Bitcoin. For
more information, see the [Bitcoin
Wiki](https://en.bitcoin.it/wiki/Target).

### MAIN\_CHAIN\_ID

Identifier of the Bitcoin main chain tracked in the `ChainsIndex`
mapping. At any point in time, the `BlockChain` with this identifier is
considered to be the main chain and will be used to transaction
inclusion verification.

### STABLE\_BITCOIN\_CONFIRMATIONS

Global security parameter (typically referred to as `k` in scientific
literature), determining the umber of confirmations (in blocks)
necessary for a transaction to be considered \"stable\" in Bitcoin.
Stable thereby means that the probability of the transaction being
excluded from the blockchain due to a fork is negligible.

### STABLE\_PARACHAIN\_CONFIRMATIONS

Global security parameter (typically referred to as `k` in scientific
literature), determining the umber of confirmations (in blocks)
necessary for a transaction to be considered \"stable\" in the BTC
Parachain. Stable thereby means that the probability of the transaction
being excluded from the blockchain due to a fork is negligible.

::: {.note}
::: {.title}
Note
:::

We use this to enforce a minimum delay on Bitcoin block header
acceptance in the BTC-Parachain in cases where a (large) number of block
headers are submitted as a batch.
:::

Structs
-------

### BlockHeader {#BlockHeader}

Representation of a Bitcoin block header, constructed by the parachain
from the `RawBlockHeader`{.interpreted-text role="ref"}. The main
differences compared to the `bitcoinBlockHeader`{.interpreted-text
role="ref"} in `bitcoin-data-model`{.interpreted-text role="ref"} is
that this contains the unpacked `target` constructed from `nBits`, and
an additional `hash` of the `BlockHeader` for convenience.

::: {.note}
::: {.title}
Note
:::

Fields marked as \[Optional\] are not critical for the secure operation
of BTC-Relay, but can be stored anyway, at the developers discretion. We
omit these fields in the rest of this specification.
:::

::: {.tabularcolumns}
l
:::

  Parameter         Type        Description
  ----------------- ----------- --------------------------------------------------------------------------------------------------------------------------------------------------
  `merkleRoot`      H256Le      Root of the Merkle tree referencing transactions included in the block.
  `target`          u256        Difficulty target of this block (converted from `nBits`, see [Bitcoin documentation](https://bitcoin.org/en/developer-reference#target-nbits).).
  `timestamp`       timestamp   UNIX timestamp indicating when this block was mined in Bitcoin.
  `hashPrevBlock`   H256Le      Block hash of the predecessor of this block.
  `hash`            H256Le      Block hash of of this block.
  .                 .           .
  `version`         i32         \[Optional\] Version of the submitted block.
  `nonce`           u32         \[Optional\] Nonce used to solve the PoW of this block.

### RichBlockHeader {#RichBlockHeader}

Representation of a Bitcoin block header containing additional metadata.
This struct is used to store Bitcoin block headers.

::: {.tabularcolumns}
l
:::

  Parameter       Type          Description
  --------------- ------------- -------------------------------------------------------------------------------------------------------------------------------
  `blockHeight`   u32           Height of this block in the Bitcoin main chain.
  `chainRef`      u32           Pointer to the `BlockChain` struct in which this block header is contained.
  `blockHeader`   BlockHeader   Associated parsed `BlockHeader` struct.
  `paraHeight`    u32           The `activeBlockCount` at the time the block header was submitted to the relay. See the security pallet for more information.

### BlockChain

Representation of a Bitcoin blockchain / fork.

::: {.tabularcolumns}
l
:::

  Parameter       Type   Description
  --------------- ------ ------------------------------------------------------------------------------------------------------
  `chainId`       u32    Unique identifier for faster lookup in `ChainsIndex`
  `startHeight`   u32    Lowest block number in this chain. Used to determine the forking point during chain reorganizations.
  `maxHeight`     u32    Max. block height in this chain.

### Transaction

Representation of a Bitcoin Transaction. It differs from the one
specified in `bitcoin-data-model`{.interpreted-text role="ref"} in that
it does not contain in lengths of the input and output vectors, because
this data is implicit in the vector. Furthermore, we use different types
for the inputs and outputs. The segregated witnesses and `flags`, if
any, are placed inside the inputs.

::: {.tabularcolumns}
l
:::

  Parameter    Type                                                       Description
  ------------ ---------------------------------------------------------- -----------------------------------
  `version`    i32                                                        Transaction version number.
  `inputs`     Vec\<`TransactionInput`{.interpreted-text role="ref"}\>    Vector of transaction inputs.
  `output`     Vec\<`TransactionOutput`{.interpreted-text role="ref"}\>   Vector of transaction inputs.
  `lockTime`   `LockTime`{.interpreted-text role="ref"}                   A Unix timestamp OR block number.

### TransactionInput {#TransactionInput}

Representation of a Bitcoin transaction input. It differs from the one
specified in `bitcoin-data-model`{.interpreted-text role="ref"} in that
it contains `flags` and the segregated witnesses. Furthermore, it
contains dedicated fields for coinbase transactions.

::: {.tabularcolumns}
l
:::

  Parameter         Type                Description
  ----------------- ------------------- -------------------------------------------------------------------------------------------------------------------------------------
  `previousHash`    H256Le,             The hash of the transaction to spend from.
  `previousIndex`   u32,                The index of the output within the transaction pointed to by `previousHash` to spend from.
  `coinbase`        bool,               True if the transaction input is the newly mined funds.
  `height`          Option\<u32\>,      An optional blockheight used in the coinbase transaction. See <https://github.com/bitcoin/bips/blob/master/bip-0034.mediawiki>
  `script`          Vec\<u8\>,          The script satisfying the output\'s script.
  `sequence`        u32,                Sequence number (default `0xffffffff`).
  `flags`           u8,                 The flags set in `Transaction` that indicates a Segrated Witness transaction. If none were set in the transaction, this value is 0.
  `witness`         Vec\<Vec\<u8\>\>,   The witness scripts of the transaction. See See <https://github.com/bitcoin/bips/blob/master/bip-0141.mediawiki>

### TransactionOutput {#TransactionOutput}

Representation if a Bitcoin transaction output

::: {.tabularcolumns}
l
:::

  Parameter   Type                                     Description
  ----------- ---------------------------------------- ----------------------------------------------------
  `value`     i64,                                     The number of satoshis to transfer to this output.
  `script`    `Script`{.interpreted-text role="ref"}   The spending condition of the output.

### Script {#Script}

Representation if a Bitcoin transaction output

::: {.tabularcolumns}
l
:::

  Parameter   Type         Description
  ----------- ------------ ---------------------------------------
  `bytes`     Vec\<u8\>,   The spending condition of the output.

Enums
-----

### LockTime {#LockTime}

Represents either a unix timestamp OR a blocknumber. See the [Bitcoin
source](https://github.com/bitcoin/bitcoin/blob/7fcf53f7b4524572d1d0c9a5fdc388e87eb02416/src/script/script.h#L39).

::: {.tabularcolumns}
L\|
:::

  Discriminant         Description
  -------------------- --------------------------------------------
  `Time(u32)`          Lock time interpreted as a unix timestamp.
  `BlockHeight(u32)`   Lock time interpreted as a block number.

Data Structures
---------------

### BlockHeaders

Mapping of `<blockHash, RichBlockHeader>`, storing all verified Bitcoin
block headers (fork and main chain) submitted to BTC-Relay.

### Chains {#Chains}

Level of indirection over `ChainsIndex`{.interpreted-text role="ref"},
i.e. the values stored in this map are keys of `ChainsIndex`.
`Chains[0]` MUST always be `0`, such that `ChainsIndex[Chains[0]]` is
the bitcoin *main chain*. The remaining items MUST sort the chains by
height, i.e. it MUST hold that for each `0 < i < j`,
`ChainsIndex[Chains[i]].maxHeight >= ChainsIndex[Chains[j]].maxHeight`.
Furthermore, keys MUST be consecutive, i.e. for each `i`, if `Chains[i]`
does not exist, `Chains[i+1]` MUST NOT exist either.

::: {.note}
::: {.title}
Note
:::

The assumption for `Chains` is that, in the majority of cases, block
headers will be appended to the *main chain* (longest chain), i.e., the
`BlockChain` entry at the most significant position in the queue/heap.
Similarly, transaction inclusion proofs
(`verifyTransactionInclusion`{.interpreted-text role="ref"}) are only
checked against the *main chain*. This means, in the average case lookup
complexity will be O(1). Furthermore, block headers can only be appended
if they (i) have a valid PoW and (ii) do not yet exist in `BlockHeaders`
- hence, spamming is very costly and unlikely. Finally, blockchain forks
and re-organizations occur infrequently, especially in Bitcoin. In
principle, optimizing lookup costs should be prioritized, ideally O(1),
while inserting of new items and re-balancing can even be O(n).
:::

### ChainsIndex {#ChainsIndex}

The main storage map of `BlockChain` structs, indexed by a *values* from
the `Chains`{.interpreted-text role="ref"}. `ChainsIndex[0]` MUST always
contain the main chain.

### BestBlock

32 byte Bitcoin block hash (double SHA256) identifying the current
blockchain tip, i.e., the `RichBlockHeader` with the highest
`blockHeight` in the `BlockChain` entry, which has the most significant
`height` in the `Chains` priority queue (topmost position).

::: {.note}
::: {.title}
Note
:::

Bitcoin uses SHA256 (32 bytes) for its block hashes, transaction
identifiers and Merkle trees. In Substrate, we hence use `H256` to
represent these hashes.
:::

### BestBlockHeight

Integer representing the maximum block height (`height`) in the `Chains`
priority queue. This is also the `blockHeight` of the `RichBlockHeader`
entry pointed to by `BestBlock`.

### ChainCounter

Integer increment-only counter used to track existing BlockChain
entries. Initialized with 1 (0 is reserved for `MAIN_CHAIN_ID`).
