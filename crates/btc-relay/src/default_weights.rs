//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::weights::{constants::RocksDbWeight as DbWeight, Weight};

impl crate::WeightInfo for () {
    fn initialize() -> Weight {
        (56_974_000 as Weight)
            .saturating_add(DbWeight::get().reads(1 as Weight))
            .saturating_add(DbWeight::get().writes(7 as Weight))
    }
    fn store_block_header() -> Weight {
        (113_143_000 as Weight)
            .saturating_add(DbWeight::get().reads(6 as Weight))
            .saturating_add(DbWeight::get().writes(6 as Weight))
    }
    fn verify_and_validate_transaction() -> Weight {
        (125_227_000 as Weight).saturating_add(DbWeight::get().reads(7 as Weight))
    }
    fn verify_transaction_inclusion() -> Weight {
        (68_197_000 as Weight).saturating_add(DbWeight::get().reads(7 as Weight))
    }
    fn validate_transaction() -> Weight {
        (9_131_000 as Weight)
    }
}
