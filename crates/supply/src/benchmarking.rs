//! Loans pallet benchmarking.
#![allow(unused_imports)]
#![cfg(feature = "runtime-benchmarks")]
use super::*;

use frame_benchmarking::v2::{benchmarks, impl_benchmark_test_suite};
use frame_system::{self, RawOrigin as SystemOrigin};
use sp_std::prelude::*;

mod default_weights;
pub use default_weights::WeightInfo;

#[benchmarks]
pub mod benchmarks {
    use super::*;
    use crate::Pallet as Supply;

    #[benchmark]
    pub fn set_start_height_and_inflation() {
        #[extrinsic_call]
        _(
            SystemOrigin::Root,
            1u32.into(),
            T::UnsignedFixedPoint::checked_from_integer(10u32).unwrap(),
        );
    }

    impl_benchmark_test_suite!(Supply, crate::mock::ExtBuilder::build(), crate::mock::Test);
}
