use super::*;
use crate::CurrencyId::Token;
use frame_benchmarking::v2::*;
use frame_support::{assert_ok, traits::Hooks};
use frame_system::RawOrigin;
use primitives::*;
use sp_std::vec;

// Pallets
use crate::Pallet as Farming;
use frame_system::Pallet as System;

fn default_reward_schedule<T: Config>(reward_currency_id: CurrencyId) -> RewardScheduleOf<T> {
    let reward_schedule = RewardSchedule {
        period_count: 100u32,
        per_period: 1000u32.into(),
    };
    let total_amount = reward_schedule.total().unwrap();

    assert_ok!(T::MultiCurrency::deposit(
        reward_currency_id,
        &T::TreasuryAccountId::get(),
        total_amount,
    ));

    reward_schedule
}

fn create_reward_schedule<T: Config>(pool_currency_id: CurrencyId, reward_currency_id: CurrencyId) {
    let reward_schedule = default_reward_schedule::<T>(reward_currency_id);

    assert_ok!(Farming::<T>::update_reward_schedule(
        RawOrigin::Root.into(),
        pool_currency_id,
        reward_currency_id,
        reward_schedule.period_count,
        reward_schedule.total().unwrap(),
    ));
}

const DEFAULT_POOL_CURRENCY_ID: CurrencyId = CurrencyId::LpToken(LpToken::Token(DOT), LpToken::Token(IBTC));

fn create_default_reward_schedule<T: Config>(c: u32) -> (CurrencyId, CurrencyId) {
    let pool_currency_id = DEFAULT_POOL_CURRENCY_ID;
    let reward_currency_id = CurrencyId::ForeignAsset(c);
    create_reward_schedule::<T>(pool_currency_id, reward_currency_id);
    (pool_currency_id, reward_currency_id)
}

fn deposit_lp_tokens<T: Config>(pool_currency_id: CurrencyId, account_id: &T::AccountId, amount: BalanceOf<T>) {
    assert_ok!(T::MultiCurrency::deposit(pool_currency_id, account_id, amount));
    assert_ok!(Farming::<T>::deposit(
        RawOrigin::Signed(account_id.clone()).into(),
        pool_currency_id,
        T::RewardPools::reward_currencies_len(&pool_currency_id)
    ));
}

fn create_multiple_reward_schedules<T: Config>(num_schedules: u32, caller: &T::AccountId) -> CurrencyId {
    let (pool_currency_id, reward_currency_id) = create_default_reward_schedule::<T>(0);
    deposit_lp_tokens::<T>(pool_currency_id, &caller, 100u32.into());
    // need to distribute rewards to add currency
    assert_ok!(T::RewardPools::distribute_reward(
        &pool_currency_id,
        reward_currency_id,
        100u32.into()
    ));
    for i in 1..num_schedules {
        let (_, reward_currency_id) = create_default_reward_schedule::<T>(i);
        assert_ok!(T::RewardPools::distribute_reward(
            &pool_currency_id,
            reward_currency_id,
            100u32.into()
        ));
    }
    assert_eq!(T::RewardPools::reward_currencies_len(&pool_currency_id), num_schedules);
    pool_currency_id
}

pub fn get_benchmarking_currency_ids() -> Vec<(CurrencyId, CurrencyId)> {
    vec![
        (Token(DOT), Token(INTR)),
        (Token(KSM), Token(KINT)),
        (Token(DOT), Token(IBTC)),
        (Token(KSM), Token(KBTC)),
    ]
}

#[benchmarks]
pub mod benchmarks {
    use super::*;

    #[benchmark]
    pub fn on_initialize(c: Linear<1, 4>) {
        let currency_ids = get_benchmarking_currency_ids();
        let block_number = T::RewardPeriod::get();

        for i in 0..c {
            let (pool_currency_id, reward_currency_id) = currency_ids[i as usize];
            create_reward_schedule::<T>(pool_currency_id, reward_currency_id);
        }

        Farming::<T>::on_initialize(1u32.into());
        System::<T>::set_block_number(block_number);
        #[block]
        {
            Farming::<T>::on_initialize(System::<T>::block_number());
        }
    }

    #[benchmark]
    pub fn update_reward_schedule() {
        let pool_currency_id = CurrencyId::LpToken(LpToken::Token(DOT), LpToken::Token(IBTC));
        let reward_currency_id = CurrencyId::Token(INTR);
        let reward_schedule = default_reward_schedule::<T>(reward_currency_id);

        #[extrinsic_call]
        _(
            RawOrigin::Root,
            pool_currency_id,
            reward_currency_id,
            reward_schedule.period_count,
            reward_schedule.total().unwrap(),
        );
    }

    #[benchmark]
    pub fn remove_reward_schedule() {
        let (pool_currency_id, reward_currency_id) = create_default_reward_schedule::<T>(0);

        #[extrinsic_call]
        _(RawOrigin::Root, pool_currency_id, reward_currency_id);
    }

    #[benchmark]
    pub fn deposit(c: Linear<1, 4>) {
        let caller = whitelisted_caller();
        let pool_currency_id = create_multiple_reward_schedules::<T>(c, &caller);
        assert_ok!(T::MultiCurrency::deposit(pool_currency_id, &caller, 100u32.into()));

        #[extrinsic_call]
        _(
            RawOrigin::Signed(caller.clone()),
            pool_currency_id,
            T::RewardPools::reward_currencies_len(&pool_currency_id),
        );

        // deposit can succeed with zero, so check stake
        assert_ok!(T::RewardPools::get_stake(&pool_currency_id, &caller), 200u32.into());
    }

    #[benchmark]
    pub fn withdraw(c: Linear<1, 4>) {
        let caller = whitelisted_caller();
        let pool_currency_id = create_multiple_reward_schedules::<T>(c, &caller);

        #[extrinsic_call]
        _(
            RawOrigin::Signed(caller.clone()),
            pool_currency_id,
            100u32.into(),
            T::RewardPools::reward_currencies_len(&pool_currency_id),
        );

        assert_ok!(T::RewardPools::get_stake(&pool_currency_id, &caller), 0u32.into());
    }

    #[benchmark]
    pub fn claim() {
        let caller = whitelisted_caller();
        let (pool_currency_id, reward_currency_id) = create_default_reward_schedule::<T>(0);
        let amount = 100u32.into();
        deposit_lp_tokens::<T>(pool_currency_id, &caller, amount);
        assert_ok!(T::RewardPools::distribute_reward(
            &pool_currency_id,
            reward_currency_id,
            amount
        ));

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), pool_currency_id, reward_currency_id);
    }

    impl_benchmark_test_suite!(Farming, crate::mock::ExtBuilder::build(), crate::mock::Test);
}
