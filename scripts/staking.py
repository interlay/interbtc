# Simplified staking model that only has a single vault

class Staking:
    def __init__(self):
        self.total_stake = 0
        self.total_current_stake = 0
        self.reward_per_token = 0
        self.reward_tally = {'alice': 0, 'bob': 0}
        self.stake = {'alice': 0, 'bob': 0}
        self.slash_tally = {'alice': 0, 'bob': 0}
        self.slash_per_token = 0

    def apply_slash(self, nominator):
        self.total_stake -= self.stake[nominator] * self.slash_per_token - self.slash_tally[nominator];
        self.stake[nominator] -= self.stake[nominator] * self.slash_per_token - self.slash_tally[nominator];
        self.slash_tally[nominator] = self.stake[nominator] * self.slash_per_token;

    def distribute_reward(self, nominator, x):
        self.reward_per_token += x / self.total_current_stake;

    def withdraw_reward(self, nominator):
        self.apply_slash(nominator);
        withdrawal_reward = self.stake[nominator] * self.reward_per_token - self.reward_tally[nominator];
        self.reward_tally[nominator] = self.stake[nominator] * self.reward_per_token;
        return withdrawal_reward

    def deposit_stake(self, nominator, x):
        self.apply_slash(nominator);
        self.stake[nominator] += x;
        self.total_stake += x;
        self.total_current_stake += x;
        self.slash_tally[nominator] += self.slash_per_token * x;
        self.reward_tally[nominator] += self.reward_per_token * x;

        self.reward_per_token += x / self.total_current_stake;

    def withdraw_stake(self, nominator, x):
        self.deposit_stake(nominator, -x)

    def slash_stake(self, x):
        self.slash_per_token += x / self.total_stake;
        self.total_current_stake -= x;
        self.reward_per_token += (self.reward_per_token * x) / self.total_current_stake;

    def compute_stake(self, nominator):
        return self.stake[nominator] - (self.stake[nominator] * self.slash_per_token - self.slash_tally[nominator])

staking = Staking()
alice = "alice"
bob = "bob"

# step 1: initial setup
staking.deposit_stake(alice, 50);
assert(staking.compute_stake(alice) == 50);

# step 2: slash
staking.slash_stake(30);
assert(staking.compute_stake(alice) == 20);

# step 3: add nominator. Both should have equal stake
staking.deposit_stake(bob, 20);
assert(staking.compute_stake(alice) == 20);
assert(staking.compute_stake(bob) == 20);

# step 4: slash stake. Both should lose equal amount of stake
staking.slash_stake(10);
# for Alice, SlashPerToken should be set to 0.7,  s.t. toSlash = 50*0.7 - 0 = 35
# for Bob,   SlashPerToken should be set to 0.85, s.t. toSlash = 20*0.85 - 12 = 5

print('actual stake', staking.compute_stake(alice)) # prints 12.857142857142854
assert(staking.compute_stake(alice) == 15); # fail!
assert(staking.compute_stake(bob) == 15);