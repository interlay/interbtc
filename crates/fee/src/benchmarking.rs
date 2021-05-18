use super::*;
use frame_benchmarking::{account, benchmarks};
use frame_system::RawOrigin;

const SEED: u32 = 0;

benchmarks! {
    withdraw_vault_backing_rewards {
        let recipient: T::AccountId = account("recipient", 0, SEED);
    }: _(RawOrigin::Signed(recipient))

    withdraw_vault_issuing_rewards {
        let recipient: T::AccountId = account("recipient", 0, SEED);
    }: _(RawOrigin::Signed(recipient))

    withdraw_relayer_backing_rewards {
        let recipient: T::AccountId = account("recipient", 0, SEED);
    }: _(RawOrigin::Signed(recipient))

    withdraw_relayer_issuing_rewards {
        let recipient: T::AccountId = account("recipient", 0, SEED);
    }: _(RawOrigin::Signed(recipient))

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{ExtBuilder, Test};
    use frame_support::assert_ok;

    #[test]
    fn test_benchmarks() {
        ExtBuilder::build().execute_with(|| {
            assert_ok!(test_benchmark_withdraw_vault_backing_rewards::<Test>());
            assert_ok!(test_benchmark_withdraw_vault_issuing_rewards::<Test>());
            assert_ok!(test_benchmark_withdraw_relayer_backing_rewards::<Test>());
            assert_ok!(test_benchmark_withdraw_relayer_issuing_rewards::<Test>());
        });
    }
}
