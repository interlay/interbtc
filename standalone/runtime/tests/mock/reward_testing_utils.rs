use crate::*;
use primitives::TruncateFixedPointToInt;
use std::collections::{BTreeMap, HashMap};

pub type StakeId = (VaultId, AccountId);

#[derive(Debug, Default)]
pub struct IdealRewardPool {
    exchange_rate: BTreeMap<CurrencyId, FixedU128>,
    secure_threshold: BTreeMap<VaultId, FixedU128>,
    accept_new_issues: BTreeMap<VaultId, bool>,
    commission: BTreeMap<VaultId, FixedU128>,
    collateral: BTreeMap<StakeId, FixedU128>,
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
        self.collateral
            .insert(account.clone(), current_collateral + FixedU128::from(amount));
        self
    }

    pub fn withdraw_nominator_collateral(&mut self, account: &StakeId, amount: u128) -> &mut Self {
        log::debug!("withdraw_nominator_collateral {amount}");
        let current_collateral = self.collateral.get(account).map(|x| *x).unwrap_or_default();
        self.collateral
            .insert(account.clone(), current_collateral - FixedU128::from(amount));
        self
    }

    pub fn slash_collateral(&mut self, account: &VaultId, amount: u128) -> &mut Self {
        log::error!("slash_collateral {amount}");
        let amount = FixedU128::from(amount);
        let nominators: Vec<_> = {
            self.collateral
                .iter()
                .filter(|((vault, _nominator), _stake)| vault == account)
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect()
        };
        let total_stake: FixedU128 = nominators
            .iter()
            .map(|(_key, value)| *value)
            .fold(Zero::zero(), |x: FixedU128, y: FixedU128| x + y);
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

        let vault_stakes: HashMap<_, _> = self
            .collateral
            .iter()
            .map(|((vault, _nominator), collateral)| (vault, collateral))
            .filter(|(vault, _)| *self.accept_new_issues.get(vault).unwrap_or(&true))
            .into_group_map()
            .into_iter()
            .map(|(vault, nominator_collaterals)| {
                let vault_collateral = nominator_collaterals
                    .into_iter()
                    .cloned()
                    .fold(Zero::zero(), |x: FixedU128, y: FixedU128| x + y);
                let threshold = self.secure_threshold[vault];
                let reward_stake = (vault_collateral / threshold).truncate_to_inner().unwrap();
                (vault, reward_stake)
            })
            .collect();

        let capacity_stakes: Vec<_> = vault_stakes
            .iter()
            .map(|(vault, stake)| (vault.collateral_currency(), stake))
            .into_group_map()
            .into_iter()
            .map(|(currency, vault_stakes)| {
                let currency_capacity: u128 = vault_stakes.into_iter().sum();
                let exchange_rate = self.exchange_rate[&currency];
                let capacity_stake = (FixedU128::from(currency_capacity) / exchange_rate)
                    .truncate_to_inner()
                    .unwrap();
                (currency, capacity_stake)
            })
            .collect();

        log::error!("Capacity_stakes: {capacity_stakes:?}");

        let total_capacity: u128 = capacity_stakes.iter().map(|(_, capacity)| capacity).sum();
        for (currency, capacity_stake) in capacity_stakes {
            let currency_reward = (reward * FixedU128::from(capacity_stake)) / FixedU128::from(total_capacity);
            let currency_reward = currency_reward.trunc();
            // reward for this currency = reward * (capacity_stake / total_capacity)
            let vaults: Vec<_> = vault_stakes
                .iter()
                .filter(|(vault, _)| vault.collateral_currency() == currency)
                .collect();
            let total_vault_stake: u128 = vaults.iter().map(|(_, stake)| **stake).sum();
            for vault_stake in vaults.iter() {
                let nominators: Vec<_> = self
                    .nominations()
                    .iter()
                    .cloned()
                    .filter(|((vault, _nominator), _stake)| &vault == vault_stake.0)
                    .map(|((_vault, nominator), stake)| (nominator, stake))
                    .collect();
                let total_nomination = nominators
                    .iter()
                    .map(|(_, nomination)| *nomination)
                    .fold(Zero::zero(), |x: FixedU128, y: FixedU128| x + y);
                let vault_reward =
                    (currency_reward * FixedU128::from(*vault_stake.1)) / FixedU128::from(total_vault_stake);
                let vault_reward = vault_reward.trunc();
                log::error!("vault_reward: {}", vault_reward.truncate_to_inner().unwrap());

                let commission = self.commission.get(vault_stake.0).cloned().unwrap_or_default();

                let vault = vault_stake.0.clone();
                let (vault_commission, old_vault_reward) =
                    self.rewards.get(&vault.account_id).cloned().unwrap_or_default();
                self.rewards.insert(
                    vault.account_id.clone(),
                    (vault_commission + vault_reward * commission, old_vault_reward),
                );

                for (nominator_id, nomination) in nominators {
                    let nominator_reward = (vault_reward - vault_reward * commission) * nomination / total_nomination;

                    let (nominator_commission, old_nominator_reward) =
                        self.rewards.get(&nominator_id).cloned().unwrap_or_default();
                    self.rewards.insert(
                        nominator_id.clone(),
                        (nominator_commission, old_nominator_reward + nominator_reward),
                    );
                }
            }
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
            .fold(Zero::zero(), |x: FixedU128, y: FixedU128| x + y)
            .truncate_to_inner()
            .unwrap()
    }

    pub fn nominations(&self) -> Vec<(StakeId, FixedU128)> {
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
