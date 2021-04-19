#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{
    traits::Get,
    weights::{constants::RocksDbWeight, Weight},
};

/// Weight functions needed for issue.
pub trait WeightInfo {
    fn request_issue() -> Weight;
    fn execute_issue() -> Weight;
    fn cancel_issue() -> Weight;
    fn set_issue_period() -> Weight;
}

// For backwards compatibility and tests
impl WeightInfo for () {
    fn request_issue() -> Weight {
        (11_798_074_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(16 as Weight))
            .saturating_add(RocksDbWeight::get().writes(5 as Weight))
    }
    fn execute_issue() -> Weight {
        (16_894_787_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(24 as Weight))
            .saturating_add(RocksDbWeight::get().writes(9 as Weight))
    }
    fn cancel_issue() -> Weight {
        (6_492_133_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(9 as Weight))
            .saturating_add(RocksDbWeight::get().writes(4 as Weight))
    }
    fn set_issue_period() -> Weight {
        (204_239_000 as Weight).saturating_add(RocksDbWeight::get().writes(1 as Weight))
    }
}
