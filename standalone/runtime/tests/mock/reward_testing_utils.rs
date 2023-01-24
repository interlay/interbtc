use crate::*;
use primitives::TruncateFixedPointToInt;
use std::collections::BTreeMap;

pub type StakeId = (VaultId, AccountId);

#[derive(Debug, Default)]
pub struct IdealRewardPool {
    exchange_rate: BTreeMap<CurrencyId, FixedU128>,
    secure_threshold: BTreeMap<VaultId, FixedU128>,
    accept_new_issues: BTreeMap<VaultId, bool>,
    commission: BTreeMap<VaultId, FixedU128>,
    collateral: BTreeMap<StakeId, u128>,
    rewards: BTreeMap<AccountId, (FixedU128, FixedU128)>,
}

impl IdealRewardPool {
    pub fn set_commission(&mut self, vault: &VaultId, rate: FixedU128) -> &mut Self {
        log::debug!("set_commission {:?}", rate);
        self.commission.insert(vault.clone(), rate);
        self
    }

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

    pub fn deposit_nominator_collateral(&mut self, account: &StakeId, amount: u128) -> &mut Self {
        log::debug!("deposit_nominator_collateral {amount}");
        let current_collateral = self.collateral.get(account).map(|x| *x).unwrap_or_default();
        self.collateral.insert(account.clone(), current_collateral + amount);
        self
    }

    pub fn withdraw_nominator_collateral(&mut self, account: &StakeId, amount: u128) -> &mut Self {
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
            .map(|(stake_id, _)| self.stake(stake_id))
            .fold(Zero::zero(), |x: FixedU128, y: FixedU128| x + y);

        if total_stake.is_zero() {
            return self;
        }

        for (stake_id, _) in self.collateral.iter() {
            let stake = self.stake(stake_id);
            let reward = (stake * reward) / total_stake;

            let (vault_id, nominator_id) = stake_id;

            let commission = self.commission.get(vault_id).cloned().unwrap_or_default();

            let (vault_commission, vault_reward) = self.rewards.get(&vault_id.account_id).cloned().unwrap_or_default();
            self.rewards.insert(
                vault_id.account_id.clone(),
                (vault_commission + reward * commission, vault_reward),
            );

            let (nominator_commission, nominator_reward) = self.rewards.get(&nominator_id).cloned().unwrap_or_default();
            self.rewards.insert(
                nominator_id.clone(),
                (nominator_commission, nominator_reward + (reward - reward * commission)),
            );
        }
        self
    }

    pub fn accept_new_issues(&mut self, vault: &VaultId, accept_new_issues: bool) -> &mut Self {
        log::debug!("accept_new_issues {:?}", accept_new_issues);
        self.accept_new_issues.insert(vault.clone(), accept_new_issues);
        self
    }

    pub fn get_total_reward_for(&self, account: &crate::AccountId) -> u128 {
        self.rewards
            .get(account)
            .map(|x| x.1)
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

    pub fn nominations(&self) -> Vec<(StakeId, u128)> {
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
            .map(|(key, (commission, rewards))| (key.clone(), (*commission + *rewards).truncate_to_inner().unwrap()))
            .collect()
    }

    pub fn stake(&self, (vault_id, nominator_id): &StakeId) -> FixedU128 {
        let currency_id = vault_id.collateral_currency();
        if !self.accept_new_issues.get(vault_id).unwrap_or(&true) {
            Zero::zero()
        } else {
            let threshold = self.secure_threshold[vault_id];
            let exchange_rate = self.exchange_rate[&currency_id];
            let collateral = self.collateral[&(vault_id.clone(), nominator_id.clone())];
            FixedU128::from(collateral) / threshold / exchange_rate
        }
    }
}
