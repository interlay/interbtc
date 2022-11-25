use primitives::TruncateFixedPointToInt;
use sp_runtime::traits::CheckedDiv;

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
    exchange_rate: BTreeMap<CurrencyId, FixedU128>,
    secure_threshold: BTreeMap<VaultId, FixedU128>,
    collateral: BTreeMap<StakerId, u128>,
    rewards: BTreeMap<crate::AccountId, FixedU128>,
}

impl IdealRewardPool {
    pub fn set_secure_threshold(&mut self, vault: &VaultId, threshold: FixedU128) -> &mut Self {
        log::debug!("set_secure_threshold {:?}", threshold);
        self.secure_threshold.insert(vault.clone(), threshold);
        self
    }

    pub fn set_exchange_rate(&mut self, currency_id: CurrencyId, rate: FixedU128) -> &mut Self {
        log::debug!("set_exchange_rate({:?}) {:?}", currency_id, rate);
        self.exchange_rate.insert(currency_id, rate);
        self
    }

    pub fn deposit_nominator_collateral(&mut self, account: &StakerId, amount: u128) -> &mut Self {
        log::debug!("deposit_nominator_collateral {amount}");
        let current_collateral = self.collateral.get(account).map(|x| *x).unwrap_or_default();
        self.collateral.insert(account.clone(), current_collateral + amount);
        self
    }

    pub fn withdraw_nominator_collateral(&mut self, account: &StakerId, amount: u128) -> &mut Self {
        log::debug!("withdraw_nominator_collateral {amount}");
        let current_collateral = self.collateral.get(account).map(|x| *x).unwrap_or_default();
        self.collateral.insert(account.clone(), current_collateral - amount);
        self
    }

    pub fn slash_collateral(&mut self, account: &VaultId, amount: u128) -> &mut Self {
        log::error!("slash_collateral {amount}");
        let nominators: Vec<_> = {
            self.collateral
                .iter()
                .filter(|((vault, _nominator), _stake)| vault == account)
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect()
        };
        let total_stake: u128 = nominators.iter().map(|(_key, value)| *value).sum();
        for (key, stake) in nominators {
            let new_stake = stake - (stake * amount) / total_stake;
            self.collateral.insert(key, new_stake);
        }
        self
    }

    pub fn distribute_reward(&mut self, reward: u128) -> &mut Self {
        log::debug!("distribute_reward {reward}");
        let reward = FixedU128::from(reward);
        let total_stake: FixedU128 = self
            .collateral
            .iter()
            .map(|(staker_id, _)| self.stake(staker_id))
            .fold(Zero::zero(), |x: FixedU128, y: FixedU128| x + y);

        for (staker_id, _) in self.collateral.iter() {
            let stake = self.stake(staker_id);
            let reward = (stake * reward) / total_stake;

            let (vault_id, nominator_id) = staker_id;

            let vault_reward = self.rewards.get(&vault_id.account_id).map(|x| *x).unwrap_or_default();
            self.rewards.insert(
                vault_id.account_id.clone(),
                vault_reward + reward * FixedU128::from_float(0.75),
            );

            let nominator_reward = self.rewards.get(&nominator_id).map(|x| *x).unwrap_or_default();
            self.rewards.insert(
                nominator_id.clone(),
                nominator_reward + reward * FixedU128::from_float(0.25),
            );
        }
        self
    }

    pub fn get_total_reward_for(&self, account: &crate::AccountId) -> u128 {
        self.rewards
            .get(account)
            .map(|x| *x)
            .unwrap_or_default()
            .truncate_to_inner()
            .unwrap()
    }
    pub fn get_nominator_collateral(&self, account: &crate::AccountId, currency_id: CurrencyId) -> u128 {
        self.collateral
            .iter()
            .filter(|((vault, nominator), _stake)| nominator == account && vault.collateral_currency() == currency_id)
            .map(|(_key, value)| *value)
            .sum()
    }
    pub fn nominations(&self) -> Vec<(StakerId, u128)> {
        self.collateral
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }
    pub fn vaults(&self) -> Vec<VaultId> {
        self.secure_threshold.iter().map(|(key, _)| key.clone()).collect()
    }
    pub fn rewards(&self) -> Vec<(crate::AccountId, u128)> {
        self.rewards
            .iter()
            .map(|(key, value)| (key.clone(), value.truncate_to_inner().unwrap()))
            .collect()
    }
    pub fn stake(&self, (vault_id, nominator_id): &StakerId) -> FixedU128 {
        let currency_id = vault_id.collateral_currency();
        let threshold = self.secure_threshold[vault_id];
        let exchange_rate = self.exchange_rate[&currency_id];
        let collateral = self.collateral[&(vault_id.clone(), nominator_id.clone())];
        FixedU128::from(collateral) / exchange_rate / threshold
    }
}
