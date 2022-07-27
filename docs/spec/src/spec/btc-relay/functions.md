Functions: Storage and Verification {#storage-verification}
===================================

initialize
----------

Initializes BTC-Relay with the first Bitcoin block to be tracked and
initializes all data structures (see `data-model`{.interpreted-text
role="ref"}).

::: {.note}
::: {.title}
Note
:::

BTC-Relay **does not** have to be initialized with Bitcoin\'s genesis
block! The first block to be tracked can be selected freely.
:::

::: {.warning}
::: {.title}
Warning
:::

Caution when setting the first block in BTC-Relay: only succeeding
blocks can be submitted and **predecessors and blocks from other chains
will be rejected**! Similarly, caution is required with the initial
block height argument, since if an incorrect value is used, all
subsequently reported block heights will be incorrect.
:::

### Specification

*Function Signature*

`initialize(relayer, rawBlockHeader, blockHeight)`

*Parameters*

-   `relayer`: the account submitting the block
-   `rawBlockHeader`: 80 byte raw Bitcoin block header, see
    `RawBlockHeader`{.interpreted-text role="ref"}.
-   `blockHeight`: integer Bitcoin block height of the submitted block
    header

*Events*

-   `Initialized(blockHeight, blockHash, relayer)`: if the first block
    header was stored successfully, emit an event with the stored
    block\'s height (`blockHeight`) and the (PoW) block hash
    (`blockHash`).

*Errors*

-   `ERR_ALREADY_INITIALIZED = "Already initialized"`: return error if
    this function is called after BTC-Relay has already been
    initialized.

*Preconditions*

-   This is the first time this function is called, i.e., when BTC-Relay
    is being deployed.
-   The blockheader MUST be parsable.
-   `blockHeight` MUST match the height on the bitcoin chain. Note that
    the parachain can not check this - it\'s the caller\'s
    responsability!
-   `rawBlockHeader` MUST match a block on the bitcoin main chain. Note
    that the parachain can not check this - it\'s the caller\'s
    responsability!
-   `rawBlockHeader` MUST be a block mined after December 2015 - see
    `bitcoinBlockHeader`{.interpreted-text role="ref"}. This is NOT
    checked by the parachain - it\'s the caller\'s responsibility!

*Postconditions*

Let `blockHeader` be the parsed `rawBlockHeader`. Then:

-   `ChainsIndex[0]` MUST be set to a new `BlockChain` value, where
    `BlockChain.chainId = 0`
    and`BlockChain.startHeight = BlockChain.maxHeight = blockHeight`
-   A value `block` of type `RichBlockHeader` MUST be added to
    `BlockHeaders`, where:
    -   `block.basic_block_header = blockHeader`
    -   `block.chainRef = 0`
    -   `block.paraHeight` is the current activeBlockCount (see the
        Security module)
    -   `block.blockHeight = blockHeight`
-   `BestBlockHeight` MUST be `ChainsIndex[0].maxHeight`
-   `BestBlock` MUST be `blockHeader.hash`
-   `StartBlockHeight` MUST be set to `blockHeight`

storeBlockHeader {#storeBlockHeader}
----------------

Method to submit block headers to the BTC-Relay. This function calls
`verifyBlockHeader`{.interpreted-text role="ref"} to check that the
block header is valid. If so, from the block header and stores the hash,
height and Merkle tree root of the given block header in `BlockHeaders`.
If the block header extends an existing `BlockChain` entry in `Chains`,
it appends the block hash to the `chains` mapping and increments the
`maxHeight`. Otherwise, a new `Blockchain` entry is created.

### Specification

*Function Signature*

`storeBlockHeader(relayer, rawBlockHeader)`

*Parameters*

-   `relayer`: the account submitting the block
-   `rawBlockHeader`: 80 byte raw Bitcoin block header, see
    `RawBlockHeader`{.interpreted-text role="ref"}.

*Events*

-   `StoreMainChainHeader(blockHeight, blockHash, relayer)`: if the
    block header was successful appended to the currently longest chain
    (*main chain*) emit an event with the stored block\'s height
    (`blockHeight`) and the (PoW) block hash (`blockHash`).
-   `StoreForkHeader(forkId, blockHeight, blockHash, relayer)`: if the
    block header was successful appended to a new or existing fork, emit
    an event with the block height (`blockHeight`) and the (PoW) block
    hash (`blockHash`).

*Invariants*

-   The values in `Chains` MUST be such that for each `0 < i < j`,
    `ChainsIndex[Chains[i]].maxHeight >= ChainsIndex[Chains[j]].maxHeight`.
-   The keys in `Chains` MUST be consecutive, i.e. for each `i`, if
    `Chains[i]` does not exist, `Chains[i+1]` MUST NOT exist either.
-   The keys in `ChainsIndex` MUST be consecutive, i.e. for each `i`, if
    `ChainsIndex[i]` does not exist, `ChainsIndex[i+1]` MUST NOT exist
    either.
-   For all `i > 0` the following MUST hold: [ChainsIndex\[i\].maxHeight
    \< ChainsIndex\[0\].maxHeight +
    STABLE\_BITCOIN\_CONFIRMATIONS]{.title-ref}.
-   For all `i`, the following MUST hold:
    `ChainsIndex[i].chainRef == i`.
-   `BestBlock.chainRef` MUST be 0
-   `BestBlock.blockHeight` MUST be `ChainsIndex[0].maxHeight`
-   `BestBlockHeight` MUST be `ChainsIndex[0].maxHeight`

*Preconditions*

-   The BTC Parachain status MUST NOT be set to `SHUTDOWN: 3`.

-   The given `rawBlockHeader` MUST parse be parsable into
    `blockHeader`.

-   There MUST be a block header `prevHeader` stored in `BlockHeaders`
    with a hash equal to `blockHeader.hashPrevBlock`.

-   A block chain `prevBlockchain` MUST be stored in
    `ChainsIndex[prevHeader.chainRef]`.

-   `VerifyBlockHeader`{.interpreted-text role="ref"} MUST return `Ok`
    when called with `blockHeader`, `prevHeader.blockHeight + 1` and
    `prevHeader`.

-   

    If `prevHeader` is the last element a chain (i.e. `blockHeader` does not create a new fork), then:

    :   -   `prevBlockChain` MUST NOT already contain a block of height
            `prevHeader.blockHeight + 1`.
        -   If `prevBlockChain.chain_id` is \_[not]() zero (i.e. the
            block is being added to a fork rather than the main chain),
            and the fork is `STABLE_BITCOIN_CONFIRMATIONS` blocks ahead
            of the main chain, then calling
            `swapMainBlockchain`{.interpreted-text role="ref"} with this
            fork MUST return `Ok`.

*Postconditions*

-   If `prevHeader` is the last element a chain (i.e. `blockHeader` does
    not create a new fork), then:
    -   `ChainsHashes[prevBlockChain.chain_id, prevHeader.blockHeight + 1]`
        MUST be set to `blockHeader.hash`.
    -   `ChainsIndex[prevBlockChain.chain_id].max_height` MUST be
        increased by 1.
    -   If `prevBlockChain.chain_id` is zero (i.e. the a block is being
        added to the main chain), then:
        -   `BestBlock` MUST be set to `blockHeader.hash`
        -   `BestBlockHeight` MUST be set to
            `prevHeader.blockHeight + 1`
    -   If `prevBlockChain.chain_id` is \_[not]() zero (i.e. the block
        is being added to a fork rather than the main chain), then:
        -   If the fork is `STABLE_BITCOIN_CONFIRMATIONS` blocks ahead
            of the main chain, i.e.
            `prevHeader.blockHeight + 1 >= BestBlockHeight + STABLE_BITCOIN_CONFIRMATIONS`,
            then the fork is moved to the mainchain. That is,
            `swapMainBlockchain`{.interpreted-text role="ref"} MUST be
            called with the fork as argument.
    -   A new `RichBlockHeader` MUST be stored in `BlockHeaders` that is
        constructed as follows:
        -   `RichBlockHeader.blockHeader = blockHeader`,
        -   `RichBlockHeader.blockHeight = prevBlock.blockHeight + 1`,
        -   `RichBlockHeader.chainRef = prevBlockChain.chainId`,
        -   `RichBlockHeader.paraHeight` is set to the current active
            block count - see the security module for details
-   If `prevHeader` is *not* the last element a chain (i.e.
    `blockHeader` creates a *new* fork), then:
    -   `ChainCounter` MUST be incremented. Let `newChainCounter` be the
        incremented value, then
    -   `ChainsHashes[newChainCounter, prevHeader.blockHeight + 1]` MUST
        be set to `blockHeader.hash`.
    -   A new blockchain MUST be inserted into `ChainsIndex`. Let
        `newChain` be the newly inserted chain. Then `newChain` MUST
        have the following values:
        -   `newChain.chainId = newChainCounter`,
        -   `newChain.startHeight = prevHeader.blockHeight + 1`,
        -   `newChain.maxHeight = prevHeader.blockHeight + 1`,
    -   A new value MUST be added to `Chains` that is equal to
        `newChainCounter` in a way that maintains the invariants
        specified above.
    -   A new `RichBlockHeader` MUST be stored in `BlockHeaders` that is
        constructed as follows:
        -   `RichBlockHeader.blockHeader = blockHeader`,
        -   `RichBlockHeader.blockHeight = newChain.blockHeight + 1`,
        -   `RichBlockHeader.chainRef = prevBlockChain.chainId`,
        -   `RichBlockHeader.paraHeight` is set to the current active
            block count - see the security module for details
-   `BestBlockHeight` MUST be set to `Chains[0].max_height`
-   `BestBlock` MUST be set to `ChainsHashes[0, Chains[0].max_height`

::: {.warning}
::: {.title}
Warning
:::

The BTC-Relay does not necessarily have the same view of the Bitcoin
blockchain as the user\'s local Bitcoin client. This can happen if (i)
the BTC-Relay is under attack, (ii) the BTC-Relay is out of sync, or,
similarly, (iii) if the user\'s local Bitcoin client is under attack or
out of sync (see `security`{.interpreted-text role="ref"}).
:::

::: {.note}
::: {.title}
Note
:::

The 80 bytes block header can be retrieved from the [bitcoin-rpc
client](https://en.bitcoin.it/wiki/Original_Bitcoin_client/API_calls_list)
by calling the
[getBlock](https://bitcoin-rpc.github.io/en/doc/0.17.99/rpc/blockchain/getblock/)
and setting verbosity to `0` (`getBlock <blockHash> 0`).
:::

swapMainBlockchain {#swapMainBlockchain}
------------------

### Specification

*Function Signature*

`swapMainBlockchain(fork)`

*Parameters*

-   `fork`: pointer to a `BlockChain` entry in `Chains`.

*Preconditions*

-   `fork` is `STABLE_BITCOIN_CONFIRMATIONS` blocks ahead of the main
    chain, i.e.
    `fork.maxHeight >= BestBlockHeight + STABLE_BITCOIN_CONFIRMATIONS`

*Postconditions*

Let `lastBlock` be the last rich block header in `fork`, i.e. the
blockheader for which `lastBlock.blockHeight == fork.maxHeight` and
`lastBlock.chainRef == fork.chainId` holds. Then:

-   Each ancestor `a` of `lastBlock` MUST move to the main chain, i.e.
    `a.chainRef` MUST be set to `MAIN_CHAIN_ID`.
-   `ChainsIndex[MAIN_CHAIN_ID].maxHeight` MUST be set to
    `lastBlock.blockHeight`.
-   Each fork `fork` except the main chain that contains an ancestor of
    `lastBlock` MUST set `fork.startHeight` to the lowest `blockHeight`
    in the fork that is not an ancestor of `lastBlock`.
-   Each block `b` in the mainchain that is not an acestor of
    `lastBlock` MUST move to `prevBlockChain`, i.e.
    `b.chainRef = prevBlockChain.chainId`.
-   `prevBlockChain.startHeight` MUST be set to the lowest `blockHeight`
    of all blocks `b` that have `b.chainRef == prevBlockChain.chainId`.
-   `prevBlockChain.maxHeight` MUST be set to the highest `blockHeight`
    of all blocks `b` that have `b.chainRef == prevBlockChain.chainId`.

The figure below ilustrates an example execution of this function.

![On the left is an example of the state of `ChainsIndex` prior to
calling `swapMainBlockchain`, and on the right is the corresponding
state after the function
returns.](../../figures/spec/btcrelay/swap_main_blockchain.png)

In contrast the the figure about, when looking up the chains through the
`Chains` map, the chains are sorted by `maxHeight`, and the same
execution would look as follows:

![On the left is an example of the state of `Chains` prior to calling
`swapMainBlockchain`, and on the right is the corresponding state after
the function
returns.](../../figures/spec/btcrelay/%5Bchains%5Dswap_main_blockchain.png)

verifyBlockHeader {#verifyBlockHeader}
-----------------

The `verifyBlockHeader` function verifies Bitcoin block headers. It
returns `Ok` if the blockheader is valid, otherwise an error.

::: {.note}
::: {.title}
Note
:::

This function does not check whether the submitted block header extends
the main chain or a fork. This check is performed in
`storeBlockHeader`{.interpreted-text role="ref"}.
:::

### Specification

*Function Signature*

`verifyBlockHeader(blockHeader, blockHeight, prevBlockHeader)`

*Parameters*

-   `blockHeader`: the `BlockHeader`{.interpreted-text role="ref"} to
    check.
-   `blockHeight`: height of the block.
-   `prevBlockHeader`: the `RichBlockHeader`{.interpreted-text
    role="ref"} that is the block header\'s predecessor.

*Returns*

-   `Ok(())` if all checks pass successfully, otherwise an error.

*Errors*

-   `ERR_DUPLICATE_BLOCK = "Block already stored"`: return error if the
    submitted block header is already stored in BTC-Relay (duplicate PoW
    `blockHash`).
-   `ERR_LOW_DIFF = "PoW hash does not meet difficulty target of header"`:
    return error when the header\'s `blockHash` does not meet the
    `target` specified in the block header.
-   `ERR_DIFF_TARGET_HEADER = "Incorrect difficulty target specified in block header"`:
    return error if the `target` specified in the block header is
    incorrect for its block height (difficulty re-target not executed).

*Preconditions*

-   A block with the `blockHeader.hash` MUST NOT already have been
    stored.
-   `blockHeader.hash` MUST be be below `BlockHeader.target`
-   `blockHeader.target` MUST match the expected target, which is
    calculated based on previous targets and timestamps. See [the
    Bitcoin Wiki](https://en.bitcoin.it/wiki/Difficulty) for more
    information.

*Postconditions*

-   `Ok(())` MUST be returned.

verifyTransactionInclusion {#verifyTransactionInclusion}
--------------------------

The `verifyTransactionInclusion` function is one of the core components
of the BTC-Relay: this function checks if a given transaction was indeed
included in a given block (as stored in `BlockHeaders` and tracked by
`Chains`), by reconstructing the Merkle tree root (given a Merkle
proof). Also checks if sufficient confirmations have passed since the
inclusion of the transaction (considering the current state of the
BTC-Relay `Chains`).

### Specification

*Function Signature*

`verifyTransactionInclusion(txId, merkleProof, confirmations, insecure)`

*Parameters*

-   `txId`: 32 byte hash identifier of the transaction.
-   `merkleProof`: Merkle tree path (concatenated LE sha256 hashes,
    dynamic sized).
-   `confirmations`: integer number of confirmation required.

::: {.note}
::: {.title}
Note
:::

The Merkle proof for a Bitcoin transaction can be retrieved using the
`bitcoin-rpc`
[gettxoutproof](https://bitcoin-rpc.github.io/en/doc/0.17.99/rpc/blockchain/gettxoutproof/)
method and dropping the first 170 characters. The Merkle proof thereby
consists of a list of SHA256 hashes, as well as an indicator in which
order the hash concatenation is to be applied (left or right).
:::

*Returns*

-   `True`: if the given `txId` appears in at the position specified by
    `txIndex` in the transaction Merkle tree of the block at height
    `blockHeight` and sufficient confirmations have passed since
    inclusion.
-   Error otherwise.

*Events*

-   `VerifyTransaction(txId, txBlockHeight, confirmations)`: if
    verification was successful, emit an event specifying the `txId`,
    the `blockHeight` and the requested number of `confirmations`.

*Errors*

-   `ERR_SHUTDOWN = "BTC Parachain has shut down"`: the BTC Parachain
    has been shutdown by a manual intervention of the Governance
    Mechanism.
-   `ERR_MALFORMED_TXID = "Malformed transaction identifier"`: return
    error if the transaction identifier (`txId`) is malformed.
-   `ERR_CONFIRMATIONS = "Transaction has less confirmations than requested"`:
    return error if the block in which the transaction specified by
    `txId` was included has less confirmations than requested.
-   `ERR_INVALID_MERKLE_PROOF = "Invalid Merkle Proof"`: return error if
    the Merkle proof is malformed or fails verification (does not hash
    to Merkle root).
-   `ERR_ONGOING_FORK = "Verification disabled due to ongoing fork"`:
    return error if the `mainChain` is not at least
    `STABLE_BITCOIN_CONFIRMATIONS` ahead of the next best fork.

### Preconditions

-   The BTC Parachain status must not be set to `SHUTDOWN: 3`. If
    `SHUTDOWN` is set, all transaction verification is disabled.

### Function Sequence

1.  Check that `txId` is 32 bytes long. Return `ERR_MALFORMED_TXID`
    error if this check fails.
2.  Check that the current `BestBlockHeight` exceeds `txBlockHeight` by
    the requested confirmations. Return `ERR_CONFIRMATIONS` if this
    check fails.

> a.  If `insecure == True`, check against user-defined `confirmations`
>     only
> b.  If `insecure == True`, check against
>     `max(confirmations, STABLE_BITCOIN_CONFIRMATIONS)`.

3.  Check if the Bitcoin block was stored for a sufficient number of
    blocks (on the parachain) to ensure that staked relayers had the
    time to flag the block as potentially invalid. Check performed
    against `STABLE_PARACHAIN_CONFIRMATIONS`.
4.  Extract the block header from `BlockHeaders` using the `blockHash`
    tracked in `Chains` at the passed `txBlockHeight`.
5.  Check that the first 32 bytes of `merkleProof` are equal to the
    `txId` and the last 32 bytes are equal to the `merkleRoot` of the
    specified block header. Also check that the `merkleProof` size is
    either exactly 32 bytes, or is 64 bytes or more and a power of 2.
    Return `ERR_INVALID_MERKLE_PROOF` if one of these checks fails.
6.  Call `computeMerkle`{.interpreted-text role="ref"} passing `txId`,
    `txIndex` and `merkleProof` as parameters.

> a.  If this call returns the `merkleRoot`, emit a
>     `VerifyTransaction(txId, txBlockHeight, confirmations)` event and
>     return `True`.
> b.  Otherwise return `ERR_INVALID_MERKLE_PROOF`.

![The steps to verify a transaction in the
`verifyTransactionInclusion`{.interpreted-text role="ref"}
function.](../../figures/spec/btcrelay/verifyTransaction-sequence.png)

validateTransaction {#validateTransaction}
-------------------

Given a raw Bitcoin transaction, this function

1)  Parses and extracts
    a.  the value and recipient address of the *Payment UTXO*,
    b.  \[Optionally\] the OP\_RETURN value of the *Data UTXO*.
2)  Validates the extracted values against the function parameters.

::: {.note}
::: {.title}
Note
:::

See `bitcoin-data-model`{.interpreted-text role="ref"} for more details
on the transaction structure, and
`accepted_bitcoin_transaction_format`{.interpreted-text role="ref"} for
the transaction format of Bitcoin transactions validated in this
function.
:::

### Specification

*Function Signature*

`validateTransaction(rawTx, paymentValue, recipientBtcAddress, opReturnId)`

*Parameters*

-   `rawTx`: raw Bitcoin transaction including the transaction inputs
    and outputs.
-   `paymentValue`: integer value of BTC sent in the (first) *Payment
    UTXO* of transaction.
-   `recipientBtcAddress`: 20 byte Bitcoin address of recipient of the
    BTC in the (first) *Payment UTXO*.
-   `opReturnId`: \[Optional\] 32 byte hash identifier expected in
    OP\_RETURN (see `replace-attacks`{.interpreted-text role="ref"}).

*Returns*

-   `True`: if the transaction was successfully parsed and validation of
    the passed values was correct.
-   Error otherwise.

*Events*

-   `ValidateTransaction(txId, paymentValue, recipientBtcAddress, opReturnId)`:
    if parsing and validation was successful, emit an event specifying
    the `txId`, the `paymentValue`, the `recipientBtcAddress` and the
    `opReturnId`.

*Errors*

-   `ERR_INSUFFICIENT_VALUE = "Value of payment below requested amount"`:
    return error the value of the (first) *Payment UTXO* is lower than
    `paymentValue`.
-   `ERR_TX_FORMAT = "Transaction has incorrect format"`: return error
    if the transaction has an incorrect format (see
    `accepted_bitcoin_transaction_format`{.interpreted-text
    role="ref"}).
-   `ERR_WRONG_RECIPIENT = "Incorrect recipient Bitcoin address"`:
    return error if the recipient specified in the (first) *Payment
    UTXO* does not match the given `recipientBtcAddress`.
-   `ERR_INVALID_OPRETURN = "Incorrect identifier in OP_RETURN field"`:
    return error if the OP\_RETURN field of the (second) *Data UTXO*
    does not match the given `opReturnId`.

### Preconditions

-   The BTC Parachain status must not be set to `SHUTDOWN: 3`. If
    `SHUTDOWN` is set, all transaction validation is disabled.

### Function Sequence

See the [raw Transaction Format section in the Bitcoin Developer
Reference](https://bitcoin.org/en/developer-reference#raw-transaction-format)
for a full specification of Bitcoin\'s transaction format (and how to
extract inputs, outputs etc. from the raw transaction format).

1.  Extract the `outputs` from `rawTx` using
    `extractOutputs`{.interpreted-text role="ref"}.

> a.  Check that the transaction (`rawTx`) has at least 2 outputs. One
>     output (*Payment UTXO*) must be a
>     [P2PKH](https://en.bitcoinwiki.org/wiki/Pay-to-Pubkey_Hash) or
>     [P2WPKH](https://github.com/libbitcoin/libbitcoin-system/wiki/P2WPKH-Transactions)
>     output. Another output (*Data UTXO*) must be an
>     [OP\_RETURN](https://bitcoin.org/en/transactions-guide#term-null-data)
>     output. Raise `ERR_TX_FORMAT` if this check fails.

2.  Extract the value of the *Payment UTXO* using
    `extractOutputValue`{.interpreted-text role="ref"} and check that it
    is equal (or greater) than `paymentValue`. Return
    `ERR_INSUFFICIENT_VALUE` if this check fails.
3.  Extract the Bitcoin address specified as recipient in the *Payment
    UTXO* using `extractOutputAddress`{.interpreted-text role="ref"} and
    check that it matches `recipientBtcAddress`. Return
    `ERR_WRONG_RECIPIENT` if this check fails, or the error returned by
    `extractOutputAddress`{.interpreted-text role="ref"} (if the output
    was malformed).
4.  Extract the OP\_RETURN value from the *Data UTXO* using
    `extractOPRETURN`{.interpreted-text role="ref"} and check that it
    matches `opReturnId`. Return `ERR_INVALID_OPRETURN` error if this
    check fails, or the error returned by
    `extractOPRETURN`{.interpreted-text role="ref"} (if the output was
    malformed).

verifyAndValidateTransaction {#verifyAndValidateTransaction}
----------------------------

The `verifyAndValidateTransaction` function is a wrapper around the
`verifyTransactionInclusion`{.interpreted-text role="ref"} and the
`validateTransaction`{.interpreted-text role="ref"} functions. It adds
an additional check to verify that the validated transaction is the one
included in the specified block.

### Specification

*Function Signature*

`verifyAndValidateTransaction(merkleProof, confirmations, rawTx, paymentValue, recipientBtcAddress, opReturnId)`

*Parameters*

-   `txId`: 32 byte hash identifier of the transaction.
-   `merkleProof`: Merkle tree path (concatenated LE sha256 hashes,
    dynamic sized).
-   `confirmations`: integer number of confirmation required.
-   `rawTx`: raw Bitcoin transaction including the transaction inputs
    and outputs.
-   `paymentValue`: integer value of BTC sent in the (first) *Payment
    UTXO* of transaction.
-   `recipientBtcAddress`: 20 byte Bitcoin address of recipient of the
    BTC in the (first) *Payment UTXO*.
-   `opReturnId`: \[Optional\] 32 byte hash identifier expected in
    OP\_RETURN (see `replace-attacks`{.interpreted-text role="ref"}).

*Returns*

-   `True`: If the same transaction has been verified and validated.
-   Error otherwise.

### Function Sequence

1.  Parse the `rawTx` to get the tx id.
2.  Call `verifyTransactionInclusion`{.interpreted-text role="ref"} with
    the applicable parameters.
3.  Call `validateTransaction`{.interpreted-text role="ref"} with the
    applicable parameters.

flagBlockError {#flagBlockError}
--------------

Flags tracked Bitcoin block headers when Staked Relayers report and
agree on a `NO_DATA_BTC_RELAY` or `INVALID_BTC_RELAY` failure.

::: {.attention}
::: {.title}
Attention
:::

This function **does not** validate the Staked Relayers accusation.
Instead, it is put up to a majority vote among all Staked Relayers in
the form of a
:::

::: {.note}
::: {.title}
Note
:::

This function can only be called from the *Security* module of interBTC,
after Staked Relayers have achieved a majority vote on a BTC Parachain
status update indicating a BTC-Relay failure.
:::

### Specification

*Function Signature*

`flagBlockError(blockHash, errors)`

*Parameters*

-   `blockHash`: SHA256 block hash of the block containing the error.
-   `errors`: list of `ErrorCode` entries which are to be flagged for
    the block with the given blockHash. Can be \"NO\_DATA\_BTC\_RELAY\"
    or \"INVALID\_BTC\_RELAY\".

*Events*

-   `FlagBTCBlockError(blockHash, chainId, errors)` - emits an event
    indicating that a Bitcoin block hash (identified `blockHash`) in a
    `BlockChain` entry (`chainId`) was flagged with errors (`errors`
    list of `ErrorCode` entries).

*Errors*

-   `ERR_UNKNOWN_ERRORCODE = "The reported error code is unknown"`: The
    reported `ErrorCode` can only be `NO_DATA_BTC_RELAY` or
    `INVALID_BTC_RELAY`.
-   `ERR_BLOCK_NOT_FOUND  = "No Bitcoin block header found with the given block hash"`:
    No `RichBlockHeader` entry exists with the given block hash.
-   `ERR_ALREADY_REPORTED = "This error has already been reported for the given block hash and is pending confirmation"`:
    The error reported for the given block hash is currently pending a
    vote by Staked Relayers.

#### Function Sequence

1.  Check if `errors` contains `NO_DATA_BTC_RELAY` or
    `INVALID_BTC_RELAY`. If neither match, return
    `ERR_UNKNOWN_ERRORCODE`.
2.  Retrieve the `RichBlockHeader` entry from `BlockHeaders` using
    `blockHash`. Return `ERR_BLOCK_NOT_FOUND` if no block header can be
    found.
3.  Retrieve the `BlockChain` entry for the given `RichBlockHeader`
    using `ChainsIndex` for lookup with the block header\'s `chainRef`
    as key.
4.  Flag errors in the `BlockChain` entry:
    a.  If `errors` contains `NO_DATA_BTC_RELAY`, append the
        `RichBlockHeader.blockHeight` to `BlockChain.noData`
    b.  If `errors` contains `INVALID_BTC_RELAY`, append the
        `RichBlockHeader.blockHeight` to `BlockChain.invalid` .
5.  Emit `FlagBTCBlockError(blockHash, chainId, errors)` event, with the
    given `blockHash`, the `chainId` of the flagged `BlockChain` entry
    and the given `errors` as parameters.
6.  Return

clearBlockError {#clearBlockError}
---------------

Clears `ErrorCode` entries given as parameters from the status of a
`RichBlockHeader`. Can be `NO_DATA_BTC_RELAY` or `INVALID_BTC_RELAY`
failure.

::: {.note}
::: {.title}
Note
:::

This function can only be called from the *Security* module of interBTC,
after Staked Relayers have achieved a majority vote on a BTC Parachain
status update indicating that a `RichBlockHeader` entry no longer has
the specified errors.
:::

### Specification

*Function Signature*

`flagBlockError(blockHash, errors)`

*Parameters*

-   `blockHash`: SHA256 block hash of the block containing the error.
-   `errors`: list of `ErrorCode` entries which are to be **cleared**
    from the block with the given blockHash. Can be `NO_DATA_BTC_RELAY`
    or `INVALID_BTC_RELAY`.

*Events*

-   `ClearBlockError(blockHash, chainId, errors)` - emits an event
    indicating that a Bitcoin block hash (identified `blockHash`) in a
    `BlockChain` entry (`chainId`) was cleared from the given errors
    (`errors` list of `ErrorCode` entries).

*Errors*

-   `ERR_UNKNOWN_ERRORCODE = "The reported error code is unknown"`: The
    reported `ErrorCode` can only be `NO_DATA_BTC_RELAY` or
    `INVALID_BTC_RELAY`.
-   `ERR_BLOCK_NOT_FOUND  = "No Bitcoin block header found with the given block hash"`:
    No `RichBlockHeader` entry exists with the given block hash.
-   `ERR_ALREADY_REPORTED = "This error has already been reported for the given block hash and is pending confirmation"`:
    The error reported for the given block hash is currently pending a
    vote by Staked Relayers.

#### Function Sequence

1.  Check if `errors` contains `NO_DATA_BTC_RELAY` or
    `INVALID_BTC_RELAY`. If neither match, return
    `ERR_UNKNOWN_ERRORCODE`.
2.  Retrieve the `RichBlockHeader` entry from `BlockHeaders` using
    `blockHash`. Return `ERR_BLOCK_NOT_FOUND` if no block header can be
    found.
3.  Retrieve the `BlockChain` entry for the given `RichBlockHeader`
    using `ChainsIndex` for lookup with the block header\'s `chainRef`
    as key.
4.  Un-flag error codes in the `BlockChain` entry.
    a.  If `errors` contains `NO_DATA_BTC_RELAY`: remove
        `RichBlockHeader.blockHeight` from `BlockChain.noData`
    b.  If `errors` contains `INVALID_BTC_RELAY`: remove
        `RichBlockHeader.blockHeight` from `BlockChain.invalid`
5.  Emit `ClearBlockError(blockHash, chainId, errors)` event, with the
    given `blockHash`, the `chainId` of the flagged `BlockChain` entry
    and the given `errors` as parameters.
6.  Return
