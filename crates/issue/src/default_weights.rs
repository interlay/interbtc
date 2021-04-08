//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::weights::{constants::RocksDbWeight as DbWeight, Weight};

impl crate::WeightInfo for () {
    fn request_issue() -> Weight {
        452_088_000_u64
            .saturating_add(DbWeight::get().reads(13_u64))
            .saturating_add(DbWeight::get().writes(5_u64))
    }
    fn execute_issue() -> Weight {
        211_260_000_u64
            .saturating_add(DbWeight::get().reads(14_u64))
            .saturating_add(DbWeight::get().writes(3_u64))
    }
    fn cancel_issue() -> Weight {
        120_760_000_u64
            .saturating_add(DbWeight::get().reads(6_u64))
            .saturating_add(DbWeight::get().writes(3_u64))
    }
    fn set_issue_period() -> Weight {
        3_480_000_u64.saturating_add(DbWeight::get().writes(1_u64))
    }
}
