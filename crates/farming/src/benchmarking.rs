use super::*;
use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite};
use frame_support::assert_ok;
use frame_system::RawOrigin;
use primitives::*;

// Pallets
use crate::Pallet as Farming;

fn default_reward_schedule<T: Config>(reward_currency: CurrencyId) -> RewardScheduleOf<T> {
    let reward_schedule = RewardSchedule {
        start_height: 0u32.into(),
        period: 10u32.into(),
        period_count: 100u32.into(),
        per_period: 1000u32.into(),
    };
    let total_amount = reward_schedule.total().unwrap();

    assert_ok!(T::MultiCurrency::deposit(
        reward_currency,
        &Farming::<T>::treasury_account_id(),
        total_amount,
    ));

    reward_schedule
}

fn create_reward_schedule<T: Config>() -> (CurrencyId, CurrencyId) {
    let pool_id = CurrencyId::LpToken(LpToken::Token(DOT), LpToken::Token(IBTC));
    let reward_currency = CurrencyId::Token(INTR);
    let reward_schedule = default_reward_schedule::<T>(reward_currency);

    assert_ok!(Farming::<T>::update_reward_schedule(
        RawOrigin::Root.into(),
        pool_id,
        reward_currency,
        reward_schedule,
    ));

    (pool_id, reward_currency)
}

fn deposit_lp_tokens<T: Config>(pool_id: CurrencyId, account_id: &T::AccountId, amount: BalanceOf<T>) {
    assert_ok!(T::MultiCurrency::deposit(pool_id, account_id, amount,));
    assert_ok!(Farming::<T>::deposit(
        RawOrigin::Signed(account_id.clone()).into(),
        pool_id,
        amount,
    ));
}

benchmarks! {
    update_reward_schedule {
        let pool_id = CurrencyId::LpToken(LpToken::Token(DOT), LpToken::Token(IBTC));
        let reward_currency = CurrencyId::Token(INTR);
        let reward_schedule = default_reward_schedule::<T>(reward_currency);

    }: _(RawOrigin::Root, pool_id, reward_currency, reward_schedule)

    remove_reward_schedule {
        let (pool_id, reward_currency) = create_reward_schedule::<T>();

    }: _(RawOrigin::Root, pool_id, reward_currency)

    deposit {
        let origin: T::AccountId = account("Origin", 0, 0);
        let (pool_id, _) = create_reward_schedule::<T>();
        let amount = 100u32.into();
        assert_ok!(T::MultiCurrency::deposit(
            pool_id,
            &origin,
            amount,
        ));

    }: _(RawOrigin::Signed(origin), pool_id, amount)

    withdraw {
        let origin: T::AccountId = account("Origin", 0, 0);
        let (pool_id, _) = create_reward_schedule::<T>();
        let amount = 100u32.into();
        deposit_lp_tokens::<T>(pool_id, &origin, amount);

    }: _(RawOrigin::Signed(origin), pool_id, amount)

    claim {
        let origin: T::AccountId = account("Origin", 0, 0);
        let (pool_id, reward_currency) = create_reward_schedule::<T>();
        let amount = 100u32.into();
        deposit_lp_tokens::<T>(pool_id, &origin, amount);
        assert_ok!(T::RewardPools::distribute_reward(&pool_id, reward_currency, amount));

    }: _(RawOrigin::Signed(origin), pool_id, reward_currency)

}

impl_benchmark_test_suite!(Farming, crate::mock::ExtBuilder::build(), crate::mock::Test);
