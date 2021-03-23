//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::weights::{constants::RocksDbWeight as DbWeight, Weight};

impl crate::WeightInfo for () {
    fn request_redeem() -> Weight {
        (179_175_000 as Weight)
            .saturating_add(DbWeight::get().reads(12 as Weight))
            .saturating_add(DbWeight::get().writes(5 as Weight))
    }
    fn liquidation_redeem() -> Weight {
        (179_175_000 as Weight)
            .saturating_add(DbWeight::get().reads(12 as Weight))
            .saturating_add(DbWeight::get().writes(5 as Weight))
    }
    fn execute_redeem() -> Weight {
        (188_681_000 as Weight)
            .saturating_add(DbWeight::get().reads(14 as Weight))
            .saturating_add(DbWeight::get().writes(4 as Weight))
    }
    fn cancel_redeem() -> Weight {
        (168_952_000 as Weight)
            .saturating_add(DbWeight::get().reads(14 as Weight))
            .saturating_add(DbWeight::get().writes(5 as Weight))
    }
    fn set_redeem_period() -> Weight {
        (3_376_000 as Weight).saturating_add(DbWeight::get().writes(1 as Weight))
    }
    // note: placeholder value
    fn mint_tokens_for_reimbursed_redeem() -> Weight {
        (168_952_000 as Weight)
            .saturating_add(DbWeight::get().reads(14 as Weight))
            .saturating_add(DbWeight::get().writes(5 as Weight))
    }
}
