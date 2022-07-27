Redeem {#redeem-protocol}
======

Overview
--------

The redeem module allows a user to receive BTC on the Bitcoin chain in
return for destroying an equivalent amount of interBTC on the BTC
Parachain. The process is initiated by a user requesting a redeem with a
vault. The vault then needs to send BTC to the user within a given time
limit. Next, the vault has to finalize the process by providing a proof
to the BTC Parachain that they have send the right amount of BTC to the
user. If the vault fails to deliver a valid proof within the time limit,
the user can claim an equivalent amount of DOT from the vault\'s locked
collateral to reimburse him for his loss in BTC.

Moreover, as part of the liquidation procedure, users are able to
directly exchange interBTC for DOT. To this end, a user is able to
execute a special liquidation redeem if one or multiple vaults have been
liquidated.

### Step-by-step

1.  Precondition: A user owns interBTC.
2.  A user locks an amount of interBTC by calling the
    `requestRedeem`{.interpreted-text role="ref"} function. In this
    function call, the user selects a vault to execute the redeem
    request from the list of vaults. The function creates a redeem
    request with a unique hash.
3.  The selected vault listens for the `RequestRedeem` event emitted by
    the user. The vault then proceeds to transfer BTC to the address
    specified by the user in the `requestRedeem`{.interpreted-text
    role="ref"} function including a unique hash in the `OP_RETURN` of
    one output.
4.  The vault executes the `executeRedeem`{.interpreted-text role="ref"}
    function by providing the Bitcoin transaction from step 3 together
    with the redeem request identifier within the time limit. If the
    function completes successfully, the locked interBTC are destroyed
    and the user received its BTC.
5.  Optional: If the user could not receive BTC within the given time
    (as required in step 4), the user calls
    `cancelRedeem`{.interpreted-text role="ref"} after the redeem time
    limit. The user can choose either to reimburse, or to retry. In case
    of reimbursement, the user transfer ownership of the tokens to the
    vault, but receives collateral in exchange. In case of retry, the
    user gets back its tokens. In either case, the user is given some
    part of the vault\'s collateral as compensation for the
    inconvenience.
    a.  Optional: If during a `cancelRedeem`{.interpreted-text
        role="ref"} the user selects reimbursement, and as a result the
        vault becomes undercollateralized, then vault does not receive
        the user\'s tokens - they are burned, and the vault\'s
        `issuedTokens` decreases. When, at some later point, it gets
        sufficient colalteral, it can call
        `mintTokensForReimbursedRedeem`{.interpreted-text role="ref"} to
        get the tokens.

### Security

-   Unique identification of Bitcoin payments:
    `op-return`{.interpreted-text role="ref"}

### Vault Registry

The data access and state changes to the vault registry are documented
in `fig-vault-registry-redeem`{.interpreted-text role="numref"} below.

> The redeem module interacts through three different functions with the
> vault registry. The green arrow indicate an increase, the red arrows a
> decrease.

### Fee Model

When the user makes a redeem request for a certain amount, it will
actually not receive that amount of BTC. This is because there are two
types of fees subtracted. First, in order to be able to pay the bitcoin
transaction cost, the vault is given a budget to spend on on the bitcoin
inclusion fee, based on `RedeemTransactionSize`{.interpreted-text
role="ref"} and the inclusion fee estimates reported by the oracle. The
actual amount spent on the inclusion fee is not checked. If the vault
does not spend the whole budget, it can keep the surplus, although it
will not be able to spend it without being liquidated for theft. It may
at some point want to withdraw all of its collateral and then to move
its bitcoin into a new account. The second fee that the user pays for is
the parachain fee that goes to the fee pool to incentivize the various
participants in the system.

The main accounting changes of a successful redeem is summarized below.
See the individual functions for more details.

> -   `redeem.amountBTC` bitcoin is transferred to the user.
> -   `redeem.amountBTC + redeem.fee + redeem.transferFeeBTC` is burned
>     from the user.
> -   The vault\'s `issuedTokens` decreases by
>     `redeem.amountBTC + redeem.transferFeeBTC`.
> -   The fee pool content increases by `redeem.fee` (if non-zero).
> -   If the vault self-redeems (the redeemer is the vault ID) no fee is
>     paid.

Data Model
----------

### Scalars

#### RedeemPeriod {#RedeemPeriod}

The time difference between when an redeem request is created and
required completion time by a vault. Concretely, this period is the
amount by which `activeBlockCount`{.interpreted-text role="ref"} is
allowed to increase before the redeem is considered to be expired. The
period has an upper limit to ensure the user gets his BTC in time and to
potentially punish a vault for inactivity or stealing BTC. Each redeem
request records the value of this field upon creation, and when checking
the expiry, the maximum of the current RedeemPeriod and the value as
recorded in the RedeemRequest is used. This way, users are not
negatively impacted by a change in the value.

#### RedeemTransactionSize {#RedeemTransactionSize}

The expected size in bytes of a redeem. This is used to set the bitcoin
inclusion fee budget.

#### RedeemBtcDustValue {#RedeemBtcDustValue}

The minimal amount in BTC a vault can be asked to transfer to the user.
Note that this is not equal to the amount requests, since an inclusion
fee is deducted from that amount.

### Maps

#### RedeemRequests

Users create redeem requests to receive BTC in return for interBTC. This
mapping provides access from a unique hash `redeemId` to a `Redeem`
struct. `<redeemId, RedeemRequest>`.

### Structs

#### RedeemRequest

Stores the status and information about a single redeem request.

::: {.tabularcolumns}
l
:::

  Parameter          Type          Description
  ------------------ ------------- -----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
  `vault`            Account       The BTC Parachain address of the vault responsible for this redeem request.
  `opentime`         u32           The `activeBlockCount`{.interpreted-text role="ref"} when the redeem request was made. Serves as start for the countdown until when the vault must transfer the BTC.
  `period`           u32           Value of `RedeemPeriod`{.interpreted-text role="ref"} when the redeem request was made, in case that value changes while this redeem is pending.
  `amountBTC`        BTC           Amount of BTC to be sent to the user.
  `transferFeeBTC`   BTC           Budget for the vault to spend in bitcoin inclusion fees.
  `fee`              interBTC      Parachain fee: amount to be transferred from the user to the fee pool upon completion of the redeem.
  `premium`          DOT           Amount of DOT to be paid as a premium to this user (if the Vault\'s collateral rate was below `PremiumRedeemThreshold` at the time of redeeming).
  `redeemer`         Account       The BTC Parachain address of the user requesting the redeem.
  `btcAddress`       bytes\[20\]   Base58 encoded Bitcoin public key of the User.
  `btcHeight`        u32           Height of newest bitcoin block in the relay at the time the request is accepted. This is used by the clients upon startup, to determine how many blocks of the bitcoin chain they need to inspect to know if a payment has been made already.
  `status`           enum          The status of the redeem: `Pending`, `Completed`, `Retried` or `Reimbursed(bool)`, where bool=true indicates that the vault minted tokens for the amount that the redeemer burned

Functions
---------

### requestRedeem {#requestRedeem}

A user requests to start the redeem procedure. This function checks the
BTC Parachain status in `security`{.interpreted-text role="ref"} and
decides how the redeem process is to be executed. The following modes
are possible:

-   **Normal Redeem** - no errors detected, full BTC value is to be
    Redeemed.
-   **Premium Redeem** - the selected Vault\'s collateral rate has
    fallen below `PremiumRedeemThreshold`. Full BTC value is to be
    redeemed, but the user is allocated a premium in DOT
    (`RedeemPremiumFee`), taken from the Vault\'s to-be-released
    collateral.

#### Specification

*Function Signature*

`requestRedeem(redeemer, amountWrapped, btcAddress, vault)`

*Parameters*

-   `redeemer`: address of the user triggering the redeem.
-   `amountWrapped`: the amount of interBTC to destroy and BTC to
    receive.
-   `btcAddress`: the address to receive BTC.
-   `vault`: the vault selected for the redeem request.

*Returns*

-   `redeemId`: A unique hash identifying the redeem request.

*Events*

-   `requestRedeemEvent`{.interpreted-text role="ref"}

*Preconditions*

Let `burnedTokens` be `amountWrapped` minus the result of the
multiplication of `redeemFee`{.interpreted-text role="ref"} and
`amountWrapped`. Then:

-   The function call MUST be signed by *redeemer*.
-   The BTC Parachain status in the `security`{.interpreted-text
    role="ref"} component MUST be set to `RUNNING:0`.
-   The selected vault MUST NOT be banned.
-   The selected vault MUST NOT be liquidated.
-   The redeemer MUST have at least `amountWrapped` free tokens.
-   `burnedTokens` minus the inclusion fee MUST be above or equal to the
    `RedeemBtcDustValue`{.interpreted-text role="ref"}, where the
    inclusion fee is the multiplication of
    `RedeemTransactionSize`{.interpreted-text role="ref"} and the fee
    rate estimate reported by the oracle.
-   The vault\'s `issuedTokens` MUST be at least
    `vault.toBeRedeemedTokens + burnedTokens`.

*Postconditions*

Let `burnedTokens` be `amountWrapped` minus the result of the
multiplication of `redeemFee`{.interpreted-text role="ref"} and
`amountWrapped`. Then:

-   The vault\'s `toBeRedeemedTokens` MUST increase by `burnedTokens`.

-   `amountWrapped` of the redeemer\'s tokens MUST be locked by this
    transaction.

-   `decreaseToBeReplacedTokens`{.interpreted-text role="ref"} MUST be
    called, supplying `vault` and `burnedTokens`. The returned
    `replaceCollateral` MUST be released by this function.

-   A new `RedeemRequest` MUST be added to the `RedeemRequests` map,
    with the following value:

    > -   `redeem.vault` MUST be the requested `vault`
    > -   `redeem.opentime` MUST be the current
    >     `activeBlockCount`{.interpreted-text role="ref"}
    > -   `redeem.fee` MUST be `redeemFee`{.interpreted-text role="ref"}
    >     multiplied by `amountWrapped` if `redeemer != vault`,
    >     otherwise this should be zero.
    > -   `redeem.transferFeeBtc` MUST be the inclusion fee, which is
    >     the multiplication of
    >     `RedeemTransactionSize`{.interpreted-text role="ref"} and the
    >     fee rate estimate reported by the oracle,
    > -   `redeem.amountBtc` MUST be
    >     `amountWrapped - redeem.fee - redeem.transferFeeBtc`,
    > -   `redeem.period` MUST be the current value of the
    >     `RedeemPeriod`{.interpreted-text role="ref"},
    > -   `redeem.redeemer` MUST be the `redeemer` argument,
    > -   `redeem.btcAddress` MUST be the `btcAddress` argument,
    > -   `redeem.btcHeight` MUST be the current height of the btc
    >     relay,
    > -   `redeem.status` MUST be `Pending`,
    > -   If the vault\'s collateralization rate is above the
    >     `PremiumCollateralThreshold`{.interpreted-text role="ref"},
    >     then `redeem.premium` MUST be `0`,
    > -   If the vault\'s collateralization rate is below the
    >     `PremiumCollateralThreshold`{.interpreted-text role="ref"},
    >     then `redeem.premium` MUST be
    >     `premiumRedeemFee`{.interpreted-text role="ref"} multiplied by
    >     the worth of `redeem.amountBtc`,

### liquidationRedeem {#liquidationRedeem}

A user executes a liquidation redeem that exchanges interBTC for
collateral from the [LiquidationVault]{.title-ref}. This function takes
a `currencyId` argument that specifies which currency to the user wishes
to receive. Since each currency uses a separate liquidation vault, the
amount of collateral received depends only on the amount of tokens and
collateral in that specific liquidation vault. If the user wants to
obtain multiple currencies, they have to call this function multiple
times, possibly through off-chain aggregation via batching. Since the
1:1 backing is being recovered in this function, interBTC is burned
without releasing any BTC.

#### Specification

*Function Signature*

`liquidationRedeem(redeemer, amountWrapped, currencyId)`

*Parameters*

-   `redeemer`: address of the user triggering the redeem.
-   `amountWrapped`: the amount of interBTC to destroy.
-   `currencyId`: the currency id of the funds to be received.

*Events*

-   `liquidationRedeemEvent`{.interpreted-text role="ref"}

*Preconditions*

-   The BTC Parachain status in the `security`{.interpreted-text
    role="ref"} component MUST NOT be set to `SHUTDOWN:2`.
-   The function call MUST be signed.
-   The redeemer MUST have at least `amountWrapped` free tokens.

*Postconditions*

-   `amountWrapped` tokens MUST be burned from the user.
-   `redeemTokensLiquidation`{.interpreted-text role="ref"} MUST be
    called with `currency_id`, `redeemer` and `amountWrapped` as
    arguments.

### executeRedeem {#executeRedeem}

A vault calls this function after receiving an `RequestRedeem` event
with their public key. Before calling the function, the vault transfers
the specific amount of BTC to the BTC address given in the original
redeem request. The vault completes the redeem with this function.

#### Specification

*Function Signature*

`executeRedeem(redeemId, rawMerkleProof, rawTx)`

*Parameters*

-   `redeemId`: the unique hash created during the `requestRedeem`
    function.
-   `rawMerkleProof`: Merkle tree path (concatenated LE SHA256 hashes).
-   `rawTx`: Raw Bitcoin transaction including the transaction inputs
    and outputs.

*Events*

-   `executeRedeemEvent`{.interpreted-text role="ref"}

*Preconditions*

-   The function call MUST be signed by *someone*, i.e. not necessarily
    the *vault*.
-   The BTC Parachain status in the `security`{.interpreted-text
    role="ref"} component MUST NOT be set to `SHUTDOWN:2`.
-   A *pending* `RedeemRequest` MUST exist with an id equal to
    `redeemId`.
-   The `rawTx` MUST decode to a valid transaction that transfers
    exactly the amount specified in the `RedeemRequest` struct. It MUST
    be a transaction to the correct address, and provide the expected
    OP\_RETURN, based on the `RedeemRequest`.
-   The `rawMerkleProof` MUST contain a valid proof of of `rawTX`.
-   The bitcoin payment MUST have been submitted to the relay chain, and
    MUST have sufficient confirmations.

*Postconditions*

-   `redeemRequest.amountBtc + redeemRequest.transferFeeBtc` of the
    tokens in the redeemer\'s account MUST be burned.
-   The user\'s [lockedTokens]{.title-ref} MUST decrease by
    [redeemRequest.amountBtc +
    redeemRequest.transferFeeBtc]{.title-ref}.
-   The vault's [toBeRedeemedTokens]{.title-ref} MUST decrease by
    [redeemRequest.amountBtc +
    redeemRequest.transferFeeBtc]{.title-ref}.
-   The vault's [issuedTokens]{.title-ref} MUST decrease by
    [redeemRequest.amountBtc +
    redeemRequest.transferFeeBtc]{.title-ref}.
-   `redeemRequest.fee` MUST be unlocked and transferred from the
    redeemer\'s account to the fee pool.
-   `redeemTokens`{.interpreted-text role="ref"} MUST be called,
    supplying `redeemRequest.vault`,
    `redeemRequest.amountBtc + redeemRequest.transferFeeBtc`,
    `redeemRequest.premium` and `redeemRequest.redeemer` as arguments.
-   `redeemRequest.status` MUST be set to `Completed`.

### cancelRedeem {#cancelRedeem}

If a redeem request is not completed on time, the redeem request can be
cancelled. The user that initially requested the redeem process calls
this function to obtain the Vault\'s collateral as compensation for not
refunding the BTC back to his address.

The failed vault is banned from further issue, redeem and replace
requests for a pre-defined time period
(`punishmentDelay`{.interpreted-text role="ref"} as defined in
`vault-registry`{.interpreted-text role="ref"}).

The user is able to choose between reimbursement and retrying. If the
user chooses the retry, it gets back the tokens, and a punishment fee is
transferred from the vault to the user. If the user chooses
reimbursement, then they receive the equivalent worth of the tokens in
collateral, plus a punishment fee. In this case, the tokens are
transferred from the user to the vault. In either case, the vault may
also be slashed an additional punishment that goes to the fee pool.

The punishment fee paid to the user stays constant (i.e., the user
always receives the punishment fee of e.g. 10%).

#### Specification

*Function Signature*

`cancelRedeem(redeemer, redeemId, reimburse)`

*Parameters*

-   `redeemer`: account cancelling this redeem request.
-   `redeemId`: the unique hash of the redeem request.
-   `reimburse`: if true, user is reimbursed in collateral (slashed from
    the vault), else interBTC is returned (to retry with another vault).

*Events*

-   `cancelRedeemEvent`{.interpreted-text role="ref"}

*Preconditions*

-   The function call MUST be signed by `redeemer`.
-   The BTC Parachain status in the `security`{.interpreted-text
    role="ref"} component MUST be set to `RUNNING:0`.
-   A *pending* `RedeemRequest` MUST exist with an id equal to
    `redeemId`.
-   The `redeemer` MUST equal `redeemRequest.redeemer`.
-   The request MUST be expired.

*Postconditions*

Let `amountIncludingParachainFee` be equal to the worth in collateral of
`redeem.amountBtc + redeem.transferFeeBtc`. Let `confiscatedCollateral`
be equal to
`vault.backingCollateral * (amountIncludingParachainFee / vault.toBeRedeemedTokens)`.
Then:

-   If the vault is liquidated:

    > -   If `reimburse` is true, an amount of `confiscatedCollateral`
    >     MUST be transferred from the vault to the redeemer.
    > -   If `reimburse` is false, an amount of `confiscatedCollateral`
    >     MUST be transferred from the vault to the liquidation vault.

-   If the vault is *not* liquidated, the following collateral changes
    are made:

    > -   If `reimburse` is true, the user SHOULD be reimbursed the
    >     worth of `amountIncludingParachainFee` in collateral. The
    >     transfer MUST be saturating, i.e. if the amount is not
    >     available, it should transfer whatever amount *is* available.
    > -   A punishment fee MUST be tranferred from the vault\'s backing
    >     collateral to the redeemer: `punishmentFee`{.interpreted-text
    >     role="ref"}. The transfer MUST be saturating, i.e. if the
    >     amount is not available, it should transfer whatever amount
    >     *is* available.

-   If `reimburse` is true:

    > -   `redeem.fee` MUST be transferred from the vault to the fee
    >     pool if non-zero.
    >
    > -   If after the loss of collateral the vault is below the
    >     `SecureCollateralThreshold`{.interpreted-text role="ref"}:
    >
    >     > -   `amountIncludingParachainFee` of the user\'s tokens are
    >     >     *burned*.
    >     > -   `decreaseTokens`{.interpreted-text role="ref"} MUST be
    >     >     called, supplying the vault, the user, and
    >     >     `amountIncludingParachainFee` as arguments.
    >     > -   The `redeem.status` is set to `Reimbursed(false)`, where
    >     >     the `false` indicates that the vault has not yet
    >     >     received the tokens.
    >
    > -   If after the loss of collateral the vault remains above the
    >     `SecureCollateralThreshold`{.interpreted-text role="ref"}:
    >
    >     > -   `amountIncludingParachainFee` of the user\'s tokens MUST
    >     >     be unlocked and transferred to the vault.
    >     > -   `decreaseToBeRedeemedTokens`{.interpreted-text
    >     >     role="ref"} MUST be called, supplying the vault and
    >     >     `amountIncludingParachainFee` as arguments.
    >     > -   The `redeem.status` is set to `Reimbursed(true)`, where
    >     >     the `true` indicates that the vault has received the
    >     >     tokens.

-   If `reimburse` is false:

    > -   All the user\'s tokens that were locked in
    >     `requestRedeem`{.interpreted-text role="ref"} MUST be
    >     unlocked, i.e. an amount of
    >     `redeem.amountBtc + redeem.fee + redeem.transferFeeBtc`.
    > -   The vault\'s `toBeRedeemedTokens` MUST decrease by
    >     `amountIncludingParachainFee`.

-   The vault MUST be banned.

### mintTokensForReimbursedRedeem {#mintTokensForReimbursedRedeem}

If a redeemrequest has the status `Reimbursed(false)`, the vault was
unable to back the to be received tokens at the time of the
`cancelRedeem`. After gaining sufficient collateral, the vault can call
this function to finally get its tokens.

#### Specification

*Function Signature*

`mintTokensForReimbursedRedeem(vault, redeemId)`

*Parameters*

-   `vault`: the vault that was unable to back the tokens.
-   `redeemId`: the unique hash of the redeem request.

*Events*

-   `mintTokensForReimbursedRedeemEvent`{.interpreted-text role="ref"}

*Preconditions*

-   The BTC Parachain status in the `security`{.interpreted-text
    role="ref"} component MUST be set to `RUNNING:0`.
-   A `RedeemRequest` MUST exist with an id equal to `redeemId`.
-   `redeem.status` MUST be `Reimbursed(false)`.
-   The `vault` MUST equal `redeemRequest.vault`.
-   The vault MUST have sufficient collateral to remain above the
    `SecureCollateralThreshold`{.interpreted-text role="ref"} after
    issuing `redeem.amountBtc + redeem.transferFeeBtc` tokens.
-   The function call MUST be signed by `redeem.vault`, i.e. this
    function can only be called by the vault.

*Postconditions*

-   `tryIncreaseToBeIssuedTokens`{.interpreted-text role="ref"} and
    `issueTokens`{.interpreted-text role="ref"} MUST be called, both
    with the vault and `redeem.amountBtc + redeem.transferFeeBtc` as
    arguments.
-   `redeem.amountBtc + redeem.transferFeeBtc` tokens MUST be minted to
    the vault.
-   The `redeem.status` MUST be set to `Reimbursed(true)`.

Events
------

### RequestRedeem {#requestRedeemEvent}

Emit an event when a redeem request is created. This event needs to be
monitored by the vault to start the redeem request.

*Event Signature*

-   `RequestRedeem(redeemID, redeemer, amountWrapped, feeWrapped, premium, vaultId, userBtcAddress, transferFeeBtc)`

*Parameters*

-   `redeemID`: the unique identifier of this redeem request.
-   `redeemer`: address of the user triggering the redeem.
-   `amountWrapped`: the amount to be received by the user.
-   `feeWrapped`: the fee to be paid to the reward pool.
-   `premium`: the premium to be given to the user, if any.
-   `vaultId`: the vault selected for the redeem request.
-   `userBtcAddress`: the address the vault is to transfer the funds to.
-   `transferFeeBtc`: the budget the vault has to spend on bitcoin
    inclusion fees, paid for by the user.

*Functions*

-   ref:[requestRedeem]{.title-ref}

### LiquidationRedeem {#liquidationRedeemEvent}

Emit an event when a user does a liquidation redeem.

*Event Signature*

`LiquidationRedeem(redeemer, amountWrapped)`

*Parameters*

-   `redeemer`: address of the user triggering the redeem.
-   `amountWrapped`: the amount of interBTC to burned.

*Functions*

-   ref:[liquidationRedeem]{.title-ref}

### ExecuteRedeem {#executeRedeemEvent}

Emit an event when a redeem request is successfully executed by a vault.

*Event Signature*

`ExecuteRedeem(redeemId, redeemer, amountWrapped, feeWrapped, vault, transferFeeBtc)`

*Parameters*

-   `redeemId`: the unique hash created during the `requestRedeem`
    function.
-   `redeemer`: address of the user triggering the redeem.
-   `amountWrapped`: the amount of interBTC to destroy and BTC to
    receive.
-   `feeWrapped`: the amount of interBTC taken for fees.
-   `vault`: the vault responsible for executing this redeem request.
-   `transferFeeBtc`: the budget for the bitcoin inclusion fees, paid
    for by the user.

*Functions*

-   ref:[executeRedeem]{.title-ref}

### CancelRedeem {#cancelRedeemEvent}

Emit an event when a user cancels a redeem request that has not been
fulfilled after the `RedeemPeriod` has passed.

*Event Signature*

`CancelRedeem(redeemId, redeemer, vault, amountSlashed, status)`

*Parameters*

-   `redeemId`: the unique hash of the redeem request.
-   `redeemer`: The redeemer starting the redeem process.
-   `vault`: the vault who failed to execute the redeem.
-   `amountSlashed`: the amount that was slashed from the vault.
-   `status`: the status of the redeem request.

*Functions*

-   ref:[cancelRedeem]{.title-ref}

### MintTokensForReimbursedRedeem {#mintTokensForReimbursedRedeemEvent}

Emit an event when a vault minted the tokens corresponding the a
cancelled redeem that was reimbursed to the user, when the vault did not
have sufficient collateral at the time of the cancellation to back the
tokens.

*Event Signature*

`MintTokensForReimbursedRedeem(vaultId, redeemId, amountMinted)`

*Parameters*

-   `vault`: if of the vault that now mints the tokens.
-   `redeemId`: the unique hash of the redeem request.
-   `amountMinted`: the amount that the vault just minted.

*Functions*

-   ref:[mintTokensForReimbursedRedeem]{.title-ref}

Error Codes
-----------

`ERR_VAULT_NOT_FOUND`

-   **Message**: \"There exists no vault with the given account id.\"
-   **Function**: `requestRedeem`{.interpreted-text role="ref"},
    `liquidationRedeem`{.interpreted-text role="ref"}
-   **Cause**: The specified vault does not exist.

`ERR_AMOUNT_EXCEEDS_USER_BALANCE`

-   **Message**: \"The requested amount exceeds the user\'s balance.\"
-   **Function**: `requestRedeem`{.interpreted-text role="ref"},
    `liquidationRedeem`{.interpreted-text role="ref"}
-   **Cause**: If the user is trying to redeem more BTC than his
    interBTC balance.

`ERR_VAULT_BANNED`

-   **Message**: \"The selected vault has been temporarily banned.\"
-   **Function**: `requestRedeem`{.interpreted-text role="ref"}
-   **Cause**: Redeem requests are not possible with temporarily banned
    Vaults

`ERR_AMOUNT_EXCEEDS_VAULT_BALANCE`

-   **Message**: \"The requested amount exceeds the vault\'s balance.\"
-   **Function**: `requestRedeem`{.interpreted-text role="ref"},
    `liquidationRedeem`{.interpreted-text role="ref"}
-   **Cause**: If the user is trying to redeem from a vault that has
    less BTC locked than requested for redeem.

`ERR_REDEEM_ID_NOT_FOUND`

-   **Message**: \"The `redeemId` cannot be found.\"
-   **Function**: `executeRedeem`{.interpreted-text role="ref"}
-   **Cause**: The `redeemId` in the `RedeemRequests` mapping returned
    `None`.

`ERR_REDEEM_PERIOD_EXPIRED`

-   **Message**: \"The redeem period expired.\"
-   **Function**: `executeRedeem`{.interpreted-text role="ref"}
-   **Cause**: The time limit as defined by the `RedeemPeriod` is not
    met.

`ERR_UNAUTHORIZED`

-   **Message**: \"Caller is not authorized to call this function.\"
-   **Function**: `cancelRedeem`{.interpreted-text role="ref"} \|
    `mintTokensForReimbursedRedeem`{.interpreted-text role="ref"}
-   **Cause**: Only the user can call `cancelRedeem`{.interpreted-text
    role="ref"}, and only the vault can call
    `mintTokensForReimbursedRedeem`{.interpreted-text role="ref"}.

`ERR_REDEEM_PERIOD_NOT_EXPIRED`

-   **Message**: \"The period to complete the redeem request is not yet
    expired.\"
-   **Function**: `cancelRedeem`{.interpreted-text role="ref"}
-   **Cause**: Raises an error if the time limit to call `executeRedeem`
    has not yet passed.

`ERR_REDEEM_CANCELLED`

-   **Message**: \"The redeem is in an unexpected cancelled state.\"
-   **Function**: `cancelRedeem`{.interpreted-text role="ref"} \|
    `mintTokensForReimbursedRedeem`{.interpreted-text role="ref"} \|
    `executeRedeem`{.interpreted-text role="ref"}
-   **Cause**: The status of the redeem is not as required for this
    call.

`ERR_REDEEM_COMPLETED`

-   **Message**: \"The redeem is already completed.\"
-   **Function**: `cancelRedeem`{.interpreted-text role="ref"} \|
    `executeRedeem`{.interpreted-text role="ref"}
-   **Cause**: The status of the redeem is not as expected for this
    call.
