Fee Model {#fee_model}
=========

The interBTC bridge uses conceptually three different and independent
fee models:

1.  **interBTC Fee Model**. The internal interBTC bridge fee model
    covers any payments made through the operation of the bridge, e.g.,
    the issue, redeem, or replace processes. This process concerns
    Users, Vaults (and its Nominators), and Relayers.
2.  **Griefing Fee Model**. These are [DOT]{.title-ref} fees paid to the
    Vault on a failed issue or replace.
3.  **Transaction Fee Model**. The transaction fees are essentially the
    [DOT]{.title-ref} fees paid on every transaction to the Collators.

Payment Flows
-------------

We detail the payment flows for both models in the figure below:

![Detailed overview of fee accrual in the interBTC bridge, showing
interBTC Fee Model and Transaction Fee Model payment flows, as well as
opportunity costs.](../figures/economics/fee-payment-flows.png)

interBTC Fee Model
------------------

### Issue and Redeem Fee Distribution

The primary fees in interBTC are paid by users during
`issue-protocol`{.interpreted-text role="ref"} and
`redeem-protocol`{.interpreted-text role="ref"} as a relative fee on the
issued or redeemed interBTC.

Vaults earn fees based on their currently backed interBTC (i.e.,
`vault.issuedTokens`). To reduce variance of payouts, the interBTC
bridge implements a pooled fee model. This means that Vaults earn a
share of each fee based on their share of issued interBTC in the bridge.

If the Vault does not back interBTC then it does not have a stake in the
system and it will not receive any rewards, i.e., its stake is 0.
Conversely, if the Vault has any issued interBTC, the Vault will earn
rewards. Thus, only Vaults directly locking Bitcoin in the system will
earn rewards from users.

Each time a user issues or redeems interBTC, they pay the following fees
to a global fee pool:

-   **Issue Fee**: A relative fee paid based on the requested interBTC
    paid in [interBTC]{.title-ref}, for the current parameterization see
    `issueFee`{.interpreted-text role="ref"}
-   **Redeem Fee**: A relative fee paid based on the requested BTC paid
    in [interBTC]{.title-ref}, for the current parameterization see
    `redeemFee`{.interpreted-text role="ref"}

::: {.note}
::: {.title}
Note
:::

Since redeem fees are backed by the Vault, they must use the Replace
protocol to exit the system. To solve this issue, we allow self-redeems
based on the Vault's account ID which sets the redeem fee to zero.
:::

From this fee pool 100% is distributed among all active Vaults.

Each Vault is receiving a fair share of this fee pool by considering its
stake in the system. The stake in the system is just the amount of BTC a
vault is currently insuring with collateral. Calculating the rewards for
a Vault is equivalent to this formula:

$$\text{rewards} = \text{stake} (\text{totalRewards} / \text{totalStake})$$

*Eq. 1: Vault reward distribution.*

::: {.note}
::: {.title}
Note
:::

As an example, if we had 1 interBTC to distribute among all Vaults with
total stake 200 and assuming the individual Vault has stake 100, the
reward share could be calculated by: 100 \* (1 interBTC / 200) = 0.5
interBTC
:::

To be exact, the stake is expressed as the interBTC issued by a Vault.
The issued interBTC are the interBTC currently being backed by the
Vault. This shows how much a Vault's collateral is "occupied" by users:

$$\text{stake} = \text{interBTCIssued}$$

*Eq. 2: Parameterized stake updates.*

#### Stake Updates

Whenever a Vault is increasing or decreasing the number of issued
interBTC it is backing, we MUST update their stake in the reward pool
accordingly. These updates are achieved through the issue, redeem, and
replace operations.

#### Fee Payouts

The Vault fee is paid each time an Issue or Redeem request is executed.
Naively speaking, the bridge behaves as if on each issue and redeem, the
bridge would loop through all Vaults to determine their share of stake,
i.e., `vault.issuedTokens / totalSupply`, and distribute a percentage of
the paid fees to the Vault.

Since a naive implementation would result in unbounded iteration, the
fee payout is implemented in a different way. However, the outcome it is
equivalent to the naive approach. The payouts are based on the
pull-based [Scalable Reward Distribution with Changing Stake
Sizes](https://solmaz.io/2019/02/24/scalable-reward-changing/). This
scheme allows rewards to be drawn by each Vault (and Nominator)
individually and at any time without the interBTC bridge having to loop
over all Vaults each time rewards are paid out. Read the
`scalableRewardDistribution`{.interpreted-text role="ref"} section if
you would like to understand how the payout system works under the hood.

### Griefing Fees

Griefing collateral is locked on `requestIssue`{.interpreted-text
role="ref"} and `requestReplace`{.interpreted-text role="ref"} to
prevent `griefing`{.interpreted-text role="ref"}. If the requests are
indeed cancelled, the griefing collateral is paid to the free balance of
the Vault that locked collateral in vain. On successful execute, the
griefing collateral is refunded to the party making the request.

-   **Issue Griefing Collateral**: A relative collateral locked based on
    the requested interBTC paid in [DOT]{.title-ref}, for the current
    parameterization see `issueGriefingCollateral`{.interpreted-text
    role="ref"}
-   **Replace Griefing Collateral**: A relative collateral locked based
    on the request interBTC paid in [DOT]{.title-ref}, for the current
    parameterization see `replaceGriefingCollateral`{.interpreted-text
    role="ref"}

#### Griefing Collateral Currency {#griefingCurrency}

The currency that is used for griefing collateral used for issue and
replace. This value is set to the currency of the transaction fees,
i.e., [DOT]{.title-ref}, regardless of the vault\'s configured backing
collateral currency.

### Premium Redeem Fee

When Vaults are below the `premiumCollateralThreshold`{.interpreted-text
role="ref"}, users are able to redeem with the Vault and receive an
extra \"bonus\" slashed from the Vault\'s collateral. This mechanism is
to ensure that (1) Vaults have a higher incentive to stay above the
`premiumCollateralThreshold`{.interpreted-text role="ref"} and (2) users
have an additional incentive to redeem with Vaults that are close to the
`liquidationThreshold`{.interpreted-text role="ref"}.

-   **Premium Redeem Fee**: A relative fee slashed from the Vault\'s
    collateral paid to the user in the vault\'s [COL]{.title-ref} if a
    Vault is below the `premiumCollateralThreshold`{.interpreted-text
    role="ref"}, for the current parameterization see
    `premiumRedeemFee`{.interpreted-text role="ref"}

### Punishment Fees

Punishment fees are slashed from the Vault\'s collateral on failed
redeems. A user can choose to either retry with another Vault or
reimburse the [interBTC]{.title-ref} amount. In both cases, the a
punishment fee is deducted from the Vault\'s collateral to ensure that
Vault\'s are punished in both cases.

-   **Punishment Fee**: A relative fee slashed from the Vault\'s
    collateral paid to the user in the vault\'s [COL]{.title-ref} if a
    Vault failed to execute a redeem request, for the current
    parameterization see `punishmentFee`{.interpreted-text role="ref"}

### Theft Fee

Relayers receive a reward for reporting Vaults for committing theft (see
`relay_function_report_vault_theft`{.interpreted-text role="ref"} and
`relay_function_report_vault_double_payment`{.interpreted-text
role="ref"}).

-   **Theft Fee**: A relative fee slashed form the Vault\'s collateral
    paid to the Relayer in the vault\'s [COL]{.title-ref} if a Vault
    commits theft, for the current parameterization see
    `theftFee`{.interpreted-text role="ref"}

### Arbitrage

Arbitrage trades are executed by anyone that exchanges
[interBTC]{.title-ref} for [COL]{.title-ref} against the
LiquidationVault. The LiquidationVault is essentially an AMM with two
balances:

-   *issuedTokens*: amount of [interBTC]{.title-ref} that have been
    liquidated through safety failures, see
    `liquidations`{.interpreted-text role="ref"}
-   *lockedCollateral*: amount of [COL]{.title-ref} that have been
    confiscated through safety failures, see
    `liquidations`{.interpreted-text role="ref"}

Anyone can now burn [interBTC]{.title-ref} for [COL]{.title-ref} at the
exchange rate of the `issuedTokens/lockedCollateral` from the
LiquidationVault. As the `liquidationThreshold`{.interpreted-text
role="ref"} is strictly above the current exchange rate of the
[BTC/COL]{.title-ref} pair at the time of liquidation, this *should*
represent an arbitrage opportunity: the value of burned
[interBTC]{.title-ref} should be lower than the value of received
[COL]{.title-ref}.

However, in practice, the arbitrage process might not work as intended.
See `externalEconomicRisks`{.interpreted-text role="ref"} for a
discussion of related problems. Note that there are no fees being
collected to execute trades against the LiquidationVault.

### Excursion: Scalable Reward Distribution {#scalableRewardDistribution}

We recommend reading first the [Scalable Reward Distribution
paper](http://batog.info/papers/scalable-reward-distribution.pdf) and
then the [extension for changing
rewards](https://solmaz.io/2019/02/24/scalable-reward-changing/). Note
that this scheme is "just" an efficient equivalent of the Vault
distribution outlined above. Last, we extend this scheme to account for
`vault_nomination`{.interpreted-text role="ref"} and
`liquidations`{.interpreted-text role="ref"}. The adopted scheme is
described in the [README of the
implementation](https://github.com/interlay/interbtc/tree/master/crates/staking).

Notable changes to the Scalable Reward Distribution with Changing
Rewards are:

-   **Staking Pools** Fees are forwarded to a *Reward Pool* and then
    distributed to a *Staking Pool*. There is one Staking Pool for each
    Vault and all of its Nominators.
-   **Slashing** On liquidation of Vaults, no more fees are forwarded to
    the Staking Pool of that Vault.

See the figure below for an indication how the Staking Pools are used.

![Distribution of fees according to Staking Pools. Each Vault and all
its Nominators are represented by a Staking Pool. This allows to
distribute the applicable fees based on the global share of issued
interBTC based on the stake of the Staking Pool as well as an individual
distribution of fees between the Vault and its Nominators based on their
share in the pool.](../figures/economics/fee-staking-pool.png)

In the scalable reward distribution, a single source of truth is used to
calculate rewards: the "stake". The "stake" can be any numeric
representation. In interBTC, stake is defined as: *the current amount of
issued interBTC*. A Vault's stake is adjusted based on the change in
issued interBTC - for instance we increase the issued interBTC on
successful issues and decrease this on executed redeems.

::: {.note}
::: {.title}
Note
:::

For example, if a Vault executes issue requests amounting to 2,456,000
interSatoshi (smallest denomination) being added to the system, its
stake would increase by 2,456,000. If the Vault then executes redeem
requests, its rewards are reduced. So if the Vault redeems all 2,456,000
interSatoshi, its stake is 0 again. On a liquidation, this is again set
to zero since the Vault no longer backs these tokens.
:::

Now, each Vault's rewards are calculated according to the following
formula (equivalent to Eq. 1):

$$\text{deposit}(\text{stakeDelta}): \text{rewardTally} \mathrel{+}= \text{rewardPerToken} \cdot \text{stakeDelta}$$

$$\text{stake} \mathrel{+}= \text{stakeDelta}$$

$$\text{totalStake} \mathrel{+}= \text{stakeDelta}$$

$$\text{distributeReward}(\text{reward}): \text{rewardPerToken} \mathrel{+}= \text{reward} / \text{totalStake}$$

$$\text{computeReward}(): \text{return stake} \cdot \text{rewardPerToken} - \text{rewardTally}$$

*Eq. 3: Vault reward distribution using the SRD.*

**Definitions**

-   **stake:** the amount of interBTC issued by this Vault.
-   **reward\_tally**: the Vault's accumulated rewards (can be negative
    or positive).
-   **stake\_delta**: the stake impact based on issuing or redeeming
    interBTC.
-   **total\_stake**: the total amount of interBTC issued by all Vaults.
-   **reward\_per\_token**: the current reward per current stake (the
    total\_stake).
-   **reward**: the rewards paid from issue and redeem requests.

The reward is influenced by the total of all stakes. So the share of
rewards paid to a Vault is determined by how many other Vaults are in
the system and their individual stake.

**Example Without Nomination**

*Current stake*

Note: stake is always non-zero.

-   Vault Alice has a stake of 250
-   Vault Bob has a stake of 30
-   Vault Charlie has a stake of 100

The total stake is therefore `380`.

*Reward claims*

Let's assume there is a total of 1 interBTC in the reward pool based on
the accumulated issue and redeem request. Then the `reward_per_token` =
`1 interBTC / 380`.

-   Vault Alice has a claim of
    `250 * 1 interBTC/380 = 0.6578947368421052 interBTC`
-   Vault Bob has a claim of
    `30 * 1 interBTC/380 = 0.07894736842105263 interBTC`
-   Vault Charlie has a claim of
    `100 * 1 interBTC/380 = 0.2631578947368421 interBTC`

**Example With Nomination**

*Current stake*

Note: stake is always non-zero.

-   Vault Alice and her Nominators have a stake of 250. Alice is fully
    nominated such that Alice is backing 200 and her Nominators are
    backing 50.
-   Vault Bob has a stake of 30
-   Vault Charlie has a stake of 100

The total stake is therefore `380`.

*Reward claims*

Let's assume there is a total of 1 interBTC in the reward pool based on
the accumulated issue and redeem request. Then the `reward_per_token` =
`1 interBTC / 380`.

-   Vault Alice has a claim of
    `200 * 1 interBTC/380 = 0.526315789 interBTC`
-   Alice\'s Nominators have a claim of
    `50 * 1 interBTC/380 = 0.131578947 interBTC`
-   Vault Bob has a claim of
    `30 * 1 interBTC/380 = 0.07894736842105263 interBTC`
-   Vault Charlie has a claim of
    `100 * 1 interBTC/380 = 0.2631578947368421 interBTC`

Transaction Fee Model
---------------------

The interBTC bridge chain adopts the Polkadot relay chain model with
[DOT]{.title-ref} as the native currency for paying transaction fees. In
this model, collators receive 100% of the transaction fees paid by
Users, Vaults, and Relayers. We refer to the official [Polkadot
documentation](https://wiki.polkadot.network/docs/learn-transaction-fees#fee-calculation)
for full details.
