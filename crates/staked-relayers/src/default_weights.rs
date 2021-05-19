//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::weights::{constants::RocksDbWeight as DbWeight, Weight};

pub trait WeightInfo {
    fn initialize() -> Weight;
    fn report_vault_theft() -> Weight;
    fn store_block_header() -> Weight;
}

impl crate::WeightInfo for () {
    // WARNING! Some components were not used: ["u"]
    fn initialize() -> Weight {
        (52_558_000 as Weight)
            .saturating_add(DbWeight::get().reads(3 as Weight))
            .saturating_add(DbWeight::get().writes(7 as Weight))
    }
    fn report_vault_theft() -> Weight {
        (251_206_000 as Weight)
            .saturating_add(DbWeight::get().reads(16 as Weight))
            .saturating_add(DbWeight::get().writes(5 as Weight))
    }
    fn store_block_header() -> Weight {
        (123_623_000 as Weight)
            .saturating_add(DbWeight::get().reads(13 as Weight))
            .saturating_add(DbWeight::get().writes(8 as Weight))
    }
}
