use super::*;
use frame_benchmarking::v2::{account, benchmarks, impl_benchmark_test_suite};
use frame_support::assert_ok;
use frame_system::RawOrigin;
use sp_std::vec;
// Pallets
use crate::Pallet as Escrow;
use frame_system::Pallet as System;

pub fn create_default_lock<T: Config>(origin: T::AccountId) {
    let start_height = System::<T>::block_number();
    let end_height = start_height + T::MaxPeriod::get();
    let amount = T::BlockNumberToBalance::convert(T::MaxPeriod::get());
    T::Currency::make_free_balance_be(&origin, amount.into());
    assert_ok!(Escrow::<T>::create_lock(
        RawOrigin::Signed(origin).into(),
        amount.into(),
        end_height
    ));
}

#[benchmarks]
pub mod benchmarks {
    use super::*;

    #[benchmark]
    pub fn create_lock() {
        let origin: T::AccountId = account("Origin", 0, 0);
        let start_height = System::<T>::block_number();
        let end_height = start_height + T::MaxPeriod::get();
        let amount = T::BlockNumberToBalance::convert(T::MaxPeriod::get());
        T::Currency::make_free_balance_be(&origin, amount.into());

        #[extrinsic_call]
        create_lock(RawOrigin::Signed(origin), amount.into(), end_height);
    }

    #[benchmark]
    pub fn increase_amount() {
        let origin: T::AccountId = account("Origin", 0, 0);
        let amount = T::BlockNumberToBalance::convert(T::MaxPeriod::get());
        create_default_lock::<T>(origin.clone());
        let free_balance = T::Currency::free_balance(&origin);
        T::Currency::make_free_balance_be(&origin, free_balance + amount.into());

        #[extrinsic_call]
        increase_amount(RawOrigin::Signed(origin), amount.into());
    }

    #[benchmark]
    pub fn increase_unlock_height() {
        let origin: T::AccountId = account("Origin", 0, 0);
        create_default_lock::<T>(origin.clone());
        let end_height = System::<T>::block_number() + T::MaxPeriod::get() - T::Span::get();
        System::<T>::set_block_number(end_height);

        #[extrinsic_call]
        increase_unlock_height(RawOrigin::Signed(origin), end_height + T::MaxPeriod::get());
    }

    #[benchmark]
    pub fn withdraw() {
        let origin: T::AccountId = account("Origin", 0, 0);
        create_default_lock::<T>(origin.clone());
        let end_height = System::<T>::block_number() + T::MaxPeriod::get() + T::Span::get();
        System::<T>::set_block_number(end_height);

        #[extrinsic_call]
        withdraw(RawOrigin::Signed(origin));
    }

    impl_benchmark_test_suite! {Escrow, crate::mock::ExtBuilder::build(), crate::mock::Test}
}
