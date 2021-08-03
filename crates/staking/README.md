# Staking Pallet

This pallet is used to manage joint funds for Vaults and Nominators.

## Slashing

We extend the [scalable reward distribution](https://solmaz.io/2019/02/24/scalable-reward-changing/) to account for the imbalance in Vault collateral after slashing. Instead of tallying the `reward_per_token` we increase `slash_per_token` to lazily calculate the proportion of collateral to subtract from a Vault or Nominator's stake on deposit or withdrawal. Slashing reduces the stake of *everyone* in the pool proportionally, meaning Nominators with a larger stake will be slashed more.

Without accounting for changing stake sizes we can reduce the staking calculation to the following formula:

```
actual_stake = stake - (stake * (slashed_amount / total_stake))
```

### Example

1. Alice deposits 100 DOT.
2. Bob deposits 50 DOT.

At this point in time, assuming we slash 50 DOT, we have the following stakes:

- Alice may withdraw `100 - (100 * (50 / 150)) = ~66.66 DOT`.
- Bob may withdraw `50 - (50 * (50 / 150)) = ~33.33 DOT`.

## Rewards

The proportionality of rewards is directly tied to the stake of a participant. Assuming stake may be slashed after reward distribution and prior to reward withdrawal, we must make slight adjustments to the [scalable reward distribution](https://solmaz.io/2019/02/24/scalable-reward-changing/). Rewards are calculated using the actual stake from the equation above, and upon slashing we increase `reward_per_token` by `(amount * reward_per_token) / total_stake` to account for lost rewards. 

Without accounting for changing stake sizes we can reduce the reward calculation to the following formula:

```
rewards = actual_stake * (total_rewards / actual_total_stake)
```

Where `actual_total_stake = total_stake - slashed_amount`.

### Example

1. Alice deposits 10 DOT.
2. Alice is slashed 10 DOT.
2. Bob deposits 20 DOT.
3. Charlie deposits 50 DOT.

At this point in time, assuming we distribute 100 DOT, we have the following reward claims:

- Alice may claim `(10 - (10 * (10 / 10))) * (10 / 70) = 0 DOT`.
- Bob may claim `20 * (100 / 70) = ~28.57 DOT`.
- Charlie may claim `50 * (100 / 70) = ~71.42 DOT`.

## Force Refunds

To allow Vaults to refund all Nominators at once, we index each staking pool by a nonce. Most operations will use the latest nonce when reading from storage, but Nominators must specify this to recover past stake or rewards. This allows for optimal complexity since no on-chain iteration is required.

- The nonce (`n`) is zero-initialized and incremented on force refund.
- Vault stake is re-distributed to the new staking pool at nonce `n+1`.
- Nominators may withdraw stake and rewards at nonce `n`.
