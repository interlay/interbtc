//! Supply pallet benchmarking.
#![allow(unused_imports)]
use super::*;

use frame_benchmarking::v2::{benchmarks, impl_benchmark_test_suite};
use frame_support::traits::OnInitialize;
use frame_system::{self, RawOrigin as SystemOrigin};
use sp_runtime::traits::One;
use sp_std::prelude::*;

#[benchmarks]
pub mod benchmarks {
    use super::*;
    use crate::Pallet as Supply;

    #[benchmark]
    pub fn on_initialize() {
        let block_number = 100u32.into();
        StartHeight::<T>::put(block_number);
        Inflation::<T>::put(T::UnsignedFixedPoint::one());

        let total_issuance = T::Currency::total_issuance();
        assert!(!total_issuance.is_zero(), "Total issuance should be non-zero");

        #[block]
        {
            Supply::<T>::on_initialize(block_number);
        }

        assert_eq!(LastEmission::<T>::get(), total_issuance);
    }

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
