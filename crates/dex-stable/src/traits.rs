// Copyright 2021-2022 Zenlink.
// Licensed under Apache 2.0.

use super::*;

pub trait ValidateCurrency<CurrencyId> {
    fn validate_pooled_currency(a: &[CurrencyId]) -> bool;
    fn validate_pool_lp_currency(a: CurrencyId) -> bool;
}

pub trait StablePoolLpCurrencyIdGenerate<CurrencyId, PoolId> {
    fn generate_by_pool_id(pool_id: PoolId) -> CurrencyId;
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo)]
pub struct StablePair<PoolId, CurrencyId> {
    pub pool_id: PoolId,
    pub base_pool_id: PoolId,
    pub token0: CurrencyId,
    pub token1: CurrencyId,
}

impl<PoolId, CurrencyId> StablePair<PoolId, CurrencyId>
where
    CurrencyId: PartialEq,
    PoolId: PartialEq,
{
    pub fn path_of(self, token: CurrencyId) -> StablePath<PoolId, CurrencyId> {
        let swap_mode = if self.pool_id == self.base_pool_id {
            StableSwapMode::Single
        } else if token == self.token0 {
            // input is in base pool
            StableSwapMode::FromBase
        } else {
            // input is in meta pool
            StableSwapMode::ToBase
        };
        let to_currency = if token == self.token0 { self.token1 } else { self.token0 };

        StablePath {
            pool_id: self.pool_id,
            base_pool_id: self.base_pool_id,
            mode: swap_mode,
            from_currency: token,
            to_currency,
        }
    }
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct StablePath<PoolId, CurrencyId> {
    pub pool_id: PoolId,
    pub base_pool_id: PoolId,
    pub mode: StableSwapMode,
    pub from_currency: CurrencyId,
    pub to_currency: CurrencyId,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum StableSwapMode {
    Single,
    FromBase,
    ToBase,
}

pub trait StableAmmApi<PoolId, CurrencyId, AccountId, Balance> {
    fn get_all_trading_pairs() -> Vec<StablePair<PoolId, CurrencyId>>;

    fn stable_amm_calculate_currency_amount(
        pool_id: PoolId,
        amounts: &[Balance],
        deposit: bool,
    ) -> Result<Balance, DispatchError>;

    fn stable_amm_calculate_swap_amount(pool_id: PoolId, i: usize, j: usize, in_balance: Balance) -> Option<Balance>;

    fn stable_amm_calculate_swap_amount_from_base(
        meta_pool_id: PoolId,
        base_pool_id: PoolId,
        token_index_from: usize,
        token_index_to: usize,
        amount: Balance,
    ) -> Result<Option<Balance>, DispatchError>;

    fn stable_amm_calculate_swap_amount_to_base(
        meta_pool_id: PoolId,
        base_pool_id: PoolId,
        token_index_from: usize,
        token_index_to: usize,
        amount: Balance,
    ) -> Result<Option<Balance>, DispatchError>;

    fn stable_amm_calculate_remove_liquidity(pool_id: PoolId, amount: Balance) -> Option<Vec<Balance>>;

    fn stable_amm_calculate_remove_liquidity_one_currency(
        pool_id: PoolId,
        amount: Balance,
        index: u32,
    ) -> Option<Balance>;

    fn currency_index(pool_id: PoolId, currency: CurrencyId) -> Option<usize>;

    fn add_liquidity(
        who: &AccountId,
        pool_id: PoolId,
        amounts: &[Balance],
        min_mint_amount: Balance,
        to: &AccountId,
    ) -> Result<Balance, sp_runtime::DispatchError>;

    fn swap(
        who: &AccountId,
        pool_id: PoolId,
        from_index: u32,
        to_index: u32,
        in_amount: Balance,
        min_out_amount: Balance,
        to: &AccountId,
    ) -> Result<Balance, sp_runtime::DispatchError>;

    fn remove_liquidity(
        who: &AccountId,
        pool_id: PoolId,
        lp_amount: Balance,
        min_amounts: &[Balance],
        to: &AccountId,
    ) -> DispatchResult;

    fn remove_liquidity_one_currency(
        who: &AccountId,
        pool_id: PoolId,
        lp_amount: Balance,
        index: u32,
        min_amount: Balance,
        to: &AccountId,
    ) -> Result<Balance, DispatchError>;

    fn remove_liquidity_imbalance(
        who: &AccountId,
        pool_id: PoolId,
        amounts: &[Balance],
        max_burn_amount: Balance,
        to: &AccountId,
    ) -> DispatchResult;

    fn swap_pool_from_base(
        who: &AccountId,
        pool_id: PoolId,
        base_pool_id: PoolId,
        in_index: u32,
        out_index: u32,
        dx: Balance,
        min_dy: Balance,
        to: &AccountId,
    ) -> Result<Balance, DispatchError>;

    fn swap_pool_to_base(
        who: &AccountId,
        pool_id: PoolId,
        base_pool_id: PoolId,
        in_index: u32,
        out_index: u32,
        dx: Balance,
        min_dy: Balance,
        to: &AccountId,
    ) -> Result<Balance, DispatchError>;
}

impl<T: Config> StableAmmApi<T::PoolId, T::CurrencyId, T::AccountId, Balance> for Pallet<T> {
    fn get_all_trading_pairs() -> Vec<StablePair<T::PoolId, T::CurrencyId>> {
        // https://github.com/zenlinkpro/dex-sdk/blob/bba0310df15893913f31c999da9aca71f0bf152c/packages/sdk-router/src/entities/tradeV2.ts#L57
        let mut pairs = vec![];
        let pools: Vec<_> = Pools::<T>::iter().map(|(id, pool)| (id, pool.info())).collect();
        for (base_pool_id, pool) in pools.clone().into_iter() {
            // pools linked by the lp token
            let related_pools: Vec<_> = pools
                .iter()
                .filter(|(_, other_pool)| other_pool.currency_ids.contains(&pool.lp_currency_id))
                .cloned()
                .collect();

            for (i, token0) in pool.currency_ids.iter().enumerate() {
                for (_j, token1) in pool.currency_ids.iter().enumerate().skip(i + 1) {
                    pairs.push(StablePair {
                        pool_id: base_pool_id,
                        base_pool_id: base_pool_id,
                        token0: *token0,
                        token1: *token1,
                    });
                }

                if related_pools.len() == 0 {
                    continue;
                }

                for (meta_pool_id, other_pool) in &related_pools {
                    for (_j, token1) in other_pool.currency_ids.iter().enumerate() {
                        if *token1 == pool.lp_currency_id {
                            // already added above
                            continue;
                        }

                        // join meta and base pools
                        pairs.push(StablePair {
                            pool_id: *meta_pool_id,
                            base_pool_id: base_pool_id,
                            token0: *token0,
                            token1: *token1,
                        });
                    }
                }
            }
        }
        pairs
    }

    fn stable_amm_calculate_currency_amount(
        pool_id: T::PoolId,
        amounts: &[Balance],
        deposit: bool,
    ) -> Result<Balance, DispatchError> {
        Self::calculate_currency_amount(pool_id, amounts.to_vec(), deposit)
    }

    fn stable_amm_calculate_remove_liquidity(pool_id: T::PoolId, amount: Balance) -> Option<Vec<Balance>> {
        if let Some(pool) = Self::pools(pool_id) {
            return match pool {
                Pool::Base(bp) => Self::calculate_base_remove_liquidity(&bp, amount),
                Pool::Meta(mp) => Self::calculate_base_remove_liquidity(&mp.info, amount),
            };
        }
        None
    }

    fn stable_amm_calculate_swap_amount(
        pool_id: T::PoolId,
        i: usize,
        j: usize,
        in_balance: Balance,
    ) -> Option<Balance> {
        if let Some(pool) = Self::pools(pool_id) {
            return match pool {
                Pool::Base(bp) => Self::calculate_base_swap_amount(&bp, i, j, in_balance),
                Pool::Meta(mp) => {
                    let virtual_price = Self::calculate_meta_virtual_price(&mp)?;
                    let res = Self::calculate_meta_swap_amount(&mp, i, j, in_balance, virtual_price)?;
                    Some(res.0)
                }
            };
        }
        None
    }

    fn stable_amm_calculate_swap_amount_from_base(
        meta_pool_id: T::PoolId,
        base_pool_id: T::PoolId,
        token_index_from: usize, // base currency
        token_index_to: usize,   // meta currency
        amount: Balance,
    ) -> Result<Option<Balance>, DispatchError> {
        if let (Some(meta_pool), Some(base_pool)) = (Self::pools(meta_pool_id), Self::pools(base_pool_id)) {
            let base_token = base_pool.get_lp_currency();
            let base_token_index = meta_pool
                .get_currency_index(base_token)
                .ok_or(Error::<T>::InvalidPooledCurrency)?;

            let mut base_amounts = vec![0; base_pool.get_balances().len()];
            base_amounts[token_index_from] = amount;

            // get LP tokens for supplying `from` currency in base pool
            let base_lp_amount = Self::stable_amm_calculate_currency_amount(base_pool_id, &base_amounts, true)?;
            if base_token_index == token_index_to {
                Ok(Some(base_lp_amount))
            } else {
                // swap new LP tokens in meta pool
                Ok(Self::stable_amm_calculate_swap_amount(
                    meta_pool_id,
                    base_token_index,
                    token_index_to,
                    base_lp_amount,
                ))
            }
        } else {
            Ok(None)
        }
    }

    fn stable_amm_calculate_swap_amount_to_base(
        meta_pool_id: T::PoolId,
        base_pool_id: T::PoolId,
        token_index_from: usize, // meta currency
        token_index_to: usize,   // base currency
        amount: Balance,
    ) -> Result<Option<Balance>, DispatchError> {
        if let (Some(meta_pool), Some(base_pool)) = (Self::pools(meta_pool_id), Self::pools(base_pool_id)) {
            let base_token = base_pool.get_lp_currency();
            let base_token_index = meta_pool
                .get_currency_index(base_token)
                .ok_or(Error::<T>::InvalidPooledCurrency)?;

            let token_lp_amount = if base_token_index != token_index_from {
                // get LP tokens for swapping `from` currency in meta pool
                Self::stable_amm_calculate_swap_amount(meta_pool_id, token_index_from, base_token_index, amount)
                    .ok_or(Error::<T>::Arithmetic)?
            } else {
                // input is already LP balance
                amount
            };

            // burn LP tokens for `to` currency in base pool
            Ok(Self::stable_amm_calculate_remove_liquidity_one_currency(
                base_pool_id,
                token_lp_amount,
                token_index_to as u32,
            ))
        } else {
            Ok(None)
        }
    }

    fn stable_amm_calculate_remove_liquidity_one_currency(
        pool_id: T::PoolId,
        amount: Balance,
        index: u32,
    ) -> Option<Balance> {
        if let Some(pool) = Self::pools(pool_id) {
            if let Some(res) = match pool {
                Pool::Base(bp) => Self::calculate_base_remove_liquidity_one_token(&bp, amount, index),
                Pool::Meta(mp) => {
                    let total_supply = T::MultiCurrency::total_issuance(mp.info.lp_currency_id);
                    Self::calculate_meta_remove_liquidity_one_currency(&mp, amount, index as usize, total_supply)
                }
            } {
                return Some(res.0);
            }
        }
        None
    }

    fn currency_index(pool_id: T::PoolId, currency: T::CurrencyId) -> Option<usize> {
        Self::get_currency_index(pool_id, currency)
    }

    fn add_liquidity(
        who: &T::AccountId,
        pool_id: T::PoolId,
        amounts: &[Balance],
        min_mint_amount: Balance,
        to: &T::AccountId,
    ) -> Result<Balance, sp_runtime::DispatchError> {
        Self::inner_add_liquidity(who, pool_id, amounts, min_mint_amount, to)
    }

    fn swap(
        who: &T::AccountId,
        pool_id: T::PoolId,
        from_index: u32,
        to_index: u32,
        in_amount: Balance,
        min_out_amount: Balance,
        to: &T::AccountId,
    ) -> Result<Balance, sp_runtime::DispatchError> {
        Self::inner_swap(
            who,
            pool_id,
            from_index as usize,
            to_index as usize,
            in_amount,
            min_out_amount,
            to,
        )
    }

    fn remove_liquidity(
        who: &T::AccountId,
        pool_id: T::PoolId,
        lp_amount: Balance,
        min_amounts: &[Balance],
        to: &T::AccountId,
    ) -> DispatchResult {
        Self::inner_remove_liquidity(pool_id, who, lp_amount, min_amounts, to)
    }

    fn remove_liquidity_one_currency(
        who: &T::AccountId,
        pool_id: T::PoolId,
        lp_amount: Balance,
        index: u32,
        min_amount: Balance,
        to: &T::AccountId,
    ) -> Result<Balance, DispatchError> {
        Self::inner_remove_liquidity_one_currency(pool_id, who, lp_amount, index, min_amount, to)
    }

    fn remove_liquidity_imbalance(
        who: &T::AccountId,
        pool_id: T::PoolId,
        amounts: &[Balance],
        max_burn_amount: Balance,
        to: &T::AccountId,
    ) -> DispatchResult {
        Self::inner_remove_liquidity_imbalance(who, pool_id, amounts, max_burn_amount, to)
    }

    fn swap_pool_from_base(
        who: &T::AccountId,
        pool_id: T::PoolId,
        base_pool_id: T::PoolId,
        in_index: u32,
        out_index: u32,
        dx: Balance,
        min_dy: Balance,
        to: &T::AccountId,
    ) -> Result<Balance, DispatchError> {
        Self::inner_swap_pool_from_base(who, pool_id, base_pool_id, in_index, out_index, dx, min_dy, to)
    }

    fn swap_pool_to_base(
        who: &T::AccountId,
        pool_id: T::PoolId,
        base_pool_id: T::PoolId,
        in_index: u32,
        out_index: u32,
        dx: Balance,
        min_dy: Balance,
        to: &T::AccountId,
    ) -> Result<Balance, DispatchError> {
        Self::inner_swap_pool_to_base(who, pool_id, base_pool_id, in_index, out_index, dx, min_dy, to)
    }
}
