// Copyright 2021-2022 Zenlink.
// Licensed under Apache 2.0.

use super::{mock::*, AssetId, Error, MultiAssetsHandler};
use crate::primitives::PairStatus::Trading;
use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;
use sp_core::U256;
use sp_runtime::{traits::Zero, DispatchError::BadOrigin};

const PAIR_DOT_BTC_ACCOUNT: u128 = 111825939709248857954450132390071529325;

const DOT_ASSET_ID: AssetId = AssetId { chain_id: 200, asset_type: LOCAL, asset_index: 2 };

const KSM_ASSET_ID: AssetId = AssetId { chain_id: 200, asset_type: LOCAL, asset_index: 3 };

const BTC_ASSET_ID: AssetId = AssetId { chain_id: 300, asset_type: RESERVED, asset_index: 3 };

const ETH_ASSET_ID: AssetId = AssetId { chain_id: 300, asset_type: NATIVE, asset_index: 0 };

const DOT_BTC_LP_ID: AssetId = AssetId { chain_id: 0, asset_type: 2, asset_index: 12885034496 };

const PAIR_DOT_BTC: u128 = 111825939709248857954450132390071529325;

const ALICE: u128 = 1;
const BOB: u128 = 2;
const CHARLIE: u128 = 3;
const DOT_UNIT: u128 = 1000_000_000_000_000;
const BTC_UNIT: u128 = 1000_000_00;
const ETH_UNIT: u128 = 1000_000_000_000;

#[test]
fn add_liquidity_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &ALICE, u128::MAX));
		assert_ok!(DexPallet::foreign_mint(BTC_ASSET_ID, &ALICE, u128::MAX));

		assert_ok!(DexPallet::create_pair(RawOrigin::Root.into(), DOT_ASSET_ID, BTC_ASSET_ID,));

		let total_supply_dot: u128 = 1 * DOT_UNIT;
		let total_supply_btc: u128 = 1 * BTC_UNIT;

		assert_ok!(DexPallet::add_liquidity(
			RawOrigin::Signed(ALICE).into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			total_supply_dot,
			total_supply_btc,
			0,
			0,
			100
		));

		let mint_liquidity =
			<Test as Config>::MultiAssetsHandler::balance_of(DOT_BTC_LP_ID, &ALICE);

		assert_eq!(mint_liquidity, 316227766016);
		let total_supply_dot = 50 * DOT_UNIT;
		let total_supply_btc = 50 * BTC_UNIT;

		assert_ok!(DexPallet::add_liquidity(
			RawOrigin::Signed(ALICE).into(),
			BTC_ASSET_ID,
			DOT_ASSET_ID,
			total_supply_btc,
			total_supply_dot,
			0,
			0,
			100
		));

		let balance_dot =
			<Test as Config>::MultiAssetsHandler::balance_of(DOT_ASSET_ID, &PAIR_DOT_BTC);
		let balance_btc =
			<Test as Config>::MultiAssetsHandler::balance_of(BTC_ASSET_ID, &PAIR_DOT_BTC);

		let mint_liquidity =
			<Test as Config>::MultiAssetsHandler::balance_of(DOT_BTC_LP_ID, &ALICE);
		assert_eq!(mint_liquidity, 16127616066816);

		assert_eq!(balance_dot, 51000000000000000);
		assert_eq!(balance_btc, 5100000000);

		assert_eq!((balance_dot / DOT_UNIT), (balance_btc / BTC_UNIT));
	});
}

#[test]
fn remove_liquidity_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &ALICE, u128::MAX));
		assert_ok!(DexPallet::foreign_mint(BTC_ASSET_ID, &ALICE, u128::MAX));

		assert_ok!(DexPallet::create_pair(RawOrigin::Root.into(), DOT_ASSET_ID, BTC_ASSET_ID,));

		let total_supply_dot = 50 * DOT_UNIT;
		let total_supply_btc = 50 * BTC_UNIT;
		assert_ok!(DexPallet::inner_add_liquidity(
			&ALICE,
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			total_supply_dot,
			total_supply_btc,
			0,
			0
		));

		assert_ok!(DexPallet::remove_liquidity(
			RawOrigin::Signed(ALICE).into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			1 * BTC_UNIT,
			0u128,
			0u128,
			BOB,
			100
		));

		let balance_dot = <Test as Config>::MultiAssetsHandler::balance_of(DOT_ASSET_ID, &BOB);
		let balance_btc = <Test as Config>::MultiAssetsHandler::balance_of(BTC_ASSET_ID, &BOB);

		assert_eq!(balance_dot, 316227766016);
		assert_eq!(balance_btc, 31622);

		assert_eq!((balance_dot / balance_btc) / (DOT_UNIT / BTC_UNIT), 1);
	})
}

#[test]
fn foreign_get_in_price_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &ALICE, u128::MAX));
		assert_ok!(DexPallet::foreign_mint(BTC_ASSET_ID, &ALICE, u128::MAX));

		assert_ok!(DexPallet::create_pair(RawOrigin::Root.into(), DOT_ASSET_ID, BTC_ASSET_ID,));

		let total_supply_dot = 10000 * DOT_UNIT;
		let total_supply_btc = 10000 * BTC_UNIT;

		assert_ok!(DexPallet::inner_add_liquidity(
			&ALICE,
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			total_supply_dot,
			total_supply_btc,
			0,
			0
		));
		let path = vec![DOT_ASSET_ID, BTC_ASSET_ID];
		let amount_in = 1 * DOT_UNIT;

		let target_amount = DexPallet::get_amount_out_by_path(amount_in, &path).unwrap();

		assert_eq!(target_amount, vec![1000000000000000, 99690060]);

		assert!(
			*target_amount.last().unwrap() < BTC_UNIT * 997 / 1000 &&
				*target_amount.last().unwrap() > BTC_UNIT * 996 / 1000
		);

		let path = vec![BTC_ASSET_ID, DOT_ASSET_ID];
		let amount_in = 1 * BTC_UNIT;

		let target_amount = DexPallet::get_amount_out_by_path(amount_in, &path).unwrap();

		assert_eq!(target_amount, vec![100000000, 996900609009281]);

		assert!(
			*target_amount.last().unwrap() < DOT_UNIT * 997 / 1000 &&
				*target_amount.last().unwrap() > DOT_UNIT * 996 / 1000
		);
	});
}

#[test]
fn foreign_get_out_price_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &ALICE, u128::MAX));
		assert_ok!(DexPallet::foreign_mint(BTC_ASSET_ID, &ALICE, u128::MAX));

		assert_ok!(DexPallet::create_pair(RawOrigin::Root.into(), DOT_ASSET_ID, BTC_ASSET_ID,));

		let total_supply_dot = 1000000 * DOT_UNIT;
		let total_supply_btc = 1000000 * BTC_UNIT;

		assert_ok!(DexPallet::inner_add_liquidity(
			&ALICE,
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			total_supply_dot,
			total_supply_btc,
			0,
			0
		));
		let path = vec![DOT_ASSET_ID, BTC_ASSET_ID];
		let amount_out = 1 * BTC_UNIT;

		let target_amount = DexPallet::get_amount_in_by_path(amount_out, &path).unwrap();

		// println!("target_amount {:#?}", target_amount);
		assert_eq!(target_amount, vec![1003010030091274, 100000000]);

		assert!(
			*target_amount.first().unwrap() > DOT_UNIT * 1003 / 1000 &&
				*target_amount.first().unwrap() < DOT_UNIT * 1004 / 1000
		);

		let path = vec![BTC_ASSET_ID, DOT_ASSET_ID];
		let amount_out = 1 * DOT_UNIT;
		let target_amount = DexPallet::get_amount_in_by_path(amount_out, &path).unwrap();

		// println!("target_amount {:#?}", target_amount);
		assert_eq!(target_amount, vec![100301004, 1000000000000000]);

		assert!(
			*target_amount.first().unwrap() > BTC_UNIT * 1003 / 1000 &&
				*target_amount.first().unwrap() < BTC_UNIT * 1004 / 1000
		);
	});
}

#[test]
fn inner_swap_exact_assets_for_assets_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &ALICE, u128::MAX));
		assert_ok!(DexPallet::foreign_mint(BTC_ASSET_ID, &ALICE, u128::MAX));

		let total_supply_dot = 50000 * DOT_UNIT;
		let total_supply_btc = 50000 * BTC_UNIT;

		assert_ok!(DexPallet::create_pair(RawOrigin::Root.into(), DOT_ASSET_ID, BTC_ASSET_ID,));

		assert_ok!(DexPallet::inner_add_liquidity(
			&ALICE,
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			total_supply_dot,
			total_supply_btc,
			0,
			0
		));
		let balance_dot =
			<Test as Config>::MultiAssetsHandler::balance_of(DOT_ASSET_ID, &PAIR_DOT_BTC);
		let balance_btc =
			<Test as Config>::MultiAssetsHandler::balance_of(BTC_ASSET_ID, &PAIR_DOT_BTC);

		// println!("balance_dot {} balance_btc {}", balance_dot, balance_btc);
		assert_eq!(balance_dot, 50000000000000000000);
		assert_eq!(balance_btc, 5000000000000);

		let path = vec![DOT_ASSET_ID, BTC_ASSET_ID];
		let amount_in = 1 * DOT_UNIT;
		let amount_out_min = BTC_UNIT * 996 / 1000;
		assert_ok!(DexPallet::inner_swap_exact_assets_for_assets(
			&ALICE,
			amount_in,
			amount_out_min,
			&path,
			&BOB,
		));

		let btc_balance = <Test as Config>::MultiAssetsHandler::balance_of(BTC_ASSET_ID, &BOB);

		// println!("btc_balance {}", btc_balance);
		assert_eq!(btc_balance, 99698012);

		assert!(btc_balance > amount_out_min);

		let path = vec![BTC_ASSET_ID.clone(), DOT_ASSET_ID.clone()];
		let amount_in = 1 * BTC_UNIT;
		let amount_out_min = DOT_UNIT * 996 / 1000;
		assert_ok!(DexPallet::inner_swap_exact_assets_for_assets(
			&ALICE,
			amount_in,
			amount_out_min,
			&path,
			&BOB,
		));
		let dot_balance = <Test as Config>::MultiAssetsHandler::balance_of(DOT_ASSET_ID, &BOB);

		// println!("dot_balance {}", dot_balance);
		assert_eq!(dot_balance, 997019939603584)
	})
}

#[test]
fn inner_swap_exact_assets_for_assets_in_pairs_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &ALICE, u128::MAX));
		assert_ok!(DexPallet::foreign_mint(BTC_ASSET_ID, &ALICE, u128::MAX));
		assert_ok!(DexPallet::foreign_mint(ETH_ASSET_ID, &ALICE, u128::MAX));

		let total_supply_dot = 5000 * DOT_UNIT;
		let total_supply_btc = 5000 * BTC_UNIT;
		let total_supply_eth = 5000 * ETH_UNIT;

		assert_ok!(DexPallet::create_pair(RawOrigin::Root.into(), DOT_ASSET_ID, BTC_ASSET_ID,));

		assert_ok!(DexPallet::inner_add_liquidity(
			&ALICE,
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			total_supply_dot,
			total_supply_btc,
			0,
			0
		));

		assert_ok!(DexPallet::create_pair(RawOrigin::Root.into(), ETH_ASSET_ID, BTC_ASSET_ID,));

		assert_ok!(DexPallet::inner_add_liquidity(
			&ALICE,
			BTC_ASSET_ID,
			ETH_ASSET_ID,
			total_supply_btc,
			total_supply_eth,
			0,
			0
		));

		let path = vec![DOT_ASSET_ID, BTC_ASSET_ID, ETH_ASSET_ID];
		let amount_in = 1 * DOT_UNIT;
		let amount_out_min = 1 * ETH_UNIT * 996 / 1000 * 996 / 1000;
		assert_ok!(DexPallet::inner_swap_exact_assets_for_assets(
			&ALICE,
			amount_in,
			amount_out_min,
			&path,
			&BOB,
		));
		let eth_balance = <Test as Config>::MultiAssetsHandler::balance_of(ETH_ASSET_ID, &BOB);

		// println!("eth_balance {}", eth_balance);
		assert_eq!(eth_balance, 993613333572);

		let path = vec![ETH_ASSET_ID, BTC_ASSET_ID, DOT_ASSET_ID];
		let amount_in = 1 * ETH_UNIT;
		let amount_out_min = 1 * DOT_UNIT * 996 / 1000 * 996 / 1000;
		let dot_balance = <Test as Config>::MultiAssetsHandler::balance_of(DOT_ASSET_ID, &BOB);

		// println!("dot_balance {}", dot_balance);
		assert_eq!(dot_balance, 0);

		assert_ok!(DexPallet::inner_swap_exact_assets_for_assets(
			&ALICE,
			amount_in,
			amount_out_min,
			&path,
			&BOB,
		));
		let dot_balance = <Test as Config>::MultiAssetsHandler::balance_of(DOT_ASSET_ID, &BOB);

		// println!("dot_balance {}", dot_balance);
		assert_eq!(dot_balance, 994405843102918);
	})
}

#[test]
fn inner_swap_assets_for_exact_assets_should_work() {
	new_test_ext().execute_with(|| {
		let total_supply_dot = 10000 * DOT_UNIT;
		let total_supply_btc = 10000 * BTC_UNIT;

		assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &ALICE, total_supply_dot));
		assert_ok!(DexPallet::foreign_mint(BTC_ASSET_ID, &ALICE, total_supply_btc));

		let supply_dot = 5000 * DOT_UNIT;
		let supply_btc = 5000 * BTC_UNIT;

		assert_ok!(DexPallet::create_pair(RawOrigin::Root.into(), DOT_ASSET_ID, BTC_ASSET_ID,));

		assert_ok!(DexPallet::inner_add_liquidity(
			&ALICE,
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			supply_dot,
			supply_btc,
			0,
			0
		));
		let path = vec![DOT_ASSET_ID, BTC_ASSET_ID];
		let amount_out = 1 * BTC_UNIT;
		let amount_in_max = 1 * DOT_UNIT * 1004 / 1000;
		assert_ok!(DexPallet::inner_swap_assets_for_exact_assets(
			&ALICE,
			amount_out,
			amount_in_max,
			&path,
			&BOB
		));
		let btc_balance = <Test as Config>::MultiAssetsHandler::balance_of(BTC_ASSET_ID, &BOB);
		assert_eq!(btc_balance, amount_out);

		let amount_in_dot =
			total_supply_dot -
				supply_dot - <Test as Config>::MultiAssetsHandler::balance_of(DOT_ASSET_ID, &ALICE);

		// println!("amount in {}", amount_in_dot);
		assert_eq!(amount_in_dot, 1003209669015047);

		assert!(amount_in_dot < amount_in_max);

		let path = vec![BTC_ASSET_ID, DOT_ASSET_ID];
		let amount_out = 1 * DOT_UNIT;
		let amount_in_max = 1 * BTC_UNIT * 1004 / 1000;
		assert_ok!(DexPallet::inner_swap_assets_for_exact_assets(
			&ALICE,
			amount_out,
			amount_in_max,
			&path,
			&BOB
		));
		let dot_balance = <Test as Config>::MultiAssetsHandler::balance_of(DOT_ASSET_ID, &BOB);

		// println!("dot_balance {}", dot_balance);
		assert_eq!(dot_balance, 1000000000000000);

		assert_eq!(dot_balance, amount_out);

		let amount_in_btc =
			total_supply_btc -
				supply_btc - <Test as Config>::MultiAssetsHandler::balance_of(BTC_ASSET_ID, &ALICE);

		// println!("amount in {}", amount_in_btc);
		assert_eq!(amount_in_btc, 100280779);

		assert!(amount_in_btc < amount_in_max);
	})
}

#[test]
fn inner_swap_assets_for_exact_assets_in_pairs_should_work() {
	new_test_ext().execute_with(|| {
		let total_supply_dot = 10000 * DOT_UNIT;
		let total_supply_btc = 10000 * BTC_UNIT;
		let total_supply_eth = 10000 * ETH_UNIT;

		assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &ALICE, total_supply_dot));
		assert_ok!(DexPallet::foreign_mint(BTC_ASSET_ID, &ALICE, total_supply_btc));
		assert_ok!(DexPallet::foreign_mint(ETH_ASSET_ID, &ALICE, total_supply_eth));

		assert_ok!(DexPallet::create_pair(RawOrigin::Root.into(), DOT_ASSET_ID, BTC_ASSET_ID,));

		assert_ok!(DexPallet::create_pair(RawOrigin::Root.into(), ETH_ASSET_ID, BTC_ASSET_ID,));

		let supply_dot = 5000 * DOT_UNIT;
		let supply_btc = 5000 * BTC_UNIT;
		let supply_dev = 5000 * ETH_UNIT;

		assert_ok!(DexPallet::inner_add_liquidity(
			&ALICE,
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			supply_dot,
			supply_btc,
			0,
			0
		));

		assert_ok!(DexPallet::inner_add_liquidity(
			&ALICE,
			BTC_ASSET_ID,
			ETH_ASSET_ID,
			supply_btc,
			supply_dev,
			0,
			0
		));

		let path = vec![DOT_ASSET_ID, BTC_ASSET_ID, ETH_ASSET_ID];
		let amount_out = 1 * ETH_UNIT;
		let amount_in_max = 1 * DOT_UNIT * 1004 / 1000 * 1004 / 1000;
		let bob_dev_balance = <Test as Config>::MultiAssetsHandler::balance_of(ETH_ASSET_ID, &BOB);
		assert_ok!(DexPallet::inner_swap_assets_for_exact_assets(
			&ALICE,
			amount_out,
			amount_in_max,
			&path,
			&BOB
		));
		let eth_balance = <Test as Config>::MultiAssetsHandler::balance_of(ETH_ASSET_ID, &BOB);

		// println!("eth_balance {}", eth_balance);
		assert_eq!(eth_balance, 1000000000000);

		assert_eq!(eth_balance - bob_dev_balance, amount_out);

		let path = vec![ETH_ASSET_ID, BTC_ASSET_ID, DOT_ASSET_ID];
		let amount_out = 1 * DOT_UNIT;
		let amount_in_max = 1 * ETH_UNIT * 1004 / 1000 * 1004 / 1000;
		assert_ok!(DexPallet::inner_swap_assets_for_exact_assets(
			&ALICE,
			amount_out,
			amount_in_max,
			&path,
			&BOB
		));
		let dot_balance = <Test as Config>::MultiAssetsHandler::balance_of(DOT_ASSET_ID, &BOB);
		assert_eq!(dot_balance, amount_out);
	})
}

#[test]
fn create_bootstrap_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(<Test as Config>::MultiAssetsHandler::deposit(DOT_ASSET_ID, &ALICE, 0));
		assert_ok!(<Test as Config>::MultiAssetsHandler::deposit(ETH_ASSET_ID, &ALICE, 0));

		assert_ok!(DexPallet::bootstrap_create(
			RawOrigin::Root.into(),
			DOT_ASSET_ID,
			ETH_ASSET_ID,
			1000,
			1000,
			1000,
			1000,
			10000,
			[].to_vec(),
			[].to_vec(),
		));

		assert_noop!(
			DexPallet::create_pair(RawOrigin::Root.into(), ETH_ASSET_ID, DOT_ASSET_ID),
			Error::<Test>::PairAlreadyExists
		);

		assert_noop!(
			DexPallet::bootstrap_create(
				RawOrigin::Root.into(),
				DOT_ASSET_ID,
				ETH_ASSET_ID,
				1000,
				1000,
				1000,
				1000,
				10000,
				[].to_vec(),
				[].to_vec(),
			),
			Error::<Test>::PairAlreadyExists
		);
	})
}

#[test]
fn update_bootstrap_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(<Test as Config>::MultiAssetsHandler::deposit(DOT_ASSET_ID, &ALICE, 0));
		assert_ok!(<Test as Config>::MultiAssetsHandler::deposit(ETH_ASSET_ID, &ALICE, 0));

		assert_ok!(DexPallet::bootstrap_create(
			RawOrigin::Root.into(),
			DOT_ASSET_ID,
			ETH_ASSET_ID,
			1000,
			1000,
			1000,
			1000,
			10000,
			[].to_vec(),
			[].to_vec(),
		));

		assert_ok!(DexPallet::bootstrap_update(
			RawOrigin::Root.into(),
			DOT_ASSET_ID,
			ETH_ASSET_ID,
			10000,
			10000,
			10000,
			10000,
			100000,
			[].to_vec(),
			[].to_vec(),
		));

		assert_noop!(
			DexPallet::bootstrap_update(
				RawOrigin::Signed(BOB).into(),
				DOT_ASSET_ID,
				ETH_ASSET_ID,
				10000,
				10000,
				10000,
				10000,
				100000,
				[].to_vec(),
				[].to_vec(),
			),
			BadOrigin
		);
	})
}

#[test]
fn bootstrap_contribute_should_work() {
	new_test_ext().execute_with(|| {
		let supply_dot = 10000 * DOT_UNIT;
		let supply_btc = 10000 * BTC_UNIT;

		assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &ALICE, supply_dot));
		assert_ok!(DexPallet::foreign_mint(BTC_ASSET_ID, &ALICE, supply_btc));
		assert_ok!(DexPallet::bootstrap_create(
			RawOrigin::Root.into(),
			DOT_ASSET_ID,
			ETH_ASSET_ID,
			20 * DOT_UNIT,
			1 * BTC_UNIT,
			20 * DOT_UNIT,
			1 * BTC_UNIT,
			10000,
			[].to_vec(),
			[].to_vec(),
		));

		assert_ok!(DexPallet::bootstrap_contribute(
			RawOrigin::Signed(ALICE).into(),
			DOT_ASSET_ID,
			ETH_ASSET_ID,
			DOT_UNIT,
			0,
			1000,
		));
		let pair = DexPallet::sort_asset_id(DOT_ASSET_ID, ETH_ASSET_ID);
		assert_eq!(DexPallet::bootstrap_personal_supply((pair, ALICE)), (DOT_UNIT, 0));
	})
}

#[test]
fn bootstrap_contribute_end_should_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let supply_dot = 10000 * DOT_UNIT;
		let supply_btc = 10000 * BTC_UNIT;

		assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &ALICE, supply_dot));
		assert_ok!(DexPallet::foreign_mint(BTC_ASSET_ID, &ALICE, supply_btc));
		assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &BOB, supply_dot));
		assert_ok!(DexPallet::foreign_mint(BTC_ASSET_ID, &BOB, supply_btc));

		assert_ok!(DexPallet::bootstrap_create(
			RawOrigin::Root.into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			20 * DOT_UNIT,
			2 * BTC_UNIT,
			20 * DOT_UNIT,
			2 * BTC_UNIT,
			2,
			[].to_vec(),
			[].to_vec(),
		));

		assert_noop!(
			DexPallet::bootstrap_end(RawOrigin::Signed(ALICE).into(), DOT_ASSET_ID, BTC_ASSET_ID),
			Error::<Test>::UnqualifiedBootstrap
		);

		assert_ok!(DexPallet::bootstrap_contribute(
			RawOrigin::Signed(ALICE).into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			10 * DOT_UNIT,
			1 * BTC_UNIT,
			1000,
		));

		assert_noop!(
			DexPallet::bootstrap_end(RawOrigin::Signed(ALICE).into(), DOT_ASSET_ID, BTC_ASSET_ID),
			Error::<Test>::UnqualifiedBootstrap
		);

		assert_ok!(DexPallet::bootstrap_contribute(
			RawOrigin::Signed(BOB).into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			10 * DOT_UNIT,
			1 * BTC_UNIT,
			1000,
		));

		System::set_block_number(3);
		assert_ok!(DexPallet::bootstrap_end(
			RawOrigin::Signed(ALICE).into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID
		));
	})
}

#[test]
fn bootstrap_contribute_claim_reward_should_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let supply_dot = 10000 * DOT_UNIT;
		let supply_btc = 10000 * BTC_UNIT;

		assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &ALICE, supply_dot));
		assert_ok!(DexPallet::foreign_mint(BTC_ASSET_ID, &ALICE, supply_btc));
		assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &BOB, supply_dot));
		assert_ok!(DexPallet::foreign_mint(BTC_ASSET_ID, &BOB, supply_btc));

		assert_ok!(DexPallet::bootstrap_create(
			RawOrigin::Root.into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			20 * DOT_UNIT,
			2 * BTC_UNIT,
			20 * DOT_UNIT,
			2 * BTC_UNIT,
			2,
			[].to_vec(),
			[].to_vec(),
		));

		assert_ok!(DexPallet::bootstrap_contribute(
			RawOrigin::Signed(ALICE).into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			10 * DOT_UNIT,
			1 * BTC_UNIT,
			1000,
		));

		assert_ok!(DexPallet::bootstrap_contribute(
			RawOrigin::Signed(BOB).into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			10 * DOT_UNIT,
			1 * BTC_UNIT,
			1000,
		));

		System::set_block_number(3);
		assert_ok!(DexPallet::bootstrap_end(
			RawOrigin::Signed(ALICE).into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID
		));

		let total_supply = 2000000000000;

		assert_ok!(match DexPallet::pair_status((DOT_ASSET_ID, BTC_ASSET_ID)) {
			Trading(x) => {
				assert_eq!(x.pair_account, PAIR_DOT_BTC_ACCOUNT);
				assert_eq!(x.total_supply, total_supply);
				Ok(())
			},
			_ => Err(()),
		});

		assert_eq!(<Test as Config>::MultiAssetsHandler::balance_of(DOT_BTC_LP_ID, &ALICE), 0);

		assert_ok!(DexPallet::bootstrap_claim(
			RawOrigin::Signed(ALICE).into(),
			ALICE,
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			1000,
		));
		assert_eq!(
			<Test as Config>::MultiAssetsHandler::balance_of(DOT_BTC_LP_ID, &ALICE),
			total_supply / 2
		);
		assert_eq!(
			<Test as Config>::MultiAssetsHandler::balance_of(DOT_BTC_LP_ID, &PAIR_DOT_BTC_ACCOUNT),
			total_supply / 2
		);

		assert_noop!(
			DexPallet::bootstrap_claim(
				RawOrigin::Signed(ALICE).into(),
				ALICE,
				DOT_ASSET_ID,
				BTC_ASSET_ID,
				1000,
			),
			Error::<Test>::ZeroContribute
		);
		assert_eq!(
			<Test as Config>::MultiAssetsHandler::balance_of(DOT_BTC_LP_ID, &ALICE),
			total_supply / 2
		);

		assert_ok!(DexPallet::bootstrap_claim(
			RawOrigin::Signed(BOB).into(),
			BOB,
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			1000,
		));
		assert_eq!(
			<Test as Config>::MultiAssetsHandler::balance_of(DOT_BTC_LP_ID, &BOB),
			total_supply / 2
		);
		assert_eq!(
			<Test as Config>::MultiAssetsHandler::balance_of(DOT_BTC_LP_ID, &PAIR_DOT_BTC_ACCOUNT),
			0
		);
	})
}

#[test]
fn refund_in_disable_bootstrap_should_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let supply_dot = 10000 * DOT_UNIT;
		let supply_btc = 10000 * BTC_UNIT;

		assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &ALICE, supply_dot));
		assert_ok!(DexPallet::foreign_mint(BTC_ASSET_ID, &ALICE, supply_btc));
		assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &BOB, supply_dot));
		assert_ok!(DexPallet::foreign_mint(BTC_ASSET_ID, &BOB, supply_btc));

		assert_ok!(DexPallet::bootstrap_create(
			RawOrigin::Root.into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			20 * DOT_UNIT,
			2 * BTC_UNIT,
			20 * DOT_UNIT,
			2 * BTC_UNIT,
			2,
			[].to_vec(),
			[].to_vec(),
		));

		assert_ok!(DexPallet::bootstrap_contribute(
			RawOrigin::Signed(ALICE).into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			10 * DOT_UNIT,
			0 * BTC_UNIT,
			1000,
		));

		assert_ok!(DexPallet::bootstrap_contribute(
			RawOrigin::Signed(BOB).into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			10 * DOT_UNIT,
			1 * BTC_UNIT,
			1000,
		));

		assert_noop!(
			DexPallet::bootstrap_claim(
				RawOrigin::Signed(BOB).into(),
				BOB,
				DOT_ASSET_ID,
				BTC_ASSET_ID,
				1000,
			),
			Error::<Test>::NotInBootstrap
		);

		System::set_block_number(3);

		assert_ok!(DexPallet::bootstrap_refund(
			RawOrigin::Signed(BOB).into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
		));
		assert_eq!(
			<Test as Config>::MultiAssetsHandler::balance_of(DOT_ASSET_ID, &BOB),
			supply_dot
		);
		assert_eq!(
			<Test as Config>::MultiAssetsHandler::balance_of(BTC_ASSET_ID, &BOB),
			supply_btc
		);

		assert_noop!(
			DexPallet::bootstrap_refund(RawOrigin::Signed(BOB).into(), DOT_ASSET_ID, BTC_ASSET_ID,),
			Error::<Test>::ZeroContribute
		);

		assert_noop!(
			DexPallet::bootstrap_refund(RawOrigin::Signed(BOB).into(), DOT_ASSET_ID, BTC_ASSET_ID,),
			Error::<Test>::ZeroContribute
		);
	})
}

#[test]
fn disable_bootstrap_removed_after_all_refund_should_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let supply_dot = 10000 * DOT_UNIT;
		let supply_btc = 10000 * BTC_UNIT;

		assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &ALICE, supply_dot));
		assert_ok!(DexPallet::foreign_mint(BTC_ASSET_ID, &ALICE, supply_btc));

		assert_ok!(DexPallet::bootstrap_create(
			RawOrigin::Root.into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			20 * DOT_UNIT,
			2 * BTC_UNIT,
			20 * DOT_UNIT,
			2 * BTC_UNIT,
			2,
			[].to_vec(),
			[].to_vec(),
		));

		assert_ok!(DexPallet::bootstrap_contribute(
			RawOrigin::Signed(ALICE).into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			10 * DOT_UNIT,
			1 * BTC_UNIT,
			1000,
		));

		System::set_block_number(3);

		assert_ok!(DexPallet::bootstrap_refund(
			RawOrigin::Signed(ALICE).into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
		));

		assert_ok!(DexPallet::create_pair(RawOrigin::Root.into(), DOT_ASSET_ID, BTC_ASSET_ID,));
	})
}

#[test]
fn bootstrap_pair_deny_swap_should_work() {
	new_test_ext().execute_with(|| {
		let supply_dot = 10000 * DOT_UNIT;
		let supply_btc = 10000 * BTC_UNIT;

		assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &ALICE, supply_dot));
		assert_ok!(DexPallet::foreign_mint(BTC_ASSET_ID, &ALICE, supply_btc));

		assert_ok!(DexPallet::bootstrap_create(
			RawOrigin::Root.into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			20 * DOT_UNIT,
			2 * BTC_UNIT,
			20 * DOT_UNIT,
			2 * BTC_UNIT,
			1,
			[].to_vec(),
			[].to_vec(),
		));

		assert_ok!(DexPallet::bootstrap_contribute(
			RawOrigin::Signed(ALICE).into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			10 * DOT_UNIT,
			1 * BTC_UNIT,
			1000,
		));

		let path = vec![BTC_ASSET_ID, DOT_ASSET_ID];
		let amount_out = 1 * DOT_UNIT;
		let amount_in_max = 1 * ETH_UNIT * 1004 / 1000 * 1004 / 1000;
		assert_noop!(
			DexPallet::swap_assets_for_exact_assets(
				RawOrigin::Signed(ALICE).into(),
				amount_out,
				amount_in_max,
				path,
				BOB,
				1000,
			),
			Error::<Test>::InvalidPath
		);

		assert_noop!(
			DexPallet::add_liquidity(
				RawOrigin::Signed(ALICE).into(),
				DOT_ASSET_ID,
				BTC_ASSET_ID,
				10 * DOT_UNIT,
				1 * BTC_UNIT,
				0,
				0,
				100,
			),
			Error::<Test>::PairNotExists
		);

		assert_noop!(
			DexPallet::remove_liquidity(
				RawOrigin::Signed(ALICE).into(),
				DOT_ASSET_ID,
				BTC_ASSET_ID,
				1000,
				1 * DOT_UNIT,
				1 * BTC_UNIT,
				BOB,
				100,
			),
			Error::<Test>::PairNotExists
		);
	})
}

#[test]
fn refund_in_success_bootstrap_should_not_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let supply_dot = 10000 * DOT_UNIT;
		let supply_btc = 10000 * BTC_UNIT;

		assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &ALICE, supply_dot));
		assert_ok!(DexPallet::foreign_mint(BTC_ASSET_ID, &ALICE, supply_btc));
		assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &BOB, supply_dot));
		assert_ok!(DexPallet::foreign_mint(BTC_ASSET_ID, &BOB, supply_btc));

		assert_ok!(DexPallet::bootstrap_create(
			RawOrigin::Root.into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			20 * DOT_UNIT,
			2 * BTC_UNIT,
			20 * DOT_UNIT,
			2 * BTC_UNIT,
			2,
			[].to_vec(),
			[].to_vec(),
		));

		assert_ok!(DexPallet::bootstrap_contribute(
			RawOrigin::Signed(ALICE).into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			10 * DOT_UNIT,
			1 * BTC_UNIT,
			1000,
		));

		assert_ok!(DexPallet::bootstrap_contribute(
			RawOrigin::Signed(BOB).into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			10 * DOT_UNIT,
			1 * BTC_UNIT,
			1000,
		));

		assert_noop!(
			DexPallet::bootstrap_claim(
				RawOrigin::Signed(BOB).into(),
				BOB,
				DOT_ASSET_ID,
				BTC_ASSET_ID,
				1000,
			),
			Error::<Test>::NotInBootstrap
		);

		assert_noop!(
			DexPallet::bootstrap_refund(RawOrigin::Signed(BOB).into(), DOT_ASSET_ID, BTC_ASSET_ID,),
			Error::<Test>::DenyRefund
		);

		System::set_block_number(3);

		assert_noop!(
			DexPallet::bootstrap_refund(RawOrigin::Signed(BOB).into(), DOT_ASSET_ID, BTC_ASSET_ID,),
			Error::<Test>::DenyRefund
		);
	})
}

#[test]
fn refund_in_ongoing_bootstrap_should_not_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let supply_dot = 10000 * DOT_UNIT;
		let supply_btc = 10000 * BTC_UNIT;

		assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &ALICE, supply_dot));
		assert_ok!(DexPallet::foreign_mint(BTC_ASSET_ID, &ALICE, supply_btc));
		assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &BOB, supply_dot));
		assert_ok!(DexPallet::foreign_mint(BTC_ASSET_ID, &BOB, supply_btc));

		assert_ok!(DexPallet::bootstrap_create(
			RawOrigin::Root.into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			20 * DOT_UNIT,
			2 * BTC_UNIT,
			20 * DOT_UNIT,
			2 * BTC_UNIT,
			2,
			[].to_vec(),
			[].to_vec(),
		));

		assert_ok!(DexPallet::bootstrap_contribute(
			RawOrigin::Signed(ALICE).into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			10 * DOT_UNIT,
			1 * BTC_UNIT,
			1000,
		));

		assert_noop!(
			DexPallet::bootstrap_refund(RawOrigin::Signed(BOB).into(), DOT_ASSET_ID, BTC_ASSET_ID,),
			Error::<Test>::DenyRefund
		);
	})
}

#[test]
fn create_pair_in_disable_bootstrap_should_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let supply_dot = 10000 * DOT_UNIT;
		let supply_btc = 10000 * BTC_UNIT;

		assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &ALICE, 1 * DOT_UNIT));
		assert_ok!(DexPallet::foreign_mint(BTC_ASSET_ID, &ALICE, 1 * BTC_UNIT));
		assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &BOB, supply_dot));
		assert_ok!(DexPallet::foreign_mint(BTC_ASSET_ID, &BOB, supply_btc));

		assert_ok!(DexPallet::bootstrap_create(
			RawOrigin::Root.into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			20 * DOT_UNIT,
			2 * BTC_UNIT,
			20 * DOT_UNIT,
			2 * BTC_UNIT,
			2,
			[].to_vec(),
			[].to_vec(),
		));

		assert_ok!(DexPallet::bootstrap_contribute(
			RawOrigin::Signed(BOB).into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			10 * DOT_UNIT,
			0 * BTC_UNIT,
			1000,
		));

		System::set_block_number(3);

		assert_ok!(DexPallet::create_pair(RawOrigin::Root.into(), DOT_ASSET_ID, BTC_ASSET_ID));
		assert_ok!(DexPallet::add_liquidity(
			RawOrigin::Signed(ALICE).into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			1 * DOT_UNIT,
			1 * BTC_UNIT,
			0,
			0,
			100
		));

		assert_ok!(DexPallet::bootstrap_refund(
			RawOrigin::Signed(BOB).into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
		));
		assert_eq!(
			<Test as Config>::MultiAssetsHandler::balance_of(DOT_ASSET_ID, &BOB),
			supply_dot
		);
		assert_eq!(
			<Test as Config>::MultiAssetsHandler::balance_of(BTC_ASSET_ID, &BOB),
			supply_btc
		);

		assert_noop!(
			DexPallet::bootstrap_refund(RawOrigin::Signed(BOB).into(), DOT_ASSET_ID, BTC_ASSET_ID,),
			Error::<Test>::ZeroContribute
		);

		assert_noop!(
			DexPallet::bootstrap_refund(RawOrigin::Signed(BOB).into(), DOT_ASSET_ID, BTC_ASSET_ID,),
			Error::<Test>::ZeroContribute
		);

		let mint_liquidity =
			<Test as Config>::MultiAssetsHandler::balance_of(DOT_BTC_LP_ID, &ALICE);
		assert_eq!(mint_liquidity, 316227766016);
	})
}

#[test]
fn create_bootstrap_in_disable_bootstrap() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let supply_dot = 10000 * DOT_UNIT;
		let supply_btc = 10000 * BTC_UNIT;

		assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &ALICE, 1 * DOT_UNIT));
		assert_ok!(DexPallet::foreign_mint(BTC_ASSET_ID, &ALICE, 1 * BTC_UNIT));
		assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &BOB, supply_dot));
		assert_ok!(DexPallet::foreign_mint(BTC_ASSET_ID, &BOB, supply_btc));

		assert_ok!(DexPallet::bootstrap_create(
			RawOrigin::Root.into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			20 * DOT_UNIT,
			2 * BTC_UNIT,
			20 * DOT_UNIT,
			2 * BTC_UNIT,
			2,
			[].to_vec(),
			[].to_vec(),
		));

		assert_ok!(DexPallet::bootstrap_contribute(
			RawOrigin::Signed(BOB).into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			10 * DOT_UNIT,
			0 * BTC_UNIT,
			1000,
		));

		System::set_block_number(3);
		assert_ok!(DexPallet::bootstrap_create(
			RawOrigin::Root.into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			20 * DOT_UNIT,
			2 * BTC_UNIT,
			20 * DOT_UNIT,
			2 * BTC_UNIT,
			4,
			[].to_vec(),
			[].to_vec(),
		));

		assert_noop!(
			DexPallet::bootstrap_refund(RawOrigin::Signed(BOB).into(), DOT_ASSET_ID, BTC_ASSET_ID,),
			Error::<Test>::DenyRefund
		);

		assert_noop!(
			DexPallet::bootstrap_end(RawOrigin::Signed(ALICE).into(), DOT_ASSET_ID, BTC_ASSET_ID),
			Error::<Test>::UnqualifiedBootstrap
		);

		assert_ok!(DexPallet::bootstrap_contribute(
			RawOrigin::Signed(BOB).into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			10 * DOT_UNIT,
			2 * BTC_UNIT,
			1000,
		));

		System::set_block_number(5);
		assert_ok!(DexPallet::bootstrap_end(
			RawOrigin::Signed(BOB).into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID
		));
		assert_ok!(DexPallet::bootstrap_claim(
			RawOrigin::Signed(BOB).into(),
			BOB,
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			1000,
		));
		assert_eq!(
			<Test as Config>::MultiAssetsHandler::balance_of(DOT_BTC_LP_ID, &BOB),
			2000000000000
		);
	})
}

#[test]
fn create_pair_in_ongoing_bootstrap_should_not_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let supply_dot = 10000 * DOT_UNIT;
		let supply_btc = 10000 * BTC_UNIT;

		assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &ALICE, 1 * DOT_UNIT));
		assert_ok!(DexPallet::foreign_mint(BTC_ASSET_ID, &ALICE, 1 * BTC_UNIT));
		assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &BOB, supply_dot));
		assert_ok!(DexPallet::foreign_mint(BTC_ASSET_ID, &BOB, supply_btc));

		assert_ok!(DexPallet::bootstrap_create(
			RawOrigin::Root.into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			20 * DOT_UNIT,
			2 * BTC_UNIT,
			20 * DOT_UNIT,
			2 * BTC_UNIT,
			2,
			[].to_vec(),
			[].to_vec(),
		));
		assert_noop!(
			DexPallet::create_pair(RawOrigin::Root.into(), DOT_ASSET_ID, BTC_ASSET_ID,),
			Error::<Test>::PairAlreadyExists
		);
	})
}

#[test]
fn liquidity_at_boundary_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &ALICE, u128::MAX));
		assert_ok!(DexPallet::foreign_mint(BTC_ASSET_ID, &ALICE, u128::MAX));

		assert_ok!(DexPallet::create_pair(RawOrigin::Root.into(), DOT_ASSET_ID, BTC_ASSET_ID,));

		assert_ok!(DexPallet::add_liquidity(
			RawOrigin::Signed(ALICE).into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			u128::MAX,
			u128::MAX,
			0,
			0,
			100
		));
		let mint_liquidity =
			<Test as Config>::MultiAssetsHandler::balance_of(DOT_BTC_LP_ID, &ALICE);
		assert_eq!(<Test as Config>::MultiAssetsHandler::balance_of(DOT_ASSET_ID, &ALICE), 0);
		assert_eq!(<Test as Config>::MultiAssetsHandler::balance_of(BTC_ASSET_ID, &ALICE), 0);

		assert_eq!(mint_liquidity, u128::MAX);

		assert_ok!(DexPallet::remove_liquidity(
			RawOrigin::Signed(ALICE).into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			u128::MAX,
			0,
			0,
			ALICE,
			100,
		));

		assert_eq!(
			<Test as Config>::MultiAssetsHandler::balance_of(DOT_ASSET_ID, &ALICE),
			u128::MAX
		);
		assert_eq!(
			<Test as Config>::MultiAssetsHandler::balance_of(BTC_ASSET_ID, &ALICE),
			u128::MAX
		);

		assert_ok!(DexPallet::add_liquidity(
			RawOrigin::Signed(ALICE).into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			u128::MAX,
			u128::MAX,
			0,
			0,
			100
		));

		assert_ok!(DexPallet::remove_liquidity(
			RawOrigin::Signed(ALICE).into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			u128::MAX / 2,
			0,
			0,
			ALICE,
			100,
		));

		assert_eq!(
			<Test as Config>::MultiAssetsHandler::balance_of(DOT_ASSET_ID, &ALICE),
			u128::MAX / 2
		);
		assert_eq!(
			<Test as Config>::MultiAssetsHandler::balance_of(BTC_ASSET_ID, &ALICE),
			u128::MAX / 2
		);
	})
}

#[test]
fn multi_bootstrap_contribute_claim_should_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &ALICE, u128::MAX / 4 - 1));
		assert_ok!(DexPallet::foreign_mint(BTC_ASSET_ID, &ALICE, u128::MAX / 4 - 1));
		assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &BOB, u128::MAX / 4 - 1));
		assert_ok!(DexPallet::foreign_mint(BTC_ASSET_ID, &BOB, u128::MAX / 4 - 1));
		assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &CHARLIE, u128::MAX / 4 - 1));
		assert_ok!(DexPallet::foreign_mint(BTC_ASSET_ID, &CHARLIE, u128::MAX / 4 - 1));

		let unit = 1_000_000_000_000_000_000u128;
		assert_ok!(DexPallet::bootstrap_create(
			RawOrigin::Root.into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			200_000_000 * unit,
			300_000_000 * unit,
			400_000_000 * unit,
			600_000_000 * unit,
			2,
			[].to_vec(),
			[].to_vec(),
		));

		assert_ok!(DexPallet::bootstrap_contribute(
			RawOrigin::Signed(ALICE).into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			200_000_000 * unit,
			0,
			1000,
		));

		assert_ok!(DexPallet::bootstrap_contribute(
			RawOrigin::Signed(BOB).into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			0,
			200_000_000 * unit,
			1000,
		));

		assert_ok!(DexPallet::bootstrap_contribute(
			RawOrigin::Signed(CHARLIE).into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			100_000_000 * unit,
			100_000_000 * unit,
			1000,
		));

		System::set_block_number(3);
		assert_ok!(DexPallet::bootstrap_end(
			RawOrigin::Signed(ALICE).into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID
		));

		let total_lp = <Test as Config>::MultiAssetsHandler::total_supply(DOT_BTC_LP_ID);
		assert_eq!(
			U256::from(300_000_000 * unit)
				.saturating_mul(U256::from(300_000_000 * unit))
				.integer_sqrt(),
			U256::from(total_lp)
		);

		assert_ok!(DexPallet::bootstrap_claim(
			RawOrigin::Signed(ALICE).into(),
			ALICE,
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			1000,
		));

		assert_ok!(DexPallet::bootstrap_claim(
			RawOrigin::Signed(BOB).into(),
			BOB,
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			1000,
		));

		assert_ok!(DexPallet::bootstrap_claim(
			RawOrigin::Signed(CHARLIE).into(),
			CHARLIE,
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			1000,
		));

		// exact_amount_0 = (amount_0_contribute * accumulated_supply_1 + amount_1_contribute
		// *accumulated_supply_0) / (accumulated_supply_1 *2) exact_amount_1 = (amount_1_contribute
		// * accumulated_supply_0 + amount_0_contribute *accumulated_supply_1) /
		// (accumulated_supply_0 *2) lp = sqrt(exact_amount_0 * exact_amount_1)

		// (200000000 * 10^18 * 300000000 * 10^18 + 0) / (300000000 * 10^18 *2) =
		// 100000000000000000000000000 (200000000 * 10^18 * 300000000 * 10^18 + 0) / (300000000 *
		// 10^18 *2) = 100000000000000000000000000 alice_lp = 100000000000000000000000000

		let alice_lp = <Test as Config>::MultiAssetsHandler::balance_of(DOT_BTC_LP_ID, &ALICE);
		assert_eq!(alice_lp, 100000000000000000000000000);

		let bob_lp = <Test as Config>::MultiAssetsHandler::balance_of(DOT_BTC_LP_ID, &BOB);
		assert_eq!(bob_lp, 100000000000000000000000000);

		let charlie_lp = <Test as Config>::MultiAssetsHandler::balance_of(DOT_BTC_LP_ID, &CHARLIE);
		assert_eq!(charlie_lp, 100000000000000000000000000);
	})
}

#[test]
fn bootstrap_set_limit_and_reward_should_work() {
	new_test_ext().execute_with(|| {
		let unit = 1_000_000_000_000_000_000u128;

		assert_ok!(DexPallet::bootstrap_create(
			RawOrigin::Root.into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			200_000_000 * unit,
			300_000_000 * unit,
			400_000_000 * unit,
			600_000_000 * unit,
			2,
			[ETH_ASSET_ID, KSM_ASSET_ID].to_vec(),
			[(DOT_ASSET_ID, 2000 * unit), (BTC_ASSET_ID, 1000 * unit)].to_vec(),
		));

		let reward = DexPallet::get_bootstrap_rewards((DOT_ASSET_ID, BTC_ASSET_ID));
		assert_eq!(reward.get(&ETH_ASSET_ID), Some(&Zero::zero()));
		assert_eq!(reward.get(&KSM_ASSET_ID), Some(&Zero::zero()));
		assert_eq!(reward.get(&DOT_ASSET_ID), None);

		let limits = DexPallet::get_bootstrap_limits((DOT_ASSET_ID, BTC_ASSET_ID));
		assert_eq!(*limits.get(&DOT_ASSET_ID).unwrap_or(&Zero::zero()), 2000 * unit);
		assert_eq!(*limits.get(&BTC_ASSET_ID).unwrap_or(&Zero::zero()), 1000 * unit);

		assert_ok!(DexPallet::bootstrap_update(
			RawOrigin::Root.into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			200_000_000 * unit,
			300_000_000 * unit,
			400_000_000 * unit,
			600_000_000 * unit,
			2,
			[ETH_ASSET_ID].to_vec(),
			[(DOT_ASSET_ID, 3000 * unit)].to_vec(),
		));

		let reward = DexPallet::get_bootstrap_rewards((DOT_ASSET_ID, BTC_ASSET_ID));
		assert_eq!(reward.get(&ETH_ASSET_ID), Some(&Zero::zero()));
		assert_eq!(reward.get(&KSM_ASSET_ID), None);
		assert_eq!(reward.get(&DOT_ASSET_ID), None);

		let limits = DexPallet::get_bootstrap_limits((DOT_ASSET_ID, BTC_ASSET_ID));
		assert_eq!(*limits.get(&DOT_ASSET_ID).unwrap_or(&Zero::zero()), 3000 * unit);
		assert_eq!(*limits.get(&BTC_ASSET_ID).unwrap_or(&Zero::zero()), Zero::zero());
	})
}

#[test]
fn bootstrap_charge_reward_should_work() {
	new_test_ext().execute_with(|| {
		let unit = 1_000_000_000_000_000_000u128;

		assert_ok!(DexPallet::bootstrap_create(
			RawOrigin::Root.into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			1_000_000 * unit,
			1_000_000 * unit,
			2_000_000 * unit,
			2_000_000 * unit,
			2,
			[ETH_ASSET_ID, KSM_ASSET_ID].to_vec(),
			[(DOT_ASSET_ID, 2000 * unit), (BTC_ASSET_ID, 1000 * unit)].to_vec(),
		));

		assert_ok!(DexPallet::foreign_mint(KSM_ASSET_ID, &BOB, 1000 * unit));
		assert_ok!(DexPallet::foreign_mint(ETH_ASSET_ID, &BOB, 2000 * unit));

		assert_ok!(DexPallet::bootstrap_charge_reward(
			RawOrigin::Signed(BOB).into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			[(ETH_ASSET_ID, 2000 * unit), (KSM_ASSET_ID, 1000 * unit)].to_vec(),
		));

		assert_eq!(
			<Test as Config>::MultiAssetsHandler::balance_of(
				ETH_ASSET_ID,
				&DexPallet::account_id()
			),
			2000 * unit
		);
		assert_eq!(
			<Test as Config>::MultiAssetsHandler::balance_of(
				KSM_ASSET_ID,
				&DexPallet::account_id()
			),
			1000 * unit
		);

		assert_eq!(<Test as Config>::MultiAssetsHandler::balance_of(ETH_ASSET_ID, &BOB), 0);
		assert_eq!(<Test as Config>::MultiAssetsHandler::balance_of(KSM_ASSET_ID, &BOB), 0);
	})
}

#[test]
fn bootstrap_withdraw_reward_after_charge_should_work() {
	new_test_ext().execute_with(|| {
		let unit = 1_000_000_000_000_000_000u128;

		assert_ok!(DexPallet::bootstrap_create(
			RawOrigin::Root.into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			2_000_000 * unit,
			3_000_000 * unit,
			4_000_000 * unit,
			9_000_000 * unit,
			2,
			[ETH_ASSET_ID, KSM_ASSET_ID].to_vec(),
			[(DOT_ASSET_ID, 2000 * unit), (BTC_ASSET_ID, 1000 * unit)].to_vec(),
		));

		assert_ok!(DexPallet::foreign_mint(KSM_ASSET_ID, &BOB, 1000 * unit));
		assert_ok!(DexPallet::foreign_mint(ETH_ASSET_ID, &BOB, 2000 * unit));

		assert_ok!(DexPallet::bootstrap_charge_reward(
			RawOrigin::Signed(BOB).into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			[(ETH_ASSET_ID, 2000 * unit), (KSM_ASSET_ID, 1000 * unit)].to_vec(),
		));

		assert_ok!(DexPallet::bootstrap_withdraw_reward(
			RawOrigin::Root.into(),
			BTC_ASSET_ID,
			DOT_ASSET_ID,
			ALICE,
		));

		assert_eq!(
			<Test as Config>::MultiAssetsHandler::balance_of(
				ETH_ASSET_ID,
				&DexPallet::account_id()
			),
			0
		);
		assert_eq!(
			<Test as Config>::MultiAssetsHandler::balance_of(
				KSM_ASSET_ID,
				&DexPallet::account_id()
			),
			0
		);

		assert_eq!(
			<Test as Config>::MultiAssetsHandler::balance_of(ETH_ASSET_ID, &ALICE),
			2000 * unit
		);
		assert_eq!(
			<Test as Config>::MultiAssetsHandler::balance_of(KSM_ASSET_ID, &ALICE),
			1000 * unit
		);

		let reward = DexPallet::get_bootstrap_rewards((DOT_ASSET_ID, BTC_ASSET_ID));
		assert_eq!(reward.get(&ETH_ASSET_ID), Some(&Zero::zero()));
		assert_eq!(reward.get(&KSM_ASSET_ID), Some(&Zero::zero()));
		assert_eq!(reward.get(&DOT_ASSET_ID), None);
	})
}

#[test]
fn bootstrap_charge_reward_with_insufficient_account_should_not_work() {
	new_test_ext().execute_with(|| {
		let unit = 1_000_000_000_000_000_000u128;

		assert_ok!(DexPallet::bootstrap_create(
			RawOrigin::Root.into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			2_000_000 * unit,
			3_000_000 * unit,
			4_000_000 * unit,
			6_000_000 * unit,
			2,
			[ETH_ASSET_ID, KSM_ASSET_ID].to_vec(),
			[(DOT_ASSET_ID, 2000 * unit), (BTC_ASSET_ID, 1000 * unit)].to_vec(),
		));

		assert_ok!(DexPallet::foreign_mint(KSM_ASSET_ID, &BOB, 1000 * unit));
		assert_ok!(DexPallet::foreign_mint(ETH_ASSET_ID, &BOB, 2000 * unit));

		assert_noop!(
			DexPallet::bootstrap_charge_reward(
				RawOrigin::Signed(BOB).into(),
				DOT_ASSET_ID,
				BTC_ASSET_ID,
				[(ETH_ASSET_ID, 2000 * unit)].to_vec(),
			),
			Error::<Test>::ChargeRewardParamsError
		);

		assert_eq!(
			<Test as Config>::MultiAssetsHandler::balance_of(
				ETH_ASSET_ID,
				&DexPallet::account_id()
			),
			0
		);
		assert_eq!(
			<Test as Config>::MultiAssetsHandler::balance_of(
				KSM_ASSET_ID,
				&DexPallet::account_id()
			),
			0
		);

		assert_eq!(
			<Test as Config>::MultiAssetsHandler::balance_of(ETH_ASSET_ID, &BOB),
			2000 * unit
		);
		assert_eq!(
			<Test as Config>::MultiAssetsHandler::balance_of(KSM_ASSET_ID, &BOB),
			1000 * unit
		);
	})
}

#[test]
fn bootstrap_contribute_below_limits_should_not_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let unit = 1_000_000_000_000_000_000u128;

		assert_ok!(DexPallet::bootstrap_create(
			RawOrigin::Root.into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			1_000_000 * unit,
			2_000_000 * unit,
			2_000_000 * unit,
			3_000_000 * unit,
			2,
			[ETH_ASSET_ID, KSM_ASSET_ID].to_vec(),
			[(DOT_ASSET_ID, 2000 * unit), (BTC_ASSET_ID, 1000 * unit)].to_vec(),
		));

		// charlie's asset  below limit,
		assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &CHARLIE, 100 * unit));
		assert_ok!(DexPallet::foreign_mint(BTC_ASSET_ID, &CHARLIE, 2000 * unit));

		assert_noop!(
			DexPallet::bootstrap_contribute(
				RawOrigin::Signed(CHARLIE).into(),
				DOT_ASSET_ID,
				BTC_ASSET_ID,
				100 * unit,
				2000 * unit,
				1000,
			),
			Error::<Test>::NotQualifiedAccount
		);

		assert_eq!(
			<Test as Config>::MultiAssetsHandler::balance_of(
				DOT_ASSET_ID,
				&DexPallet::account_id()
			),
			0
		);
		assert_eq!(
			<Test as Config>::MultiAssetsHandler::balance_of(
				BTC_ASSET_ID,
				&DexPallet::account_id()
			),
			0
		);

		assert_eq!(
			<Test as Config>::MultiAssetsHandler::balance_of(DOT_ASSET_ID, &CHARLIE),
			100 * unit
		);
		assert_eq!(
			<Test as Config>::MultiAssetsHandler::balance_of(BTC_ASSET_ID, &CHARLIE),
			2000 * unit
		);
	})
}

#[test]
fn bootstrap_contribute_exceed_limits_should_work() {
	new_test_ext().execute_with(|| {
		let unit = 1_000_000_000_000_000_000u128;

		assert_ok!(DexPallet::bootstrap_create(
			RawOrigin::Root.into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			2_000_000 * unit,
			4_000_000 * unit,
			4_000_000 * unit,
			8_000_000 * unit,
			2,
			[ETH_ASSET_ID, KSM_ASSET_ID].to_vec(),
			[(DOT_ASSET_ID, 2000 * unit), (BTC_ASSET_ID, 1000 * unit)].to_vec(),
		));

		// alice mint asset
		assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &ALICE, 2_000_000 * unit));
		assert_ok!(DexPallet::foreign_mint(BTC_ASSET_ID, &ALICE, 4_000_000 * unit));

		assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &CHARLIE, 2_000_000 * unit));
		assert_ok!(DexPallet::foreign_mint(BTC_ASSET_ID, &CHARLIE, 4_000_000 * unit));

		// alice will charge
		assert_ok!(DexPallet::foreign_mint(ETH_ASSET_ID, &ALICE, 20_000 * unit));
		assert_ok!(DexPallet::foreign_mint(KSM_ASSET_ID, &ALICE, 10_000 * unit));

		assert_ok!(DexPallet::bootstrap_charge_reward(
			RawOrigin::Signed(ALICE).into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			[(ETH_ASSET_ID, 20_000 * unit), (KSM_ASSET_ID, 10_000 * unit)].to_vec(),
		));

		// bob's asset == limit
		assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &BOB, 2_000 * unit));
		assert_ok!(DexPallet::foreign_mint(BTC_ASSET_ID, &BOB, 1_000 * unit));

		assert_ok!(DexPallet::bootstrap_contribute(
			RawOrigin::Signed(ALICE).into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			2_000_000 * unit,
			4_000_000 * unit,
			1000,
		));

		assert_ok!(DexPallet::bootstrap_contribute(
			RawOrigin::Signed(BOB).into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			2_000 * unit,
			1_000 * unit,
			1000,
		));

		System::set_block_number(3);

		assert_ok!(DexPallet::bootstrap_end(
			RawOrigin::Signed(ALICE).into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID
		));

		assert_ok!(DexPallet::add_liquidity(
			RawOrigin::Signed(CHARLIE).into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			1 * unit,
			1 * unit,
			0,
			0,
			100
		));

		assert_ok!(DexPallet::bootstrap_claim(
			RawOrigin::Signed(ALICE).into(),
			ALICE,
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			1000,
		));

		let path = vec![BTC_ASSET_ID.clone(), DOT_ASSET_ID.clone()];
		let amount_in = 1 * unit;
		assert_ok!(DexPallet::inner_swap_exact_assets_for_assets(
			&CHARLIE, amount_in, 0, &path, &BOB,
		));

		assert_ok!(DexPallet::bootstrap_claim(
			RawOrigin::Signed(BOB).into(),
			BOB,
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			1000,
		));

		// alice_lp =  sqrt((2000000000000000000000000 * 4001000000000000000000000 +
		// 4000000000000000000000000 * 2002000000000000000000000) / (4001000000000000000000000 *2)
		// * (2000000000000000000000000 * 4001000000000000000000000 + 4000000000000000000000000 *
		//   2002000000000000000000000) / (2002000000000000000000000 *2))
		// = 2_828_427_323_371_633_862_327_510
		let alice_lp = <Test as Config>::MultiAssetsHandler::balance_of(DOT_BTC_LP_ID, &ALICE);
		assert_eq!(alice_lp, 2_828_427_323_371_633_862_327_509u128);

		// bob_lp = sqrt((2000000000000000000000 * 4001000000000000000000000 +
		// 1000000000000000000000 * 2002000000000000000000000) / (4001000000000000000000000 *2)
		// * (2000000000000000000000 * 4001000000000000000000000 + 1000000000000000000000 *
		//   2002000000000000000000000) / (2002000000000000000000000 *2))
		// = 1767_369_577_951_894_138_583
		let bob_lp = <Test as Config>::MultiAssetsHandler::balance_of(DOT_BTC_LP_ID, &BOB);
		assert_eq!(bob_lp, 1767_369_577_951_894_138_582u128);

		// bootstrap_mint_lp = 2828427323371633862327510 + 1767369577951894138583 =
		// sqrt(2002000000000000000000000 × 4001000000000000000000000)

		//bob_reward_eth = 1767369577951894138582 * 20000000000000000000000 /
		// (2828427323371633862327510 + 1767369577951894138583) = 12_489_385_146_220_937_273
		// bob_reward_ksm = 1767369577951894138582 * 10000000000000000000000 /
		// (2828427323371633862327510 + 1767369577951894138583) = 6_244_692_573_110_468_636
		assert_eq!(
			<Test as Config>::MultiAssetsHandler::balance_of(ETH_ASSET_ID, &BOB),
			12_489_385_146_220_937_273
		);
		assert_eq!(
			<Test as Config>::MultiAssetsHandler::balance_of(KSM_ASSET_ID, &BOB),
			6_244_692_573_110_468_636
		);

		//alice_reward_eth = 2828427323371633862327510 * 20000000000000000000000 /
		// (2828427323371633862327510 + 1767369577951894138583) = 19_987_510_614_853_779_062_726
		// alice_reward_ksm = 2828427323371633862327510 * 10000000000000000000000 /
		// (2828427323371633862327510 + 1767369577951894138583) = 9_993_755_307_426_889_531_363
		assert_eq!(
			<Test as Config>::MultiAssetsHandler::balance_of(ETH_ASSET_ID, &ALICE),
			19_987_510_614_853_779_062_726
		);
		assert_eq!(
			<Test as Config>::MultiAssetsHandler::balance_of(KSM_ASSET_ID, &ALICE),
			9_993_755_307_426_889_531_363
		);
	})
}

#[test]
fn bootstrap_zero_reward_claim_should_work() {
	new_test_ext().execute_with(|| {
		let unit = 1_000_000_000_000_000_000u128;

		assert_ok!(DexPallet::bootstrap_create(
			RawOrigin::Root.into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			2_000_000 * unit,
			4_000_000 * unit,
			6_000_000 * unit,
			12_000_000 * unit,
			2,
			[ETH_ASSET_ID, KSM_ASSET_ID].to_vec(),
			[(DOT_ASSET_ID, 2000 * unit), (BTC_ASSET_ID, 1000 * unit)].to_vec(),
		));

		assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &ALICE, 2_000_000 * unit));
		assert_ok!(DexPallet::foreign_mint(BTC_ASSET_ID, &ALICE, 4_000_000 * unit));

		assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &BOB, 2_000 * unit));
		assert_ok!(DexPallet::foreign_mint(BTC_ASSET_ID, &BOB, 1_000 * unit));

		assert_ok!(DexPallet::bootstrap_contribute(
			RawOrigin::Signed(ALICE).into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			2_000_000 * unit,
			4_000_000 * unit,
			1000,
		));

		assert_ok!(DexPallet::bootstrap_contribute(
			RawOrigin::Signed(BOB).into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			2_000 * unit,
			1_000 * unit,
			1000,
		));

		System::set_block_number(3);

		assert_ok!(DexPallet::bootstrap_end(
			RawOrigin::Signed(ALICE).into(),
			DOT_ASSET_ID,
			BTC_ASSET_ID
		));

		assert_ok!(DexPallet::bootstrap_claim(
			RawOrigin::Signed(ALICE).into(),
			ALICE,
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			1000,
		));

		assert_ok!(DexPallet::bootstrap_claim(
			RawOrigin::Signed(BOB).into(),
			BOB,
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			1000,
		));

		assert_eq!(
			<Test as Config>::MultiAssetsHandler::balance_of(DOT_BTC_LP_ID, &ALICE),
			2_828_427_323_371_633_862_327_509
		);

		assert_eq!(
			<Test as Config>::MultiAssetsHandler::balance_of(DOT_BTC_LP_ID, &BOB),
			1767_369_577_951_894_138_582
		);

		assert_eq!(
			<Test as Config>::MultiAssetsHandler::total_supply(DOT_BTC_LP_ID),
			2_830_194_692_949_585_756_466_093
		);

		assert_eq!(<Test as Config>::MultiAssetsHandler::balance_of(ETH_ASSET_ID, &BOB), 0);
		assert_eq!(<Test as Config>::MultiAssetsHandler::balance_of(KSM_ASSET_ID, &BOB), 0);

		assert_eq!(<Test as Config>::MultiAssetsHandler::balance_of(ETH_ASSET_ID, &ALICE), 0);
		assert_eq!(<Test as Config>::MultiAssetsHandler::balance_of(KSM_ASSET_ID, &ALICE), 0);
	})
}
