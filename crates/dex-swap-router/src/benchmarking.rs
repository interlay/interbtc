// Copyright 2021-2022 Zenlink.
// Licensed under Apache 2.0.

#![cfg(feature = "runtime-benchmarks")]

use super::{StableSwapMode::*, *};

use frame_benchmarking::v2::*;
use frame_support::assert_ok;
use frame_system::RawOrigin;
use sp_std::prelude::*;

#[allow(unused)]
use crate::Pallet as SwapRouterPallet;
use dex_general::Pallet as GeneralAmmPallet;
use dex_stable::Pallet as StableAmmPallet;
use orml_traits::MultiCurrency;

const UNIT: u128 = 1_000_000_000_000u128;

const INITIAL_A_VALUE: u128 = 50;
const SWAP_FEE: u128 = 10000000;
const ADMIN_FEE: u128 = 0;
const FEE_RATE: u128 = 30;

#[benchmarks(
    where
        T: Config
            + dex_general::Config<AssetId = <T as Config>::CurrencyId>
            + dex_stable::Config<
                CurrencyId = <T as Config>::CurrencyId,
                PoolId = <T as Config>::StablePoolId
            >,
        <T as Config>::CurrencyId: From<u32>,
)]
pub mod benchmarks {
    use super::*;

    #[benchmark]
    pub fn swap_exact_tokens_for_tokens_general(a: Linear<2, 10>) {
        let caller: T::AccountId = whitelisted_caller();

        for asset in 0..a {
            assert_ok!(<T as dex_general::Config>::MultiCurrency::deposit(
                asset.into(),
                &caller,
                1000 * UNIT
            ));
        }

        let path: Vec<T::AssetId> = (0..a).map(Into::into).collect();
        for &[asset_0, asset_1] in path.array_windows::<2>() {
            assert_ok!(GeneralAmmPallet::<T>::create_pair(
                RawOrigin::Root.into(),
                asset_0,
                asset_1,
                FEE_RATE,
            ));

            assert_ok!(GeneralAmmPallet::<T>::add_liquidity(
                RawOrigin::Signed(caller.clone()).into(),
                asset_0,
                asset_1,
                100 * UNIT,
                100 * UNIT,
                0,
                0,
                100u32.into()
            ));
        }

        #[extrinsic_call]
        swap_exact_tokens_for_tokens(
            RawOrigin::Signed(caller.clone()),
            100u32.into(),
            0u32.into(),
            path.array_windows::<2>()
                .map(|[asset_0, asset_1]| {
                    Route::General(GeneralPath {
                        asset_0: asset_0.clone(),
                        asset_1: asset_1.clone(),
                    })
                })
                .collect::<Vec<_>>(),
            caller.clone(),
            1000u32.into(),
        );
    }

    #[benchmark]
    pub fn swap_exact_tokens_for_tokens_stable(c: Linear<2, 10>) {
        let caller: T::AccountId = whitelisted_caller();

        for currency in 0..c {
            assert_ok!(<T as dex_stable::Config>::MultiCurrency::deposit(
                currency.into(),
                &caller,
                1000 * UNIT
            ));
        }

        let path: Vec<<T as dex_stable::Config>::CurrencyId> = (0..c).map(Into::into).collect();
        let mut pools = Vec::new();

        for &[currency_0, currency_1] in path.array_windows::<2>() {
            let pool_id = StableAmmPallet::<T>::next_pool_id();
            assert_ok!(StableAmmPallet::<T>::create_base_pool(
                RawOrigin::Root.into(),
                [currency_0, currency_1].to_vec(),
                [12, 12].to_vec(),
                INITIAL_A_VALUE,
                SWAP_FEE,
                ADMIN_FEE,
                caller.clone(),
                Vec::from("stable_pool_lp_0")
            ));
            pools.push((currency_0, currency_1, pool_id));

            assert_ok!(StableAmmPallet::<T>::add_liquidity(
                RawOrigin::Signed(caller.clone()).into(),
                pool_id,
                [10 * UNIT, 10 * UNIT].to_vec(),
                0,
                caller.clone(),
                1000u32.into()
            ));
        }

        #[extrinsic_call]
        swap_exact_tokens_for_tokens(
            RawOrigin::Signed(caller.clone()),
            100u32.into(),
            0u32.into(),
            pools
                .into_iter()
                .map(|(currency_0, currency_1, pool_id)| {
                    Route::Stable(StablePath::<T::StablePoolId, <T as Config>::CurrencyId> {
                        pool_id: pool_id,
                        base_pool_id: pool_id,
                        mode: Single,
                        from_currency: currency_0,
                        to_currency: currency_1,
                    })
                })
                .collect::<Vec<_>>(),
            caller.clone(),
            1000u32.into(),
        );
    }

    impl_benchmark_test_suite!(SwapRouterPallet, crate::mock::ExtBuilder::build(), crate::mock::Test);
}
