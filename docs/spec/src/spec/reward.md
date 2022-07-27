Reward {#rewards}
======

Overview
--------

This pallet provides a way distribute rewards to any number of accounts,
proportionally to their stake. It does so using the [Scalable Reward
Distribution](https://solmaz.io/2019/02/24/scalable-reward-changing/)
algorithm. It does not directly transfer any rewards - rather, the
stakeholders have to actively withdraw their accumulated rewards, which
they can do at any time. Stakeholders can also change their stake at any
time, without impacting the rewards gained in the past.

Invariants
----------

-   For each `currencyId`,
    -   `TotalStake[currencyId]` MUST be equal to the sum of
        `Stake[currencyId, accountId]` over all accounts.
    -   `TotalReward[currencyId]` MUST be equal to the sum of
        `Stake[currencyId, accountId] * RewardPerToken[currencyId] - RewardTally[currencyId, accountId]`
        over all accounts.
    -   For each `accountId`,
        -   `RewardTally[currencyId, accountId]` MUST be smaller than or
            equal to
            `Stake[currencyId, accountId] * RewardPerToken[currencyId]`
        -   `Stake[currencyId, accountId]` MUST NOT be negative
        -   `RewardTally[currencyId, accountId]` MUST NOT be negative

Data Model
----------

### Maps

#### TotalStake

The total stake deposited to the reward with the given currency.

#### TotalRewards

The total unclaimed rewards in the given currency distributed to this
reward pool. This value is currently only used for testing purposes.

#### RewardPerToken

The amount of reward the stakeholders get for the given currency per
unit of stake.

#### Stake

The stake in the given currency for the given account.

#### RewardTally

The amount of rewards in the given currency a given account has already
withdrawn, plus a compensation that is added on stake changes.

Functions
---------

### getTotalRewards {#getTotalRewards}

This function gets the total amount of rewards distributed in the pool
with the given currencyId.

#### Specification

*Function Signature*

`getTotalRewards(currencyId)`

*Parameters*

-   `currencyId`: Determines of which currency the amount is returned.

*Postconditions*

-   The function MUST return the total amount of rewards that have been
    distributed in the given currency.

### depositStake {#reward_depositStake}

Adds a stake for the given account and currency in the reward pool.

#### Specification

*Function Signature*

`depositStake(currencyId, accountId, amount)`

*Parameters*

-   `currencyId`: The currency for which to add the stake
-   `accountId`: The account for which to add the stake
-   `amount`: The amount by which the stake is to increase

*Events*

-   `depositStakeEvent`{.interpreted-text role="ref"}

*Preconditions*

*Postconditions*

-   `Stake[currencyId, accountId]` MUST increase by `amount`
-   `TotalStake[currencyId]` MUST increase by `amount`
-   `RewardTally[currencyId, accountId]` MUST increase by
    `RewardPerToken[currencyId] * amount`. This ensures the amount of
    rewards the given accountId can withdraw remains unchanged.

### distributeReward {#reward_distributeReward}

Distributes rewards to the stakeholders.

#### Specification

*Function Signature*

`distributeReward(currencyId, reward)`

*Parameters*

-   `currencyId`: The currency being distributed
-   `reward`: The amount being distributed

*Events*

-   `distributeRewardEvent`{.interpreted-text role="ref"}

*Preconditions*

-   `TotalStake[currencyId]` MUST NOT be zero.

*Postconditions*

-   `RewardPerToken[currencyId]` MUST increase by
    `reward / TotalStake[currencyId]`
-   `TotalRewards[currencyId]` MUST increase by `reward`

### computeReward {#computeReward}

Computes the amount a given account can withdraw in the given currency.

#### Specification

*Function Signature*

`computeReward(currencyId, accountId)`

*Parameters*

-   `currencyId`: The currency for which the rewards are being
    calculated
-   `accountId`: Account for which the rewards are being calculated.

*Postconditions*

-   The function MUST return
    `Stake[currencyId, accountId] * RewardPerToken[currencyId] - RewardTally[currencyId, accountId]`.

### withdrawStake {#reward_withdrawStake}

Decreases a stake for the given account and currency in the reward pool.

#### Specification

*Function Signature*

`withdrawStake(currencyId, accountId, amount)`

*Parameters*

-   `currencyId`: The currency for which to decrease the stake
-   `accountId`: The account for which to decrease the stake
-   `amount`: The amount by which the stake is to decrease

*Events*

-   `withdrawStakeEvent`{.interpreted-text role="ref"}

*Preconditions*

-   `amount` MUST NOT be greater than `Stake[currencyId, accountId]`

*Postconditions*

-   `Stake[currencyId, accountId]` MUST decrease by `amount`
-   `TotalStake[currencyId]` MUST decrease by `amount`
-   `RewardTally[currencyId, accountId]` MUST decrease by
    `RewardPerToken[currencyId] * amount`. This ensures the amount of
    rewards the given accountId can withdraw remains unchanged.

### withdrawReward {#withdrawReward}

Withdraw all available rewards of a given account and currency

#### Specification

*Function Signature*

`withdrawReward(currencyId, reward)`

*Parameters*

-   `currencyId`: The currency being withdrawn
-   `accountId`: The account for which to withdraw the rewards

*Events*

-   `withdrawRewardEvent`{.interpreted-text role="ref"}

*Preconditions*

-   `TotalStake[currencyId]` MUST NOT be zero.

*Postconditions*

Let `reward` be the result `computeReward`{.interpreted-text role="ref"}
when it is called with `currencyId` and `accountId` as arguments. Then:

-   `TotalRewards[currencyId]` MUST decrease by `reward`
-   `RewardPerToken[currencyId]` MUST be set to
    `RewardPerToken[currencyId] * Stake[currencyId, accountId]`

Events
------

### DepositStake {#depositStakeEvent}

*Event Signature*

`DepositStake(currencyId, accountId, amount)`

*Parameters*

-   `currencyId`: the currency for which the stake has been changed
-   `accountId`: the account for which the stake has been changed
-   `amount`: the increase in stake

*Functions*

-   `reward_depositStake`{.interpreted-text role="ref"}

### WithdrawStake {#withdrawStakeEvent}

*Event Signature*

`WithdrawStake(currencyId, accountId, amount)`

*Parameters*

-   `currencyId`: the currency for which the stake has been changed
-   `accountId`: the account for which the stake has been changed
-   `amount`: the decrease in stake

*Functions*

-   `reward_withdrawStake`{.interpreted-text role="ref"}

### DistributeReward {#distributeRewardEvent}

*Event Signature*

`DistributeReward(currencyId, accountId, amount)`

*Parameters*

-   `currencyId`: the currency for which the reward has been withdrawn
-   `amount`: the distributed amount

*Functions*

-   `reward_distributeReward`{.interpreted-text role="ref"}

### WithdrawReward {#withdrawRewardEvent}

*Event Signature*

`WithdrawReward(currencyId, accountId, amount)`

*Parameters*

-   `currencyId`: the currency for which the reward has been withdrawn
-   `accountId`: the account for which the reward has been withdrawn
-   `amount`: the withdrawn amount

*Functions*

-   `withdrawReward`{.interpreted-text role="ref"}
