# Staking Pallet

This pallet is used to manage joint funds for Vaults and Nominators.

## Slashing

We extend the [scalable reward distribution](https://solmaz.io/2019/02/24/scalable-reward-changing/) to account for the imbalance in Vault collateral. Instead of tallying the `reward_per_token` we increase `slash_per_token` to lazily calculate the proportion of collateral to subtract from a Vault or Nominator's stake on deposit or withdrawal.

We present the simple solution (without rewards) below:

```python
class Slashing:
    def __init__(self):
        self.total_stake = 0
        self.slash_per_token = 0
        self.stake = {}
        self.slash_tally = {}

    def deposit_stake(self, address, amount):
        if address not in self.stake:
            self.stake[address] = 0
            self.slash_tally[address] = 0

        self.stake[address] += amount
        self.slash_tally[address] += self.slash_per_token * amount
        self.total_stake += amount

    def slash_stake(self, amount):
        self.slash_per_token += amount / self.total_stake

    def compute_stake(self, address):
        to_slash = self.stake[address] * self.slash_per_token - self.slash_tally[address]
        return self.stake[address] - to_slash

    def withdraw_stake(self, address, amount):
        if amount > self.compute_stake(address):
            raise Exception("Invalid amount")

        self.stake[address] -= amount
        self.slash_tally[address] -= self.slash_per_token * amount
        self.total_stake -= - amount
```

Without accounting for changing stake sizes we can reduce the staking calculation to the following formula:

```
actual_stake = stake - (stake * (slashed_amount / total_stake))
```

### Example

1. Alice deposits 100 DOT in staking pool A.
2. Bob deposits 50 DOT in staking pool A.

At this point in time, assuming we slash 50 DOT in A, we have the following stakes:

- Alice may withdraw `100 - (100 * (50 / 150)) = 66.66 DOT`.
- Bob may withdraw `50 - (50 * (50 / 150)) = 33.33 DOT`.

### Unslashing

It is reasonable to assume that stake may be re-distributed to participants after slashing. In this case we would want to "unslash" participant stake proportionally. Since we calculate the amount to slash dynamically we can handle this case by reducing `slash_per_token` by the amount to release. An example of where this might be useful is if the Vault is liquidated with open redeem requests, upon completion we should re-distribute collateral to the Vault and its Nominators.

## Rewards

The proportionality of rewards is directly tied to the stake of a participant. Assuming stake may be slashed after reward distribution and prior to reward withdrawal, we must make slight adjustments to the [scalable reward distribution](https://solmaz.io/2019/02/24/scalable-reward-changing/). For instance, upon slashing we increase `reward_per_token` by `(amount * reward_per_token) / total_stake` to account for lost rewards. Additionally, we must re-compute stake in `withdraw_rewards` to get the correct balance for an account.

We present the simple solution (without adjustments) below:

```python
class Rewards:
    def __init__(self):
        self.total_stake = 0
        self.reward_per_token = 0
        self.stake = {}
        self.reward_tally = {}

    def deposit_stake(self, address, amount):
        if address not in self.stake:
            self.stake[address] = 0
            self.reward_tally[address] = 0

        self.stake[address] += amount
        self.reward_tally[address] += self.reward_per_token * amount
        self.total_stake += amount

    def distribute_reward(self, reward):
        self.reward_per_token += reward / self.total_stake

    def compute_reward(self, address):
        return self.stake[address] * self.reward_per_token - self.reward_tally[address]

    def withdraw_stake(self, address, amount):
        if amount > self.stake[address]:
            raise Exception("Invalid amount")

        self.stake[address] -= amount
        self.reward_tally[address] -= self.reward_per_token * amount
        self.total_stake -= amount

    def withdraw_reward(self, address):
        reward = self.compute_reward(address)
        self.reward_tally[address] = self.stake[address] * self.reward_per_token
        return reward
```

Without accounting for changing stake sizes we can reduce the reward calculation to the following formula:

```
rewards = stake * (total_rewards / total_stake)
```

### Example

1. Alice deposits 100 DOT in staking pool A.
2. Bob deposits 20 DOT in staking pool A.
3. Charlie deposits 50 DOT in staking pool B.

At this point in time, assuming we distribute 100 DOT in A and 100 DOT in B, we have the following reward claims:

- Alice may claim `100 * (100 / 120) = 83.33 DOT`.
- Bob may claim `20 * (100 / 120) = 16.66 DOT`.
- Charlie may claim `50 * (100 / 50) = 100 DOT`.

## Force Refunds

To allow Vaults to refund all Nominators at once, we index each staking pool by a nonce. Most operations will use the latest nonce when reading from storage, but Nominators must specify this to recover past stake or rewards. This allows for optimal complexity since no on-chain iteration is required.

- The nonce (`n`) is zero-initialized and incremented on force refund.
- Vault stake is re-distributed to the new staking pool at nonce `n+1`.
- Nominators may withdraw stake and rewards at nonce `n`.
