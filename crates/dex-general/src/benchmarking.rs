// Copyright 2021-2022 Zenlink.
// Licensed under Apache 2.0.

#![cfg(feature = "runtime-benchmarks")]

use super::*;

use frame_benchmarking::v2::*;
use frame_support::assert_ok;
use frame_system::RawOrigin;
use sp_runtime::SaturatedConversion;

const UNIT: u128 = 1_000_000_000_000;

const ASSET_0: u32 = 0;
const ASSET_1: u32 = 1;

pub fn lookup_of_account<T: Config>(
    who: T::AccountId,
) -> <<T as frame_system::Config>::Lookup as StaticLookup>::Source {
    <T as frame_system::Config>::Lookup::unlookup(who)
}

pub fn run_to_block<T: Config>(n: u32) {
    type System<T> = frame_system::Pallet<T>;

    while System::<T>::block_number() < n.saturated_into() {
        System::<T>::on_finalize(System::<T>::block_number());
        System::<T>::set_block_number(System::<T>::block_number() + 1u128.saturated_into());
        System::<T>::on_initialize(System::<T>::block_number());
    }
}

#[benchmarks(where T::AssetId: From<u32>)]
pub mod benchmarks {
    use super::*;
    use crate::Pallet as DexGeneral;

    #[benchmark]
    pub fn set_fee_receiver() {
        let caller: T::AccountId = whitelisted_caller();
        #[extrinsic_call]
        DexGeneral::set_fee_receiver(RawOrigin::Root, lookup_of_account::<T>(caller.clone()).into());
    }

    #[benchmark]
    pub fn set_fee_point() {
        #[extrinsic_call]
        DexGeneral::set_fee_point(RawOrigin::Root, 5);
    }

    #[benchmark]
    pub fn create_pair() {
        let caller: T::AccountId = whitelisted_caller();

        assert_ok!(<T as Config>::MultiCurrency::deposit(
            ASSET_0.into(),
            &caller,
            1000 * UNIT
        ));
        assert_ok!(<T as Config>::MultiCurrency::deposit(
            ASSET_1.into(),
            &caller,
            1000 * UNIT
        ));

        #[extrinsic_call]
        DexGeneral::create_pair(RawOrigin::Root, ASSET_0.into(), ASSET_1.into(), DEFAULT_FEE_RATE);
    }

    #[benchmark]
    pub fn bootstrap_create(r: Linear<1, 10>, l: Linear<1, 10>) {
        let rewards: Vec<T::AssetId> = (0..r).map(Into::into).collect();
        let limits: Vec<(T::AssetId, u128)> = (0..l).map(|a| (a.into(), 0)).collect();

        #[extrinsic_call]
        _(
            RawOrigin::Root,
            ASSET_0.into(),
            ASSET_1.into(),
            1000,
            1000,
            1000_000_000,
            1000_000_000,
            100u128.saturated_into(),
            rewards,
            limits,
        );
    }

    #[benchmark]
    pub fn bootstrap_contribute() {
        let caller: T::AccountId = whitelisted_caller();

        assert_ok!(<T as Config>::MultiCurrency::deposit(
            ASSET_0.into(),
            &caller,
            1000 * UNIT
        ));
        assert_ok!(<T as Config>::MultiCurrency::deposit(
            ASSET_1.into(),
            &caller,
            1000 * UNIT
        ));

        let reward: Vec<T::AssetId> = vec![ASSET_0.into()];
        let reward_amounts: Vec<(T::AssetId, u128)> = vec![(ASSET_1.into(), 0)];
        assert_ok!(DexGeneral::<T>::bootstrap_create(
            (RawOrigin::Root).into(),
            ASSET_0.into(),
            ASSET_1.into(),
            1000,
            1000,
            1000_000_000,
            1000_000_000,
            100u128.saturated_into(),
            reward,
            reward_amounts,
        ));

        #[extrinsic_call]
        DexGeneral::bootstrap_contribute(
            RawOrigin::Signed(caller.clone()),
            ASSET_0.into(),
            ASSET_1.into(),
            UNIT,
            UNIT,
            100u128.saturated_into(),
        );
    }

    #[benchmark]
    pub fn bootstrap_claim() {
        let caller: T::AccountId = whitelisted_caller();

        assert_ok!(<T as Config>::MultiCurrency::deposit(
            ASSET_0.into(),
            &caller,
            1000 * UNIT
        ));
        assert_ok!(<T as Config>::MultiCurrency::deposit(
            ASSET_1.into(),
            &caller,
            1000 * UNIT
        ));

        let reward: Vec<T::AssetId> = vec![ASSET_0.into()];
        let reward_amounts: Vec<(T::AssetId, u128)> = vec![(ASSET_1.into(), 0)];

        assert_ok!(DexGeneral::<T>::bootstrap_create(
            (RawOrigin::Root).into(),
            ASSET_0.into(),
            ASSET_1.into(),
            1000,
            1000,
            10 * UNIT,
            10 * UNIT,
            99u128.saturated_into(),
            reward,
            reward_amounts,
        ));

        assert_ok!(DexGeneral::<T>::bootstrap_contribute(
            RawOrigin::Signed(caller.clone()).into(),
            ASSET_0.into(),
            ASSET_1.into(),
            10 * UNIT,
            10 * UNIT,
            99u128.saturated_into()
        ));

        run_to_block::<T>(100);

        assert_ok!(DexGeneral::<T>::bootstrap_end(
            RawOrigin::Signed(caller.clone()).into(),
            ASSET_0.into(),
            ASSET_1.into(),
        ));

        #[extrinsic_call]
        DexGeneral::bootstrap_claim(
            RawOrigin::Signed(caller.clone()),
            lookup_of_account::<T>(caller.clone()),
            ASSET_0.into(),
            ASSET_1.into(),
            120u128.saturated_into(),
        );
    }

    #[benchmark]
    pub fn bootstrap_end() {
        let caller: T::AccountId = whitelisted_caller();

        assert_ok!(<T as Config>::MultiCurrency::deposit(
            ASSET_0.into(),
            &caller,
            1000 * UNIT
        ));
        assert_ok!(<T as Config>::MultiCurrency::deposit(
            ASSET_1.into(),
            &caller,
            1000 * UNIT
        ));

        let reward: Vec<T::AssetId> = vec![ASSET_0.into()];
        let reward_amounts: Vec<(T::AssetId, u128)> = vec![(ASSET_1.into(), 0)];

        assert_ok!(DexGeneral::<T>::bootstrap_create(
            (RawOrigin::Root).into(),
            ASSET_0.into(),
            ASSET_1.into(),
            1000,
            1000,
            10 * UNIT,
            10 * UNIT,
            99u128.saturated_into(),
            reward,
            reward_amounts,
        ));

        assert_ok!(DexGeneral::<T>::bootstrap_contribute(
            RawOrigin::Signed(caller.clone()).into(),
            ASSET_0.into(),
            ASSET_1.into(),
            10 * UNIT,
            10 * UNIT,
            99u128.saturated_into()
        ));

        run_to_block::<T>(100);

        #[extrinsic_call]
        DexGeneral::bootstrap_end(RawOrigin::Signed(caller.clone()), ASSET_0.into(), ASSET_1.into());
    }

    #[benchmark]
    pub fn bootstrap_update(r: Linear<1, 10>, l: Linear<1, 10>) {
        let caller: T::AccountId = whitelisted_caller();

        let rewards: Vec<T::AssetId> = (0..r).map(Into::into).collect();
        let limits: Vec<(T::AssetId, u128)> = (0..l).map(|a| (a.into(), 0)).collect();

        assert_ok!(DexGeneral::<T>::bootstrap_create(
            (RawOrigin::Root).into(),
            ASSET_0.into(),
            ASSET_1.into(),
            1000,
            1000,
            10 * UNIT,
            10 * UNIT,
            99u128.saturated_into(),
            rewards.clone(),
            limits.clone(),
        ));

        #[extrinsic_call]
        _(
            RawOrigin::Root,
            ASSET_0.into(),
            ASSET_1.into(),
            1000,
            1000,
            1000_000_000,
            1000_000_000,
            100u128.saturated_into(),
            rewards,
            limits,
        );
    }

    #[benchmark]
    pub fn bootstrap_refund() {
        let caller: T::AccountId = whitelisted_caller();

        assert_ok!(<T as Config>::MultiCurrency::deposit(
            ASSET_0.into(),
            &caller,
            1000 * UNIT
        ));
        assert_ok!(<T as Config>::MultiCurrency::deposit(
            ASSET_1.into(),
            &caller,
            1000 * UNIT
        ));

        let reward: Vec<T::AssetId> = vec![ASSET_0.into()];
        let reward_amounts: Vec<(T::AssetId, u128)> = vec![(ASSET_1.into(), 0)];

        assert_ok!(DexGeneral::<T>::bootstrap_create(
            (RawOrigin::Root).into(),
            ASSET_0.into(),
            ASSET_1.into(),
            2 * UNIT,
            2 * UNIT,
            10 * UNIT,
            10 * UNIT,
            99u128.saturated_into(),
            reward,
            reward_amounts,
        ));

        assert_ok!(DexGeneral::<T>::bootstrap_contribute(
            RawOrigin::Signed(caller.clone()).into(),
            ASSET_0.into(),
            ASSET_1.into(),
            1 * UNIT,
            1 * UNIT,
            99u128.saturated_into()
        ));
        run_to_block::<T>(100);

        #[extrinsic_call]
        DexGeneral::bootstrap_refund(RawOrigin::Signed(caller.clone()), ASSET_0.into(), ASSET_1.into());
    }

    #[benchmark]
    pub fn add_liquidity() {
        let caller: T::AccountId = whitelisted_caller();
        assert_ok!(<T as Config>::MultiCurrency::deposit(
            ASSET_0.into(),
            &caller,
            1000 * UNIT
        ));
        assert_ok!(<T as Config>::MultiCurrency::deposit(
            ASSET_1.into(),
            &caller,
            1000 * UNIT
        ));

        assert_ok!(DexGeneral::<T>::create_pair(
            (RawOrigin::Root).into(),
            ASSET_0.into(),
            ASSET_1.into(),
            DEFAULT_FEE_RATE
        ));

        assert_ok!(DexGeneral::<T>::set_fee_receiver(
            (RawOrigin::Root).into(),
            lookup_of_account::<T>(caller.clone()).into()
        ));

        #[extrinsic_call]
        DexGeneral::add_liquidity(
            RawOrigin::Signed(caller.clone()),
            ASSET_0.into(),
            ASSET_1.into(),
            10 * UNIT,
            10 * UNIT,
            0,
            0,
            100u32.saturated_into(),
        );
    }

    #[benchmark]
    pub fn remove_liquidity() {
        let caller: T::AccountId = whitelisted_caller();
        assert_ok!(<T as Config>::MultiCurrency::deposit(
            ASSET_0.into(),
            &caller,
            1000 * UNIT
        ));
        assert_ok!(<T as Config>::MultiCurrency::deposit(
            ASSET_1.into(),
            &caller,
            1000 * UNIT
        ));

        assert_ok!(DexGeneral::<T>::create_pair(
            (RawOrigin::Root).into(),
            ASSET_0.into(),
            ASSET_1.into(),
            DEFAULT_FEE_RATE
        ));

        assert_ok!(DexGeneral::<T>::set_fee_receiver(
            (RawOrigin::Root).into(),
            lookup_of_account::<T>(caller.clone()).into()
        ));

        assert_ok!(DexGeneral::<T>::add_liquidity(
            RawOrigin::Signed(caller.clone()).into(),
            ASSET_0.into(),
            ASSET_1.into(),
            10 * UNIT,
            10 * UNIT,
            0,
            0,
            100u32.saturated_into()
        ));

        #[extrinsic_call]
        DexGeneral::remove_liquidity(
            RawOrigin::Signed(caller.clone()),
            ASSET_0.into(),
            ASSET_1.into(),
            1 * UNIT,
            0,
            0,
            lookup_of_account::<T>(caller.clone()).into(),
            100u32.saturated_into(),
        );
    }

    #[benchmark]
    pub fn swap_exact_assets_for_assets(a: Linear<2, 10>) {
        let caller: T::AccountId = whitelisted_caller();

        for asset in 0..a {
            assert_ok!(<T as Config>::MultiCurrency::deposit(
                asset.into(),
                &caller,
                1000 * UNIT
            ));
        }

        let path: Vec<T::AssetId> = (0..a).map(Into::into).collect();
        for &[asset_0, asset_1] in path.array_windows::<2>() {
            assert_ok!(DexGeneral::<T>::create_pair(
                (RawOrigin::Root).into(),
                asset_0,
                asset_1,
                DEFAULT_FEE_RATE
            ));

            assert_ok!(DexGeneral::<T>::add_liquidity(
                RawOrigin::Signed(caller.clone()).into(),
                asset_0,
                asset_1,
                10 * UNIT,
                10 * UNIT,
                0,
                0,
                100u32.saturated_into()
            ));
        }

        #[extrinsic_call]
        _(
            RawOrigin::Signed(caller.clone()),
            1 * UNIT,
            0,
            path,
            lookup_of_account::<T>(caller.clone()).into(),
            100u32.saturated_into(),
        );
    }

    #[benchmark]
    pub fn swap_assets_for_exact_assets(a: Linear<2, 10>) {
        let caller: T::AccountId = whitelisted_caller();

        for asset in 0..a {
            assert_ok!(<T as Config>::MultiCurrency::deposit(
                asset.into(),
                &caller,
                1000 * UNIT
            ));
        }

        let path: Vec<T::AssetId> = (0..a).map(Into::into).collect();
        for &[asset_0, asset_1] in path.array_windows::<2>() {
            assert_ok!(DexGeneral::<T>::create_pair(
                (RawOrigin::Root).into(),
                asset_0,
                asset_1,
                DEFAULT_FEE_RATE
            ));

            assert_ok!(DexGeneral::<T>::add_liquidity(
                RawOrigin::Signed(caller.clone()).into(),
                asset_0,
                asset_1,
                10 * UNIT,
                10 * UNIT,
                0,
                0,
                100u32.saturated_into()
            ));
        }

        #[extrinsic_call]
        _(
            RawOrigin::Signed(caller.clone()),
            1 * UNIT,
            100 * UNIT,
            path,
            lookup_of_account::<T>(caller.clone()).into(),
            100u32.saturated_into(),
        );
    }

    #[benchmark]
    pub fn bootstrap_charge_reward(r: Linear<1, 10>) {
        let caller: T::AccountId = whitelisted_caller();

        for asset in 0..r {
            assert_ok!(<T as Config>::MultiCurrency::deposit(
                asset.into(),
                &caller,
                1000 * UNIT
            ));
        }

        let rewards: Vec<T::AssetId> = (0..r).map(Into::into).collect();
        let limits: Vec<(T::AssetId, u128)> = vec![(ASSET_1.into(), 0)];

        assert_ok!(DexGeneral::<T>::bootstrap_create(
            (RawOrigin::Root).into(),
            ASSET_0.into(),
            ASSET_1.into(),
            2 * UNIT,
            2 * UNIT,
            10 * UNIT,
            10 * UNIT,
            99u128.saturated_into(),
            rewards,
            limits,
        ));

        let charge_rewards: Vec<(T::AssetId, u128)> = (0..r).map(|a| (a.into(), 100 * UNIT)).collect();

        #[extrinsic_call]
        _(
            RawOrigin::Signed(caller.clone()),
            ASSET_0.into(),
            ASSET_1.into(),
            charge_rewards,
        );
    }

    // TODO: parameterize by number of rewards
    #[benchmark]
    pub fn bootstrap_withdraw_reward() {
        let caller: T::AccountId = whitelisted_caller();

        assert_ok!(<T as Config>::MultiCurrency::deposit(
            ASSET_0.into(),
            &caller,
            1000 * UNIT
        ));
        assert_ok!(<T as Config>::MultiCurrency::deposit(
            ASSET_1.into(),
            &caller,
            1000 * UNIT
        ));

        let rewards: Vec<T::AssetId> = vec![ASSET_0.into()];
        let limits: Vec<(T::AssetId, u128)> = vec![(ASSET_1.into(), 0)];

        assert_ok!(DexGeneral::<T>::bootstrap_create(
            (RawOrigin::Root).into(),
            ASSET_0.into(),
            ASSET_1.into(),
            2 * UNIT,
            2 * UNIT,
            10 * UNIT,
            10 * UNIT,
            99u128.saturated_into(),
            rewards,
            limits.clone(),
        ));

        #[extrinsic_call]
        _(
            RawOrigin::Root,
            ASSET_0.into(),
            ASSET_1.into(),
            lookup_of_account::<T>(caller.clone()).into(),
        );
    }

    impl_benchmark_test_suite!(
        DexGeneral,
        crate::fee::mock::ExtBuilder::build(),
        crate::fee::mock::Test
    );
}
