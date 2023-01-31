// Copyright 2021-2022 Zenlink.
// Licensed under Apache 2.0.

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::Pallet as ZenlinkPallet;

use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_support::assert_ok;
use frame_system::RawOrigin;

const UNIT: u128 = 1_000_000_000_000;

const ASSET_0: AssetId = AssetId { chain_id: 2001, asset_type: 2, asset_index: 515 };

const ASSET_1: AssetId = AssetId { chain_id: 2001, asset_type: 2, asset_index: 516 };

const ASSET_2: AssetId = AssetId { chain_id: 2001, asset_type: 2, asset_index: 518 };

pub fn lookup_of_account<T: Config>(
	who: T::AccountId,
) -> <<T as frame_system::Config>::Lookup as StaticLookup>::Source {
	<T as frame_system::Config>::Lookup::unlookup(who)
}

fn run_to_block<T: Config>(n: u32) {
	type System<T> = frame_system::Pallet<T>;

	while System::<T>::block_number() < n.saturated_into() {
		System::<T>::on_finalize(System::<T>::block_number());
		System::<T>::set_block_number(System::<T>::block_number() + 1u128.saturated_into());
		System::<T>::on_initialize(System::<T>::block_number());
	}
}

benchmarks! {

	where_clause { where T::AssetId: From<AssetId>}

	set_fee_receiver{
		let caller: T::AccountId = whitelisted_caller();
	}:_(RawOrigin::Root, lookup_of_account::<T>(caller.clone()).into())

	set_fee_point{

	}:_(RawOrigin::Root, 5)

	create_pair {
		let caller: T::AccountId = whitelisted_caller();

		assert_ok!(<T as Config>::MultiAssetsHandler::deposit(ASSET_0.into(), &caller, 1000 * UNIT));
		assert_ok!(<T as Config>::MultiAssetsHandler::deposit(ASSET_1.into(), &caller, 1000 * UNIT));

	} : _(RawOrigin::Root, ASSET_0.into(), ASSET_1.into())

	bootstrap_create {
		let reward:Vec<T::AssetId> =  vec![ASSET_0.into()];
		let reward_amounts: Vec<(T::AssetId, u128)> = vec![(ASSET_1.into(), 0)];
	}: _(RawOrigin::Root, ASSET_0.into(), ASSET_1.into(), 1000, 1000, 1000_000_000, 1000_000_000, 100u128.saturated_into(), reward, reward_amounts)

	bootstrap_contribute{
		let caller: T::AccountId = whitelisted_caller();

		assert_ok!(<T as Config>::MultiAssetsHandler::deposit(ASSET_0.into(), &caller, 1000 * UNIT));
		assert_ok!(<T as Config>::MultiAssetsHandler::deposit(ASSET_1.into(), &caller, 1000 * UNIT));

		let reward:Vec<T::AssetId> =  vec![ASSET_0.into()];
		let reward_amounts: Vec<(T::AssetId, u128)> = vec![(ASSET_1.into(), 0)];
		assert_ok!(ZenlinkPallet::<T>::bootstrap_create(
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

	}: _(RawOrigin::Signed(caller.clone()), ASSET_0.into(), ASSET_1.into(), UNIT, UNIT, 100u128.saturated_into())

	bootstrap_claim{
		let caller: T::AccountId = whitelisted_caller();

		assert_ok!(<T as Config>::MultiAssetsHandler::deposit(ASSET_0.into(), &caller, 1000 * UNIT));
		assert_ok!(<T as Config>::MultiAssetsHandler::deposit(ASSET_1.into(), &caller, 1000 * UNIT));

		let reward:Vec<T::AssetId> =  vec![ASSET_0.into()];
		let reward_amounts: Vec<(T::AssetId, u128)> = vec![(ASSET_1.into(), 0)];

		assert_ok!(ZenlinkPallet::<T>::bootstrap_create(
			(RawOrigin::Root).into(),
			ASSET_0.into(),
			ASSET_1.into(),
			1000,
			1000,
			10*UNIT,
			10*UNIT,
			99u128.saturated_into(),
			reward,
			reward_amounts,
		));

		assert_ok!(ZenlinkPallet::<T>::bootstrap_contribute(
			RawOrigin::Signed(caller.clone()).into(),
			ASSET_0.into(),
			ASSET_1.into(),
			10*UNIT,
			10*UNIT,
			99u128.saturated_into()
		));

		run_to_block::<T>(100);

		assert_ok!(ZenlinkPallet::<T>::bootstrap_end(
			RawOrigin::Signed(caller.clone()).into(),
			ASSET_0.into(),
			ASSET_1.into(),
		));

	}:_(RawOrigin::Signed(caller.clone()), lookup_of_account::<T>(caller.clone()), ASSET_0.into(), ASSET_1.into(), 120u128.saturated_into())

	bootstrap_end{
		let caller: T::AccountId = whitelisted_caller();

		assert_ok!(<T as Config>::MultiAssetsHandler::deposit(ASSET_0.into(), &caller, 1000 * UNIT));
		assert_ok!(<T as Config>::MultiAssetsHandler::deposit(ASSET_1.into(), &caller, 1000 * UNIT));

		let reward:Vec<T::AssetId> =  vec![ASSET_0.into()];
		let reward_amounts: Vec<(T::AssetId, u128)> = vec![(ASSET_1.into(), 0)];

		assert_ok!(ZenlinkPallet::<T>::bootstrap_create(
			(RawOrigin::Root).into(),
			ASSET_0.into(),
			ASSET_1.into(),
			1000,
			1000,
			10*UNIT,
			10*UNIT,
			99u128.saturated_into(),
			reward,
			reward_amounts,
		));

		assert_ok!(ZenlinkPallet::<T>::bootstrap_contribute(
			RawOrigin::Signed(caller.clone()).into(),
			ASSET_0.into(),
			ASSET_1.into(),
			10*UNIT,
			10*UNIT,
			99u128.saturated_into()
		));

		run_to_block::<T>(100);
	}:_(RawOrigin::Signed(caller.clone()), ASSET_0.into(), ASSET_1.into())

	bootstrap_update{
		let caller: T::AccountId = whitelisted_caller();

		let reward:Vec<T::AssetId> =  vec![ASSET_0.into()];
		let reward_amounts: Vec<(T::AssetId, u128)> = vec![(ASSET_1.into(), 0)];

		assert_ok!(ZenlinkPallet::<T>::bootstrap_create(
			(RawOrigin::Root).into(),
			ASSET_0.into(),
			ASSET_1.into(),
			1000,
			1000,
			10*UNIT,
			10*UNIT,
			99u128.saturated_into(),
			reward.clone(),
			reward_amounts.clone(),
		));

	}:_(RawOrigin::Root, ASSET_0.into(), ASSET_1.into(), 1000, 1000, 1000_000_000, 1000_000_000, 100u128.saturated_into(), reward, reward_amounts)

	bootstrap_refund{
		let caller: T::AccountId = whitelisted_caller();

		assert_ok!(<T as Config>::MultiAssetsHandler::deposit(ASSET_0.into(), &caller, 1000 * UNIT));
		assert_ok!(<T as Config>::MultiAssetsHandler::deposit(ASSET_1.into(), &caller, 1000 * UNIT));

		let reward:Vec<T::AssetId> =  vec![ASSET_0.into()];
		let reward_amounts: Vec<(T::AssetId, u128)> = vec![(ASSET_1.into(), 0)];

		assert_ok!(ZenlinkPallet::<T>::bootstrap_create(
			(RawOrigin::Root).into(),
			ASSET_0.into(),
			ASSET_1.into(),
			2*UNIT,
			2*UNIT,
			10*UNIT,
			10*UNIT,
			99u128.saturated_into(),
			reward,
			reward_amounts,
		));

		assert_ok!(ZenlinkPallet::<T>::bootstrap_contribute(
			RawOrigin::Signed(caller.clone()).into(),
			ASSET_0.into(),
			ASSET_1.into(),
			1*UNIT,
			1*UNIT,
			99u128.saturated_into()
		));
		run_to_block::<T>(100);
	}:_(RawOrigin::Signed(caller.clone()), ASSET_0.into(), ASSET_1.into())

	add_liquidity{
		let caller: T::AccountId = whitelisted_caller();
		assert_ok!(<T as Config>::MultiAssetsHandler::deposit(ASSET_0.into(), &caller, 1000 * UNIT));
		assert_ok!(<T as Config>::MultiAssetsHandler::deposit(ASSET_1.into(), &caller, 1000 * UNIT));

		assert_ok!(ZenlinkPallet::<T>::create_pair((RawOrigin::Root).into(), ASSET_0.into(), ASSET_1.into()));

		assert_ok!(ZenlinkPallet::<T>::set_fee_receiver((RawOrigin::Root).into(), lookup_of_account::<T>(caller.clone()).into()));

	}:_(RawOrigin::Signed(caller.clone()), ASSET_0.into(), ASSET_1.into(), 10 * UNIT, 10* UNIT, 0,0, 100u32.saturated_into())

	remove_liquidity{
		let caller: T::AccountId = whitelisted_caller();
		assert_ok!(<T as Config>::MultiAssetsHandler::deposit(ASSET_0.into(), &caller, 1000 * UNIT));
		assert_ok!(<T as Config>::MultiAssetsHandler::deposit(ASSET_1.into(), &caller, 1000 * UNIT));

		assert_ok!(ZenlinkPallet::<T>::create_pair((RawOrigin::Root).into(), ASSET_0.into(), ASSET_1.into()));

		assert_ok!(ZenlinkPallet::<T>::set_fee_receiver((RawOrigin::Root).into(), lookup_of_account::<T>(caller.clone()).into()));

		assert_ok!(ZenlinkPallet::<T>::add_liquidity(
			RawOrigin::Signed(caller.clone()).into(),
			ASSET_0.into(),
			ASSET_1.into(),
			10 * UNIT,
			10* UNIT,
			0,
			0,
			100u32.saturated_into()));

	}:_(RawOrigin::Signed(caller.clone()), ASSET_0.into(), ASSET_1.into(), 1 * UNIT, 0, 0, lookup_of_account::<T>(caller.clone()).into(), 100u32.saturated_into())

	swap_exact_assets_for_assets{
		let caller: T::AccountId = whitelisted_caller();
		assert_ok!(<T as Config>::MultiAssetsHandler::deposit(ASSET_0.into(), &caller, 1000 * UNIT));
		assert_ok!(<T as Config>::MultiAssetsHandler::deposit(ASSET_1.into(), &caller, 1000 * UNIT));
		assert_ok!(<T as Config>::MultiAssetsHandler::deposit(ASSET_2.into(), &caller, 1000 * UNIT));

		assert_ok!(ZenlinkPallet::<T>::create_pair((RawOrigin::Root).into(), ASSET_0.into(), ASSET_1.into()));
		assert_ok!(ZenlinkPallet::<T>::create_pair((RawOrigin::Root).into(), ASSET_1.into(), ASSET_2.into()));

		assert_ok!(ZenlinkPallet::<T>::add_liquidity(
			RawOrigin::Signed(caller.clone()).into(),
			ASSET_0.into(),
			ASSET_1.into(),
			10 * UNIT,
			10* UNIT,
			0,
			0,
			100u32.saturated_into()));

		assert_ok!(ZenlinkPallet::<T>::add_liquidity(
			RawOrigin::Signed(caller.clone()).into(),
			ASSET_1.into(),
			ASSET_2.into(),
			10 * UNIT,
			10* UNIT,
			0,
			0,
			100u32.saturated_into()));

		let path: Vec<T::AssetId> = vec![ASSET_0.into(), ASSET_1.into(), ASSET_2.into()];

	}:_(RawOrigin::Signed(caller.clone()), 1* UNIT, 0,path, lookup_of_account::<T>(caller.clone()).into(), 100u32.saturated_into())

	swap_assets_for_exact_assets{
		let caller: T::AccountId = whitelisted_caller();
		assert_ok!(<T as Config>::MultiAssetsHandler::deposit(ASSET_0.into(), &caller, 1000 * UNIT));
		assert_ok!(<T as Config>::MultiAssetsHandler::deposit(ASSET_1.into(), &caller, 1000 * UNIT));
		assert_ok!(<T as Config>::MultiAssetsHandler::deposit(ASSET_2.into(), &caller, 1000 * UNIT));

		assert_ok!(ZenlinkPallet::<T>::create_pair((RawOrigin::Root).into(), ASSET_1.into(), ASSET_2.into()));
		assert_ok!(ZenlinkPallet::<T>::create_pair((RawOrigin::Root).into(), ASSET_0.into(), ASSET_1.into()));

		assert_ok!(ZenlinkPallet::<T>::add_liquidity(
			RawOrigin::Signed(caller.clone()).into(),
			ASSET_1.into(),
			ASSET_2.into(),
			10 * UNIT,
			10* UNIT,
			0,
			0,
			100u32.saturated_into()));

		assert_ok!(ZenlinkPallet::<T>::add_liquidity(
			RawOrigin::Signed(caller.clone()).into(),
			ASSET_0.into(),
			ASSET_1.into(),
			10 * UNIT,
			10* UNIT,
			0,
			0,
			100u32.saturated_into()));

		let path: Vec<T::AssetId> = vec![ASSET_0.into(), ASSET_1.into(), ASSET_2.into()];
	}:_(RawOrigin::Signed(caller.clone()), 1* UNIT, 10*UNIT,path, lookup_of_account::<T>(caller.clone()).into(), 100u32.saturated_into())
}
