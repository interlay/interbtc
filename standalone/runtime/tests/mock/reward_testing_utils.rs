use crate::*;
use std::collections::BTreeMap;

#[derive(Clone, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub enum StakeHolder {
    Vault(VaultId),
    Nominator(AccountId),
}

type Balance = f64;

#[derive(Debug)]
pub struct BasicRewardPool<AccountId> {
    // note: we use BTreeMaps such that the debug print output is sorted, for easier diffing
    stake: BTreeMap<AccountId, Balance>,
    reward_tally: BTreeMap<AccountId, Balance>,
    total_stake: Balance,
    reward_per_token: Balance,
}

impl<AccountId> Default for BasicRewardPool<AccountId> {
    fn default() -> Self {
        Self {
            stake: BTreeMap::new(),
            reward_tally: BTreeMap::new(),
            total_stake: Default::default(),
            reward_per_token: Default::default(),
        }
    }
}

impl<AccountId: Ord + Clone> BasicRewardPool<AccountId> {
    pub fn deposit_stake(&mut self, account: &AccountId, amount: Balance) -> &mut Self {
        let stake = self.stake.remove(account).unwrap_or(0.0);
        self.stake.insert(account.clone(), stake + amount);
        let reward_tally = self.reward_tally.remove(&account).unwrap_or(0.0);
        self.reward_tally
            .insert(account.clone(), reward_tally + self.reward_per_token * amount);
        self.total_stake += amount;
        self
    }

    pub fn withdraw_stake(&mut self, account: &AccountId, amount: Balance) -> &mut Self {
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

    pub fn compute_reward(&self, account: &AccountId) -> Balance {
        self.stake.get(account).cloned().unwrap_or(0.0) * self.reward_per_token
            - self.reward_tally.get(account).cloned().unwrap_or(0.0)
    }

    pub fn withdraw_reward(&mut self, account: &AccountId) -> &mut Self {
        let stake = self.stake.get(account).unwrap_or(&0.0);
        self.reward_tally.insert(account.clone(), self.reward_per_token * stake);
        self
    }
}
