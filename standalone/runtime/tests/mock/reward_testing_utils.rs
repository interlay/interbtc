use crate::*;
use primitives::TruncateFixedPointToInt;
use std::collections::BTreeMap;

type Balance = f64;

#[derive(Clone, Debug)]
pub struct BasicRewardPool<StakeId> {
    // note: we use BTreeMaps such that the debug print output is sorted, for easier diffing
    stake: BTreeMap<StakeId, Balance>,
    reward_tally: BTreeMap<StakeId, Balance>,
    total_stake: Balance,
    reward_per_token: Balance,
}

impl<StakeId> Default for BasicRewardPool<StakeId> {
    fn default() -> Self {
        Self {
            stake: BTreeMap::new(),
            reward_tally: BTreeMap::new(),
            total_stake: 0.0,
            reward_per_token: 0.0,
        }
    }
}

impl<StakeId: Clone + Ord> BasicRewardPool<StakeId> {
    pub fn deposit_stake(&mut self, account: &StakeId, amount: Balance) -> &mut Self {
        let stake = self.stake.remove(account).unwrap_or(0.0);
        self.stake.insert(account.clone(), stake + amount);
        let reward_tally = self.reward_tally.remove(&account).unwrap_or(0.0);
        self.reward_tally
            .insert(account.clone(), reward_tally + self.reward_per_token * amount);
        self.total_stake += amount;
        self
    }

    pub fn withdraw_stake(&mut self, account: &StakeId, amount: Balance) -> &mut Self {
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

    pub fn set_stake(&mut self, account: &StakeId, amount: Balance) -> &mut Self {
        let current_stake = self.stake.get(account).cloned().unwrap_or_default();
        if current_stake < amount {
            let additional_stake = amount - current_stake;
            self.deposit_stake(account, additional_stake)
        } else if current_stake > amount {
            let surplus_stake = current_stake - amount;
            self.withdraw_stake(account, surplus_stake)
        } else {
            self
        }
    }

    pub fn distribute(&mut self, reward: Balance) -> &mut Self {
        let reward_per_token = self.reward_per_token + reward / self.total_stake;
        self.reward_per_token = reward_per_token.is_nan().then_some(0.0).unwrap_or(reward_per_token);
        self
    }

    pub fn compute_reward(&self, account: &StakeId) -> Balance {
        self.stake.get(account).cloned().unwrap_or(0.0) * self.reward_per_token
            - self.reward_tally.get(account).cloned().unwrap_or(0.0)
    }

    pub fn withdraw_reward(&mut self, account: &StakeId) -> Balance {
        let reward = self.compute_reward(account);
        let stake = self.stake.get(account).cloned().unwrap_or(0.0);
        self.reward_tally.insert(account.clone(), self.reward_per_token * stake);
        reward
    }
}

#[derive(Debug, Default)]
pub struct BasicVaultRegistry {
    // reward pools
    capacity_rewards: BasicRewardPool<CurrencyId>,
    vault_rewards: BTreeMap<CurrencyId, BasicRewardPool<VaultId>>,
    staking_rewards: BTreeMap<VaultId, BasicRewardPool<AccountId>>,

    // vault registry / fee specific
    exchange_rate: BTreeMap<CurrencyId, Balance>,
    secure_threshold: BTreeMap<VaultId, Balance>,
    commission_rate: BTreeMap<VaultId, Balance>,
}

impl BasicVaultRegistry {
    pub fn set_secure_threshold(&mut self, vault_id: &VaultId, threshold: Balance) -> &mut Self {
        self.secure_threshold.insert(vault_id.clone(), threshold);
        self
    }

    pub fn set_exchange_rate(&mut self, currency_id: CurrencyId, rate: Balance) -> &mut Self {
        self.exchange_rate.insert(currency_id, rate);
        self
    }

    pub fn set_commission_rate(&mut self, vault_id: &VaultId, commission: Balance) -> &mut Self {
        self.commission_rate.insert(vault_id.clone(), commission);
        self
    }

    fn with_mut_vault_rewards<U>(&mut self, vault_id: &VaultId, f: impl Fn(&mut BasicRewardPool<VaultId>) -> U) -> U {
        let currency_id = vault_id.currencies.collateral;
        let mut reward_pool = self.vault_rewards.get(&currency_id).cloned().unwrap_or_default();
        let ret = f(&mut reward_pool);
        self.vault_rewards.insert(currency_id, reward_pool);
        ret
    }

    fn with_mut_staking_rewards<U>(
        &mut self,
        vault_id: &VaultId,
        f: impl Fn(&mut BasicRewardPool<AccountId>) -> U,
    ) -> U {
        let mut reward_pool = self.staking_rewards.get(vault_id).cloned().unwrap_or_default();
        let ret = f(&mut reward_pool);
        self.staking_rewards.insert(vault_id.clone(), reward_pool);
        ret
    }

    fn drain_rewards(&mut self, vault_id: &VaultId) {
        let reward = self.capacity_rewards.withdraw_reward(&vault_id.currencies.collateral);

        let reward = self.with_mut_vault_rewards(vault_id, |reward_pool| {
            reward_pool.distribute(reward);
            reward_pool.withdraw_reward(vault_id)
        });

        let commission = self
            .commission_rate
            .get(vault_id)
            .map_or(0.0, |commission_rate| reward * commission_rate);
        let remainder = reward - commission;

        self.with_mut_staking_rewards(vault_id, |reward_pool| {
            reward_pool.distribute(remainder);
        });
    }

    fn update_capacity_stake(&mut self, currency_id: &CurrencyId) -> &mut Self {
        // TotalCollateralDivThreshold / ExchangeRate
        let total_stake = self
            .vault_rewards
            .get(currency_id)
            .map_or(0.0, |reward_pool| reward_pool.total_stake);
        let rate = self.exchange_rate[&currency_id];
        let capacity_stake = total_stake / rate;

        self.capacity_rewards.set_stake(currency_id, capacity_stake);
        self
    }

    fn update_reward_stake(&mut self, vault_id: &VaultId) -> &mut Self {
        let total_stake = self
            .staking_rewards
            .get(vault_id)
            .map(|reward_pool| reward_pool.total_stake)
            .unwrap_or_default();
        let threshold = self.secure_threshold[vault_id];

        // Collateral / SecureThreshold
        self.with_mut_vault_rewards(vault_id, |reward_pool| {
            reward_pool.set_stake(vault_id, total_stake / threshold);
        });

        self.update_capacity_stake(&vault_id.currencies.collateral)
    }

    pub fn deposit_collateral(&mut self, vault_id: &VaultId, nominator_id: &AccountId, amount: Balance) -> &mut Self {
        self.drain_rewards(vault_id);
        self.with_mut_staking_rewards(vault_id, |reward_pool| {
            reward_pool.deposit_stake(nominator_id, amount);
        });
        self.update_reward_stake(vault_id)
    }

    pub fn distribute(&mut self, amount: Balance) -> &mut Self {
        self.capacity_rewards.distribute(amount);
        self
    }

    pub fn withdraw_reward(&mut self, vault_id: &VaultId, nominator_id: &AccountId) -> Balance {
        self.drain_rewards(vault_id);
        self.with_mut_staking_rewards(vault_id, |reward_pool| reward_pool.withdraw_reward(nominator_id))
    }
}

pub type StakeId = (VaultId, AccountId);

#[derive(Debug, Default)]
pub struct IdealRewardPool {
    exchange_rate: BTreeMap<CurrencyId, FixedU128>,
    secure_threshold: BTreeMap<VaultId, FixedU128>,
    collateral: BTreeMap<StakeId, u128>,
    rewards: BTreeMap<AccountId, FixedU128>,
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
            .map(|(key, value)| (key.clone(), value.truncate_to_inner().unwrap()))
            .collect()
    }

    pub fn stake(&self, (vault_id, nominator_id): &StakeId) -> FixedU128 {
        let currency_id = vault_id.collateral_currency();
        let threshold = self.secure_threshold[vault_id];
        let exchange_rate = self.exchange_rate[&currency_id];
        let collateral = self.collateral[&(vault_id.clone(), nominator_id.clone())];
        FixedU128::from(collateral) / exchange_rate / threshold
    }
}
