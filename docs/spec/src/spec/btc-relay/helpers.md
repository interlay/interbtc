Functions: Utils {#utils}
================

There are several helper methods available that abstract Bitcoin
internals away in the main function implementation.

sha256d
-------

Bitcoin uses a double SHA256 hash to protect against
[\"length-extension\"
attacks](https://en.wikipedia.org/wiki/Length_extension_attack).

::: {.note}
::: {.title}
Note
:::

Bitcoin uses little endian representations when sending hashes across
the network and for storing values internally. For more details, see the
[documentation](https://en.bitcoin.it/wiki/Protocol_documentation#common-structures).
The output of the SHA256 function is big endian by default.
:::

*Function Signature*

`sha256d(data)`

*Parameters*

-   `data`: bytes encoded input.

*Returns*

-   `hash`: the double SHA256 hash encodes as a bytes from `data`.

### Function Sequence

1.  Hash `data` with sha256.
2.  Hash the result of step 1 with sha256.
3.  Return `hash`.

concatSha256d {#concatSha256d}
-------------

A function that computes a parent hash from two child nodes. This
function is used in the reconstruction of the Merkle tree.

*Function Signature*

`concatSha256d(left, right)`

*Parameters*

-   `left`: 32 bytes of input data that are added first.
-   `right`: 32 bytes of input data that are added second.

*Returns*

-   `hash`: the double sha256 hash encoded as a bytes from `left` and
    `right`.

### Function Sequence

1.  Concatenate `left` and `right` into a 64 bytes.
2.  Call the [sha256d](#sha256d) function to hash the concatenated
    bytes.
3.  Return `hash`.

nBitsToTarget {#nBitsToTarget}
-------------

This function calculates the PoW difficulty target from a compressed
nBits representation. See the [Bitcoin
documentation](https://bitcoin.org/en/developer-reference#target-nbit)
for further details. The computation for the difficulty is as follows:

$$\text{target} = \text{significand} * \text{base}^{(\text{exponent} - 3)}$$

*Function Signature*

`nBitsToTarget(nBits)`

*Parameters*

-   `nBits`: 4 bytes compressed PoW target representation.

*Returns*

-   `target`: PoW difficulty target computed from nBits.

### Function Sequence

1.  Extract the *exponent* by shifting the `nBits` to the right by 24.
2.  Extract the *significand* by taking the first three bytes of
    `nBits`.
3.  Calculate the `target` via the equation above and using 2 as the
    *base* (as we use the U256 type).
4.  Return `target`.

checkCorrectTarget {#checkCorrectTarget}
------------------

Verifies the currently submitted block header has the correct difficulty
target.

*Function Signature*

`checkCorrectTarget(hashPrevBlock, blockHeight, target)`

*Parameters*

-   `hashPrevBlock`: 32 bytes previous block hash (necessary to retrieve
    previous target).
-   `blockHeight`: height of the current block submission.
-   `target`: PoW difficulty target computed from nBits.

*Returns*

-   `True`: if the difficulty target is set correctly.
-   `False`: otherwise.

### Function Sequence

1.  Retrieve the previous block header with the `hashPrevBlock` from the
    `BlockHeaders` storage and the difficulty target (`prevTarget`) of
    this (previous) block.

2.  Check if the `prevTarget` difficulty should be adjusted at this
    `blockHeight`.

    > a.  If the difficulty should not be adjusted, check if the
    >     `target` of the submitted block matches the `prevTarget` of
    >     the previous block and check that `prevTarget`is not `0`.
    >     Return false if either of these checks fails.
    >
    > b.  The difficulty should be adjusted. Calculate the new expected
    >     target by calling the [computeNewTarget](#computenewtarget)
    >     function and passing the timestamp of the previous block (get
    >     using `hashPrevBlock` key in `BlockHeaders`), the timestamp of
    >     the last re-target (get block hash from `Chains` using
    >     `blockHeight - 2016` as key, then query `BlockHeaders`) and
    >     the target of the previous block (get using `hashPrevBlock`
    >     key in `BlockHeaders`) as parameters. Check that the new
    >     target matches the `target` of the current block (i.e., the
    >     block\'s target was set correctly).
    >
    >     > i.  If the newly calculated target difficulty matches
    >     >     `target`, return `True`.
    >     > ii. Otherwise, return `False`.

computeNewTarget {#computeNewTarget}
----------------

Computes the new difficulty target based on the given parameters, [as
implemented in the Bitcoin core
client](https://github.com/bitcoin/bitcoin/blob/78dae8caccd82cfbfd76557f1fb7d7557c7b5edb/src/pow.cpp).

*Function Signature*

`computeNewTarget(prevTime, startTime, prevTarget)`

*Parameters*

-   `prevTime`: timestamp of previous block.
-   `startTime`: timestamp of last re-target.
-   `prevTarget`: PoW difficulty target of the previous block.

*Returns*

-   `newTarget`: PoW difficulty target of the current block.

### Function Sequence

1.  Compute the actual time span between `prevTime` and `startTime`.
2.  Compare if the actual time span is smaller than the target interval
    divided by 4 (default target interval in Bitcoin is two weeks). If
    true, set the actual time span to the target interval divided by 4.
3.  Compare if the actual time span is greater than the target interval
    multiplied by 4. If true, set the actual time span to the target
    interval multiplied by 4.
4.  Calculate the `newTarget` by multiplying the actual time span with
    the `prevTarget` and dividing by the target time span (2 weeks for
    Bitcoin).
5.  If the `newTarget` is greater than the maximum target in Bitcoin,
    set the `newTarget` to the maximum target (Bitcoin maximum target is
    $2^{224}-1$).
6.  Return the `newTarget`.

computeMerkle {#computeMerkle}
-------------

The computeMerkle function calculates the root of the Merkle tree of
transactions in a Bitcoin block. Further details are included in the
[Bitcoin developer
reference](https://bitcoin.org/en/developer-reference#parsing-a-merkleblock-message).

*Function Signature*

`computeMerkle(txId, txIndex, merkleProof)`

*Parameters*

-   `txId`: the hash identifier of the transaction.
-   `txIndex`: index of transaction in the block\'s transaction Merkle
    tree.
-   `merkleProof`: Merkle tree path (concatenated LE sha256 hashes).

*Returns*

-   `merkleRoot`: the hash of the Merkle root.

*Errors*

-   `ERR_INVALID_MERKLE_PROOF = "Invalid Merkle Proof structure"`: raise
    an exception if the Merkle proof is malformed.

### Function Sequence

1.  Check if the length of the Merkle proof is 32 bytes long.

    > a.  If true, only the coinbase transaction is included in the
    >     block and the Merkle proof is the `merkleRoot`. Return the
    >     `merkleRoot`.
    > b.  If false, continue function execution.

2.  Check if the length of the Merkle proof is greater or equal to 64
    and if it is a power of 2.

    > a.  If true, continue function execution.
    > b.  If false, raise `ERR_INVALID_MERKLE_PROOF`.

3.  Calculate the `merkleRoot`. For each 32 bytes long hash in the
    Merkle proof:

    > a.  Determine the position of transaction hash (or the last
    >     resulting hash) at either `0` or `1`.
    > b.  Slice the next 32 bytes from the Merkle proof.
    > c.  Concatenate the transaction hash (or last resulting hash) with
    >     the 32 bytes of the Merkle proof in the right order (depending
    >     on the transaction/last calculated hash position).
    > d.  Calculate the double SHA256 hash of the concatenated input
    >     with the [concatSha256d](#concatsha256d) function.
    > e.  Repeat until there are no more hashes in the `merkleProof`.

4.  The last resulting hash from step 3 is the `merkleRoot`. Return
    `merkleRoot`.

### Example

Assume we have the following input:

-   txId:
    `330dbbc15169c538583073fd0a7708d8de2d3dc155d75b361cbf5c24b73f3586`
-   txIndex: `0`
-   merkleProof:
    `86353fb7245cbf1c365bd755c13d2dded808770afd73305838c56951c1bb0d33b635f586cf6c4763f3fc98b99daf8ac14ce1146dc775777c2cd2c4290578ef2e`

The `computeMerkle` function would go past step 1 as our proof is longer
than 32 bytes. Next, step 2 would also be passed as the proof length is
equal to 64 bytes and a power of 2. Last, we calculate the Merkle root
in step 3 as shown below.

![An example of the `computeMerkle` function with a transaction from a
block that contains two transactions in
total.](../../figures/spec/btcrelay/computeMerkle.png)

calculateDifficulty {#calculateDifficulty}
-------------------

Given the `target`, calculates the Proof-of-Work `difficulty` value, as
defined in [the Bitcoin wiki](https://en.bitcoin.it/wiki/Difficulty).

*Function Signature*

`calculateDifficulty(target)`

*Parameters*

-   `target`: target as specified in a Bitcoin block header.

*Returns*

-   `difficulty`: difficulty calculated from given `target`.

### Function Sequence

1.  Return `0xffff0000000000000000000000000000000000000000000000000000`
    (max. possible target, also referred to as \"difficulty 1\") divided
    by `target`.

getForkIdByBlockHash {#getForkIdByBlockHash}
--------------------

Helper function allowing to query the list of tracked forks (`Forks`)
for the identifier of a fork given its last submitted (\"highest\")
block hash.

### Specification

*Function Signature*

`getForkIdByBlockHash(blockHash)`

*Parameters*

-   `blockHash`: block hash of the last submitted block to a fork.

*Returns*

-   `forkId`: if there exists a fork with `blockHash` as latest
    submitted block in `forkHashes`.
-   `ERR_FORK_ID_NOT_FOUND`: otherwise.

*Errors*

-   `ERR_FORK_ID_NOT_FOUND = Fork ID not found for specified block hash."`:
    return this error if there exists no `forkId` for the given
    `blockHash`.

### Function Sequence

1.  Loop over all entries in `Forks` and check if
    `forkHashes[forkHashes.length -1] == blockhash`

    > a.  If `True`: return the corresponding `forkId`.

2.  Return `ERR_FORK_ID_NOT_FOUND` otherwise.

incrementChainCounter {#getChainsCounter}
---------------------

Increments the current `ChainCounter` and returns the new value.

### Specification

*Function Signature*

`incrementChainsCounter()`

*Returns*

-   `chainCounter`: the new integer value of the `ChainCounter`.

### Function Sequence

1.  `ChainCounter++`
2.  Return `ChainCounter`
