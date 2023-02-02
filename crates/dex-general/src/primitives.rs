// Copyright 2021-2022 Zenlink.
// Licensed under Apache 2.0.

use super::*;
use scale_info::TypeInfo;

pub type AssetBalance = u128;

// 0.3% exchange fee rate
pub const DEFAULT_FEE_RATE: u128 = 3;
pub const FEE_ADJUSTMENT: u128 = 1000;

pub trait AssetInfo {
    fn is_support(&self) -> bool;
}

/// Status for TradingPair
#[derive(Clone, Copy, Encode, Decode, RuntimeDebug, PartialEq, Eq, MaxEncodedLen, TypeInfo)]
pub enum PairStatus<Balance, BlockNumber, Account> {
    /// Pair is Trading,
    /// can add/remove liquidity and swap.
    Trading(PairMetadata<Balance, Account>),
    /// pair is Bootstrap,
    /// can add liquidity.
    Bootstrap(BootstrapParameter<Balance, BlockNumber, Account>),
    /// nothing in pair
    Disable,
}

impl<Balance, BlockNumber, Account> Default for PairStatus<Balance, BlockNumber, Account> {
    fn default() -> Self {
        Self::Disable
    }
}

impl<BlockNumber, Account> PairStatus<AssetBalance, BlockNumber, Account> {
    pub fn fee_rate(&self) -> AssetBalance {
        match self {
            Self::Trading(pair) => pair.fee_rate,
            _ => DEFAULT_FEE_RATE,
        }
    }
}

/// Parameters of pair in Bootstrap status
#[derive(Encode, Decode, Clone, Copy, RuntimeDebug, PartialEq, Eq, MaxEncodedLen, TypeInfo)]
pub struct BootstrapParameter<Balance, BlockNumber, Account> {
    /// target supply that trading pair could to normal.
    pub target_supply: (Balance, Balance),
    /// max supply in this bootstrap pair
    pub capacity_supply: (Balance, Balance),
    /// accumulated supply in this bootstrap pair.
    pub accumulated_supply: (Balance, Balance),
    /// bootstrap pair end block number.
    pub end_block_number: BlockNumber,
    /// bootstrap pair account.
    pub pair_account: Account,
}

#[derive(Encode, Decode, Clone, Copy, RuntimeDebug, PartialEq, Eq, MaxEncodedLen, TypeInfo)]
pub struct PairMetadata<Balance, Account> {
    pub pair_account: Account,
    pub total_supply: Balance,
    pub fee_rate: Balance,
}
