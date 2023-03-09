// Copyright 2021-2022 Zenlink.
// Licensed under Apache 2.0.

//! Runtime API definition for swap router.

#![cfg_attr(not(feature = "std"), no_std)]
// The `too_many_arguments` warning originates from `decl_runtime_apis` macro.
#![allow(clippy::too_many_arguments)]
// The `unnecessary_mut_passed` warning originates from `decl_runtime_apis` macro.
#![allow(clippy::unnecessary_mut_passed)]
use codec::Codec;
use sp_std::vec::Vec;

pub use dex_swap_router::Route;

sp_api::decl_runtime_apis! {
    pub trait DexSwapRouterApi<Balance, CurrencyId, PoolId> where
        Balance: Codec,
        CurrencyId: Codec,
        PoolId: Codec,
    {
        fn find_best_trade_exact_in(
            input_amount: Balance,
            input_currency: CurrencyId,
            output_currency: CurrencyId
        ) -> Option<(Balance, Vec<Route<PoolId, CurrencyId>>)>;
    }
}
