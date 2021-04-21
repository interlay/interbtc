//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::weights::{constants::RocksDbWeight as DbWeight, Weight};

pub trait WeightInfo {
    fn initialize() -> Weight;
    fn register_staked_relayer() -> Weight;
    fn deregister_staked_relayer() -> Weight;
    fn report_vault_theft() -> Weight;
    fn slash_staked_relayer() -> Weight;
    fn store_block_header() -> Weight;
}

impl crate::WeightInfo for () {
    // WARNING! Some components were not used: ["u"]
    fn initialize() -> Weight {
        (52_558_000 as Weight)
            .saturating_add(DbWeight::get().reads(3 as Weight))
            .saturating_add(DbWeight::get().writes(7 as Weight))
    }
    fn register_staked_relayer() -> Weight {
        (79_756_000 as Weight)
            .saturating_add(DbWeight::get().reads(4 as Weight))
            .saturating_add(DbWeight::get().writes(3 as Weight))
    }
    fn deregister_staked_relayer() -> Weight {
        (93_929_000 as Weight)
            .saturating_add(DbWeight::get().reads(5 as Weight))
            .saturating_add(DbWeight::get().writes(4 as Weight))
    }
    fn slash_staked_relayer() -> Weight {
        (109_555_000 as Weight)
            .saturating_add(DbWeight::get().reads(6 as Weight))
            .saturating_add(DbWeight::get().writes(5 as Weight))
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
