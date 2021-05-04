//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::weights::{constants::RocksDbWeight as DbWeight, Weight};

pub trait WeightInfo {
    fn withdraw_issuing() -> Weight;
    fn withdraw_backing() -> Weight;
}

impl crate::WeightInfo for () {
    fn withdraw_issuing() -> Weight {
        124_557_000_u64
            .saturating_add(DbWeight::get().reads(5_u64))
            .saturating_add(DbWeight::get().writes(4_u64))
    }
    fn withdraw_backing() -> Weight {
        127_327_000_u64
            .saturating_add(DbWeight::get().reads(5_u64))
            .saturating_add(DbWeight::get().writes(4_u64))
    }
}
