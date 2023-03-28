// Copyright 2021-2022 Zenlink.
// Licensed under Apache 2.0.

#![cfg(feature = "runtime-benchmarks")]

// use crate::fee::mock::CurrencyId;

use super::*;

use frame_benchmarking::v2::{account, benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_support::assert_ok;
use frame_system::RawOrigin;
use sp_runtime::{traits::Convert, SaturatedConversion};

const SEED: u32 = 0;
const UNIT: u128 = 1_000_000_000_000;

#[derive(Encode, Decode, Eq, Hash, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord, TypeInfo, MaxEncodedLen)]
pub enum CurrencyId {
    Token(u8),
    LpToken(u8, u8),
}

impl CurrencyId {
    pub fn join_lp_token(currency_id_0: Self, currency_id_1: Self) -> Option<Self> {
        let lp_token_0 = match currency_id_0 {
            CurrencyId::Token(x) => x,
            _ => return None,
        };
        let lp_token_1 = match currency_id_1 {
            CurrencyId::Token(y) => y,
            _ => return None,
        };
        Some(CurrencyId::LpToken(lp_token_0, lp_token_1))
    }
}

impl AssetInfo for CurrencyId {
    fn is_support(&self) -> bool {
        match self {
            Self::Token(_) => true,
            _ => false,
        }
    }
}

const ASSET_0: CurrencyId = CurrencyId::Token(0);
const ASSET_1: CurrencyId = CurrencyId::Token(1);
const ASSET_2: CurrencyId = CurrencyId::Token(2);

// const ASSET_0: u32 = 0;
// const ASSET_1: u32 = 1;
// const ASSET_2: u32 = 2;

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

#[benchmarks]
pub mod benchmarks {
    use super::*;
    use crate::Pallet as DexGeneral;

    #[benchmark]
    pub fn set_fee_receiver() {
        let receiver: T::AccountId = account("receiver", 0, SEED);

        #[extrinsic_call]
        set_fee_receiver(RawOrigin::Root, Some(lookup_of_account::<T>(receiver)));
    }
    //
    //     #[benchmark]
    //     pub fn set_fee_point() {
    //         #[extrinsic_call]
    //         DexGeneral::set_fee_point(RawOrigin::Root, 5);
    //     }
    //
    #[benchmark]
    pub fn create_pair() {
        let caller: T::AccountId = whitelisted_caller();

        let asset_0 = T::GetBenchmarkAsset::convert(0);
        let asset_1 = T::GetBenchmarkAsset::convert(1);
        assert_ok!(<T as Config>::MultiCurrency::deposit(asset_0, &caller, 1000 * UNIT));
        assert_ok!(<T as Config>::MultiCurrency::deposit(asset_1, &caller, 1000 * UNIT));

        #[extrinsic_call]
        create_pair(RawOrigin::Root, asset_0, asset_1, DEFAULT_FEE_RATE);
    }
    //
    //     #[benchmark]
    //     pub fn bootstrap_create() {
    //         let reward: Vec<T::AssetId> = vec![ASSET_0];
    //         let reward_amounts: Vec<(T::AssetId, u128)> = vec![(ASSET_1, 0)];
    //
    //         #[extrinsic_call]
    //         DexGeneral::bootstrap_create(
    //             RawOrigin::Root,
    //             ASSET_0,
    //             ASSET_1,
    //             1000,
    //             1000,
    //             1000_000_000,
    //             1000_000_000,
    //             100u128.saturated_into(),
    //             reward,
    //             reward_amounts,
    //         );
    //     }
    //
    //     #[benchmark]
    //     pub fn bootstrap_contribute() {
    //         let caller: T::AccountId = whitelisted_caller();
    //
    //         assert_ok!(<T as Config>::MultiCurrency::deposit(
    //             ASSET_0,
    //             &caller,
    //             1000 * UNIT
    //         ));
    //         assert_ok!(<T as Config>::MultiCurrency::deposit(
    //             ASSET_1,
    //             &caller,
    //             1000 * UNIT
    //         ));
    //
    //         let reward: Vec<T::AssetId> = vec![ASSET_0];
    //         let reward_amounts: Vec<(T::AssetId, u128)> = vec![(ASSET_1, 0)];
    //         assert_ok!(DexGeneral::<T>::bootstrap_create(
    //             (RawOrigin::Root),
    //             ASSET_0,
    //             ASSET_1,
    //             1000,
    //             1000,
    //             1000_000_000,
    //             1000_000_000,
    //             100u128.saturated_into(),
    //             reward,
    //             reward_amounts,
    //         ));
    //
    //         #[extrinsic_call]
    //         DexGeneral::bootstrap_contribute(
    //             RawOrigin::Signed(caller.clone()),
    //             ASSET_0,
    //             ASSET_1,
    //             UNIT,
    //             UNIT,
    //             100u128.saturated_into(),
    //         );
    //     }
    //
    //     #[benchmark]
    //     pub fn bootstrap_claim() {
    //         let caller: T::AccountId = whitelisted_caller();
    //
    //         assert_ok!(<T as Config>::MultiCurrency::deposit(
    //             ASSET_0,
    //             &caller,
    //             1000 * UNIT
    //         ));
    //         assert_ok!(<T as Config>::MultiCurrency::deposit(
    //             ASSET_1,
    //             &caller,
    //             1000 * UNIT
    //         ));
    //
    //         let reward: Vec<T::AssetId> = vec![ASSET_0];
    //         let reward_amounts: Vec<(T::AssetId, u128)> = vec![(ASSET_1, 0)];
    //
    //         assert_ok!(DexGeneral::<T>::bootstrap_create(
    //             (RawOrigin::Root),
    //             ASSET_0,
    //             ASSET_1,
    //             1000,
    //             1000,
    //             10 * UNIT,
    //             10 * UNIT,
    //             99u128.saturated_into(),
    //             reward,
    //             reward_amounts,
    //         ));
    //
    //         assert_ok!(DexGeneral::<T>::bootstrap_contribute(
    //             RawOrigin::Signed(caller.clone()),
    //             ASSET_0,
    //             ASSET_1,
    //             10 * UNIT,
    //             10 * UNIT,
    //             99u128.saturated_into()
    //         ));
    //
    //         run_to_block::<T>(100);
    //
    //         assert_ok!(DexGeneral::<T>::bootstrap_end(
    //             RawOrigin::Signed(caller.clone()),
    //             ASSET_0,
    //             ASSET_1,
    //         ));
    //
    //         #[extrinsic_call]
    //         DexGeneral::bootstrap_claim(
    //             RawOrigin::Signed(caller.clone()),
    //             lookup_of_account::<T>(caller.clone()),
    //             ASSET_0,
    //             ASSET_1,
    //             120u128.saturated_into(),
    //         );
    //     }
    //
    //     #[benchmark]
    //     pub fn bootstrap_end() {
    //         let caller: T::AccountId = whitelisted_caller();
    //
    //         assert_ok!(<T as Config>::MultiCurrency::deposit(
    //             ASSET_0,
    //             &caller,
    //             1000 * UNIT
    //         ));
    //         assert_ok!(<T as Config>::MultiCurrency::deposit(
    //             ASSET_1,
    //             &caller,
    //             1000 * UNIT
    //         ));
    //
    //         let reward: Vec<T::AssetId> = vec![ASSET_0];
    //         let reward_amounts: Vec<(T::AssetId, u128)> = vec![(ASSET_1, 0)];
    //
    //         assert_ok!(DexGeneral::<T>::bootstrap_create(
    //             (RawOrigin::Root),
    //             ASSET_0,
    //             ASSET_1,
    //             1000,
    //             1000,
    //             10 * UNIT,
    //             10 * UNIT,
    //             99u128.saturated_into(),
    //             reward,
    //             reward_amounts,
    //         ));
    //
    //         assert_ok!(DexGeneral::<T>::bootstrap_contribute(
    //             RawOrigin::Signed(caller.clone()),
    //             ASSET_0,
    //             ASSET_1,
    //             10 * UNIT,
    //             10 * UNIT,
    //             99u128.saturated_into()
    //         ));
    //
    //         run_to_block::<T>(100);
    //
    //         #[extrinsic_call]
    //         DexGeneral::bootstrap_end(RawOrigin::Signed(caller.clone()), ASSET_0, ASSET_1);
    //     }
    //
    //     #[benchmark]
    //     pub fn bootstrap_update() {
    //         let caller: T::AccountId = whitelisted_caller();
    //
    //         let reward: Vec<T::AssetId> = vec![ASSET_0];
    //         let reward_amounts: Vec<(T::AssetId, u128)> = vec![(ASSET_1, 0)];
    //
    //         assert_ok!(DexGeneral::<T>::bootstrap_create(
    //             (RawOrigin::Root),
    //             ASSET_0,
    //             ASSET_1,
    //             1000,
    //             1000,
    //             10 * UNIT,
    //             10 * UNIT,
    //             99u128.saturated_into(),
    //             reward.clone(),
    //             reward_amounts.clone(),
    //         ));
    //
    //         #[extrinsic_call]
    //         DexGeneral::bootstrap_update(
    //             RawOrigin::Root,
    //             ASSET_0,
    //             ASSET_1,
    //             1000,
    //             1000,
    //             1000_000_000,
    //             1000_000_000,
    //             100u128.saturated_into(),
    //             reward,
    //             reward_amounts,
    //         );
    //     }
    //
    //     #[benchmark]
    //     pub fn bootstrap_refund() {
    //         let caller: T::AccountId = whitelisted_caller();
    //
    //         assert_ok!(<T as Config>::MultiCurrency::deposit(
    //             ASSET_0,
    //             &caller,
    //             1000 * UNIT
    //         ));
    //         assert_ok!(<T as Config>::MultiCurrency::deposit(
    //             ASSET_1,
    //             &caller,
    //             1000 * UNIT
    //         ));
    //
    //         let reward: Vec<T::AssetId> = vec![ASSET_0];
    //         let reward_amounts: Vec<(T::AssetId, u128)> = vec![(ASSET_1, 0)];
    //
    //         assert_ok!(DexGeneral::<T>::bootstrap_create(
    //             (RawOrigin::Root),
    //             ASSET_0,
    //             ASSET_1,
    //             2 * UNIT,
    //             2 * UNIT,
    //             10 * UNIT,
    //             10 * UNIT,
    //             99u128.saturated_into(),
    //             reward,
    //             reward_amounts,
    //         ));
    //
    //         assert_ok!(DexGeneral::<T>::bootstrap_contribute(
    //             RawOrigin::Signed(caller.clone()),
    //             ASSET_0,
    //             ASSET_1,
    //             1 * UNIT,
    //             1 * UNIT,
    //             99u128.saturated_into()
    //         ));
    //         run_to_block::<T>(100);
    //
    //         #[extrinsic_call]
    //         DexGeneral::bootstrap_refund(RawOrigin::Signed(caller.clone()), ASSET_0, ASSET_1);
    //     }
    //
    //     #[benchmark]
    //     pub fn add_liquidity() {
    //         let caller: T::AccountId = whitelisted_caller();
    //         assert_ok!(<T as Config>::MultiCurrency::deposit(
    //             ASSET_0,
    //             &caller,
    //             1000 * UNIT
    //         ));
    //         assert_ok!(<T as Config>::MultiCurrency::deposit(
    //             ASSET_1,
    //             &caller,
    //             1000 * UNIT
    //         ));
    //
    //         assert_ok!(DexGeneral::<T>::create_pair(
    //             (RawOrigin::Root),
    //             ASSET_0,
    //             ASSET_1,
    //             DEFAULT_FEE_RATE
    //         ));
    //
    //         assert_ok!(DexGeneral::<T>::set_fee_receiver(
    //             (RawOrigin::Root),
    //             lookup_of_account::<T>(caller.clone())
    //         ));
    //
    //         #[extrinsic_call]
    //         DexGeneral::add_liquidity(
    //             RawOrigin::Signed(caller.clone()),
    //             ASSET_0,
    //             ASSET_1,
    //             10 * UNIT,
    //             10 * UNIT,
    //             0,
    //             0,
    //             100u32.saturated_into(),
    //         );
    //     }
    //
    //     #[benchmark]
    //     pub fn remove_liquidity() {
    //         let caller: T::AccountId = whitelisted_caller();
    //         assert_ok!(<T as Config>::MultiCurrency::deposit(
    //             ASSET_0,
    //             &caller,
    //             1000 * UNIT
    //         ));
    //         assert_ok!(<T as Config>::MultiCurrency::deposit(
    //             ASSET_1,
    //             &caller,
    //             1000 * UNIT
    //         ));
    //
    //         assert_ok!(DexGeneral::<T>::create_pair(
    //             (RawOrigin::Root),
    //             ASSET_0,
    //             ASSET_1,
    //             DEFAULT_FEE_RATE
    //         ));
    //
    //         assert_ok!(DexGeneral::<T>::set_fee_receiver(
    //             (RawOrigin::Root),
    //             lookup_of_account::<T>(caller.clone())
    //         ));
    //
    //         assert_ok!(DexGeneral::<T>::add_liquidity(
    //             RawOrigin::Signed(caller.clone()),
    //             ASSET_0,
    //             ASSET_1,
    //             10 * UNIT,
    //             10 * UNIT,
    //             0,
    //             0,
    //             100u32.saturated_into()
    //         ));
    //
    //         #[extrinsic_call]
    //         DexGeneral::remove_liquidity(
    //             RawOrigin::Signed(caller.clone()),
    //             ASSET_0,
    //             ASSET_1,
    //             1 * UNIT,
    //             0,
    //             0,
    //             lookup_of_account::<T>(caller.clone()),
    //             100u32.saturated_into(),
    //         );
    //     }
    //
    //     #[benchmark]
    //     pub fn swap_exact_assets_for_assets() {
    //         let caller: T::AccountId = whitelisted_caller();
    //         assert_ok!(<T as Config>::MultiCurrency::deposit(
    //             ASSET_0,
    //             &caller,
    //             1000 * UNIT
    //         ));
    //         assert_ok!(<T as Config>::MultiCurrency::deposit(
    //             ASSET_1,
    //             &caller,
    //             1000 * UNIT
    //         ));
    //         assert_ok!(<T as Config>::MultiCurrency::deposit(
    //             ASSET_2,
    //             &caller,
    //             1000 * UNIT
    //         ));
    //
    //         assert_ok!(DexGeneral::<T>::create_pair(
    //             (RawOrigin::Root),
    //             ASSET_0,
    //             ASSET_1,
    //             DEFAULT_FEE_RATE
    //         ));
    //         assert_ok!(DexGeneral::<T>::create_pair(
    //             (RawOrigin::Root),
    //             ASSET_1,
    //             ASSET_2,
    //             DEFAULT_FEE_RATE
    //         ));
    //
    //         assert_ok!(DexGeneral::<T>::add_liquidity(
    //             RawOrigin::Signed(caller.clone()),
    //             ASSET_0,
    //             ASSET_1,
    //             10 * UNIT,
    //             10 * UNIT,
    //             0,
    //             0,
    //             100u32.saturated_into()
    //         ));
    //
    //         assert_ok!(DexGeneral::<T>::add_liquidity(
    //             RawOrigin::Signed(caller.clone()),
    //             ASSET_1,
    //             ASSET_2,
    //             10 * UNIT,
    //             10 * UNIT,
    //             0,
    //             0,
    //             100u32.saturated_into()
    //         ));
    //
    //         let path: Vec<T::AssetId> = vec![ASSET_0, ASSET_1, ASSET_2];
    //
    //         #[extrinsic_call]
    //         DexGeneral::swap_exact_assets_for_assets(
    //             RawOrigin::Signed(caller.clone()),
    //             1 * UNIT,
    //             0,
    //             path,
    //             lookup_of_account::<T>(caller.clone()),
    //             100u32.saturated_into(),
    //         );
    //     }
    //
    //     #[benchmark]
    //     pub fn swap_assets_for_exact_assets() {
    //         let caller: T::AccountId = whitelisted_caller();
    //         assert_ok!(<T as Config>::MultiCurrency::deposit(
    //             ASSET_0,
    //             &caller,
    //             1000 * UNIT
    //         ));
    //         assert_ok!(<T as Config>::MultiCurrency::deposit(
    //             ASSET_1,
    //             &caller,
    //             1000 * UNIT
    //         ));
    //         assert_ok!(<T as Config>::MultiCurrency::deposit(
    //             ASSET_2,
    //             &caller,
    //             1000 * UNIT
    //         ));
    //
    //         assert_ok!(DexGeneral::<T>::create_pair(
    //             (RawOrigin::Root),
    //             ASSET_1,
    //             ASSET_2,
    //             DEFAULT_FEE_RATE
    //         ));
    //         assert_ok!(DexGeneral::<T>::create_pair(
    //             (RawOrigin::Root),
    //             ASSET_0,
    //             ASSET_1,
    //             DEFAULT_FEE_RATE
    //         ));
    //
    //         assert_ok!(DexGeneral::<T>::add_liquidity(
    //             RawOrigin::Signed(caller.clone()),
    //             ASSET_1,
    //             ASSET_2,
    //             10 * UNIT,
    //             10 * UNIT,
    //             0,
    //             0,
    //             100u32.saturated_into()
    //         ));
    //
    //         assert_ok!(DexGeneral::<T>::add_liquidity(
    //             RawOrigin::Signed(caller.clone()),
    //             ASSET_0,
    //             ASSET_1,
    //             10 * UNIT,
    //             10 * UNIT,
    //             0,
    //             0,
    //             100u32.saturated_into()
    //         ));
    //
    //         let path: Vec<T::AssetId> = vec![ASSET_0, ASSET_1, ASSET_2];
    //
    //         #[extrinsic_call]
    //         DexGeneral::swap_assets_for_exact_assets(
    //             RawOrigin::Signed(caller.clone()),
    //             1 * UNIT,
    //             10 * UNIT,
    //             path,
    //             lookup_of_account::<T>(caller.clone()),
    //             100u32.saturated_into(),
    //         );
    //     }

    impl_benchmark_test_suite!(
        DexGeneral,
        crate::fee::mock::ExtBuilder::build(),
        crate::fee::mock::Test
    );
}
