use super::*;
use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite};
use frame_system::RawOrigin;
use primitives::VaultId;

#[cfg(test)]
use crate::Pallet as Fee;

const SEED: u32 = 0;

benchmarks! {
    withdraw_rewards {
        let nominator: T::AccountId = account("recipient", 0, SEED);
        let vault_id = VaultId::new(nominator.clone(), T::GetWrappedCurrencyId::get(), T::GetWrappedCurrencyId::get());
    }: _(RawOrigin::Signed(nominator), vault_id, None)
}

impl_benchmark_test_suite!(Fee, crate::mock::ExtBuilder::build(), crate::mock::Test);
