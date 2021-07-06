use super::*;
use crate::Pallet as Fee;
use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite};
use frame_system::RawOrigin;

const SEED: u32 = 0;

benchmarks! {
    withdraw_rewards {
        let recipient: T::AccountId = account("recipient", 0, SEED);
    }: _(RawOrigin::Signed(recipient.clone()), recipient.clone())
}

impl_benchmark_test_suite!(Fee, crate::mock::ExtBuilder::build(), crate::mock::Test);
