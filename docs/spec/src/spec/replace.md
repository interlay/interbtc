Replace {#replace-protocol}
=======

Overview
--------

The Replace module allows a Vault (*oldVault*) to be replaced by
transferring the BTC it is holding locked to another Vault (*newVault*)
which provides the necessary DOT collateral. The DOT collateral of the
*oldVault*, corresponding to the amount of replaced BTC, is then
unlocked. The *oldVault* must provide griefing collateral for spam
protection which is paid to *newVault* on failure.

The *oldVault* is responsible for ensuring that it has sufficient BTC to
pay for the transaction fees.

Conceptually, the Replace protocol resembles a SPV atomic cross-chain
swap.

### Step-by-Step

1.  Precondition: a Vault (*oldVault*) has locked DOT collateral in the
    `vault-registry`{.interpreted-text role="ref"} and has issued
    interBTC tokens - i.e., holds BTC on Bitcoin.
2.  *oldVault* submits a replace request, indicating how much BTC is to
    be migrated by calling the `requestReplace`{.interpreted-text
    role="ref"} function.
    -   *oldVault* is required to lock some amount of DOT collateral
        (`replaceGriefingCollateral`{.interpreted-text role="ref"}) as
        griefing protection, to prevent *oldVault* from holding
        *newVault*\'s DOT collateral locked in the BTC Parachain without
        ever finalizing the redeem protocol (transfer of BTC).
3.  Optional: *oldVault* can withdraw the request by calling the
    `withdrawReplace`{.interpreted-text role="ref"} function with a
    specified amount. For example, if *oldVault* requested a replacement
    for 10 tokens, and 2 tokens have been accepted by some *newVault*,
    then it can withdraw up to 8 tokens from being replaced.
4.  A new candidate Vault (*newVault*), commits to accepting the
    replacement by locking up the necessary DOT collateral to back the
    to-be-transferred BTC (according to the
    `secureCollateralThreshold`{.interpreted-text role="ref"}) by
    calling the `acceptReplace`{.interpreted-text role="ref"} function.
    -   Note: from the *oldVault*\'s perspective a redeem is very
        similar to an accepted replace. That is, its goal is to get rid
        of tokens, and it is not important if this is achieved by a user
        redeeming, or by a Vault accepting the replace request. As such,
        when a user requests a redeem with a Vault that has requested a
        replacement, the *oldVault*\'s `toBeReplacedTokens` is decreased
        by the amount of tokens redeemed by the user.
5.  Within a pre-defined delay, *oldVault* must release the BTC on
    Bitcoin to *newVault*\'s BTC address, and submit a valid transaction
    inclusion proof by calling the `executeReplace`{.interpreted-text
    role="ref"} function (call to `verifyTransactionInclusion` in
    `btc_relay`{.interpreted-text role="ref"}). If *oldVault* releases
    the BTC to *newVault* correctly and submits the transaction
    inclusion proof to Replace module on time, *oldVault*\'s DOT
    collateral is released - *newVault* has now replaced *oldVault*.
    -   Note: as with redeems, to prevent *oldVault* from trying to
        re-use old transactions (or other payments to *newVaults* on
        Bitcoin) as fake proofs, we require *oldVault* to include a
        `nonce` in an OP\_RETURN output of the transfer transaction on
        Bitcoin.
6.  Optional: If *oldVault* fails to provide the correct transaction
    inclusion proof on time, the *newVault*\'s `collateral` is unlocked
    and *oldVault*\'s `griefingCollateral` is sent to the *newVault* as
    reimbursement for the opportunity costs of locking up DOT collateral
    via the `cancelReplace`{.interpreted-text role="ref"}.

### Security

-   Unique identification of Bitcoin payments:
    `op-return`{.interpreted-text role="ref"}

### Vault Registry

The data access and state changes to the
`vault-registry`{.interpreted-text role="ref"} are documented in
`fig-vault-registry-replace`{.interpreted-text role="numref"} below.

> The replace module interacts with functions in the Vault-Registry to
> handle updating token balances of vaults. The green lines indicate an
> increase, the red lines a decrease.

### Fee Model

-   If a replace request is cancelled, the griefing collateral is
    transferred to the *newVault*.
-   If a replace request is executed, the griefing collateral is
    transferred to the *oldVault*.

Data Model
----------

### Scalars

#### ReplaceBtcDustValue

The minimum amount a *newVault* can accept - this is to ensure the
*oldVault* is able to make the Bitcoin transfer. Furthermore, it puts a
limit on the transaction fees that the *oldVault* needs to pay.

#### ReplacePeriod {#ReplacePeriod}

The time difference between a replace request is accepted by another
Vault and the transfer of BTC (and submission of the transaction
inclusion proof) by the to-be-replaced Vault. Concretely, this period is
the amount by which `activeBlockCount`{.interpreted-text role="ref"} is
allowed to increase before the redeem is considered to be expired. The
replace period has an upper limit to prevent griefing of Vault
collateral. Each accepted replace request records the value of this
field upon creation, and when checking the expiry, the maximum of the
current ReplacePeriod and the value as recorded in the ReplaceRequest is
used. This way, vaults are not negatively impacted by a change in the
value.

### Maps

#### ReplaceRequests

Vaults create replace requests if they want to have (a part of) their
DOT collateral to be replaced by other Vaults. This mapping provides
access from a unique hash `ReplaceId` to a `ReplaceRequest` struct.
`<ReplaceId, Replace>`.

### Structs

#### Replace

Stores the status and information about a single replace request.

::: {.tabularcolumns}
l
:::

  Parameter              Type          Description
  ---------------------- ------------- -----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
  `oldVault`             AccountId     Account of the *oldVault* that is to be replaced.
  `newVault`             AccountId     Account of the *newVault*, which accepts the replace request.
  `amount`               interBTC      Amount of BTC / interBTC to be replaced.
  `griefingCollateral`   DOT           Griefing protection collateral locked by *oldVault*.
  `collateral`           DOT           Collateral locked by the new Vault.
  `acceptTime`           BlockNumber   The `activeBlockCount`{.interpreted-text role="ref"} when the replace request was accepted by a new Vault. Serves as start for the countdown until when the old Vault must transfer the BTC.
  `period`               BlockNumber   Value of `ReplacePeriod`{.interpreted-text role="ref"} when the redeem request was made, in case that value changes while this replace is pending.
  `btcAddress`           BtcAddress    Vault\'s Bitcoin payment address.
  `btcHeight`            u32           Height of newest bitcoin block in the relay at the time the request is accepted. This is used by the clients upon startup, to determine how many blocks of the bitcoin chain they need to inspect to know if a payment has been made already.
  `status`               Enum          Status of the request: Pending, Completed or Cancelled

Functions
---------

### requestReplace {#requestReplace}

The *oldVault* (to-be-replaced) submits a request to be (partially)
replaced. If it requests more than it can fulfill (i.e. the sum of
`toBeReplacedTokens` and `toBeRedeemedTokens` exceeds its
`issuedTokens`), then the request amount is reduced such that the sum of
`toBeReplacedTokens` and `toBeRedeemedTokens` is exactly equal to
`issuedTokens`.

#### Specification

*Function Signature*

`requestReplace(oldVault, btcAmount, griefingCollateral)`

*Parameters*

-   `oldVault`: Account identifier of the Vault to be replaced (as
    tracked in `Vaults` in `vault-registry`{.interpreted-text
    role="ref"}).
-   `btcAmount`: Integer amount of BTC / interBTC to be replaced.
-   `griefingCollateral`: collateral locked by the *oldVault* as
    griefing protection.

*Events*

-   `requestReplaceEvent`{.interpreted-text role="ref"}

*Preconditions*

-   The function call MUST be signed by *oldVault*.
-   The BTC Parachain status in the `security`{.interpreted-text
    role="ref"} component MUST be set to `RUNNING:0`.
-   The *oldVault* MUST be registered.
-   The *oldVault* MUST NOT be banned.
-   The *oldVault* MUST NOT be nominated (if
    `vault_nomination`{.interpreted-text role="ref"} is enabled).
-   If the `btcAmount` is greater than the Vault\'s
    `replacableTokens = issuedTokens - toBeRedeemTokens - toBeReplaceTokens`,
    set the `btcAmount` to the `replaceableTokens` amount.
-   The *oldVault* MUST provide sufficient `griefingCollateral` such
    that the ratio of all of its `toBeReplacedTokens` and
    `replaceCollateral` is above
    `replaceGriefingCollateral`{.interpreted-text role="ref"}.
-   The *oldVault* MUST request sufficient `btcAmount` to be replaced
    such that its total is above `ReplaceBtcDustValue`.

*Postconditions*

-   The *oldVault*\'s `toBeReplacedTokens` MUST be increased by
    `tokenIncrease = min(btcAmount, vault.toBeIssuedTokens - vault.toBeRedeemedTokens)`.
-   An amount of `griefingCollateral * (tokenIncrease / btcAmount)` MUST
    be locked in the `griefingCurrency`{.interpreted-text role="ref"} by
    the *oldVault* in this transaction.
-   The *oldVault*\'s `replaceCollateral` MUST be increased by the
    amount of collateral locked in this transaction.

### withdrawReplace {#withdrawReplace}

The *oldVault* decreases its `toBeReplacedTokens`.

#### Specification

*Function Signature*

`withdrawReplace(oldVault, tokens)`

*Parameters*

-   `oldVault`: Account identifier of the Vault withdrawing it\'s
    replace request (as tracked in `Vaults` in
    `vault-registry`{.interpreted-text role="ref"})
-   `tokens`: The amount of `toBeReplacedTokens` to withdraw.

*Events*

-   `withdrawReplaceEvent`{.interpreted-text role="ref"}

*Preconditions*

-   The function call MUST be signed by *oldVault*.
-   The BTC Parachain status in the `security`{.interpreted-text
    role="ref"} component MUST NOT be set to `SHUTDOWN: 2`.
-   The *oldVault* MUST be registered.
-   The *oldVault* MUST have a non-zero amount of `toBeReplacedTokens`.

*Postconditions*

-   The *oldVault*\'s `toBeReplacedTokens` MUST decrease by an amount of
    `tokenDecrease = min(toBeReplacedTokens, tokens)`
-   The *oldVault*\'s `replaceCollateral` MUST decreased by the amount
    `releasedCollateral = replaceCollateral * (tokenDecrease / toBeReplacedTokens)`.
-   The *oldVault*\'s `releasedCollateral` MUST be unlocked.

### acceptReplace {#acceptReplace}

A *newVault* accepts an existing replace request. It can optionally lock
additional DOT collateral specifically for this replace. If the replace
is cancelled, this amount will be unlocked again.

#### Specification

*Function Signature*

`acceptReplace(oldVault, newVault, btcAmount, collateral, btcAddress)`

*Parameters*

-   `oldVault`: Account identifier of the *oldVault* who requested
    replacement (as tracked in `Vaults` in
    `vault-registry`{.interpreted-text role="ref"}).
-   `newVault`: Account identifier of the *newVault* accepting the
    replace request (as tracked in `Vaults` in
    `vault-registry`{.interpreted-text role="ref"}).
-   `replaceId`: The identifier of the replace request in
    `ReplaceRequests`.
-   `collateral`: DOT collateral provided to match the replace request
    (i.e., for backing the locked BTC). Can be more than the necessary
    amount.
-   `btcAddress`: The *newVault*\'s Bitcoin payment address for
    transaction verification.

*Events*

-   `acceptReplaceEvent`{.interpreted-text role="ref"}

*Preconditions*

-   The function call MUST be signed by *newVault*.
-   The BTC Parachain status in the `security`{.interpreted-text
    role="ref"} component MUST NOT be set to `SHUTDOWN: 2`.
-   The *oldVault* and *newVault* MUST be registered.
-   The *oldVault* MUST NOT be equal to *newVault*.
-   The *newVault* MUST NOT be banned.
-   The *newVault*\'s free balance MUST be enough to lock `collateral`.
-   The *newVault* MUST have lock sufficient collateral to remain above
    the `SecureCollateralThreshold`{.interpreted-text role="ref"} after
    accepting `btcAmount`.
-   The *newVault*\'s `btcAddress` MUST NOT be registered.
-   The replaced tokens MUST be at least`ReplaceBtcDustValue`.

*Postconditions*

The actual amount of replaced tokens is calculated to be
`redeemableTokens = min(oldVault.toBeReplacedTokens, btcAmount)`. The
amount of griefingCollateral used is
`consumedGriefingCollateral = oldVault.replaceCollateral * (redeemableTokens / oldVault.toBeReplacedTokens)`.

-   The *oldVault*\'s `replaceCollateral` MUST be decreased by
    `consumedGriefingCollateral`.
-   The *oldVault*\'s `toBeReplacedTokens` MUST be decreased by
    `redeemableTokens`.
-   The *oldVault*\'s `toBeRedeemedTokens` MUST be increased by
    `redeemableTokens`.
-   The *newVault*\'s `toBeIssuedTokens` MUST be increased by
    `redeemableTokens`.
-   The *newVault* locks additional collateral; its `backingCollateral`
    MUST be increased by
    `collateral * (redeemableTokens / oldVault.toBeReplacedTokens)`.
-   A unique [replaceId]{.title-ref} must be generated from
    `generateSecureId`{.interpreted-text role="ref"}.
-   A new `ReplaceRequest` MUST be added to the replace request mapping
    at the [replaceId]{.title-ref} key.
    -   `oldVault`: MUST be the `oldVault`.
    -   `newVault`: MUST be the `newVault`.
    -   `amount`: MUST be `redeemableTokens`.
    -   `griefingCollateral`: MUST be `consumedGriefingCollateral`
    -   `collateral`: MUST be `collateral`.
    -   `accept_time`: MUST be the current active block number.
    -   `period`: MUST be the current `ReplacePeriod`.
    -   `btcAddress`: MUST be the `btcAddress` argument.
    -   `btcHeight`: UST be the current height of the btc-relay.
    -   `status`: MUST be `pending`.

### executeReplace {#executeReplace}

The to-be-replaced Vault finalizes the replace process by submitting a
proof that it transferred the correct amount of BTC to the BTC address
of the new Vault, as specified in the `ReplaceRequest`. This function
calls *verifyAndValidateTransaction* in `btc_relay`{.interpreted-text
role="ref"}.

#### Specification

*Function Signature*

`executeReplace(replaceId, rawMerkleProof, rawTx)`

*Parameters*

-   `replaceId`: The identifier of the replace request in
    `ReplaceRequests`.
-   `rawMerkleProof`: Raw Merkle tree path (concatenated LE SHA256
    hashes).
-   `rawTx`: Raw Bitcoin transaction including the transaction inputs
    and outputs.

*Events*

-   `executeReplaceEvent`{.interpreted-text role="ref"}

*Preconditions*

-   The BTC Parachain status in the `security`{.interpreted-text
    role="ref"} component MUST NOT be set to `SHUTDOWN:2`.
-   Both *oldVault* and *newVault* (as specified in the request) MUST be
    registered in the `vault-registry`{.interpreted-text role="ref"}.
-   A pending `ReplaceRequest` MUST exist with id `replaceId`.
-   The request MUST NOT have expired.
-   The `rawTx` MUST decode to a valid transaction that transfers at
    least the amount specified in the `ReplaceRequest` struct. It MUST
    be a transaction to the correct address, and provide the expected
    OP\_RETURN, based on the `replaceId`.
-   The `rawMerkleProof` MUST contain a valid proof of of `rawTx`.
-   The Bitcoin payment MUST have been submitted to the relay chain, and
    MUST have sufficient confirmations.

*Postconditions*

-   The `replaceTokens`{.interpreted-text role="ref"} function in the
    `vault-registry`{.interpreted-text role="ref"} MUST have been
    called, providing the `oldVault`, `newVault`,
    `replaceRequest.amount`, and `replaceRequest.collateral` as
    arguments.
-   The griefing collateral as specifified in the `ReplaceRequest` MUST
    be released back to *oldVault*\'s free balance in the
    `griefingCurrency`{.interpreted-text role="ref"}.
-   The `replaceRequest.status` MUST be set to `Completed`.

### cancelReplace {#cancelReplace}

If a replace request is not executed on time, the replace can be
cancelled by the *newVault*. Since the *newVault* provided additional
collateral in vain, it can claim the *oldVault*\'s griefing collateral.

#### Specification

*Function Signature*

`cancelReplace(newVault, replaceId)`

*Parameters*

-   `newVault`: Account identifier of the Vault accepting the replace
    request (as tracked in `Vaults` in
    `vault-registry`{.interpreted-text role="ref"}).
-   `replaceId`: The identifier of the replace request in
    `ReplaceRequests`.

*Events*

-   `cancelReplaceEvent`{.interpreted-text role="ref"}

*Preconditions*

-   The BTC Parachain status in the `security`{.interpreted-text
    role="ref"} component MUST NOT be set to `SHUTDOWN:2`.
-   Both *oldVault* and *newVault* (as specified in the request) MUST be
    registered in the `vault-registry`{.interpreted-text role="ref"}.
-   A pending `ReplaceRequest` MUST exist with id `replaceId`.
-   The `newVault` MUST be equal to the *newVault* specified in the
    `ReplaceRequest`.
-   The request MUST have expired.

*Postconditions*

-   The `cancelReplaceTokens`{.interpreted-text role="ref"} function in
    the `vault-registry`{.interpreted-text role="ref"} MUST have been
    called, providing the `oldVault`, `newVault`,
    `replaceRequest.amount`, and `replaceRequest.amount` as arguments.

-   If *newVault* IS NOT liquidated:

    > -   If unlocking `replaceRequest.collateral` does not put the
    >     collateralization rate of the *newVault* below
    >     `SecureCollateralThreshold`, the collateral MUST be unlocked
    >     and its `backingCollateral` MUST decrease by the same amount.

-   The griefing collateral MUST BE slashed from the *oldVault* to the
    *newVault*\'s free balance.

-   The `replaceRequest.status` MUST be set to `Cancelled`.

Events
------

### RequestReplace {#requestReplaceEvent}

Emit an event when a replace request is made by an *oldVault*.

*Event Signature* \* `RequestReplace(oldVault, btcAmount, replaceId)`

*Parameters*

-   `oldVault`: Account identifier of the Vault to be replaced (as
    tracked in `Vaults` in `vault-registry`{.interpreted-text
    role="ref"}).
-   `btcAmount`: Integer amount of BTC / interBTC to be replaced.
-   `replaceId`: The unique identified of a replace request.

*Functions*

-   `requestReplace`{.interpreted-text role="ref"}

### WithdrawReplace {#withdrawReplaceEvent}

Emits an event stating that a Vault (*oldVault*) has withdrawn some
amount of `toBeReplacedTokens`.

*Event Signature*

`WithdrawReplace(oldVault, withdrawnTokens, withdrawnGriefingCollateral)`

*Parameters*

-   `oldVault`: Account identifier of the Vault requesting the replace
    (as tracked in `Vaults` in `vault-registry`{.interpreted-text
    role="ref"})
-   `withdrawnTokens`: The amount by which `toBeReplacedTokens` has
    decreased.
-   `withdrawnGriefingCollateral`: The amount of griefing collateral
    unlocked.

*Functions*

-   ref:[withdrawReplace]{.title-ref}

### AcceptReplace {#acceptReplaceEvent}

Emits an event stating which Vault (*newVault*) has accepted the
`ReplaceRequest` request (`requestId`), and how much collateral in DOT
it provided (`collateral`).

*Event Signature*

`AcceptReplace(replaceId, oldVault, newVault, btcAmount, collateral, btcAddress)`

*Parameters*

-   `replaceId`: The identifier of the replace request in
    `ReplaceRequests`.
-   `oldVault`: Account identifier of the Vault being replaced (as
    tracked in `Vaults` in `vault-registry`{.interpreted-text
    role="ref"})
-   `newVault`: Account identifier of the Vault that accepted the
    replace request (as tracked in `Vaults` in
    `vault-registry`{.interpreted-text role="ref"})
-   `btcAmount`: Amount of tokens the *newVault* just accepted.
-   `collateral`: Amount of collateral the *newVault* locked for this
    replace.
-   `btcAddress`: The address that *oldVault* should transfer the btc
    to.

*Functions*

-   ref:[acceptReplace]{.title-ref}

### ExecuteReplace {#executeReplaceEvent}

Emits an event stating that the old Vault (*oldVault*) has executed the
BTC transfer to the new Vault (*newVault*), finalizing the
`ReplaceRequest` request (`requestId`).

*Event Signature*

`ExecuteReplace(oldVault, newVault, replaceId)`

*Parameters*

-   `oldVault`: Account identifier of the Vault being replaced (as
    tracked in `Vaults` in `vault-registry`{.interpreted-text
    role="ref"})
-   `newVault`: Account identifier of the Vault that accepted the
    replace request (as tracked in `Vaults` in
    `vault-registry`{.interpreted-text role="ref"})
-   `replaceId`: The identifier of the replace request in
    `ReplaceRequests`.

*Functions*

-   ref:[executeReplace]{.title-ref}

### CancelReplace {#cancelReplaceEvent}

Emits an event stating that the old Vault (*oldVault*) has not completed
the replace request, that the new Vault (*newVault*) cancelled the
`ReplaceRequest` request (`requestId`), and that `slashedCollateral` has
been slashed from *oldVault* to *newVault*.

*Event Signature*

`CancelReplace(replaceId, newVault, oldVault, slashedCollateral)`

*Parameters*

-   `replaceId`: The identifier of the replace request in
    `ReplaceRequests`.
-   `oldVault`: Account identifier of the Vault being replaced (as
    tracked in `Vaults` in `vault-registry`{.interpreted-text
    role="ref"})
-   `newVault`: Account identifier of the Vault that accepted the
    replace request (as tracked in `Vaults` in
    `vault-registry`{.interpreted-text role="ref"})
-   `slashedCollateral`: Amount of griefingCollateral slashed to
    *newVault*.

*Functions*

-   ref:[cancelReplace]{.title-ref}

Error Codes
-----------

`ERR_UNAUTHORIZED`

-   **Message**: \"Unauthorized: Caller must be *newVault*.\"
-   **Function**: `cancelReplace`{.interpreted-text role="ref"}
-   **Cause**: The caller of this function is not the associated
    *newVault*, and hence not authorized to take this action.

`ERR_INSUFFICIENT_COLLATERAL`

-   **Message**: \"The provided collateral is too low.\"
-   **Function**: `requestReplace`{.interpreted-text role="ref"}
-   **Cause**: The provided collateral is insufficient to match the
    amount of tokens requested for replacement.

`ERR_REPLACE_PERIOD_EXPIRED`

-   **Message**: \"The replace period expired.\"
-   **Function**: `executeReplace`{.interpreted-text role="ref"}
-   **Cause**: The time limit as defined by the `ReplacePeriod` is not
    met.

`ERR_REPLACE_PERIOD_NOT_EXPIRED`

-   **Message**: \"The period to complete the replace request is not yet
    expired.\"
-   **Function**: `cancelReplace`{.interpreted-text role="ref"}
-   **Cause**: A Vault tried to cancel a replace before it expired.

`ERR_AMOUNT_BELOW_BTC_DUST_VALUE`

-   **Message**: \"To be replaced amount is too small.\"
-   **Function**: `requestReplace`{.interpreted-text role="ref"},
    `acceptReplace`{.interpreted-text role="ref"}
-   **Cause**: The Vault requests or accepts an insufficient number of
    tokens.

`ERR_NO_PENDING_REQUEST`

-   **Message**: \"Could not withdraw to-be-replaced tokens because it
    was already zero.\"
-   **Function**: `requestReplace`{.interpreted-text role="ref"} \|
    `acceptReplace`{.interpreted-text role="ref"}
-   **Cause**: The Vault requests or accepts an insufficient number of
    tokens.

`ERR_REPLACE_SELF_NOT_ALLOWED`

-   **Message**: \"Vaults can not accept replace request created by
    themselves.\"
-   **Function**: `acceptReplace`{.interpreted-text role="ref"}
-   **Cause**: A Vault tried to accept a replace that it itself had
    created.

`ERR_REPLACE_COMPLETED`

-   **Message**: \"Request is already completed.\"
-   **Function**: `executeReplace`{.interpreted-text role="ref"} \|
    `cancelReplace`{.interpreted-text role="ref"}
-   **Cause**: A Vault tried to operate on a request that already
    completed.

`ERR_REPLACE_CANCELLED`

-   **Message**: \"Request is already cancelled.\"
-   **Function**: `executeReplace`{.interpreted-text role="ref"} \|
    `cancelReplace`{.interpreted-text role="ref"}
-   **Cause**: A Vault tried to operate on a request that already
    cancelled.

`ERR_REPLACE_ID_NOT_FOUND`

-   **Message**: \"Invalid replace ID\"
-   **Function**: `executeReplace`{.interpreted-text role="ref"} \|
    `cancelReplace`{.interpreted-text role="ref"}
-   **Cause**: An invalid replaceID was given - it is not found in the
    `ReplaceRequests` map.

`ERR_VAULT_NOT_FOUND`

-   **Message**: \"The `Vault` cannot be found.\"
-   **Function**: `requestReplace`{.interpreted-text role="ref"} \|
    `acceptReplace`{.interpreted-text role="ref"} \|
    `cancelReplace`{.interpreted-text role="ref"}
-   **Cause**: The Vault was not found in the existing `Vaults` list in
    `VaultRegistry`.

::: {.note}
::: {.title}
Note
:::

It is possible that functions in this pallet return errors defined in
other pallets.
:::
