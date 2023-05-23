use super::*;
use frame_benchmarking::v2::*;
use frame_support::{assert_ok, traits::Currency};
use frame_system::RawOrigin;
use sp_runtime::traits::One;

// Pallets
use crate::Pallet as Annuity;

#[instance_benchmarks]
pub mod benchmarks {
    use super::*;

    #[benchmark]
    fn on_initialize() {
        let caller = whitelisted_caller();
        assert_ok!(T::BlockRewardProvider::deposit_stake(&caller, One::one()));
        T::Currency::make_free_balance_be(&Annuity::<T, I>::account_id(), Annuity::<T, I>::min_reward_per_block());

        #[block]
        {
            assert_ok!(Annuity::<T, I>::begin_block(1u32.into()));
        }
    }

    #[benchmark]
    fn withdraw_rewards() -> Result<(), BenchmarkError> {
        let caller = whitelisted_caller();
        assert_ok!(T::BlockRewardProvider::deposit_stake(&caller, One::one()));

        // this only returns an error for `VaultAnnuity`,
        // since calls are not enabled for that instance we
        // can set the weight of this call to zero
        T::BlockRewardProvider::can_withdraw_reward()
            .then(|| ())
            .ok_or(BenchmarkError::Weightless)?;

        let account_id = Annuity::<T, I>::account_id();
        let balance = T::BlockNumberToBalance::convert(T::EmissionPeriod::get());
        T::Currency::make_free_balance_be(&account_id, balance);
        assert_ok!(T::BlockRewardProvider::distribute_block_reward(&account_id, balance));

        let rewards_before = T::Currency::free_balance(&caller);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller.clone()));

        // only one account with stake so they get all of the rewards
        assert_eq!(T::Currency::free_balance(&caller), rewards_before + balance);

        Ok(())
    }

    #[benchmark]
    fn update_rewards() {
        T::Currency::make_free_balance_be(
            &Annuity::<T, I>::account_id(),
            T::BlockNumberToBalance::convert(T::EmissionPeriod::get()),
        );

        #[extrinsic_call]
        _(RawOrigin::Root);

        assert_eq!(RewardPerBlock::<T, I>::get(), One::one());
    }

    #[benchmark]
    fn set_reward_per_wrapped() {
        #[extrinsic_call]
        _(RawOrigin::Root, One::one());

        assert_eq!(RewardPerWrapped::<T, I>::get(), Some(One::one()));
    }

    impl_benchmark_test_suite! {Annuity, crate::mock::ExtBuilder::build(), crate::mock::Test}
}
