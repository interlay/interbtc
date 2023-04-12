use super::{Pallet as Security, *};
use crate::Pallet;
use frame_benchmarking::v2::{benchmarks, impl_benchmark_test_suite};
use frame_support::{assert_ok, traits::Hooks};
use frame_system::RawOrigin;
use sp_std::prelude::*;

#[benchmarks]
pub mod benchmarks {
    use super::*;

    #[benchmark]
    fn on_initialize() {
        assert_ok!(Pallet::<T>::set_parachain_status(
            RawOrigin::Root.into(),
            StatusCode::Running
        ));
        let previous_block_number = Pallet::<T>::active_block_number();
        #[block]
        {
            Pallet::<T>::on_initialize(1u32.into());
        }
        let new_block_number = Pallet::<T>::active_block_number();
        assert_eq!(previous_block_number + 1u32.into(), new_block_number);
    }

    #[benchmark]
    fn set_parachain_status() {
        #[extrinsic_call]
        set_parachain_status(RawOrigin::Root, StatusCode::Running);
    }

    #[benchmark]
    fn insert_parachain_error() {
        #[extrinsic_call]
        insert_parachain_error(RawOrigin::Root, ErrorCode::OracleOffline);
    }

    #[benchmark]
    fn remove_parachain_error() {
        let _ = Security::<T>::insert_parachain_error(RawOrigin::Root.into(), ErrorCode::OracleOffline);

        #[extrinsic_call]
        remove_parachain_error(RawOrigin::Root, ErrorCode::OracleOffline);
    }

    impl_benchmark_test_suite!(Security, crate::mock::ExtBuilder::build(), crate::mock::Test);
}
