// Copyright 2021-2022 Zenlink.
// Licensed under Apache 2.0.

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::Pallet as StablePallet;

use frame_benchmarking::v2::*;
use frame_support::assert_ok;
use frame_system::RawOrigin;

const UNIT: u128 = 1_000_000_000_000;
const LP_UNIT: u128 = 1_000_000_000_000_000_000;

const INITIAL_A_VALUE: Balance = 50;
const SWAP_FEE: Balance = 10000000;
const ADMIN_FEE: Balance = 0;

pub fn lookup_of_account<T: Config>(
    who: T::AccountId,
) -> <<T as frame_system::Config>::Lookup as StaticLookup>::Source {
    <T as frame_system::Config>::Lookup::unlookup(who)
}

fn base_currencies<T: Config>(c: u32) -> Vec<T::CurrencyId>
where
    <T as Config>::CurrencyId: From<u32>,
{
    (0..c).map(Into::into).collect()
}

fn meta_currencies<T: Config>(c: u32, base_pool_id: T::PoolId) -> Vec<T::CurrencyId>
where
    <T as Config>::CurrencyId: From<u32>,
{
    let mut meta_currency_ids: Vec<T::CurrencyId> = (0..c - 1)
        .map(|c| c + T::PoolCurrencyLimit::get())
        .map(Into::into)
        .collect();
    meta_currency_ids.push(T::LpGenerate::generate_by_pool_id(base_pool_id));
    meta_currency_ids
}

fn setup_base_pool<T: Config>(caller: T::AccountId, base_currency_ids: Vec<T::CurrencyId>) -> T::PoolId {
    let base_pool_id = StablePallet::<T>::next_pool_id();

    assert_ok!(StablePallet::<T>::create_base_pool(
        (RawOrigin::Root).into(),
        base_currency_ids.clone(),
        vec![12; base_currency_ids.len()],
        INITIAL_A_VALUE,
        SWAP_FEE,
        ADMIN_FEE,
        caller.clone(),
        vec![0; T::PoolCurrencySymbolLimit::get() as usize],
    ));

    base_pool_id
}

fn setup_base_pool_and_add_liquidity<T: Config>(
    caller: T::AccountId,
    base_currency_ids: Vec<T::CurrencyId>,
) -> T::PoolId {
    let base_pool_id = setup_base_pool::<T>(caller.clone(), base_currency_ids.clone());

    for currency_id in &base_currency_ids {
        assert_ok!(T::MultiCurrency::deposit(currency_id.clone(), &caller, UNIT * 1000));
    }

    assert_ok!(StablePallet::<T>::add_liquidity(
        RawOrigin::Signed(caller.clone()).into(),
        base_pool_id,
        vec![100 * UNIT; base_currency_ids.len()],
        0,
        caller.clone(),
        1000u32.into()
    ));

    base_pool_id
}

fn setup_meta_pool<T: Config>(caller: T::AccountId, meta_currency_ids: Vec<T::CurrencyId>) -> T::PoolId {
    let mut meta_currency_decimals = vec![12; meta_currency_ids.len() - 1];
    meta_currency_decimals.push(POOL_LP_CURRENCY_ID_DECIMAL as u32);

    let meta_pool_id = StablePallet::<T>::next_pool_id();

    assert_ok!(StablePallet::<T>::create_meta_pool(
        RawOrigin::Root.into(),
        meta_currency_ids.clone(),
        meta_currency_decimals,
        INITIAL_A_VALUE,
        SWAP_FEE,
        ADMIN_FEE,
        caller.clone(),
        vec![0; T::PoolCurrencySymbolLimit::get() as usize],
    ));

    meta_pool_id
}

fn setup_meta_pool_and_add_liquidity<T: Config>(
    caller: T::AccountId,
    meta_currency_ids: Vec<T::CurrencyId>,
) -> T::PoolId {
    let meta_pool_id = setup_meta_pool::<T>(caller.clone(), meta_currency_ids.clone());

    for currency_id in meta_currency_ids.iter().rev().skip(1) {
        // skip lp token
        assert_ok!(T::MultiCurrency::deposit(currency_id.clone(), &caller, UNIT * 1000));
    }

    let mut amounts = vec![5 * UNIT; meta_currency_ids.len() - 1];
    amounts.push(5 * LP_UNIT);

    assert_ok!(StablePallet::<T>::add_liquidity(
        RawOrigin::Signed(caller.clone()).into(),
        meta_pool_id,
        amounts,
        0,
        caller.clone(),
        1000u32.into()
    ));

    meta_pool_id
}

#[benchmarks(where T: Config, T::CurrencyId: From<u32>)]
pub mod benchmarks {
    use super::*;

    #[benchmark]
    pub fn create_base_pool(b: Linear<2, 10>, s: Linear<0, 50>) {
        let admin_fee_receiver: T::AccountId = whitelisted_caller();
        let currency_ids: Vec<T::CurrencyId> = base_currencies::<T>(b);

        #[extrinsic_call]
        _(
            RawOrigin::Root,
            currency_ids,
            vec![12; b as usize],
            INITIAL_A_VALUE,
            SWAP_FEE,
            ADMIN_FEE,
            admin_fee_receiver,
            vec![0; s as usize],
        );
    }

    #[benchmark]
    pub fn create_meta_pool(m: Linear<2, 10>, s: Linear<0, 50>) {
        let caller: T::AccountId = whitelisted_caller();

        let base_pool_id =
            setup_base_pool_and_add_liquidity::<T>(caller.clone(), base_currencies::<T>(T::PoolCurrencyLimit::get()));
        let meta_currency_ids = meta_currencies::<T>(m, base_pool_id);

        let mut meta_currency_decimals = vec![12; meta_currency_ids.len() - 1];
        meta_currency_decimals.push(POOL_LP_CURRENCY_ID_DECIMAL as u32);

        #[extrinsic_call]
        _(
            RawOrigin::Root,
            meta_currency_ids,
            meta_currency_decimals,
            INITIAL_A_VALUE,
            SWAP_FEE,
            ADMIN_FEE,
            caller,
            vec![0; s as usize],
        );
    }

    // TODO: benchmark meta pool
    #[benchmark]
    pub fn add_liquidity(b: Linear<2, 10>) {
        let caller: T::AccountId = whitelisted_caller();
        let base_currency_ids = base_currencies::<T>(b);
        let base_pool_id = setup_base_pool::<T>(caller.clone(), base_currency_ids.clone());

        for currency_id in &base_currency_ids {
            assert_ok!(T::MultiCurrency::deposit(currency_id.clone(), &caller, UNIT * 1000));
        }

        #[extrinsic_call]
        StablePallet::add_liquidity(
            RawOrigin::Signed(caller.clone()),
            base_pool_id,
            vec![10 * UNIT; base_currency_ids.len()],
            0,
            caller.clone(),
            1000u32.into(),
        );
    }

    // TODO: benchmark meta pool
    // TODO: parameterize pool size?
    #[benchmark]
    pub fn swap() {
        let caller: T::AccountId = whitelisted_caller();
        let base_pool_id =
            setup_base_pool_and_add_liquidity::<T>(caller.clone(), base_currencies::<T>(T::PoolCurrencyLimit::get()));

        #[extrinsic_call]
        _(
            RawOrigin::Signed(caller.clone()),
            base_pool_id,
            0u32,
            1u32,
            1 * UNIT,
            0,
            caller.clone(),
            1000u32.into(),
        );
    }

    #[benchmark]
    pub fn remove_liquidity(b: Linear<2, 10>) {
        let caller: T::AccountId = whitelisted_caller();
        let base_pool_id = setup_base_pool_and_add_liquidity::<T>(caller.clone(), base_currencies::<T>(b));

        #[extrinsic_call]
        _(
            RawOrigin::Signed(caller.clone()),
            base_pool_id,
            1 * UNIT,
            vec![0; b as usize],
            caller.clone(),
            1000u32.into(),
        );
    }

    // TODO: benchmark meta pool
    #[benchmark]
    pub fn remove_liquidity_one_currency() {
        let caller: T::AccountId = whitelisted_caller();
        let base_pool_id =
            setup_base_pool_and_add_liquidity::<T>(caller.clone(), base_currencies::<T>(T::PoolCurrencyLimit::get()));

        #[extrinsic_call]
        _(
            RawOrigin::Signed(caller.clone()),
            base_pool_id,
            1 * UNIT,
            1,
            0,
            caller.clone(),
            1000u32.into(),
        );
    }

    // TODO: benchmark meta pool
    #[benchmark]
    pub fn remove_liquidity_imbalance(b: Linear<2, 10>) {
        let caller: T::AccountId = whitelisted_caller();
        let base_pool_id = setup_base_pool_and_add_liquidity::<T>(caller.clone(), base_currencies::<T>(b));
        let mut amounts = vec![10 * UNIT; (b - 1) as usize];
        amounts.push(1 * UNIT);

        #[extrinsic_call]
        _(
            RawOrigin::Signed(caller.clone()),
            base_pool_id,
            amounts,
            100 * LP_UNIT,
            caller.clone(),
            1000u32.into(),
        );
    }

    #[benchmark]
    pub fn add_pool_and_base_pool_liquidity(b: Linear<2, 10>, m: Linear<2, 10>) {
        let caller: T::AccountId = whitelisted_caller();
        let base_pool_id = setup_base_pool_and_add_liquidity::<T>(caller.clone(), base_currencies::<T>(b));
        let meta_currency_ids = meta_currencies::<T>(m, base_pool_id);
        let meta_pool_id = setup_meta_pool::<T>(caller.clone(), meta_currency_ids.clone());

        for currency_id in meta_currency_ids.iter().rev().skip(1) {
            // skip lp token
            assert_ok!(T::MultiCurrency::deposit(currency_id.clone(), &caller, UNIT * 1000));
        }

        let base_amounts = vec![10 * UNIT; b as usize];
        let meta_amounts = vec![10 * UNIT; meta_currency_ids.len()];

        #[extrinsic_call]
        _(
            RawOrigin::Signed(caller.clone()),
            meta_pool_id,
            base_pool_id,
            meta_amounts,
            base_amounts,
            0,
            caller.clone(),
            1000u32.into(),
        );
    }

    #[benchmark]
    pub fn remove_pool_and_base_pool_liquidity(b: Linear<2, 10>, m: Linear<2, 10>) {
        let caller: T::AccountId = whitelisted_caller();
        let base_pool_id = setup_base_pool_and_add_liquidity::<T>(caller.clone(), base_currencies::<T>(b));
        let meta_pool_id =
            setup_meta_pool_and_add_liquidity::<T>(caller.clone(), meta_currencies::<T>(m, base_pool_id));

        let min_amounts_base = vec![0; b as usize];
        let min_amounts_meta = vec![0; m as usize];

        #[extrinsic_call]
        _(
            RawOrigin::Signed(caller.clone()),
            meta_pool_id,
            base_pool_id,
            10 * LP_UNIT,
            min_amounts_meta,
            min_amounts_base,
            caller.clone(),
            1000u32.into(),
        );
    }

    #[benchmark]
    pub fn remove_pool_and_base_pool_liquidity_one_currency() {
        let caller: T::AccountId = whitelisted_caller();
        let base_pool_id =
            setup_base_pool_and_add_liquidity::<T>(caller.clone(), base_currencies::<T>(T::PoolCurrencyLimit::get()));
        let meta_pool_id = setup_meta_pool_and_add_liquidity::<T>(
            caller.clone(),
            meta_currencies::<T>(T::PoolCurrencyLimit::get(), base_pool_id),
        );

        #[extrinsic_call]
        _(
            RawOrigin::Signed(caller.clone()),
            meta_pool_id,
            base_pool_id,
            10 * LP_UNIT,
            0,
            0,
            caller.clone(),
            1000u32.into(),
        );
    }

    #[benchmark]
    pub fn swap_pool_from_base() {
        let caller: T::AccountId = whitelisted_caller();
        let base_pool_id =
            setup_base_pool_and_add_liquidity::<T>(caller.clone(), base_currencies::<T>(T::PoolCurrencyLimit::get()));
        let meta_pool_id = setup_meta_pool_and_add_liquidity::<T>(
            caller.clone(),
            meta_currencies::<T>(T::PoolCurrencyLimit::get(), base_pool_id),
        );

        #[extrinsic_call]
        _(
            RawOrigin::Signed(caller.clone()),
            meta_pool_id,
            base_pool_id,
            0,
            0,
            1 * UNIT,
            0,
            caller.clone(),
            1000u32.into(),
        );
    }

    #[benchmark]
    pub fn swap_pool_to_base() {
        let caller: T::AccountId = whitelisted_caller();
        let base_pool_id =
            setup_base_pool_and_add_liquidity::<T>(caller.clone(), base_currencies::<T>(T::PoolCurrencyLimit::get()));
        let meta_pool_id = setup_meta_pool_and_add_liquidity::<T>(
            caller.clone(),
            meta_currencies::<T>(T::PoolCurrencyLimit::get(), base_pool_id),
        );

        #[extrinsic_call]
        _(
            RawOrigin::Signed(caller.clone()),
            meta_pool_id,
            base_pool_id,
            0,
            0,
            1 * UNIT,
            0,
            caller.clone(),
            1000u32.into(),
        );
    }

    #[benchmark]
    pub fn swap_meta_pool_underlying() {
        let caller: T::AccountId = whitelisted_caller();
        let base_pool_id =
            setup_base_pool_and_add_liquidity::<T>(caller.clone(), base_currencies::<T>(T::PoolCurrencyLimit::get()));
        let meta_pool_id = setup_meta_pool_and_add_liquidity::<T>(
            caller.clone(),
            meta_currencies::<T>(T::PoolCurrencyLimit::get(), base_pool_id),
        );

        #[extrinsic_call]
        _(
            RawOrigin::Signed(caller.clone()),
            meta_pool_id,
            0,
            2,
            1 * UNIT,
            0,
            caller.clone(),
            1000u32.into(),
        );
    }

    #[benchmark]
    pub fn withdraw_admin_fee() {
        let caller: T::AccountId = whitelisted_caller();
        let base_pool_id =
            setup_base_pool_and_add_liquidity::<T>(caller.clone(), base_currencies::<T>(T::PoolCurrencyLimit::get()));

        assert_ok!(StablePallet::<T>::swap(
            RawOrigin::Signed(caller.clone()).into(),
            base_pool_id,
            0u32,
            1u32,
            1 * UNIT,
            0,
            caller.clone(),
            1000u32.into()
        ));

        #[extrinsic_call]
        _(RawOrigin::Signed(caller.clone()), base_pool_id);
    }

    impl_benchmark_test_suite!(StablePallet, crate::mock::ExtBuilder::build(), crate::mock::Test);
}
