use crate::*;
use std::collections::BTreeMap;

#[derive(Clone, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub enum StakeHolder {
    Vault(VaultId),
    Nominator(crate::AccountId),
}
type Balance = f64;

#[derive(Debug, Default)]
pub struct BasicRewardPool {
    // note: we use BTreeMaps such that the debug print output is sorted, for easier diffing
    stake: BTreeMap<StakeHolder, Balance>,
    reward_tally: BTreeMap<StakeHolder, Balance>,
    total_stake: Balance,
    reward_per_token: Balance,
}

impl BasicRewardPool {
    pub fn deposit_stake(&mut self, account: &StakeHolder, amount: Balance) -> &mut Self {
        let stake = self.stake.remove(account).unwrap_or(0.0);
        self.stake.insert(account.clone(), stake + amount);
        let reward_tally = self.reward_tally.remove(&account).unwrap_or(0.0);
        self.reward_tally
            .insert(account.clone(), reward_tally + self.reward_per_token * amount);
        self.total_stake += amount;
        self
    }

    pub fn withdraw_stake(&mut self, account: &StakeHolder, amount: Balance) -> &mut Self {
        let stake = self.stake.remove(account).unwrap_or(0.0);
        if stake - amount < 0 as f64 {
            return self;
        }
        self.stake.insert(account.clone(), stake - amount);
        let reward_tally = self.reward_tally.remove(account).unwrap_or(0.0);
        self.reward_tally
            .insert(account.clone(), reward_tally - self.reward_per_token * amount);
        self.total_stake -= amount;
        self
    }

    pub fn distribute(&mut self, reward: Balance) -> &mut Self {
        self.reward_per_token = self.reward_per_token + reward / self.total_stake;
        self
    }

    pub fn compute_reward(&self, account: &StakeHolder) -> Balance {
        self.stake.get(account).cloned().unwrap_or(0.0) * self.reward_per_token
            - self.reward_tally.get(account).cloned().unwrap_or(0.0)
    }

    pub fn withdraw_reward(&mut self, account: &StakeHolder) -> &mut Self {
        let stake = self.stake.get(account).unwrap_or(&0.0);
        self.reward_tally.insert(account.clone(), self.reward_per_token * stake);
        self
    }
}
