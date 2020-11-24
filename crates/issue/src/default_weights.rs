//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::weights::{constants::RocksDbWeight as DbWeight, Weight};

impl crate::WeightInfo for () {
    fn request_issue() -> Weight {
        (197_974_000 as Weight)
            .saturating_add(DbWeight::get().reads(13 as Weight))
            .saturating_add(DbWeight::get().writes(5 as Weight))
    }
    fn execute_issue() -> Weight {
        (211_260_000 as Weight)
            .saturating_add(DbWeight::get().reads(14 as Weight))
            .saturating_add(DbWeight::get().writes(3 as Weight))
    }
    fn cancel_issue() -> Weight {
        (120_760_000 as Weight)
            .saturating_add(DbWeight::get().reads(6 as Weight))
            .saturating_add(DbWeight::get().writes(3 as Weight))
    }
    fn set_issue_period() -> Weight {
        (3_480_000 as Weight).saturating_add(DbWeight::get().writes(1 as Weight))
    }
}
