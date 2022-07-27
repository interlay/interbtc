Error Codes {#errors}
===========

A summary of error codes raised in exceptions by BTC-Relay, and their
meanings, are provided below.

`ERR_ALREADY_INITIALIZED`

-   **Message:** \"Already initialized.\"
-   **Function:** `initialize`{.interpreted-text role="ref"}
-   **Cause**: Raised if the `initialize` function is called when
    BTC-Relay has already been initialized.

`ERR_NOT_MAIN_CHAIN`

-   **Message:** \"Main chain submission indicated, but submitted block
    is on a fork\"
-   **Function:** `storeBlockHeader`{.interpreted-text role="ref"}
-   **Cause**: Raised if the block header submission indicates that it
    is extending the current longest chain, but is actually on a (new)
    fork.

`ERR_FORK_PREV_BLOCK`

-   **Message:** \"Previous block hash does not match last block in fork
    submission\"
-   **Function:** `storeBlockHeader`{.interpreted-text role="ref"}
-   **Cause**: Raised if the block header does not reference the highest
    block in the fork specified by `forkId` (via `prevBlockHash`).

`ERR_NOT_FORK`

-   **Message**: \"Indicated fork submission, but block is in main
    chain\"
-   **Function**: `storeBlockHeader`{.interpreted-text role="ref"}
-   **Cause**: Raised if raise exception if the submitted block header
    is actually extending the current longest chain tracked by BTC-Relay
    (`Chains`), instead of a fork.

`ERR_INVALID_FORK_ID`

-   **Message**: \"Incorrect fork identifier.\"
-   **Function**: `storeBlockHeader`{.interpreted-text role="ref"}
-   **Cause**: Raised if a non-existent fork identifier is passed.

`ERR_INVALID_HEADER_SIZE`

-   **Message**: \"Invalid block header size\":
-   **Function**: `parseBlockHeader`{.interpreted-text role="ref"}
-   **Cause**: Raised if the submitted block header is not exactly 80
    bytes long.

`ERR_DUPLICATE_BLOCK`

-   **Message**: \"Block already stored\"
-   **Function**: `verifyBlockHeader`{.interpreted-text role="ref"}
-   **Cause**: Raised if the submitted block header is already stored in
    the BTC-Relay (duplicate PoW `blockHash`).

`ERR_PREV_BLOCK`

-   **Message**: \"Previous block hash not found\"
-   **Function**: `verifyBlockHeader`{.interpreted-text role="ref"}
-   **Cause**: Raised if the submitted block does not reference an
    already stored block header as predecessor (via `prevBlockHash`).

`ERR_LOW_DIFF`

-   **Message**:\"PoW hash does not meet difficulty target of header\"
-   **Function**: `verifyBlockHeader`{.interpreted-text role="ref"}
-   **Cause**: Raised if the header\'s `blockHash` does not meet the
    `target` specified in the block header.

`ERR_DIFF_TARGET_HEADER`

-   **Message**: \"Incorrect difficulty target specified in block
    header\"
-   **Function**: `verifyBlockHeader`{.interpreted-text role="ref"}
-   **Cause**: Raised if the `target` specified in the block header is
    incorrect for its block height (difficulty re-target not executed).

`ERR_MALFORMED_TXID`

-   **Message**: \"Malformed transaction identifier\"
-   **Function**: `verifyTransactionInclusion`{.interpreted-text
    role="ref"}
-   **Cause**: Raised if the transaction id (`txId`) is malformed.

`ERR_CONFIRMATIONS`

-   **Message**: \"Transaction has less confirmations than requested\"
-   **Function**: `verifyTransactionInclusion`{.interpreted-text
    role="ref"}
-   **Cause**: Raised if the number of confirmations is less than
    required.

`ERR_INVALID_MERKLE_PROOF`

-   **Message**: \"Invalid Merkle Proof\"
-   **Function**: `verifyTransactionInclusion`{.interpreted-text
    role="ref"}
-   **Cause**: Exception raised in `verifyTransactionInclusion` when the
    Merkle proof is malformed.

`ERR_FORK_ID_NOT_FOUND`

-   **Message**: \"Fork ID not found for specified block hash\"
-   **Function**: `getForkIdByBlockHash`{.interpreted-text role="ref"}
-   **Cause**: Return this error if there exists no `forkId` for the
    given `blockHash`.

`ERR_NO_DATA`

-   **Message**: \"BTC-Relay has a NO\_DATA failure and the requested
    block cannot be verified reliably\"
-   **Function**: `verifyTransactionInclusion`{.interpreted-text
    role="ref"}
-   **Cause**: The BTC Parachain has been partially deactivated for all
    blocks with a higher block height than the lowest blocked flagged
    with `NO_DATA_BTC_RELAY`.

`ERR_INVALID`

-   **Message**: \"BTC-Relay has detected an invalid block in the
    current main chain, and has been halted\"
-   **Function**: `verifyTransactionInclusion`{.interpreted-text
    role="ref"}
-   **Cause**: The BTC Parachain has been halted because Staked Relayers
    reported an invalid block.

`ERR_SHUTDOWN`

-   **Message**: \"BTC Parachain has shut down\"
-   **Function**: `verifyTransactionInclusion`{.interpreted-text
    role="ref"} \| `storeBlockHeader`{.interpreted-text role="ref"} \|
    `storeBlockHeader`{.interpreted-text role="ref"}
-   **Cause**: The BTC Parachain has been shutdown by a manual
    intervention of the Governance Mechanism.

`ERR_INVALID_TXID`

-   **Message**: \"Transaction hash does not match given txid\"
-   **Function**: `validateTransaction`{.interpreted-text role="ref"}
-   **Cause**: The transaction identifier (`txId`) does not match the
    actual hash of the transaction.

`ERR_INSUFFICIENT_VALUE`:

-   **Message**: \"Value of payment below requested amount\"
-   **Function**: `validateTransaction`{.interpreted-text role="ref"}
-   **Cause**: The value of the (first) *Payment UTXO* in the validated
    transaction is lower than the specified `paymentValue`.

`ERR_TX_FORMAT`:

-   **Message**: \"Transaction has incorrect format\"
-   **Function**: `validateTransaction`{.interpreted-text role="ref"}
-   **Cause**: The parsed transaction has an incorrect format (see
    `accepted_bitcoin_transaction_format`{.interpreted-text
    role="ref"}).

`ERR_WRONG_RECIPIENT`

-   **Message**: \"Incorrect recipient Bitcoin address\"
-   **Function**: `validateTransaction`{.interpreted-text role="ref"}
-   **Cause**: The recipient specified in the (first) *Payment UTXO* of
    the validated transaction does not match the specified
    `recipientBtcAddress`.

`ERR_INVALID_OPRETURN`

-   **Message**: \"Incorrect identifier in OP\_RETURN field\"
-   **Function**: `validateTransaction`{.interpreted-text role="ref"}
-   **Cause**: The OP\_RETURN field of the (second) *Data UTXO* of the
    validated transaction does not match the specified `opReturnId`.

`ERR_INVALID_TX_VERSION`

-   **Message**: \"Invalid transaction version\"
-   **Function**: `getOutputStartIndex`{.interpreted-text role="ref"}
-   **Cause**: : The version of the given transaction is not 1 or 2. See
    [transaction format
    details](https://bitcoin.org/en/developer-reference#raw-transaction-format)
    in the Bitcoin Developer Reference.

`ERR_NOT_OP_RETURN`

-   **Message**: \"Expecting OP\_RETURN output, but got another type.\"
-   **Function**: `extractOPRETURN`{.interpreted-text role="ref"}
-   **Cause**: The given output was not an OP\_RETURN output.

`ERR_ONGOING_FORK`

-   **Message**: \"Verification disabled due to ongoing fork\"
-   **Function**: `verifyTransactionInclusion`{.interpreted-text
    role="ref"}
-   **Cause**: The `mainChain` is not at least
    `STABLE_BITCOIN_CONFIRMATIONS` ahead of the next best fork.
