Fee
===

Overview
--------

The fee model crate implements the fee model outlined in
`fee_model`{.interpreted-text role="ref"}.

### Step-by-step

1.  Fees are paid by Users (e.g., during issue and redeem requests) and
    forwarded to a reward pool.
2.  Fees are then split between incentivised network participants (i.e.
    Vaults).
3.  Network participants can claim these rewards from the pool based on
    their stake.
4.  Stake is determined by their participation in the network - through
    incentivized actions.
5.  Rewards are paid in `interBTC`.

Data Model
----------

### Scalars (Fees)

#### IssueFee {#issueFee}

Issue fee share (configurable parameter, as percentage) that users need
to pay upon issuing `interBTC`.

-   Paid in `interBTC`
-   Initial value: 0.5%

#### IssueGriefingCollateral {#issueGriefingCollateral}

Issue griefing collateral as a percentage of the locked collateral of a
Vault a user has to lock to issue `interBTC`.

-   Paid in collateral
-   Initial value: 0.005%

#### RefundFee {#refundFee}

Refund fee (configurable parameter, as percentage) that users need to
pay to refund overpaid `interBTC`.

-   Paid in `interBTC`
-   Initial value: 0.5%

#### RedeemFee {#redeemFee}

Redeem fee share (configurable parameter, as percentage) that users need
to pay upon request redeeming `interBTC`.

-   Paid in `interBTC`
-   Initial value: 0.5%

#### PremiumRedeemFee {#premiumRedeemFee}

Fee for users to premium redeem (as percentage). If users execute a
redeem with a Vault flagged for premium redeem, they earn a premium
slashed from the Vault's collateral.

-   Paid in collateral
-   Initial value: 5%

#### PunishmentFee {#punishmentFee}

Fee (as percentage) that a Vault has to pay if it fails to execute
redeem requests (for redeem, on top of the slashed value of the
request). The fee is paid in collateral based on the `interBTC` amount
at the current exchange rate.

-   Paid in collateral
-   Initial value: 10%

#### TheftFee {#theftFee}

Fee (as percentage) that a reporter receives if another Vault commits
theft. The fee is paid in collateral taken from the liquidated Vault.

-   Paid in collateral
-   Initial value: 5%

#### TheftFeeMax {#theftFeeMax}

Upper bound to the reward that can be payed to a reporter on success.
This is expressed in Bitcoin to ensure consistency between assets.

-   Initial value: 0.1 BTC

#### ReplaceGriefingCollateral {#replaceGriefingCollateral}

Default griefing collateral as a percentage of the to-be-locked
collateral of the new Vault, Vault has to lock to be replaced by another
Vault. This collateral will be slashed and allocated to the replacing
Vault if the to-be-replaced Vault does not transfer BTC on time.

-   Paid in collateral
-   Initial value: 0.005%

Functions
---------

### distributeRewards

Distributes fees among incentivised network participants.

#### Specification

*Function Signature*

`distributeRewards(amount)`

*Preconditions*

-   There MUST be at least one registered Vault OR a treasury account.

*Postconditions*

-   If there are no registered funds, rewards MUST be sent to the
    treasury account.
-   Otherwise, rewards MUST be distributed according to
    `reward_distributeReward`{.interpreted-text role="ref"}.

### withdrawRewards {#withdrawRewards}

A function that allows incentivised network participants to withdraw all
earned rewards.

#### Specification

*Function Signature*

`withdrawRewards(accountId, vaultId)`

*Parameters*

-   `accountId`: the account withdrawing `interBTC` rewards.
-   `vaultId`: the vault that generated `interBTC` rewards.

*Events*

-   `withdrawRewardsEvent`{.interpreted-text role="ref"}

*Preconditions*

-   The function call MUST be signed by `accountId`.
-   The BTC Parachain status in the `security`{.interpreted-text
    role="ref"} component MUST NOT be `SHUTDOWN:2`.
-   The `accountId` MUST have available rewards for `interBTC`.

*Postconditions*

-   The account\'s balance MUST increase by the available rewards.
-   The account\'s withdrawable rewards MUST decrease by the withdrawn
    rewards.

Events
------

### WithdrawRewards {#withdrawRewardsEvent}

*Event Signature*

`WithdrawRewards(account, amount)`

*Parameters*

-   `account`: the account withdrawing rewards
-   `amount`: the amount of rewards withdrawn

*Functions*

-   `withdrawRewards`{.interpreted-text role="ref"}
