//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::weights::{constants::RocksDbWeight as DbWeight, Weight};

pub trait WeightInfo {
    fn request_replace() -> Weight;
    fn withdraw_replace() -> Weight;
    fn accept_replace() -> Weight;
    fn auction_replace() -> Weight;
    fn execute_replace() -> Weight;
    fn cancel_replace() -> Weight;
    fn set_replace_period() -> Weight;
}

impl crate::WeightInfo for () {
    fn request_replace() -> Weight {
        142_819_000_u64
            .saturating_add(DbWeight::get().reads(6_u64))
            .saturating_add(DbWeight::get().writes(5_u64))
    }
    fn withdraw_replace() -> Weight {
        132_256_000_u64
            .saturating_add(DbWeight::get().reads(10_u64))
            .saturating_add(DbWeight::get().writes(3_u64))
    }
    fn accept_replace() -> Weight {
        124_104_000_u64
            .saturating_add(DbWeight::get().reads(10_u64))
            .saturating_add(DbWeight::get().writes(3_u64))
    }
    fn auction_replace() -> Weight {
        188_428_000_u64
            .saturating_add(DbWeight::get().reads(13_u64))
            .saturating_add(DbWeight::get().writes(5_u64))
    }
    fn execute_replace() -> Weight {
        218_546_000_u64
            .saturating_add(DbWeight::get().reads(12_u64))
            .saturating_add(DbWeight::get().writes(3_u64))
    }
    fn cancel_replace() -> Weight {
        97_129_000_u64
            .saturating_add(DbWeight::get().reads(5_u64))
            .saturating_add(DbWeight::get().writes(3_u64))
    }
    fn set_replace_period() -> Weight {
        3_300_000_u64.saturating_add(DbWeight::get().writes(1_u64))
    }
}
