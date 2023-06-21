Refund {#refund-protocol}
======

Overview
--------

The Refund module is a user failsafe mechanism. In case a user
accidentally locks more Bitcoin than the actual issue request, the
refund mechanism seeks to ensure that either (1) the initial issue
request is increased to issue more interBTC or (2) the BTC are returned
to the sending user.

### Step-by-step

If a user falsely sends additional BTC (i.e.,
$|\text{BTC}| > |\text{interBTC}|$) during the issue process:

1.  

    **Case 1: The originally selected vault has sufficient collateral locked to cover the entire BTC amount sent by the user**:

    :   a.  Increase the issue request interBTC amount and the fee to
            reflect the actual BTC amount paid by the user.
        b.  As before, issue the interBTC to the user and forward the
            fees.
        c.  Emit an event that the issue amount was increased.

2.  

    **Case 2: The originally selected vault does NOT have sufficient collateral locked to cover the additional BTC amount sent by the user**:

    :   a.  Automatically create a return request from the issue module
            that includes a return fee (deducted from the originial BTC
            payment) paid to the vault returning the BTC.
        b.  The vault fulfills the return request via a transaction
            inclusion proof (similar to execute issue). However, this
            does not create new interBTC.

::: {.note}
::: {.title}
Note
:::

Only case 2 is handled in this module. Case 1 is handled directly by the
issue module.
:::

::: {.note}
::: {.title}
Note
:::

Normally, enforcing actions by a vault is achieved by locking collateral
of the vault and slashing the vault in case of misbehavior. In the case
where a user sends too many BTC and the vault does not have enough
"free" collateral left, we cannot lock more collateral. However, the
original vault cannot move the additional BTC sent as this would be
flagged as theft and the vault would get slashed. The vault can possibly
take the overpaid BTC though if the vault would not be backing any
interBTC any longer (e.g. due to redeem/replace).
:::

### Security

-   Unique identification of Bitcoin payments:
    `op-return`{.interpreted-text role="ref"}

Data Model
----------

### Scalars

#### RefundBtcDustValue {#refundBtcDustValue}

The minimum amount of BTC that is required for refund requests; lower
values would risk the rejection of payment on Bitcoin.

### Maps

#### RefundRequests {#refundRequests}

Overpaid issue payments create refund requests to return BTC. This
mapping provides access from a unique hash `RefundId` to a `Refund`
struct. `<RefundId, Refund>`.

### Structs

#### Refund

Stores the status and information about a single refund request.

::: {.tabularcolumns}
l
:::

  Parameter         Type         Description
  ----------------- ------------ --------------------------------------------------------
  `vault`           AccountId    The account of the Vault responsible for this request.
  `amountWrapped`   interBTC     Amount of interBTC to be refunded.
  `fee`             interBTC     Fee charged to the user for refunding.
  `amountBtc`       interBTC     Total amount that was overpaid.
  `issuer`          AccountId    Account that overpaid on issue.
  `btcAddress`      BtcAddress   User\'s Bitcoin address.
  `issueId`         H256         The id of the issue request.
  `completed`       bool         True if the refund was processed successfully.

External Functions
------------------

### executeRefund {#executeRefund}

This function finalizes a refund, also referred to as a user failsafe.
It is typically called by the vault client that performed the refund.

#### Specification

*Function Signature*

`executeRefund(caller, refundId, merkleProof, rawTx)`

*Parameters*

-   `caller`: address of the user finalizing the refund. Typically the
    vault client that performed the refund.
-   `refundId`: the unique hash created during the internal
    `requestRefund` function.
-   `rawMerkleProof`: raw Merkle tree path (concatenated LE SHA256
    hashes).
-   `rawTx`: raw Bitcoin transaction of the refund payment, including
    the transaction inputs and outputs.

*Events*

-   `executeRefundEvent`{.interpreted-text role="ref"}

*Preconditions*

-   The function call MUST be signed by *someone*, i.e., not necessarily
    the Vault that performed the refund.
-   The BTC Parachain status in the `security`{.interpreted-text
    role="ref"} component MUST NOT be set to `SHUTDOWN:2`.
-   A *pending* `RefundRequest` MUST exist with an id equal to
    `refundId`.
-   `refundRequest.completed` MUST be `false`.
-   The `rawTx` MUST decode to a valid transaction that transfers the
    amount specified in the `RefundRequest` struct. It MUST be a
    transaction to the correct address, and provide the expected
    OP\_RETURN, based on the `RefundRequest`.
-   The `rawMerkleProof` MUST be valid and prove inclusion to the main
    chain.
-   The `vault.status` MUST be `active`.
-   The refunding vault MUST have enough collateral to mint an amount
    equal to the refund fee.

*Postconditions*

-   The `vault.issuedTokens` MUST increase by `fee`.
-   The vault\'s free balance in wrapped currency MUST increase by
    `fee`.
-   `refundRequest.completed` MUST be `true`.

Internal Functions
------------------

### requestRefund {#requestRefund}

Used to request a refund if too much BTC was sent to a Vault by mistake.

#### Specification

*Function Signature*

`requestRefund(amount, vault, issuer, btcAddress, issueId)`

*Parameters*

-   `amount`: the amount that the user has overpaid.
-   `vault`: id of the vault the issue was made to.
-   `issuer`: id of the user that made the issue request.
-   `btcAddress`: the btc address that should receive the refund.
-   `issueId`: corresponding issue request which was overpaid.

*Events*

-   `requestRefundEvent`{.interpreted-text role="ref"}

*Preconditions*

-   The function call MUST only be called by
    `executeIssue`{.interpreted-text role="ref"}.
-   The BTC Parachain status in the `security`{.interpreted-text
    role="ref"} component MUST NOT be set to `SHUTDOWN:2`.
-   The `amount - fee` MUST be greater than or equal to
    `refundBtcDustValue`{.interpreted-text role="ref"}.
-   A new unique `refundId` MUST be generated via the
    `generateSecureId`{.interpreted-text role="ref"} function.

*Postconditions*

-   The new refund request MUST be created as follows:

    > -   `refund.vault`: MUST be the `vault`.
    > -   `refund.amountWrapped`: MUST be the `amount - fee`
    > -   `refund.fee`: MUST equal `amount` multiplied by
    >     `refundFee`{.interpreted-text role="ref"}.
    > -   `refund.amountBtc`: MUST be the `amount`.
    > -   `refund.issuer`: MUST be the `issuer`.
    > -   `refund.btcAddress`: MUST be the `btcAddress`.
    > -   `refund.issueId`: MUST be the `issueId`.
    > -   `refund.completed`: MUST be false.

-   The new refund request MUST be inserted into
    `refundRequests`{.interpreted-text role="ref"} using the generated
    `refundId` as the key.

Events
------

### RequestRefund {#requestRefundEvent}

*Event Signature*

`RequestRefund(refundId, issuer, amount, vault, btcAddress, issueId, fee)`

*Parameters*

-   `refundId`: A unique hash created via
    `generateSecureId`{.interpreted-text role="ref"}.
-   `issuer`: The user\'s account identifier.
-   `amount`: The amount of interBTC overpaid.
-   `vault`: The address of the Vault involved in this refund request.
-   `issueId`: The unique hash created during
    `requestIssue`{.interpreted-text role="ref"}.
-   `fee`: The amount of interBTC to mint as fees.

### ExecuteRefund {#executeRefundEvent}

*Event Signature*

`ExecuteRefund(refundId, issuer, vault, amount, fee)`

*Parameters*

-   `refundId`: The unique hash created during via
    `` `requestRefund ``{.interpreted-text role="ref"}\`.
-   `issuer`: The user\'s account identifier.
-   `vault`: The address of the Vault involved in this refund request.
-   `amount`: The amount of interBTC refunded.
-   `fee`: The amount of interBTC to mint as fees.
