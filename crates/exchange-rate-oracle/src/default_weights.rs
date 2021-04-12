//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::weights::{constants::RocksDbWeight as DbWeight, Weight};

pub trait WeightInfo {
    fn set_exchange_rate() -> Weight;
    fn set_btc_tx_fees_per_byte() -> Weight;
    fn insert_authorized_oracle() -> Weight;
    fn remove_authorized_oracle() -> Weight;
}

impl crate::WeightInfo for () {
    // WARNING! Some components were not used: ["u"]
    fn set_exchange_rate() -> Weight {
        42_788_000_u64
            .saturating_add(DbWeight::get().reads(5_u64))
            .saturating_add(DbWeight::get().writes(2_u64))
    }
    fn set_btc_tx_fees_per_byte() -> Weight {
        30_015_705_u64
            .saturating_add(DbWeight::get().reads(2_u64))
            .saturating_add(DbWeight::get().writes(1_u64))
    }
    fn insert_authorized_oracle() -> Weight {
        6_788_000_u64.saturating_add(DbWeight::get().writes(1_u64))
    }
    fn remove_authorized_oracle() -> Weight {
        6_021_000_u64.saturating_add(DbWeight::get().writes(1_u64))
    }
}
