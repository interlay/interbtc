use crate::SlaPallet;
use sp_arithmetic::FixedI128;
use sp_runtime::traits::{One, Zero};
use std::collections::HashMap;

pub const VAULT_REWARDS: f64 = 0.7;
pub const RELAYER_REWARDS: f64 = 0.2;

pub fn vault_rewards(amount: u128) -> u128 {
    (amount as f64 * VAULT_REWARDS) as u128
}

pub fn relayer_rewards(amount: u128) -> u128 {
    (amount as f64 * RELAYER_REWARDS) as u128
}

type AccountId = [u8; 32];
type Balance = f64;

#[derive(Debug, Default)]
pub struct BasicRewardPool {
    stake: HashMap<AccountId, Balance>,
    total_stake: Balance,
    reward_tally: HashMap<AccountId, Balance>,
    reward_per_token: Balance,
}

impl BasicRewardPool {
    pub fn deposit_stake(&mut self, account: AccountId, amount: Balance) -> &mut Self {
        let stake = self.stake.remove(&account).unwrap_or(0.0);
        self.stake.insert(account, stake + amount);
        let reward_tally = self.reward_tally.remove(&account).unwrap_or(0.0);
        self.reward_tally
            .insert(account, reward_tally + self.reward_per_token * amount);
        self.total_stake += amount;
        self
    }

    pub fn withdraw_stake(&mut self, account: AccountId, amount: Balance) -> &mut Self {
        let stake = self.stake.remove(&account).unwrap_or(0.0);
        if stake - amount < 0 as f64 {
            return self;
        }
        self.stake.insert(account, stake - amount);
        let reward_tally = self.reward_tally.remove(&account).unwrap_or(0.0);
        self.reward_tally
            .insert(account, reward_tally - self.reward_per_token * amount);
        self.total_stake -= amount;
        self
    }

    pub fn distribute(&mut self, reward: Balance) -> &mut Self {
        self.reward_per_token = self.reward_per_token + reward / self.total_stake;
        self
    }

    pub fn compute_reward(&self, account: AccountId) -> Balance {
        self.stake.get(&account).cloned().unwrap_or(0.0) * self.reward_per_token
            - self.reward_tally.get(&account).cloned().unwrap_or(0.0)
    }

    pub fn withdraw_reward(&mut self, account: AccountId) -> &mut Self {
        let stake = self.stake.get(&account).unwrap_or(&0.0);
        self.reward_tally.insert(account, self.reward_per_token * stake);
        self
    }
}

pub struct SlaBuilder {
    sla: FixedI128,
    average_deposit: FixedI128,
    average_deposit_count: FixedI128,
    max_sla_deposit: FixedI128,
}

impl Default for SlaBuilder {
    fn default() -> Self {
        Self {
            sla: FixedI128::from(0),
            average_deposit: FixedI128::from(0),
            average_deposit_count: FixedI128::from(0),
            max_sla_deposit: SlaPallet::vault_deposit_max_sla_change(),
        }
    }
}

fn limit(min: FixedI128, value: FixedI128, max: FixedI128) -> FixedI128 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

impl SlaBuilder {
    pub fn deposit_collateral(&mut self, amount: FixedI128) -> &mut Self {
        // new_average = (old_average * (n-1) + new_value) / n
        self.average_deposit_count = self.average_deposit_count + FixedI128::one();
        let n = self.average_deposit_count;
        self.average_deposit = (self.average_deposit * (n - FixedI128::one()) + amount) / n;

        // increase = (amount / average) * max_sla_change
        let increase = (amount / self.average_deposit) * self.max_sla_deposit;

        self.sla = self.sla + limit(FixedI128::zero(), increase, self.max_sla_deposit);
        self
    }

    pub fn get_sla(&mut self) -> FixedI128 {
        self.sla.clone()
    }
}
