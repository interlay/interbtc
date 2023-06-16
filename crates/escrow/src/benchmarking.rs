use super::*;
use frame_benchmarking::v2::{account, benchmarks, impl_benchmark_test_suite};
use frame_support::assert_ok;
use frame_system::RawOrigin;
use primitives::{Balance, CurrencyId};
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

fn distribute_rewards<T: Config>()
where
    T::EscrowRewards: reward::RewardsApi<(), T::AccountId, Balance, CurrencyId = CurrencyId>,
{
    assert_ok!(T::EscrowRewards::deposit_stake(
        &(),
        &account("Staker", 0, 0),
        1000u32.into()
    ));
    assert_ok!(T::EscrowRewards::distribute_reward(
        &(),
        CurrencyId::ForeignAsset(0),
        1000u32.into(),
    ));
}

#[benchmarks(
    where
        T::EscrowRewards: reward::RewardsApi<(), T::AccountId, Balance, CurrencyId = CurrencyId>
)]
pub mod benchmarks {
    use super::*;

    #[benchmark]
    pub fn create_lock() {
        let origin: T::AccountId = account("Origin", 0, 0);
        // 52 weeks, i.e. 1 year. Since `create_lock` iterates ones per elapsed span,
        // we simulate a very bad case: 1 year without calls to `deposit_for`.
        // This should be a pretty safe upper bound
        System::<T>::set_block_number(T::Span::get() * 52u32.into());
        let start_height = System::<T>::block_number();
        let end_height = start_height + T::MaxPeriod::get();
        let amount = T::BlockNumberToBalance::convert(T::MaxPeriod::get());
        T::Currency::make_free_balance_be(&origin, amount.into());
        distribute_rewards::<T>();

        #[extrinsic_call]
        create_lock(RawOrigin::Signed(origin), amount.into(), end_height);
    }

    #[benchmark]
    pub fn increase_amount() {
        let origin: T::AccountId = account("Origin", 0, 0);
        let amount = T::BlockNumberToBalance::convert(T::MaxPeriod::get());
        // 52 weeks, i.e. 1 year. Since `increase_amount` iterates ones per elapsed span,
        // we simulate a very bad case: 1 year without calls to `deposit_for`.
        // This should be a pretty safe upper bound
        System::<T>::set_block_number(T::Span::get() * 52u32.into());
        create_default_lock::<T>(origin.clone());
        distribute_rewards::<T>();

        let free_balance = T::Currency::free_balance(&origin);
        T::Currency::make_free_balance_be(&origin, free_balance + amount.into());

        #[extrinsic_call]
        increase_amount(RawOrigin::Signed(origin), amount.into());
    }

    #[benchmark]
    pub fn increase_unlock_height() {
        let origin: T::AccountId = account("Origin", 0, 0);
        // 52 weeks, i.e. 1 year. Since `increase_unlock_height` iterates ones per elapsed span,
        // we simulate a very bad case: 1 year without calls to `deposit_for`.
        // This should be a pretty safe upper bound
        System::<T>::set_block_number(T::Span::get() * 52u32.into());
        create_default_lock::<T>(origin.clone());
        let end_height = System::<T>::block_number() + T::MaxPeriod::get() - T::Span::get();
        System::<T>::set_block_number(end_height);
        distribute_rewards::<T>();

        #[extrinsic_call]
        increase_unlock_height(RawOrigin::Signed(origin), end_height + T::MaxPeriod::get());
    }

    #[benchmark]
    pub fn withdraw() {
        let origin: T::AccountId = account("Origin", 0, 0);
        // 52 weeks, i.e. 1 year. Since `increase_unlock_height` iterates ones per elapsed span,
        // we simulate a very bad case: 1 year without calls to `deposit_for`.
        // This should be a pretty safe upper bound
        System::<T>::set_block_number(T::Span::get() * 52u32.into());
        create_default_lock::<T>(origin.clone());
        let end_height = System::<T>::block_number() + T::MaxPeriod::get() + T::Span::get();
        System::<T>::set_block_number(end_height);
        distribute_rewards::<T>();

        #[extrinsic_call]
        withdraw(RawOrigin::Signed(origin));
    }

    impl_benchmark_test_suite! {Escrow, crate::mock::ExtBuilder::build(), crate::mock::Test}
}
