// Copyright 2021-2022 Zenlink.
// Licensed under Apache 2.0.
#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;
use sp_std::prelude::*;

#[benchmarks(where T::CurrencyId: From<u32>)]
pub mod benchmarks {
    use super::*;
    use crate::Pallet as DexSwapRouter;

    #[benchmark]
    pub fn validate_routes(a: Linear<2, 10>) {
        let path: Vec<T::CurrencyId> = (0..a).map(Into::into).collect();
        let routes: Vec<_> = path
            .array_windows::<2>()
            .into_iter()
            .map(|[i, o]| Route::<T::StablePoolId, T::CurrencyId>::General(vec![*i, *o]))
            .collect();

        #[block]
        {
            DexSwapRouter::<T>::validate_routes(&routes).expect("Routes are valid");
        }
    }

    impl_benchmark_test_suite!(DexSwapRouter, crate::mock::ExtBuilder::build(), crate::mock::Test);
}
