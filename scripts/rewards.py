class Vault:
    def __init__(self, address, currency):
        self.address = address
        self.currency = currency
        self.secure_threshold = 0

    def set_secure_threshold(self, value):
        self.secure_threshold = value


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

        self.stake[address] = self.stake[address] + amount
        self.reward_tally[address] = self.reward_tally[address] + \
            self.reward_per_token * amount
        self.total_stake = self.total_stake + amount

    def distribute(self, reward):
        if reward == 0:
            return
        if self.total_stake == 0:
            raise Exception("Invalid total stake")

        self.reward_per_token = self.reward_per_token + reward / self.total_stake

    def compute_reward(self, address):
        return self.stake[address] * self.reward_per_token - self.reward_tally[address]

    def withdraw_stake(self, address, amount):
        if address not in self.stake:
            raise Exception("Stake not found")

        if amount > self.stake[address]:
            raise Exception("Amount is too high")

        self.stake[address] = self.stake[address] - amount
        self.reward_tally[address] = self.reward_tally[address] - \
            self.reward_per_token * amount
        self.total_stake = self.total_stake - amount
        return amount

    def withdraw_reward(self, address):
        reward = self.compute_reward(address)
        self.reward_tally[address] = self.stake[address] * \
            self.reward_per_token
        return reward

    def update_stake(self, address, amount):
        if amount > 0:
            self.deposit_stake(address, amount)
        else:
            self.withdraw_stake(address, abs(amount))


class RewardsRouter:
    def __init__(self):
        self.root = Rewards()
        self.leaf = {}

    def update_root_stake(self, root_key, root_value):
        self.root.update_stake(root_key, root_value)

    def update_stake(self, root_key, root_value, leaf_key, leaf_value):
        if root_key not in self.leaf:
            self.leaf[root_key] = Rewards()

        self.root.update_stake(root_key, root_value)
        reward = self.root.withdraw_reward(root_key)
        self.leaf[root_key].distribute(reward)
        self.leaf[root_key].update_stake(leaf_key, leaf_value)

    def distribute(self, reward):
        self.root.distribute(reward)

    def withdraw_reward(self, root_key, leaf_key):
        reward = self.root.withdraw_reward(root_key)
        self.leaf[root_key].distribute(reward)
        return self.leaf[root_key].withdraw_reward(leaf_key)


class VaultRegistry:
    def __init__(self):
        self.exchange_rate = {}
        self.secure_threshold = {}

        self.rewards = RewardsRouter()

        self.collateral = {}
        self.total_collateral_div_threshold = {}

    # relative to btc
    def set_exchange_rate(self, currency, value):
        if currency not in self.exchange_rate:
            self.exchange_rate[currency] = value
            return

        previous_collateral_capacity = self.get_collateral_capacity(currency)
        self.exchange_rate[currency] = value
        collateral_capacity = self.get_collateral_capacity(currency)
        capacity_delta = collateral_capacity - previous_collateral_capacity
        self.rewards.update_root_stake(currency, capacity_delta)

    def set_global_secure_threshold(self, currency, value):
        # this is tricky to update without summing
        self.secure_threshold[currency] = value

    def get_secure_threshold(self, vault):
        return max(self.secure_threshold[vault.currency], vault.secure_threshold)

    def update_collateral_and_threshold(self, vault, amount, secure_threshold):
        currency = vault.currency

        if secure_threshold is None:
            secure_threshold = self.get_secure_threshold(vault)

        previous_collateral_capacity = self.get_collateral_capacity(currency)
        previous_secure_threshold = self.get_secure_threshold(vault)
        previous_collateral_div_threshold = self.collateral[vault] / \
            previous_secure_threshold

        self.collateral[vault] += amount
        collateral_div_threshold = self.collateral[vault] / secure_threshold
        collateral_div_threshold_delta = collateral_div_threshold - \
            previous_collateral_div_threshold
        self.total_collateral_div_threshold[currency] += collateral_div_threshold_delta

        collateral_capacity = self.get_collateral_capacity(currency)
        capacity_delta = collateral_capacity - previous_collateral_capacity

        self.rewards.update_stake(
            currency,
            capacity_delta,
            vault.address,
            collateral_div_threshold_delta
        )

    def set_custom_secure_threshold(self, vault, value):
        secure_threshold = max(self.secure_threshold[vault.currency], value)
        self.update_collateral_and_threshold(vault, 0, secure_threshold)
        vault.set_secure_threshold(value)

    def deposit_collateral(self, vault, amount):
        currency = vault.currency
        if vault not in self.collateral:
            self.collateral[vault] = 0
        if currency not in self.total_collateral_div_threshold:
            self.total_collateral_div_threshold[currency] = 0

        self.update_collateral_and_threshold(vault, amount, None)

    def withdraw_collateral(self, vault, amount):
        self.update_collateral_and_threshold(vault, -abs(amount), None)

    def get_collateral_capacity(self, currency):
        return self.total_collateral_div_threshold[currency] \
            / self.exchange_rate[currency]

    def distribute(self, reward):
        self.rewards.distribute(reward)

    def withdraw_reward(self, vault):
        return self.rewards.withdraw_reward(vault.currency, vault.address)


# +++++++++++++
# + EXAMPLE 1 +
# +++++++++++++

vault_registry = VaultRegistry()

vault1 = Vault(0x1, 'DOT')
vault2 = Vault(0x2, 'KSM')

vault_registry.set_exchange_rate('DOT', 1000)
vault_registry.set_exchange_rate('KSM', 500)

vault_registry.set_global_secure_threshold('DOT', 200/100)
vault_registry.set_global_secure_threshold('KSM', 200/100)

vault_registry.deposit_collateral(vault1, 2000)
vault_registry.deposit_collateral(vault2, 1000)
assert (vault_registry.get_collateral_capacity('DOT') == 1.0)
assert (vault_registry.get_collateral_capacity('KSM') == 1.0)

# equal capacity = equal rewards
vault_registry.distribute(10)
assert (vault_registry.withdraw_reward(vault1) == 5.0)
assert (vault_registry.withdraw_reward(vault2) == 5.0)

# double DOT minting capacity
vault_registry.set_exchange_rate('DOT', 500)
assert (vault_registry.get_collateral_capacity('DOT') == 2.0)
assert (vault_registry.get_collateral_capacity('KSM') == 1.0)

# vault1 now receives more rewards
vault_registry.distribute(10)
assert (vault_registry.withdraw_reward(vault1) == 6.66666666666667)
assert (vault_registry.withdraw_reward(vault2) == 3.3333333333333357)

# +++++++++++++
# + EXAMPLE 2 +
# +++++++++++++

vault_registry = VaultRegistry()
vault1 = Vault(0x1, 'DOT')
vault2 = Vault(0x2, 'DOT')
vault_registry.set_exchange_rate('DOT', 1000)
vault_registry.set_global_secure_threshold('DOT', 200/100)

vault_registry.deposit_collateral(vault1, 1000)
vault_registry.deposit_collateral(vault2, 1000)
assert (vault_registry.get_collateral_capacity('DOT') == 1.0)

# equal capacity = equal rewards
vault_registry.distribute(10)
assert (vault_registry.withdraw_reward(vault1) == 5.0)
assert (vault_registry.withdraw_reward(vault2) == 5.0)

# vault1 sets higher custom threshold
vault_registry.set_custom_secure_threshold(vault1, 300/100)
assert (vault_registry.get_collateral_capacity('DOT') == 0.8333333333333333)

# vault1 now receives less rewards
vault_registry.distribute(10)
assert (vault_registry.withdraw_reward(vault1) == 3.999999999999999)
assert (vault_registry.withdraw_reward(vault2) == 6.0)
