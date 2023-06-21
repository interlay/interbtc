Staking
=======

Overview
--------

This pallet is very similar to the `rewards`{.interpreted-text
role="ref"} pallet - it is also based on the [Scalable Reward
Distribution](https://solmaz.io/2019/02/24/scalable-reward-changing/)
algorithm. The reward pallet keeps track of how much rewards vaults have
earned. However, when nomination is enabled, there needs to be a way to
relay parts of the vault\'s rewards to its nominators. Furthermore, the
nominator\'s collaterals can be consumed, e.g., when a redeem is
cancelled. This pallet is responsible for both tracking the rewards, and
the current amount of contributed collaterals of vaults and nominators.

The idea is to have one reward pool per vault, where both the vault and
all of its nominators have a stake equal to their contributed
collateral. However, when collateral is consumed, either in
`cancelRedeem`{.interpreted-text role="ref"} or
`liquidateVault`{.interpreted-text role="ref"}, the collateral of each
of these stakeholders should decrease proportionally to their stake. To
be able to achieve this without iteration, in addition to tracking
`RewardPerToken`, a similar value `SlashPerToken` is introduced.
Similarly, in addition to `RewardTally`, we now also maintain a
`SlashTally` is for each stakeholder. When calculating a reward for a
stakeholder, a compensated stake is calculated, based on `Stake`,
`SlashPerToken` and `SlashTally`.

When a vault opts out of nomination, all nominators should receive their
collateral back. This is achieved by distributing all funds from the
vault\'s shared collateral as rewards. However, a vault is free to opt
back into nominator after having opted out. It is possible for the vault
to do this before all nominators have withdrawn their reward. To ensure
that the bookkeeping remains intact for the nominators to get their
rewards at a later point, all variables are additionally indexed by a
nonce, which increases every time a vault opts out of nomination.
Effectively, this create a new pool for every nominated period.

::: {.note}
::: {.title}
Note
:::

Most of the functions in this pallet that have a `_at_index` also have a
version without this suffix that does not take a `nonce` argument, and
instead uses the value stored in `Nonce`{.interpreted-text role="ref"}.
For brevity, these functions without the suffix are omitted in this
specification.
:::

Data Model
----------

### Maps

#### TotalStake

Maps `(currencyId, nonce, vaultId)` to the total stake deposited by the
given vault and its nominators, with the given nonce and currencyId.

#### TotalCurrentStake

Maps `(currencyId, nonce, vaultId)` to the total stake deposited by the
given vault and its nominators, with the given nonce and currencyId,
excluding stake that has been slashed.

#### TotalRewards

Maps `(currencyId, nonce, vaultId)` to the total rewards distributed to
the vault and its nominators. This value is currently only used for
testing purposes.

#### RewardPerToken {#RewardPerToken}

Maps `(currencyId, nonce, vaultId)` to the amount of reward the vault
and its nominators get per unit of stake.

#### RewardTally {#RewardTally}

Maps `(currencyId, nonce, vaultId, nominatorId)` to the reward tally the
given nominator has for the given vault\'s reward pool, in the given
nonce and currency. The tally influences how much the nominator can
withdraw.

#### Stake

Maps `(currencyId, nonce, vaultId, nominatorId)` to the stake the given
nominator has in the given vault\'s reward pool, in the given nonce and
currency. Initially, the stake is equal to its contributed collateral.
However, after a slashing has occurred, the nominator\'s collateral must
be compensated, using `computeStakeAtIndex`{.interpreted-text
role="ref"}.

#### SlashPerToken

Akin to `RewardPerToken`{.interpreted-text role="ref"}: maps
`(currencyId, nonce, vaultId)` to the amount the vault and its
nominators got slashed for per unit of stake. It is used for computing
the effective stake (or equivalently, its collateral) in
`computeStakeAtIndex`{.interpreted-text role="ref"}.

#### SlashTally

Akin to `RewardTally`{.interpreted-text role="ref"}: maps
`(currencyId, nonce, vaultId, nominatorId)` to the slash tally the given
nominator has for the given vault\'s reward pool, in the given nonce and
currency. It is used for computing the effective stake (or equivalently,
its collateral) in `computeStakeAtIndex`{.interpreted-text role="ref"}.

#### Nonce {#Nonce}

Maps `(currencyId, vaultId)` current value of the nonce the given vault
uses in the given currency. The nonce is increased every time
`forceRefund`{.interpreted-text role="ref"} is called, i.e., when a
vault opts out of nomination. Since nominators get their collateral back
as a withdrawable reward, the bookkeeping must remain intact when the
vault once again opts into nomination. By incrementing this nonce,
effectively a new reward pool is created for the new session. All
externally callable functions use the nonce stored in this map, except
for the reward withdrawal function
`withdrawRewardAtIndex`{.interpreted-text role="ref"}.

Functions
---------

### depositStake {#staking_depositStake}

Adds a stake for the given account and currency in the reward pool.

#### Specification

*Function Signature*

`depositStake(currencyId, vaultId, nominatorId, amount)`

*Parameters*

-   `currencyId`: The currency for which to add the stake
-   `vaultId`: Account of the vault
-   `nominatorId`: Account of the nominator
-   `amount`: The amount by which the stake is to increase

*Events*

-   `staking_DepositStakeEvent`{.interpreted-text role="ref"}

*Postconditions*

-   `Stake[currencyId, nonce, vaultId, nominatorId]` MUST increase by
    `amount`
-   `TotalStake[currencyId, nonce, vaultId]` MUST increase by `amount`
-   `TotalCurrentStake[currencyId, nonce, vaultId]` MUST increase by
    `amount`
-   `RewardTally[currencyId, nonce, vaultId, nominatorId]` MUST increase
    by `RewardPerToken[currencyId, nonce, vaultId] * amount`.
-   `SlashTally[currencyId, nonce, vaultId, nominatorId]` MUST increase
    by `SlashPerToken[currencyId, nonce, vaultId] * amount`.

### withdrawStake {#staking_withdrawStake}

Withdraws the given amount stake for the given nominator or vault. This
function also modifies the nominator\'s `SlashTally` and `Stake`, such
that the `Stake` is once again equal to its collateral.

#### Specification

*Function Signature*

`withdrawStake(currencyId, vaultId, nominatorId, amount)`

*Parameters*

-   `currencyId`: The currency for which to add the stake
-   `vaultId`: Account of the vault
-   `nominatorId`: Account of the nominator
-   `amount`: The amount by which the stake is to decrease

*Events*

-   `staking_withdrawStakeEvent`{.interpreted-text role="ref"}

*Preconditions*

-   Let `nonce` be `Nonce[currencyId, vaultId]`, and
-   Let `stake` be `Stake[nonce, currencyId, vaultId, nominatorId]`, and
-   Let `slashPerToken` be `SlashPerToken[currencyId, nonce, vaultId]`,
    and
-   Let `slashTally` be
    `slashTally[nonce, currencyId, vaultId, nominatorId]`, and
-   Let `toSlash` be `stake * slashPerToken - slashTally`

Then:

-   `stake - toSlash` MUST be greater than or equal to `amount`

*Postconditions*

-   Let `nonce` be `Nonce[currencyId, vaultId]`, and
-   Let `stake` be `Stake[nonce, currencyId, vaultId, nominatorId]`, and
-   Let `slashPerToken` be `SlashPerToken[currencyId, nonce, vaultId]`,
    and
-   Let `slashTally` be
    `slashTally[nonce, currencyId, vaultId, nominatorId]`, and
-   Let `toSlash` be `stake * slashPerToken - slashTally`

Then:

-   `Stake[currencyId, nonce, vaultId, nominatorId]` MUST decrease by
    `toSlash + amount`
-   `TotalStake[currencyId, nonce, vaultId]` MUST decrease by
    `toSlash + amount`
-   `TotalCurrentStake[currencyId, nonce, vaultId]` MUST decrease by
    `amount`
-   `SlashTally[nonce, currencyId, vaultId, nominatorId]` MUST be set to
    `(stake - toSlash - amount) * slashPerToken`
-   `RewardTally[nonce, currencyId, vaultId, nominatorId]` MUST decrease
    by `rewardPerToken * amount`

### slashStake {#slashStake}

Slashes a vault\'s stake in the given currency in the reward pool.
Conceptually, this decreases the stakes, and thus the collaterals, of
all of the vault\'s stakeholders. Indeed,
`computeStakeAtIndex`{.interpreted-text role="ref"} will reflect the
stake changes on the stakeholder.

#### Specification

*Function Signature*

`slashStake(currencyId, vaultId, amount)`

*Parameters*

-   `currencyId`: The currency for which to add the stake
-   `vaultId`: Account of the vault
-   `amount`: The amount by which the stake is to decrease

*Preconditions*

-   `TotalStake[currencyId, Nonce[currencyId, vaultId], vaultId]` MUST
    NOT be zero

*Postconditions*

Let `nonce` be `Nonce[currencyId, vaultId]`, and `initialTotalStake` be
`TotalCurrentStake[currencyId, nonce, vaultId]`. Then:

-   `SlashPerToken[currencyId, nonce, vaultId]` MUST increase by
    `amount / TotalStake[currencyId, nonce, vaultId]`
-   `TotalCurrentStake[currencyId, nonce, vaultId]` MUST decrease by
    `amount`
-   if `initialTotalStake - amount` is NOT zero,
    `RewardPerToken[currencyId, nonce, vaultId]` MUST increase by
    `RewardPerToken[currencyId, nonce, vaultId] * amount / (initialTotalStake - amount)`

### computeStakeAtIndex {#computeStakeAtIndex}

Computes a vault\'s stakeholder\'s effective stake. This is also the
amount collateral that belongs to the stakeholder.

#### Specification

*Function Signature*

`computeStakeAtIndex(nonce, currencyId, vaultId, amount)`

*Parameters*

-   `nonce`: The nonce to compute the stake at
-   `currencyId`: The currency for which to compute the stake
-   `vaultId`: Account of the vault
-   `nominatorId`: Account of the nominator

*Postconditions*

Let `stake` be `Stake[nonce, currencyId, vaultId, nominatorId]`, and Let
`slashPerToken` be `SlashPerToken[currencyId, nonce, vaultId]`, and Let
`slashTally` be `slashTally[nonce, currencyId, vaultId, nominatorId]`,
then

-   The function MUST return
    `stake - stake * slash_per_token + slash_tally`.

### distributeReward {#staking_distributeReward}

Distributes rewards to the vault\'s stakeholders.

#### Specification

*Function Signature*

`distributeReward(currencyId, reward)`

*Parameters*

-   `currencyId`: The currency being distributed
-   `vaultId`: the vault for which distribute rewards
-   `reward`: The amount being distributed

*Events*

-   `staking_distributeRewardEvent`{.interpreted-text role="ref"}

*Postconditions*

Let `nonce` be `Nonce[currencyId, vaultId]`, and Let
`initialTotalCurrentStake` be
`TotalCurrentStake[currencyId, nonce, vaultId]`, then:

-   If `initialTotalCurrentStake` is zero, or if `reward` is zero, then:
    -   The function MUST return zero.
-   Otherwise (if `initialTotalCurrentStake` and `reward` are not zero),
    then:
    -   `RewardPerToken[currencyId, nonce, vaultId)]` MUST increase by
        `reward / initialTotalCurrentStake`
    -   `TotalRewards[currencyId, nonce, vaultId]` MUST increase by
        `reward`
    -   The function MUST return `reward`.

### computeRewardAtIndex {#staking_computeRewardAtIndex}

Calculates the amount of rewards the vault\'s stakeholder can withdraw.

#### Specification

*Function Signature*

`computeRewardAtIndex(nonce, currencyId, vaultId, amount)`

*Parameters*

-   `nonce`: The nonce to compute the stake at
-   `currencyId`: The currency for which to compute the stake
-   `vaultId`: Account of the vault
-   `nominatorId`: Account of the nominator

*Postconditions*

Let `stake` be the result of
`computeStakeAtIndex(nonce, currencyId, vaultId, nominatorId)`, then:
Let `rewardPerToken` be `RewardPerToken[currencyId, nonce, vaultId]`,
and Let `rewardTally` be
`rewardTally[nonce, currencyId, vaultId, nominatorId]`, then

-   The function MUST return
    `max(0, stake * rewardPerToken - reward_tally)`

### withdrawRewardAtIndex {#withdrawRewardAtIndex}

Withdraws the rewards the given vault\'s stakeholder has accumulated.

#### Specification

*Function Signature*

`withdrawRewardAtIndex(currencyId, vaultId, amount)`

*Parameters*

-   `nonce`: The nonce to compute the stake at
-   `currencyId`: The currency for which to compute the stake
-   `vaultId`: Account of the vault
-   `nominatorId`: Account of the nominator

*Events*

-   `staking_withdrawRewardEvent`{.interpreted-text role="ref"}

*Preconditions*

-   `computeRewardAtIndex(nonce, currencyId, vaultId, nominatorId)` MUST
    NOT return an error

*Postconditions*

Let `reward` be the result of
`computeRewardAtIndex(nonce, currencyId, vaultId, nominatorId)`, then:
Let `stake` be `Stake(nonce, currencyId, vaultId, nominatorId)`, then:
Let `rewardPerToken` be `RewardPerToken[currencyId, nonce, vaultId]`,
and

-   `TotalRewards[currency_id, nonce, vault_id]` MUST decrease by
    `reward`
-   `RewardTally[currencyId, nonce, vaultId, nominatorId]` MUST be set
    to `stake * rewardPerToken`
-   The function MUST return `reward`

### forceRefund {#forceRefund}

This is called when the vault opts out of nomination. All collateral is
distributed among the stakeholders, after which the vault withdraws his
part immediately.

#### Specification

*Function Signature*

`forceRefund(currencyId, vaultId)`

*Parameters*

-   `currencyId`: The currency for which to compute the stake
-   `vaultId`: Account of the vault

*Events*

-   `forceRefundEvent`{.interpreted-text role="ref"}
-   `increaseNonceEvent`{.interpreted-text role="ref"}

*Preconditions*

Let `nonce` be `Nonce[currencyId, vaultId]`, then:

-   `distributeReward(currencyId, vaultId, TotalCurrentStake[currencyId, nonce, vaultId])`
    MUST NOT return an error
-   `withdrawRewardAtIndex(nonce, currencyId, vaultId, vaultId)` MUST
    NOT return an error
-   `depositStake(currencyId, vaultId, vaultId, reward)` MUST NOT return
    an error
-   `Nonce[currencyId, vaultId]` MUST be increased by 1

*Postconditions*

Let `nonce` be `Nonce[currencyId, vaultId]`, then:

-   `distributeReward(currencyId, vaultId, TotalCurrentStake[currencyId, nonce, vaultId])`
    MUST have been called
-   `withdrawRewardAtIndex(nonce, currencyId, vaultId, vaultId)` MUST
    have been called
-   `Nonce[currencyId, vaultId]` MUST be increased by 1
-   `depositStake(currencyId, vaultId, vaultId, reward)` MUST have been
    called AFTER having increased the nonce

### DepositStake {#staking_DepositStakeEvent}

*Event Signature*

`DepositStake(currencyId, vaultId, nominatorId, amount)`

*Parameters*

-   `currencyId`: The currency of the reward pool
-   `vaultId`: Account of the vault
-   `nominatorId`: Account of the nominator
-   `amount`: The amount by which the stake is to increase

*Functions*

-   `staking_depositStake`{.interpreted-text role="ref"}

### WithdrawStake {#staking_withdrawStakeEvent}

*Event Signature*

`WithdrawStake(currencyId, vaultId, nominatorId, amount)`

*Parameters*

-   `currencyId`: The currency of the reward pool
-   `vaultId`: Account of the vault
-   `nominatorId`: Account of the nominator
-   `amount`: The amount by which the stake is to increase

*Functions*

-   `staking_WithdrawStake`{.interpreted-text role="ref"}

### DistributeReward {#staking_distributeRewardEvent}

*Event Signature*

`DistributeReward(currencyId, vaultId, amount)`

*Parameters*

-   `currencyId`: The currency of the reward pool
-   `vaultId`: Account of the vault
-   `amount`: The amount by which the stake is to increase

*Functions*

-   `staking_distributeReward`{.interpreted-text role="ref"}

### WithdrawReward {#staking_withdrawRewardEvent}

*Event Signature*

`WithdrawReward(currencyId, vaultId, nominatorId, amount)`

*Parameters*

-   `currencyId`: The currency of the reward pool
-   `vaultId`: Account of the vault
-   `nominatorId`: Account of the nominator
-   `amount`: The amount by which the stake is to increase

*Functions*

-   `withdrawRewardAtIndex`{.interpreted-text role="ref"}

### ForceRefund {#forceRefundEvent}

*Event Signature*

`ForceRefund(currencyId, vaultId)`

*Parameters*

-   `currencyId`: The currency of the reward pool
-   `vaultId`: Account of the vault

*Functions*

-   `ForceRefund`{.interpreted-text role="ref"}

### IncreaseNonce {#increaseNonceEvent}

*Event Signature*

`IncreaseNonce(currencyId, vaultId, nominatorId, amount)`

*Parameters*

-   `currencyId`: The currency of the reward pool
-   `vaultId`: Account of the vault
-   `amount`: The amount by which the stake is to increase

*Functions*

-   `forceRefund`{.interpreted-text role="ref"}
