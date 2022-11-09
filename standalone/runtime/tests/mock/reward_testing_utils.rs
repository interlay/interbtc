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

pub type StakerId = (VaultId, crate::AccountId);

#[derive(Debug, Default)]
pub struct IdealRewardPool {
    vault_stake: BTreeMap<VaultId, u128>,
    stake: BTreeMap<StakerId, u128>,
    rewards: BTreeMap<crate::AccountId, u128>,
}

impl IdealRewardPool {
    pub fn deposit_vault_stake(&mut self, vault: &VaultId, amount: u128) -> &mut Self {
        log::debug!("deposit_vault_stake {amount}");
        let current_stake = self.vault_stake.get(vault).map(|x| *x).unwrap_or_default();
        self.vault_stake.insert(vault.clone(), current_stake + amount);
        self
    }

    pub fn withdraw_vault_stake(&mut self, vault: &VaultId, amount: u128) -> &mut Self {
        log::debug!("withdraw_vault_stake {amount}");
        let current_stake = self.vault_stake.get(vault).map(|x| *x).unwrap_or_default();
        self.vault_stake.insert(vault.clone(), current_stake - amount);
        self
    }

    pub fn deposit_nominator_stake(&mut self, account: &StakerId, amount: u128) -> &mut Self {
        log::debug!("deposit_nominator_stake {amount}");
        let current_stake = self.stake.get(account).map(|x| *x).unwrap_or_default();
        self.stake.insert(account.clone(), current_stake + amount);
        self
    }

    pub fn withdraw_nominator_stake(&mut self, account: &StakerId, amount: u128) -> &mut Self {
        log::debug!("withdraw_nominator_stake {amount}");
        let current_stake = self.stake.get(account).map(|x| *x).unwrap_or_default();
        self.stake.insert(account.clone(), current_stake - amount);
        self
    }

    pub fn slash_stake(&mut self, account: &VaultId, amount: u128) -> &mut Self {
        let nominators: Vec<_> = {
            self.stake
                .iter()
                .filter(|((vault, _nominator), _stake)| vault == account)
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect()
        };
        let total_stake: u128 = nominators.iter().map(|(_key, value)| *value).sum();
        for (key, stake) in nominators {
            let new_stake = stake - (stake * amount) / total_stake;
            self.stake.insert(key, new_stake);
        }
        self
    }

    pub fn distribute_reward(&mut self, reward: u128) -> &mut Self {
        log::debug!("distribute_reward {reward}");
        let total_vault_stake: u128 = self.vault_stake.iter().map(|(_, value)| *value).sum();

        for (vault, &vault_stake) in self.vault_stake.iter() {
            let vault_reward = (vault_stake * reward) / total_vault_stake;

            let current_reward = self.rewards.get(&vault.account_id).map(|x| *x).unwrap_or_default();
            let operator_reward = (vault_reward * 3) / 4; // 75% commission
            self.rewards
                .insert(vault.account_id.clone(), current_reward + operator_reward);

            let nominators: Vec<_> = self
                .stake
                .iter()
                .filter(|((operator, _nominator), _stake)| operator == vault)
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect();
            let total_nominator_stake: u128 = nominators.iter().map(|(_key, value)| *value).sum();
            for ((_operator, nominator), nominator_stake) in nominators {
                let nominator_reward = ((nominator_stake * vault_reward) / total_nominator_stake) / 4;
                let current_reward = self.rewards.get(&nominator).map(|x| *x).unwrap_or_default();
                self.rewards.insert(nominator, current_reward + nominator_reward);
            }
        }
        self
    }

    pub fn get_total_reward_for(&self, account: &crate::AccountId) -> u128 {
        self.rewards.get(account).map(|x| *x).unwrap_or_default()
    }
    pub fn get_nominator_stake(&self, account: &crate::AccountId) -> u128 {
        self.stake
            .iter()
            .filter(|((_vault, nominator), _stake)| nominator == account)
            .map(|(_key, value)| *value)
            .sum()
    }
    pub fn nominations(&self) -> Vec<(StakerId, u128)> {
        self.stake
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }
    pub fn vaults(&self) -> Vec<VaultId> {
        self.vault_stake.iter().map(|(key, _)| key.clone()).collect()
    }
    pub fn rewards(&self) -> Vec<(crate::AccountId, u128)> {
        self.rewards
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }
}
