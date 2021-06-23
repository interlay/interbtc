class StakingDistribution:
    def __init__(self):
        self.total_stake = 0
        self.reward_per_token = 0
        self.stake = {}
        self.reward_tally = {}
        self.total_slashed_stake = 0
        self.slash_per_token = 0
        self.slash_tally = {}

    def deposit_stake(self, address, amount):
        if address not in self.stake:
            self.stake[address] = 0
            self.reward_tally[address] = 0
            self.slash_tally[address] = 0

        self.stake[address] = self.stake[address] + amount
        self.reward_tally[address] = self.reward_tally[address] + self.reward_per_token * amount
        self.slash_tally[address] = self.slash_tally[address] + self.slash_per_token * amount
        self.total_stake = self.total_stake + amount
        self.total_slashed_stake = self.total_slashed_stake + amount

    def slash_stake(self, amount):
        self.total_slashed_stake = self.total_slashed_stake - amount
        self.slash_per_token = self.slash_per_token + amount / self.total_stake
        self.reward_per_token = self.reward_per_token + self.reward_per_token * amount / self.total_slashed_stake

    def compute_stake(self, address):
        to_slash = self.stake[address] * self.slash_per_token - self.slash_tally[address]
        return max(0, self.stake[address] - to_slash)

    def distribute_reward(self, reward):
        if self.total_stake == 0:
            raise Exception("Cannot distribute with 0 stake")

        self.reward_per_token = self.reward_per_token + reward / self.total_slashed_stake

    def compute_reward(self, address):
        return self.compute_stake(address) * self.reward_per_token - self.reward_tally[address]

    def withdraw_stake(self, address, amount):
        to_slash = self.stake[address] * self.slash_per_token - self.slash_tally[address]
        self.stake[address] = self.stake[address] - to_slash
        self.total_stake = self.total_stake - to_slash
        self.slash_tally[address] = self.stake[address] * self.slash_per_token
        
        if amount > self.stake[address]:
            raise Exception("Requested amount greater than staked amount")

        self.stake[address] = self.stake[address] - amount
        self.reward_tally[address] = self.reward_tally[address] - self.reward_per_token * amount
        self.slash_tally[address] = self.slash_tally[address] - self.slash_per_token * amount
        self.total_stake = self.total_stake - amount
        self.total_slashed_stake = self.total_slashed_stake - amount


addr1 = 0x1
addr2 = 0x2
addr3 = 0x3

contract = StakingDistribution()

contract.deposit_stake(addr1, 10000)
contract.deposit_stake(addr2, 10000)

contract.distribute_reward(1000)
# reward should be 500 and 500
contract.slash_stake(50)
contract.slash_stake(50)
# stake should be 9950 and 9950
contract.deposit_stake(addr1, 1000)
# stake should be 10950 and 9950
contract.distribute_reward(1000)

# 500 is the first reward each get, because they have equal stake
# then, we scale the next 1000 reward by the stake for each address
expected_reward_1 = 500 + 1000 * (10950 / (10950 + 9950))
expected_reward_2 = 500 + 1000 * (9950 / (10950 + 9950))

assert(contract.compute_stake(addr1) == 10950)
assert(contract.compute_stake(addr2) == 9950)
assert(contract.compute_stake(addr1) + contract.compute_stake(addr2) == 20900)

assert(contract.compute_reward(addr1) == expected_reward_1)
assert(contract.compute_reward(addr2) == expected_reward_2)
assert(contract.compute_reward(addr1) + contract.compute_reward(addr2) == expected_reward_1 + expected_reward_2)

contract.withdraw_stake(addr1, 10000)
contract.withdraw_stake(addr1, 950)
contract.deposit_stake(addr2, 10000)
contract.distribute_reward(1000)
contract.slash_stake(10000)

# print(contract.compute_stake(addr1))
# print(contract.compute_stake(addr2))
# print(contract.compute_reward(addr1))
# print(contract.compute_reward(addr2))

assert(contract.compute_stake(addr1) == 0)
assert(round(contract.compute_stake(addr2)) == 9950)

assert(round(contract.compute_reward(addr1)) == 1024)
assert(round(contract.compute_reward(addr2)) == 1976)
