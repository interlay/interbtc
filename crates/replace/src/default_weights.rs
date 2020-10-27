//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::weights::{constants::RocksDbWeight as DbWeight, Weight};

impl crate::WeightInfo for () {
    fn request_replace() -> Weight {
        (142_819_000 as Weight)
            .saturating_add(DbWeight::get().reads(6 as Weight))
            .saturating_add(DbWeight::get().writes(5 as Weight))
    }
    fn withdraw_replace() -> Weight {
        (132_256_000 as Weight)
            .saturating_add(DbWeight::get().reads(10 as Weight))
            .saturating_add(DbWeight::get().writes(3 as Weight))
    }
    fn accept_replace() -> Weight {
        (124_104_000 as Weight)
            .saturating_add(DbWeight::get().reads(10 as Weight))
            .saturating_add(DbWeight::get().writes(3 as Weight))
    }
    fn auction_replace() -> Weight {
        (188_428_000 as Weight)
            .saturating_add(DbWeight::get().reads(13 as Weight))
            .saturating_add(DbWeight::get().writes(5 as Weight))
    }
    fn execute_replace() -> Weight {
        (218_546_000 as Weight)
            .saturating_add(DbWeight::get().reads(12 as Weight))
            .saturating_add(DbWeight::get().writes(3 as Weight))
    }
    fn cancel_replace() -> Weight {
        (97_129_000 as Weight)
            .saturating_add(DbWeight::get().reads(5 as Weight))
            .saturating_add(DbWeight::get().writes(3 as Weight))
    }
}
