// Copyright 2021-2022 Zenlink.
// Licensed under Apache 2.0.

use codec::{Decode, Encode};
use frame_support::pallet_prelude::*;
use sp_std::fmt::Debug;

pub type Balance = u128;
pub type Number = Balance;

pub const FEE_DENOMINATOR: Number = 10_000_000_000;
pub const POOL_LP_CURRENCY_ID_DECIMAL: u8 = 18;

pub const BASE_VIRTUAL_PRICE_PRECISION: Balance = 1_000_000_000_000_000_000;

// protect from division loss when run approximation loop
pub const A_PRECISION: Number = 100;

// the number of iterations to sum d and y
pub const MAX_ITERATION: u32 = 255;
pub const POOL_TOKEN_COMMON_DECIMALS: u32 = 18;

pub const DAY: u32 = 86400;
pub const MIN_RAMP_TIME: u32 = DAY;

pub const MINUTE: u64 = 3600;
pub const BASE_CACHE_EXPIRE_TIME: u64 = 10 * MINUTE;

// max_a with precision
pub const MAX_A: Number = 1_000_000;
pub const MAX_A_CHANGE: u32 = 10;
pub const MAX_ADMIN_FEE: Number = 10_000_000_000; // 100%
pub const MAX_SWAP_FEE: Number = 100_000_000; // 1%

#[derive(CloneNoBound, PartialEqNoBound, EqNoBound, RuntimeDebugNoBound, TypeInfo, Encode, Decode, MaxEncodedLen)]
#[codec(mel_bound(skip_type_params(PoolCurrencyLimit, PoolCurrencySymbolLimit)))]
#[scale_info(skip_type_params(PoolCurrencyLimit, PoolCurrencySymbolLimit))]

pub struct BasePool<CurrencyId, AccountId, PoolCurrencyLimit: Get<u32>, PoolCurrencySymbolLimit: Get<u32>>
where
    AccountId: Clone + Debug + Eq + PartialEq,
    CurrencyId: Clone + Debug + Eq + PartialEq,
{
    pub currency_ids: BoundedVec<CurrencyId, PoolCurrencyLimit>,
    pub lp_currency_id: CurrencyId,
    // token i multiplier to reach POOL_TOKEN_COMMON_DECIMALS
    pub token_multipliers: BoundedVec<Balance, PoolCurrencyLimit>,
    // effective balance which might different from token balance of the pool account because it
    // hold admin fee as well
    pub balances: BoundedVec<Balance, PoolCurrencyLimit>,
    // swap fee ratio. Change on any action which move balance state far from the ideal state
    pub fee: Number,
    // admin fee in ratio of swap fee.
    pub admin_fee: Number,
    // observation of A, multiplied with A_PRECISION
    pub initial_a: Number,
    pub future_a: Number,
    pub initial_a_time: Number,
    pub future_a_time: Number,
    // the pool's account
    pub account: AccountId,
    pub admin_fee_receiver: AccountId,
    pub lp_currency_symbol: BoundedVec<u8, PoolCurrencySymbolLimit>,
    pub lp_currency_decimal: u8,
}

#[derive(CloneNoBound, PartialEqNoBound, EqNoBound, RuntimeDebugNoBound, TypeInfo, Encode, Decode, MaxEncodedLen)]
#[codec(mel_bound(skip_type_params(PoolCurrencyLimit, PoolCurrencySymbolLimit)))]
#[scale_info(skip_type_params(PoolCurrencyLimit, PoolCurrencySymbolLimit))]

pub struct MetaPool<PoolId, CurrencyId, AccountId, PoolCurrencyLimit: Get<u32>, PoolCurrencySymbolLimit: Get<u32>>
where
    AccountId: Clone + Debug + Eq + PartialEq,
    CurrencyId: Clone + Debug + Eq + PartialEq,
    PoolId: Clone + Debug + Eq + PartialEq,
{
    pub base_pool_id: PoolId,
    pub base_virtual_price: Balance,
    pub base_cache_last_updated: u64,
    pub base_currencies: BoundedVec<CurrencyId, PoolCurrencyLimit>,

    pub info: BasePool<CurrencyId, AccountId, PoolCurrencyLimit, PoolCurrencySymbolLimit>,
}

#[derive(CloneNoBound, PartialEqNoBound, EqNoBound, RuntimeDebugNoBound, TypeInfo, Encode, Decode, MaxEncodedLen)]
#[codec(mel_bound(skip_type_params(PoolCurrencyLimit, PoolCurrencySymbolLimit)))]
#[scale_info(skip_type_params(PoolCurrencyLimit, PoolCurrencySymbolLimit))]
pub enum Pool<PoolId, CurrencyId, AccountId, PoolCurrencyLimit: Get<u32>, PoolCurrencySymbolLimit: Get<u32>>
where
    AccountId: Clone + Debug + Eq + PartialEq,
    CurrencyId: Clone + Debug + Eq + PartialEq,
    PoolId: Clone + Debug + Eq + PartialEq,
{
    Base(BasePool<CurrencyId, AccountId, PoolCurrencyLimit, PoolCurrencySymbolLimit>),
    Meta(MetaPool<PoolId, CurrencyId, AccountId, PoolCurrencyLimit, PoolCurrencySymbolLimit>),
}

impl<PoolId, CurrencyId, AccountId: Clone, PoolCurrencyLimit: Get<u32>, PoolCurrencySymbolLimit: Get<u32>>
    Pool<PoolId, CurrencyId, AccountId, PoolCurrencyLimit, PoolCurrencySymbolLimit>
where
    AccountId: Clone + Debug + Eq + PartialEq,
    CurrencyId: Clone + Copy + Debug + Eq + PartialEq,
    PoolId: Clone + Copy + Debug + Eq + PartialEq,
{
    pub fn info(self) -> BasePool<CurrencyId, AccountId, PoolCurrencyLimit, PoolCurrencySymbolLimit> {
        match self {
            Pool::Base(bp) => bp,
            Pool::Meta(mp) => mp.info,
        }
    }

    pub fn get_currency_ids(self) -> BoundedVec<CurrencyId, PoolCurrencyLimit> {
        match self {
            Pool::Base(bp) => bp.currency_ids,
            Pool::Meta(mp) => mp.info.currency_ids,
        }
    }

    pub fn get_lp_currency(&self) -> CurrencyId {
        match self {
            Pool::Base(bp) => bp.lp_currency_id,
            Pool::Meta(mp) => mp.info.lp_currency_id,
        }
    }

    pub fn get_initial_a_time(&self) -> Number {
        match self {
            Pool::Base(bp) => bp.initial_a_time,
            Pool::Meta(mp) => mp.info.initial_a_time,
        }
    }

    pub fn get_token_multipliers(self) -> BoundedVec<Balance, PoolCurrencyLimit> {
        match self {
            Pool::Base(bp) => bp.token_multipliers,
            Pool::Meta(mp) => mp.info.token_multipliers,
        }
    }

    pub fn get_balances(&self) -> BoundedVec<Balance, PoolCurrencyLimit> {
        match self {
            Pool::Base(bp) => bp.balances.clone(),
            Pool::Meta(mp) => mp.info.balances.clone(),
        }
    }

    pub fn get_fee(&self) -> Number {
        match self {
            Pool::Base(bp) => bp.fee,
            Pool::Meta(mp) => mp.info.fee,
        }
    }

    pub fn get_account(&self) -> AccountId {
        match self {
            Pool::Base(bp) => bp.account.clone(),
            Pool::Meta(mp) => mp.info.account.clone(),
        }
    }

    pub fn set_admin_fee(&mut self, admin_fee: Balance) {
        match self {
            Pool::Base(bp) => bp.admin_fee = admin_fee,
            Pool::Meta(mp) => mp.info.admin_fee = admin_fee,
        }
    }

    pub fn set_fee(&mut self, fee: Balance) {
        match self {
            Pool::Base(bp) => bp.fee = fee,
            Pool::Meta(mp) => mp.info.fee = fee,
        }
    }

    pub fn set_admin_fee_receiver(&mut self, receiver: AccountId) {
        match self {
            Pool::Base(bp) => bp.admin_fee_receiver = receiver,
            Pool::Meta(mp) => mp.info.admin_fee_receiver = receiver,
        }
    }
}
