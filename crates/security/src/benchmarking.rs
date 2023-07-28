use super::*;
use crate::Pallet;
use frame_benchmarking::v2::{benchmarks, impl_benchmark_test_suite};
use frame_support::traits::Hooks;
use frame_system::RawOrigin;
use sp_std::prelude::*;

#[benchmarks]
pub mod benchmarks {
    use super::*;

    #[cfg(test)]
    use super::Pallet as Security;

    #[benchmark]
    fn on_initialize() {
        let previous_block_number = Pallet::<T>::active_block_number();
        #[block]
        {
            Pallet::<T>::on_initialize(1u32.into());
        }
        let new_block_number = Pallet::<T>::active_block_number();
        assert_eq!(previous_block_number + 1u32.into(), new_block_number);
    }

    #[benchmark]
    fn activate_counter() {
        #[extrinsic_call]
        activate_counter(RawOrigin::Root, true);
    }

    impl_benchmark_test_suite!(Security, crate::mock::ExtBuilder::build(), crate::mock::Test);
}
