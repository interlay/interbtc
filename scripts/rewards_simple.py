class Vault:
    def __init__(self, address, currency):
        self.address = address
        self.currency = currency


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


class VaultRegistry:
    def __init__(self):
        self.exchange_rate = {}
        self.secure_threshold = {}

        self.collateral = {}
        self.vault_rewards = Rewards()

    # relative to btc
    def set_exchange_rate(self, currency, value):
        self.exchange_rate[currency] = value

    def set_secure_threshold(self, currency, value):
        self.secure_threshold[currency] = value

    def deposit_collateral(self, vault, amount):
        if vault not in self.collateral:
            self.collateral[vault] = 0

        self.collateral[vault] = self.collateral[vault] + amount

        capacity = 0
        if vault in self.vault_rewards.stake:
            capacity = self.vault_rewards.stake[vault]

        capacity_delta = self.vault_capacity(vault) - capacity
        self.vault_rewards.update_stake(vault, capacity_delta)

    def vault_capacity(self, vault):
        return self.collateral[vault] \
            / self.exchange_rate[vault.currency] \
            / self.secure_threshold[vault.currency]

    def distribute(self, reward):
        self.vault_rewards.distribute(reward)

    def compute_reward(self, vault):
        return self.vault_rewards.compute_reward(vault)


vault1 = Vault(0x1, 'DOT')
vault2 = Vault(0x2, 'KSM')

vault_registry = VaultRegistry()

# vault_registry.set_exchange_rate('DOT', 3068)
vault_registry.set_exchange_rate('DOT', 1000)
# vault_registry.set_exchange_rate('KSM', 567)
vault_registry.set_exchange_rate('KSM', 500)

vault_registry.set_secure_threshold('DOT', 200/100)
vault_registry.set_secure_threshold('KSM', 200/100)

vault_registry.deposit_collateral(vault1, 1000)
vault_registry.deposit_collateral(vault2, 500)

vault_registry.distribute(10)
print(vault_registry.compute_reward(vault1))
print(vault_registry.compute_reward(vault2))

vault_registry.set_exchange_rate('DOT', 500)
vault_registry.deposit_collateral(vault1, 1000)
vault_registry.distribute(10)
print(vault_registry.compute_reward(vault1))
print(vault_registry.compute_reward(vault2))
