use super::*;
use crate::Pallet as Fee;
use frame_benchmarking::{account, benchmarks};
use frame_support::{traits::Currency, StorageMap};
use frame_system::RawOrigin;
use sp_std::prelude::*;

const SEED: u32 = 0;
// existential deposit multiplier
const ED_MULTIPLIER: u32 = 10;

benchmarks! {
    withdraw_polka_btc {
        let fee_pool: T::AccountId = Fee::<T>::fee_pool_account_id();

        let existential_deposit = <<T as treasury::Config>::PolkaBTC as Currency<_>>::minimum_balance();
        let balance = existential_deposit.saturating_mul(ED_MULTIPLIER.into());
        let _ = <<T as treasury::Config>::PolkaBTC as Currency<_>>::make_free_balance_be(&fee_pool, balance);

        let recipient: T::AccountId = account("recipient", 0, SEED);
        let amount = existential_deposit.saturating_mul((ED_MULTIPLIER - 1).into()) + 1u32.into();
        <TotalRewardsPolkaBTC<T>>::insert(recipient.clone(), amount);

    }: _(RawOrigin::Signed(recipient), amount)

    withdraw_dot {
        let fee_pool: T::AccountId = Fee::<T>::fee_pool_account_id();

        let existential_deposit = <<T as collateral::Config>::DOT as Currency<_>>::minimum_balance();
        let balance = existential_deposit.saturating_mul(ED_MULTIPLIER.into());
        let _ = <<T as collateral::Config>::DOT as Currency<_>>::make_free_balance_be(&fee_pool, balance);

        let recipient: T::AccountId = account("recipient", 0, SEED);
        let amount = existential_deposit.saturating_mul((ED_MULTIPLIER - 1).into()) + 1u32.into();
        <TotalRewardsDOT<T>>::insert(recipient.clone(), amount);

    }: _(RawOrigin::Signed(recipient), amount)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{ExtBuilder, Test};
    use frame_support::assert_ok;

    #[test]
    fn test_benchmarks() {
        ExtBuilder::build().execute_with(|| {
            assert_ok!(test_benchmark_withdraw_polka_btc::<Test>());
            assert_ok!(test_benchmark_withdraw_dot::<Test>());
        });
    }
}
