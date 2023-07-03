use super::*;
use frame_benchmarking::v2::{account, benchmarks, impl_benchmark_test_suite};
use sp_std::vec;

use crate::Pallet as Fee;
use traits::NominationApi;

const SEED: u32 = 0;

#[benchmarks]
pub mod benchmarks {
    use super::*;
    use frame_system::RawOrigin;
    use primitives::VaultCurrencyPair;

    fn distribute_rewards<T: Config>(currency: CurrencyId<T>) {
        let amount = Amount::<T>::new(1000u32.into(), currency);
        amount.mint_to(&Fee::<T>::fee_pool_account_id()).unwrap();

        // distribute
        Fee::<T>::distribute_rewards(&amount).unwrap();
    }

    #[benchmark]
    fn withdraw_rewards() {
        let nominator: T::AccountId = account("recipient", 0, SEED);
        let vault_id = VaultId::new(
            nominator.clone(),
            T::GetWrappedCurrencyId::get(),
            T::GetWrappedCurrencyId::get(),
        );
        let wrapped = T::GetWrappedCurrencyId::get();
        let native = T::GetNativeCurrencyId::get();

        // set stakes so that we don't bail early
        T::CapacityRewards::set_stake(&(), &wrapped, 1000u32.into()).unwrap();
        T::CapacityRewards::set_stake(&(), &native, 1000u32.into()).unwrap();
        T::VaultRewards::set_stake(&wrapped, &vault_id, 1000u32.into()).unwrap();
        T::VaultStaking::set_stake(&(None, vault_id.clone()), &nominator, 1000u32.into()).unwrap();
        // slash stake so we hit the apply_slash
        T::VaultStaking::slash_stake(&vault_id, 500u32.into()).unwrap();

        // distribute rewards so we hit more code, and we can check that it works
        distribute_rewards::<T>(T::GetWrappedCurrencyId::get());
        distribute_rewards::<T>(T::GetNativeCurrencyId::get());

        #[extrinsic_call]
        withdraw_rewards(RawOrigin::Signed(nominator.clone()), vault_id, None);

        assert!(orml_tokens::module::Accounts::<T>::get(&nominator, T::GetWrappedCurrencyId::get()).free > 0u32.into());
        assert!(orml_tokens::module::Accounts::<T>::get(&nominator, T::GetNativeCurrencyId::get()).free > 0u32.into());
    }

    #[benchmark]
    fn set_issue_fee() {
        let fee = Fee::<T>::get_max_expected_value();

        #[extrinsic_call]
        set_issue_fee(RawOrigin::Root, fee);
    }

    #[benchmark]
    fn set_issue_griefing_collateral() {
        let rate = Fee::<T>::get_max_expected_value();

        #[extrinsic_call]
        set_issue_griefing_collateral(RawOrigin::Root, rate);
    }

    #[benchmark]
    fn set_redeem_fee() {
        let rate = Fee::<T>::get_max_expected_value();

        #[extrinsic_call]
        set_redeem_fee(RawOrigin::Root, rate);
    }

    #[benchmark]
    fn set_premium_redeem_fee() {
        let rate = Fee::<T>::get_max_expected_value();

        #[extrinsic_call]
        set_premium_redeem_fee(RawOrigin::Root, rate);
    }

    #[benchmark]
    fn set_punishment_fee() {
        let rate = Fee::<T>::get_max_expected_value();

        #[extrinsic_call]
        set_punishment_fee(RawOrigin::Root, rate);
    }

    #[benchmark]
    fn set_replace_griefing_collateral() {
        let rate = Fee::<T>::get_max_expected_value();

        #[extrinsic_call]
        set_replace_griefing_collateral(RawOrigin::Root, rate);
    }

    #[benchmark]
    fn set_commission() {
        let nominator: T::AccountId = account("recipient", 0, SEED);
        let arbitrary_pair = VaultCurrencyPair {
            collateral: T::GetNativeCurrencyId::get(),
            wrapped: T::GetNativeCurrencyId::get(),
        };
        let commission = Fee::<T>::get_max_expected_value(); //arbitrary value
        let vault_id = VaultId::new(
            nominator.clone(),
            T::GetNativeCurrencyId::get(),
            T::GetNativeCurrencyId::get(),
        );
        T::NominationApi::opt_in_to_nomination(&vault_id);
        #[extrinsic_call]
        set_commission(RawOrigin::Signed(nominator), arbitrary_pair, commission);
    }

    impl_benchmark_test_suite! { Fee, crate::mock::ExtBuilder::build(), crate::mock::Test }
}
