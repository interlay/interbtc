use super::*;
use frame_benchmarking::{account, benchmarks_instance_pallet, impl_benchmark_test_suite};
use frame_support::{assert_ok, traits::Currency};
use frame_system::RawOrigin;
use sp_runtime::traits::One;

// Pallets
use crate::Pallet as Annuity;

benchmarks_instance_pallet! {
    withdraw_rewards {
        let origin: T::AccountId = account("Origin", 0, 0);
        assert_ok!(T::BlockRewardProvider::deposit_stake(&origin, One::one()));
        let account_id = Annuity::<T, I>::account_id();
        let balance = T::BlockNumberToBalance::convert(T::EmissionPeriod::get());
        assert_ok!(T::BlockRewardProvider::distribute_block_reward(&account_id, balance));
    }: _(RawOrigin::Signed(origin))

    update_rewards {
        T::Currency::make_free_balance_be(&Annuity::<T, I>::account_id(), T::BlockNumberToBalance::convert(T::EmissionPeriod::get()));
    }: _(RawOrigin::Root)
    verify {
        assert_eq!(RewardPerBlock::<T, I>::get(), One::one());
    }
}

impl_benchmark_test_suite!(Annuity, crate::mock::ExtBuilder::build(), crate::mock::Test);
