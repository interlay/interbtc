// Copyright 2021-2022 Zenlink.
// Licensed under Apache 2.0.

use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;
use sp_runtime::DispatchError::BadOrigin;

use super::{
	mock::{CurrencyId::*, *},
	*,
};
use crate::{base_pool_tests::setup_test_base_pool, traits::StableAmmApi};

const INITIAL_A_VALUE: Number = 50;
const SWAP_FEE: Number = 1e7 as Number;
const ADMIN_FEE: Number = 0;

fn create_mock_base_pool() {
	assert_ok!(StableAmm::create_base_pool(
		RawOrigin::Root.into(),
		vec![Token(TOKEN1_SYMBOL), Token(TOKEN3_SYMBOL), Token(TOKEN4_SYMBOL)],
		vec![TOKEN1_DECIMAL, TOKEN3_DECIMAL, TOKEN4_DECIMAL],
		200,
		4e6 as Number,
		0,
		ALICE,
		Vec::from("base_pool_lp"),
	));
}

fn create_mock_meta_pool() {
	let base_pool = StableAmm::pools(0).unwrap().get_pool_info();
	let base_pool_lp_currency = base_pool.lp_currency_id;

	assert_ok!(StableAmm::create_meta_pool(
		RawOrigin::Root.into(),
		vec![Token(TOKEN2_SYMBOL), base_pool_lp_currency],
		vec![TOKEN2_DECIMAL, STABLE_LP_DECIMAL],
		INITIAL_A_VALUE,
		SWAP_FEE,
		0,
		ALICE,
		Vec::from("meta_pool_lp"),
	));
}

fn setup_test_meta_pool() -> (u32, u32) {
	create_mock_base_pool();

	let base_pool = StableAmm::pools(0).unwrap().get_pool_info();
	let base_pool_lp_currency = base_pool.lp_currency_id;

	assert_ok!(StableAmm::add_liquidity(
		RawOrigin::Signed(ALICE).into(),
		0,
		vec![1e20 as Balance, 1e8 as Balance, 1e8 as Balance],
		0,
		ALICE,
		u64::MAX,
	));

	assert_ok!(StableAmm::add_liquidity(
		RawOrigin::Signed(BOB).into(),
		0,
		vec![1e20 as Balance, 1e8 as Balance, 1e8 as Balance],
		0,
		BOB,
		u64::MAX,
	));

	assert_ok!(StableAmm::add_liquidity(
		RawOrigin::Signed(CHARLIE).into(),
		0,
		vec![1e20 as Balance, 1e8 as Balance, 1e8 as Balance],
		0,
		CHARLIE,
		u64::MAX,
	));

	create_mock_meta_pool();

	assert_ok!(StableAmm::add_liquidity(
		RawOrigin::Signed(ALICE).into(),
		1,
		vec![1e18 as Balance, 1e18 as Balance],
		0,
		ALICE,
		u64::MAX
	));

	let pool = StableAmm::pools(1).unwrap().get_pool_info();

	assert_eq!(get_user_balance(Token(TOKEN2_SYMBOL), &pool.account), 1e18 as Balance);
	assert_eq!(get_user_balance(base_pool_lp_currency, &pool.account), 1e18 as Balance);

	(0, 1)
}

#[test]
fn create_meta_pool_with_incorrect_parameter_should_not_work() {
	new_test_ext().execute_with(|| {
		let (_, base_pool_lp_currency) = setup_test_base_pool();

		// only root can create pool
		assert_noop!(
			StableAmm::create_meta_pool(
				RawOrigin::Signed(ALICE).into(),
				vec![
					Token(TOKEN1_SYMBOL),
					Token(TOKEN2_SYMBOL),
					Token(TOKEN3_SYMBOL),
					Token(TOKEN4_SYMBOL)
				],
				vec![TOKEN1_DECIMAL, TOKEN2_DECIMAL, TOKEN3_DECIMAL, TOKEN4_DECIMAL],
				0,
				0,
				0,
				ALICE,
				Vec::from("stable_pool_lp"),
			),
			BadOrigin
		);
		assert_eq!(StableAmm::next_pool_id(), 1);
		assert_eq!(StableAmm::pools(1), None);

		// create meta_pool use not exist base pool id should failed
		assert_noop!(
			StableAmm::create_meta_pool(
				RawOrigin::Root.into(),
				vec![
					Token(TOKEN1_SYMBOL),
					Token(TOKEN2_SYMBOL),
					Token(TOKEN3_SYMBOL),
					StableLPV2(1)
				],
				vec![TOKEN1_DECIMAL, TOKEN2_DECIMAL, TOKEN3_DECIMAL, STABLE_LP_DECIMAL],
				0,
				0,
				0,
				ALICE,
				Vec::from("meta_pool_lp"),
			),
			Error::<Test>::InvalidBasePoolLpCurrency
		);
		assert_eq!(StableAmm::next_pool_id(), 1);
		assert_eq!(StableAmm::pools(1), None);

		//create mismatch parameter should not work
		assert_noop!(
			StableAmm::create_base_pool(
				RawOrigin::Root.into(),
				vec![Token(TOKEN1_SYMBOL), Token(TOKEN2_SYMBOL), base_pool_lp_currency],
				vec![TOKEN1_DECIMAL, TOKEN2_DECIMAL, TOKEN3_DECIMAL, STABLE_LP_DECIMAL],
				0,
				0,
				0,
				ALICE,
				Vec::from("meta_pool_lp"),
			),
			Error::<Test>::MismatchParameter
		);

		assert_eq!(StableAmm::next_pool_id(), 1);
		assert_eq!(StableAmm::pools(1), None);

		// create with forbidden token should not work
		assert_noop!(
			StableAmm::create_base_pool(
				RawOrigin::Root.into(),
				vec![
					Forbidden(TOKEN1_SYMBOL),
					Token(TOKEN2_SYMBOL),
					Token(TOKEN3_SYMBOL),
					base_pool_lp_currency,
				],
				vec![TOKEN1_DECIMAL, TOKEN2_DECIMAL, TOKEN3_DECIMAL, TOKEN4_DECIMAL],
				0,
				0,
				0,
				ALICE,
				Vec::from("stable_pool_lp"),
			),
			Error::<Test>::InvalidPooledCurrency
		);
		assert_eq!(StableAmm::next_pool_id(), 1);
		assert_eq!(StableAmm::pools(1), None);

		// Create with invalid decimal should not work
		assert_noop!(
			StableAmm::create_base_pool(
				RawOrigin::Root.into(),
				vec![Token(TOKEN1_SYMBOL), Token(TOKEN2_SYMBOL), base_pool_lp_currency,],
				vec![TOKEN1_DECIMAL, 20, TOKEN3_DECIMAL],
				0,
				0,
				0,
				ALICE,
				Vec::from("stable_pool_lp"),
			),
			Error::<Test>::InvalidCurrencyDecimal
		);
		assert_eq!(StableAmm::next_pool_id(), 1);
		assert_eq!(StableAmm::pools(1), None);
	});
}

#[test]
fn create_meta_pool_with_parameters_exceed_threshold_should_not_work() {
	new_test_ext().execute_with(|| {
		// exceed max swap fee
		let (_, base_lp_currency) = setup_test_base_pool();
		assert_noop!(
			StableAmm::create_meta_pool(
				RawOrigin::Root.into(),
				vec![
					Token(TOKEN1_SYMBOL),
					Token(TOKEN2_SYMBOL),
					Token(TOKEN3_SYMBOL),
					base_lp_currency
				],
				vec![TOKEN1_DECIMAL, TOKEN2_DECIMAL, TOKEN3_DECIMAL, STABLE_LP_DECIMAL],
				0,
				(MAX_SWAP_FEE + 1).into(),
				0,
				ALICE,
				Vec::from("meta_pool_lp"),
			),
			Error::<Test>::ExceedMaxFee
		);
		assert_eq!(StableAmm::next_pool_id(), 1);
		assert_eq!(StableAmm::pools(1), None);

		// exceed max admin fee
		assert_noop!(
			StableAmm::create_base_pool(
				RawOrigin::Root.into(),
				vec![
					Token(TOKEN1_SYMBOL),
					Token(TOKEN2_SYMBOL),
					Token(TOKEN3_SYMBOL),
					base_lp_currency
				],
				vec![TOKEN1_DECIMAL, TOKEN2_DECIMAL, TOKEN3_DECIMAL, STABLE_LP_DECIMAL],
				0,
				(MAX_SWAP_FEE).into(),
				(MAX_ADMIN_FEE + 1).into(),
				ALICE,
				Vec::from("meta_pool_lp"),
			),
			Error::<Test>::ExceedMaxAdminFee
		);
		assert_eq!(StableAmm::next_pool_id(), 1);
		assert_eq!(StableAmm::pools(1), None);

		// exceed max a
		assert_noop!(
			StableAmm::create_meta_pool(
				RawOrigin::Root.into(),
				vec![
					Token(TOKEN1_SYMBOL),
					Token(TOKEN2_SYMBOL),
					Token(TOKEN3_SYMBOL),
					base_lp_currency
				],
				vec![TOKEN1_DECIMAL, TOKEN2_DECIMAL, TOKEN3_DECIMAL, STABLE_LP_DECIMAL],
				MAX_A.into(),
				(MAX_SWAP_FEE - 1).into(),
				(MAX_ADMIN_FEE - 1).into(),
				ALICE,
				Vec::from("meta_pool_lp"),
			),
			Error::<Test>::ExceedMaxA
		);
		assert_eq!(StableAmm::next_pool_id(), 1);
		assert_eq!(StableAmm::pools(1), None);
	});
}

#[test]
fn create_meta_pool_should_work() {
	new_test_ext().execute_with(|| {
		let (base_pool_id, base_pool_lp_currency) = setup_test_base_pool();
		let meta_pool_lp_currency = CurrencyId::StableLPV2(1);
		assert_eq!(StableAmm::lp_currencies(meta_pool_lp_currency), None);

		let now = 100;
		set_block_timestamp(now);

		assert_ok!(StableAmm::create_meta_pool(
			RawOrigin::Root.into(),
			vec![
				Token(TOKEN1_SYMBOL),
				Token(TOKEN2_SYMBOL),
				Token(TOKEN3_SYMBOL),
				base_pool_lp_currency
			],
			vec![TOKEN1_DECIMAL, TOKEN2_DECIMAL, TOKEN3_DECIMAL, STABLE_LP_DECIMAL],
			INITIAL_A_VALUE,
			SWAP_FEE,
			ADMIN_FEE,
			ALICE,
			Vec::from("stable_pool_lp"),
		));

		let base_pool = StableAmm::pools(base_pool_id).unwrap();
		let base_virtual_price = StableAmm::get_pool_virtual_price(&base_pool).unwrap();

		assert_eq!(StableAmm::next_pool_id(), 2);

		assert_eq!(
			StableAmm::pools(1),
			Some(MockPool::Meta(MetaPool {
				base_pool_id,
				base_virtual_price,
				base_cache_last_updated: now,
				base_currencies: vec![Token(TOKEN1_SYMBOL), Token(TOKEN2_SYMBOL),],
				info: BasePool {
					currency_ids: vec![
						Token(TOKEN1_SYMBOL),
						Token(TOKEN2_SYMBOL),
						Token(TOKEN3_SYMBOL),
						base_pool_lp_currency,
					],
					lp_currency_id: meta_pool_lp_currency,
					token_multipliers: vec![
						checked_pow(10, (POOL_TOKEN_COMMON_DECIMALS - TOKEN1_DECIMAL) as usize)
							.unwrap(),
						checked_pow(10, (POOL_TOKEN_COMMON_DECIMALS - TOKEN2_DECIMAL) as usize)
							.unwrap(),
						checked_pow(10, (POOL_TOKEN_COMMON_DECIMALS - TOKEN3_DECIMAL) as usize)
							.unwrap(),
						checked_pow(10, (POOL_TOKEN_COMMON_DECIMALS - STABLE_LP_DECIMAL) as usize)
							.unwrap(),
					],
					balances: vec![Zero::zero(); 4],
					fee: SWAP_FEE,
					admin_fee: ADMIN_FEE,
					initial_a: INITIAL_A_VALUE * (A_PRECISION as Balance),
					future_a: INITIAL_A_VALUE * (A_PRECISION as Balance),
					initial_a_time: 0,
					future_a_time: 0,
					account: POOL1ACCOUNTID,
					admin_fee_receiver: ALICE,
					lp_currency_symbol: BoundedVec::<u8, PoolCurrencySymbolLimit>::try_from(
						Vec::from("stable_pool_lp")
					)
					.unwrap(),
					lp_currency_decimal: 18,
				}
			}))
		);

		assert_eq!(StableAmm::lp_currencies(meta_pool_lp_currency), Some(1))
	});
}

#[test]
fn add_liquidity_with_incorrect_params_in_meta_pool_should_not_work() {
	new_test_ext().execute_with(|| {
		let (base_pool_id, base_pool_lp_currency) = setup_test_base_pool();

		assert_ok!(StableAmm::create_meta_pool(
			RawOrigin::Root.into(),
			vec![Token(TOKEN1_SYMBOL), base_pool_lp_currency],
			vec![TOKEN1_DECIMAL, STABLE_LP_DECIMAL],
			INITIAL_A_VALUE,
			SWAP_FEE,
			ADMIN_FEE,
			ALICE,
			Vec::from("stable_pool_lp"),
		));

		let meta_pool_id = base_pool_id + 1;

		// case0: add_liquidity with incorrect pool id
		assert_noop!(
			StableAmm::add_liquidity(
				RawOrigin::Signed(BOB).into(),
				meta_pool_id + 1,
				vec![1e16 as Balance, 2e18 as Balance],
				0,
				BOB,
				u64::MAX,
			),
			Error::<Test>::InvalidPoolId
		);

		// case1: add_liquidity with invalid amounts length
		assert_noop!(
			StableAmm::add_liquidity(
				RawOrigin::Signed(BOB).into(),
				meta_pool_id,
				vec![1e16 as Balance],
				0,
				BOB,
				u64::MAX,
			),
			Error::<Test>::MismatchParameter
		);

		// case2: initial add liquidity require all currencies
		assert_noop!(
			StableAmm::add_liquidity(
				RawOrigin::Signed(BOB).into(),
				meta_pool_id,
				vec![1e16 as Balance, 0 as Balance],
				0,
				BOB,
				u64::MAX,
			),
			Error::<Test>::RequireAllCurrencies
		);
	});
}

#[test]
fn add_liquidity_when_withdraw_more_than_available_in_meta_pool_should_fail() {
	new_test_ext().execute_with(|| {
		setup_test_meta_pool();
		let meta_pool_id = 1;
		assert_noop!(
			StableAmm::calculate_currency_amount(
				meta_pool_id,
				vec![Balance::MAX, 3e18 as Balance],
				false
			),
			Error::<Test>::Arithmetic
		);
	})
}

#[test]
fn add_liquidity_with_expected_lp_amount_in_meta_pool_should_success() {
	new_test_ext().execute_with(|| {
		setup_test_meta_pool();
		let meta_pool_id = 1;
		let calculated_pool_token_amount = StableAmm::calculate_currency_amount(
			meta_pool_id,
			vec![1e18 as Balance, 3e18 as Balance],
			true,
		)
		.unwrap();
		let calculated_pool_token_amount_with_slippage = calculated_pool_token_amount * 999 / 1000;
		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![1e18 as Balance, 3e18 as Balance],
			calculated_pool_token_amount_with_slippage,
			BOB,
			u64::MAX,
		));

		let pool = StableAmm::pools(1).unwrap().get_pool_info();

		assert_eq!(get_user_balance(pool.lp_currency_id, &BOB), 3991672211258372957);
	})
}

#[test]
fn add_liquidity_when_lp_token_amount_has_small_slippage_in_meta_pool_should_work() {
	new_test_ext().execute_with(|| {
		setup_test_meta_pool();
		let meta_pool_id = 1;
		let calculated_pool_token_amount = StableAmm::calculate_currency_amount(
			meta_pool_id,
			vec![1e18 as Balance, 3e18 as Balance],
			true,
		)
		.unwrap();
		let calculated_pool_token_amount_with_negative_slippage =
			calculated_pool_token_amount * 999 / 1000;
		let calculated_pool_token_amount_with_positive_slippage =
			calculated_pool_token_amount * 1001 / 1000;
		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![1e18 as Balance, 3e18 as Balance],
			calculated_pool_token_amount_with_negative_slippage,
			BOB,
			u64::MAX,
		));

		let pool = StableAmm::pools(1).unwrap().get_pool_info();

		let actual_lp_out_amount = get_user_balance(pool.lp_currency_id, &BOB);
		assert!(actual_lp_out_amount < calculated_pool_token_amount_with_positive_slippage);
		assert!(actual_lp_out_amount > calculated_pool_token_amount_with_negative_slippage);
	})
}

#[test]
fn add_liquidity_in_meta_pool_update_pool_balances_should_correct() {
	new_test_ext().execute_with(|| {
		setup_test_meta_pool();
		let meta_pool_id = 1;
		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![1e18 as Balance, 3e18 as Balance],
			0,
			BOB,
			u64::MAX,
		));

		let pool = StableAmm::pools(1).unwrap().get_pool_info();

		assert_eq!(pool.balances[0], 2e18 as Balance);
		assert_eq!(pool.balances[1], 4e18 as Balance);

		assert_eq!(get_user_balance(pool.currency_ids[0], &pool.account), 2e18 as Balance);
		assert_eq!(get_user_balance(pool.currency_ids[1], &pool.account), 4e18 as Balance);
	})
}

#[test]
fn add_liquidity_in_meta_pool_when_mint_amount_not_reach_due_to_front_running_should_not_work() {
	new_test_ext().execute_with(|| {
		setup_test_meta_pool();
		let meta_pool_id = 1;

		let calculated_pool_token_amount = StableAmm::calculate_currency_amount(
			meta_pool_id,
			vec![1e18 as Balance, 3e18 as Balance],
			true,
		)
		.unwrap();
		let calculated_pool_token_amount_with_negative_slippage =
			calculated_pool_token_amount * 999 / 1000;

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![1e18 as Balance, 3e18 as Balance],
			0,
			BOB,
			u64::MAX,
		));

		assert_noop!(
			StableAmm::add_liquidity(
				RawOrigin::Signed(BOB).into(),
				meta_pool_id,
				vec![1e18 as Balance, 3e18 as Balance],
				calculated_pool_token_amount_with_negative_slippage,
				BOB,
				u64::MAX,
			),
			Error::<Test>::AmountSlippage
		);
	})
}

#[test]
fn add_liquidity_in_meta_pool_when_block_expired_should_revert() {
	new_test_ext().execute_with(|| {
		setup_test_meta_pool();
		let meta_pool_id = 1;

		System::set_block_number(100);
		assert_noop!(
			StableAmm::add_liquidity(
				RawOrigin::Signed(BOB).into(),
				meta_pool_id,
				vec![1e18 as Balance, 3e18 as Balance],
				0,
				BOB,
				10,
			),
			Error::<Test>::Deadline
		);
	})
}

#[test]
fn remove_liquidity_in_meta_pool_exceed_total_supply_should_not_work() {
	new_test_ext().execute_with(|| {
		setup_test_meta_pool();
		let meta_pool_id = 1;
		let pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();
		assert!(StableAmm::calculate_base_remove_liquidity(&pool, Balance::MAX) == None);
	})
}

#[test]
fn remove_liquidity_in_meta_pool_with_incorrect_min_amounts_length_should_not_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();
		assert_noop!(
			StableAmm::remove_liquidity(
				RawOrigin::Signed(ALICE).into(),
				meta_pool_id,
				2e18 as Balance,
				vec![0],
				ALICE,
				u64::MAX,
			),
			Error::<Test>::MismatchParameter
		);
	})
}

#[test]
fn remove_liquidity_in_meta_pool_should_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			1,
			vec![2e18 as Balance, 1e16 as Balance],
			0,
			BOB,
			u64::MAX
		));

		let pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();
		let bob_balance = get_user_balance(pool.lp_currency_id, &BOB);
		assert_eq!(bob_balance, 1996275270169644725);

		assert_ok!(StableAmm::remove_liquidity(
			RawOrigin::Signed(ALICE).into(),
			meta_pool_id,
			2e18 as Balance,
			vec![0, 0],
			ALICE,
			u64::MAX,
		),);

		assert_ok!(StableAmm::remove_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			bob_balance,
			vec![0, 0],
			BOB,
			u64::MAX,
		),);

		assert_eq!(get_user_balance(pool.currency_ids[0], &pool.account), 0);
		assert_eq!(get_user_balance(pool.currency_ids[1], &pool.account), 0);
	})
}

#[test]
fn remove_liquidity_in_meta_pool_with_expected_return_amount_underlying_currency_should_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![2e18 as Balance, 1e16 as Balance],
			0,
			CHARLIE,
			u64::MAX,
		));

		let meta_pool = StableAmm::pools(meta_pool_id).unwrap();
		let meta_pool_info = meta_pool.get_pool_info();

		let first_token_balance_before = get_user_balance(meta_pool_info.currency_ids[0], &CHARLIE);
		let second_token_balance_before =
			get_user_balance(meta_pool_info.currency_ids[1], &CHARLIE);
		let pool_token_balance_before = get_user_balance(meta_pool_info.lp_currency_id, &CHARLIE);

		assert_eq!(pool_token_balance_before, 1996275270169644725);
		let expected_balances =
			StableAmm::calculate_base_remove_liquidity(&meta_pool_info, pool_token_balance_before)
				.unwrap();
		assert_eq!(expected_balances[0], 1498601924450190405);
		assert_eq!(expected_balances[1], 504529314564897436);

		assert_ok!(StableAmm::remove_liquidity(
			RawOrigin::Signed(CHARLIE).into(),
			meta_pool_id,
			pool_token_balance_before,
			expected_balances.clone(),
			CHARLIE,
			u64::MAX
		));

		let first_token_balance_after = get_user_balance(meta_pool_info.currency_ids[0], &CHARLIE);
		let second_token_balance_after = get_user_balance(meta_pool_info.currency_ids[1], &CHARLIE);

		assert_eq!(first_token_balance_after - first_token_balance_before, expected_balances[0]);
		assert_eq!(second_token_balance_after - second_token_balance_before, expected_balances[1]);
	})
}

#[test]
fn remove_liquidity_in_meta_pool_exceed_own_lp_tokens_should_not_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![2e18 as Balance, 1e16 as Balance],
			0,
			BOB,
			u64::MAX,
		));

		let pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();

		let pool_token_balance = get_user_balance(pool.lp_currency_id, &BOB);
		assert_eq!(pool_token_balance, 1996275270169644725);
		assert_noop!(
			StableAmm::remove_liquidity(
				RawOrigin::Signed(BOB).into(),
				meta_pool_id,
				pool_token_balance + 1,
				vec![Balance::MAX, Balance::MAX],
				BOB,
				u64::MAX
			),
			Error::<Test>::AmountSlippage
		);
	})
}

#[test]
fn remove_liquidity_in_meta_pool_when_min_amounts_not_reached_due_to_front_running_should_not_work()
{
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![2e18 as Balance, 1e16 as Balance],
			0,
			BOB,
			u64::MAX,
		));

		let pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();

		let pool_token_balance = get_user_balance(pool.lp_currency_id, &BOB);
		assert_eq!(pool_token_balance, 1996275270169644725);

		let expected_balances =
			StableAmm::calculate_base_remove_liquidity(&pool, pool_token_balance).unwrap();
		assert_eq!(expected_balances[0], 1498601924450190405);
		assert_eq!(expected_balances[1], 504529314564897436);

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(CHARLIE).into(),
			meta_pool_id,
			vec![1e16 as Balance, 2e18 as Balance],
			0,
			CHARLIE,
			u64::MAX,
		));

		assert_noop!(
			StableAmm::remove_liquidity(
				RawOrigin::Signed(BOB).into(),
				meta_pool_id,
				pool_token_balance,
				expected_balances,
				BOB,
				u64::MAX
			),
			Error::<Test>::AmountSlippage
		);
	})
}

#[test]
fn remove_liquidity_in_meta_pool_with_expired_deadline_should_not_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();
		let pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![2e18 as Balance, 1e16 as Balance],
			0,
			BOB,
			u64::MAX,
		));
		let pool_token_balance = get_user_balance(pool.lp_currency_id, &BOB);

		System::set_block_number(100);

		assert_noop!(
			StableAmm::remove_liquidity(
				RawOrigin::Signed(BOB).into(),
				meta_pool_id,
				pool_token_balance,
				vec![0, 0],
				BOB,
				99
			),
			Error::<Test>::Deadline
		);
	})
}

#[test]
fn remove_liquidity_imbalance_in_meta_pool_with_mismatch_amounts_should_not_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();
		assert_noop!(
			StableAmm::remove_liquidity_imbalance(
				RawOrigin::Signed(ALICE).into(),
				meta_pool_id,
				vec![1e18 as Balance],
				Balance::MAX,
				ALICE,
				u64::MAX
			),
			Error::<Test>::MismatchParameter
		);
	})
}

#[test]
fn remove_liquidity_imbalance_in_meta_pool_when_withdraw_more_than_available_should_not_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();
		assert_noop!(
			StableAmm::remove_liquidity_imbalance(
				RawOrigin::Signed(ALICE).into(),
				meta_pool_id,
				vec![Balance::MAX, Balance::MAX],
				1,
				ALICE,
				u64::MAX
			),
			Error::<Test>::Arithmetic
		);
	})
}

#[test]
fn remove_liquidity_imbalance_in_meta_pool_with_max_burn_lp_token_amount_range_should_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();
		let pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![2e18 as Balance, 1e16 as Balance],
			0,
			BOB,
			u64::MAX,
		));

		// calculates amount of pool token to be burned
		let max_pool_token_amount_to_be_burned = StableAmm::calculate_currency_amount(
			meta_pool_id,
			vec![1e18 as Balance, 1e16 as Balance],
			false,
		)
		.unwrap();
		assert_eq!(1000688044155287276, max_pool_token_amount_to_be_burned);

		let max_pool_token_amount_to_be_burned_negative_slippage =
			max_pool_token_amount_to_be_burned * 1001 / 1000;
		let max_pool_token_amount_to_be_burned_positive_slippage =
			max_pool_token_amount_to_be_burned * 999 / 1000;

		let mut currency_ids = pool.currency_ids.clone();
		currency_ids.push(pool.lp_currency_id);

		let balance_before = get_user_token_balances(&currency_ids, &BOB);

		assert_ok!(StableAmm::remove_liquidity_imbalance(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![1e18 as Balance, 1e16 as Balance],
			max_pool_token_amount_to_be_burned_negative_slippage,
			BOB,
			u64::MAX
		));

		let balance_after = get_user_token_balances(&currency_ids, &BOB);

		// Check the actual returned token amounts match the requested amounts
		assert_eq!(balance_after[0] - balance_before[0], 1e18 as Balance);
		assert_eq!(balance_after[1] - balance_before[1], 1e16 as Balance);
		let actual_pool_token_burned = balance_before[2] - balance_after[2];
		assert_eq!(actual_pool_token_burned, 1000934178112841889);

		assert!(actual_pool_token_burned > max_pool_token_amount_to_be_burned_positive_slippage);
		assert!(actual_pool_token_burned < max_pool_token_amount_to_be_burned_negative_slippage);
	})
}

#[test]
fn remove_liquidity_imbalance_in_meta_pool_exceed_own_lp_token_amount_should_not_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();
		let pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![2e18 as Balance, 1e16 as Balance],
			0,
			BOB,
			u64::MAX,
		));

		let current_balance = get_user_balance(pool.lp_currency_id, &BOB);
		assert_eq!(current_balance, 1996275270169644725);

		assert_noop!(
			StableAmm::remove_liquidity_imbalance(
				RawOrigin::Signed(BOB).into(),
				meta_pool_id,
				vec![2e18 as Balance, 1e16 as Balance],
				current_balance + 1,
				BOB,
				u64::MAX
			),
			Error::<Test>::AmountSlippage
		);
	})
}

#[test]
fn remove_liquidity_imbalance_in_meta_pool_when_min_amounts_of_underlying_tokens_not_reached_should_not_work(
) {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();
		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![2e18 as Balance, 1e16 as Balance],
			0,
			BOB,
			u64::MAX,
		));

		let max_pool_token_amount_to_be_burned = StableAmm::calculate_currency_amount(
			meta_pool_id,
			vec![1e18 as Balance, 1e16 as Balance],
			false,
		)
		.unwrap();

		let max_pool_token_amount_to_be_burned_negative_slippage =
			max_pool_token_amount_to_be_burned * 1001 / 1000;

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(CHARLIE).into(),
			meta_pool_id,
			vec![1e16 as Balance, 2e20 as Balance],
			0,
			ALICE,
			u64::MAX,
		));

		assert_noop!(
			StableAmm::remove_liquidity_imbalance(
				RawOrigin::Signed(BOB).into(),
				meta_pool_id,
				vec![1e18 as Balance, 1e16 as Balance],
				max_pool_token_amount_to_be_burned_negative_slippage,
				BOB,
				u64::MAX
			),
			Error::<Test>::AmountSlippage
		);
	})
}

#[test]
fn remove_liquidity_imbalance_in_meta_pool_with_expired_deadline_should_not_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();
		let pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![2e18 as Balance, 1e16 as Balance],
			0,
			BOB,
			u64::MAX,
		));
		let current_balance = get_user_balance(pool.lp_currency_id, &BOB);
		System::set_block_number(100);

		assert_noop!(
			StableAmm::remove_liquidity_imbalance(
				RawOrigin::Signed(BOB).into(),
				meta_pool_id,
				vec![1e18 as Balance, 1e16 as Balance],
				current_balance,
				BOB,
				99
			),
			Error::<Test>::Deadline
		);
	})
}

#[test]
fn remove_liquidity_one_currency_in_meta_pool_with_currency_index_out_range_should_not_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();
		assert_eq!(
			StableAmm::stable_amm_calculate_remove_liquidity_one_currency(meta_pool_id, 1, 5),
			None
		);
	})
}

#[test]
fn remove_liquidity_one_currency_calculation_should_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();
		let pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![2e18 as Balance, 1e16 as Balance],
			0,
			BOB,
			u64::MAX,
		));

		let pool_token_balance = get_user_balance(pool.lp_currency_id, &BOB);
		assert_eq!(pool_token_balance, 1996275270169644725);

		assert_eq!(
			StableAmm::stable_amm_calculate_remove_liquidity_one_currency(
				meta_pool_id,
				2 * pool_token_balance,
				0
			)
			.unwrap(),
			2999998601797183633
		);
	})
}

#[test]
fn remove_liquidity_one_currency_calculated_amount_as_min_amount_should_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();
		let pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![2e18 as Balance, 1e16 as Balance],
			0,
			BOB,
			u64::MAX,
		));

		let pool_token_balance = get_user_balance(pool.lp_currency_id, &BOB);
		assert_eq!(pool_token_balance, 1996275270169644725);

		let calculated_first_token_amount =
			StableAmm::stable_amm_calculate_remove_liquidity_one_currency(
				meta_pool_id,
				pool_token_balance,
				0,
			)
			.unwrap();
		assert_eq!(calculated_first_token_amount, 2008990034631583696);

		let before = get_user_balance(pool.currency_ids[0], &BOB);

		assert_ok!(StableAmm::remove_liquidity_one_currency(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			pool_token_balance,
			0,
			calculated_first_token_amount,
			BOB,
			u64::MAX
		));

		let after = get_user_balance(pool.currency_ids[0], &BOB);
		assert_eq!(after - before, 2008990034631583696);
	})
}

#[test]
fn remove_liquidity_one_currency_with_lp_token_amount_exceed_own_should_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();
		let pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![2e18 as Balance, 1e16 as Balance],
			0,
			BOB,
			u64::MAX,
		));

		let pool_token_balance = get_user_balance(pool.lp_currency_id, &BOB);
		assert_eq!(pool_token_balance, 1996275270169644725);

		assert_noop!(
			StableAmm::remove_liquidity_one_currency(
				RawOrigin::Signed(BOB).into(),
				meta_pool_id,
				pool_token_balance + 1,
				0,
				0,
				BOB,
				u64::MAX
			),
			Error::<Test>::InsufficientSupply
		);
	})
}

#[test]
fn remove_liquidity_one_currency_with_min_amount_not_reached_due_to_front_running_should_not_work()
{
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();
		let pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![2e18 as Balance, 1e16 as Balance],
			0,
			BOB,
			u64::MAX,
		));

		let pool_token_balance = get_user_balance(pool.lp_currency_id, &BOB);
		assert_eq!(pool_token_balance, 1996275270169644725);

		let calculated_first_token_amount =
			StableAmm::stable_amm_calculate_remove_liquidity_one_currency(
				meta_pool_id,
				pool_token_balance,
				0,
			)
			.unwrap();
		assert_eq!(calculated_first_token_amount, 2008990034631583696);

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(CHARLIE).into(),
			meta_pool_id,
			vec![1e16 as Balance, 1e20 as Balance],
			0,
			BOB,
			u64::MAX,
		));

		assert_noop!(
			StableAmm::remove_liquidity_one_currency(
				RawOrigin::Signed(BOB).into(),
				meta_pool_id,
				pool_token_balance,
				0,
				calculated_first_token_amount,
				BOB,
				u64::MAX
			),
			Error::<Test>::AmountSlippage
		);
	})
}

#[test]
fn remove_liquidity_one_currency_with_expired_deadline_should_not_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();
		let pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![2e18 as Balance, 1e16 as Balance],
			0,
			BOB,
			u64::MAX,
		));

		let pool_token_balance = get_user_balance(pool.lp_currency_id, &BOB);

		System::set_block_number(100);

		assert_noop!(
			StableAmm::remove_liquidity_one_currency(
				RawOrigin::Signed(BOB).into(),
				meta_pool_id,
				pool_token_balance,
				0,
				0,
				BOB,
				99
			),
			Error::<Test>::Deadline
		);
	})
}

#[test]
fn swap_with_currency_index_out_of_index_should_not_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();

		assert_eq!(
			StableAmm::stable_amm_calculate_swap_amount(meta_pool_id, 0, 9, 1e17 as Balance),
			None
		);
	})
}

#[test]
fn swap_with_currency_amount_exceed_own_should_not_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();
		assert_noop!(
			StableAmm::swap(
				RawOrigin::Signed(BOB).into(),
				meta_pool_id,
				0,
				1,
				Balance::MAX,
				0,
				BOB,
				u64::MAX
			),
			Error::<Test>::InsufficientReserve
		);
	})
}

#[test]
fn swap_with_expected_amounts_should_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();
		let pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();

		let calculated_swap_return =
			StableAmm::stable_amm_calculate_swap_amount(meta_pool_id, 0, 1, 1e17 as Balance)
				.unwrap();

		assert_eq!(calculated_swap_return, 99702611562565289);

		let token_from_balance_before = get_user_balance(pool.currency_ids[0], &BOB);
		let token_to_balance_before = get_user_balance(pool.currency_ids[1], &CHARLIE);

		assert_ok!(StableAmm::swap(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			0,
			1,
			1e17 as Balance,
			calculated_swap_return,
			CHARLIE,
			u64::MAX
		));
		let token_from_balance_after = get_user_balance(pool.currency_ids[0], &BOB);
		let token_to_balance_after = get_user_balance(pool.currency_ids[1], &CHARLIE);

		assert_eq!(token_from_balance_before - token_from_balance_after, 1e17 as Balance);
		assert_eq!(token_to_balance_after - token_to_balance_before, calculated_swap_return);
	})
}

#[test]
fn swap_when_min_amount_receive_not_reached_due_to_front_running_should_not_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();
		let calculated_swap_return =
			StableAmm::stable_amm_calculate_swap_amount(meta_pool_id, 0, 1, 1e17 as Balance)
				.unwrap();

		assert_eq!(calculated_swap_return, 99702611562565289);

		assert_ok!(StableAmm::swap(
			RawOrigin::Signed(CHARLIE).into(),
			meta_pool_id,
			0,
			1,
			1e17 as Balance,
			calculated_swap_return,
			CHARLIE,
			u64::MAX
		));

		assert_noop!(
			StableAmm::swap(
				RawOrigin::Signed(BOB).into(),
				meta_pool_id,
				0,
				1,
				1e17 as Balance,
				calculated_swap_return,
				BOB,
				u64::MAX
			),
			Error::<Test>::AmountSlippage
		);
	})
}

#[test]
fn swap_with_lower_min_dy_when_transaction_is_front_ran_should_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();

		let pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();

		let token_from_balance_before = get_user_balance(pool.currency_ids[0], &BOB);
		let token_to_balance_before = get_user_balance(pool.currency_ids[1], &BOB);

		// BOB calculates how much token to receive with 1% slippage
		let calculated_swap_return =
			StableAmm::stable_amm_calculate_swap_amount(meta_pool_id, 0, 1, 1e17 as Balance)
				.unwrap();
		assert_eq!(calculated_swap_return, 99702611562565289);
		let calculated_swap_return_with_negative_slippage = calculated_swap_return * 99 / 100;

		// CHARLIE swaps before User 1 does
		assert_ok!(StableAmm::swap(
			RawOrigin::Signed(CHARLIE).into(),
			meta_pool_id,
			0,
			1,
			1e17 as Balance,
			0,
			CHARLIE,
			u64::MAX
		));

		// BOB swap with slippage
		assert_ok!(StableAmm::swap(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			0,
			1,
			1e17 as Balance,
			calculated_swap_return_with_negative_slippage,
			BOB,
			u64::MAX
		));

		let token_from_balance_after = get_user_balance(pool.currency_ids[0], &BOB);
		let token_to_balance_after = get_user_balance(pool.currency_ids[1], &BOB);

		assert_eq!(token_from_balance_before - token_from_balance_after, 1e17 as Balance);

		let actual_received_amount = token_to_balance_after - token_to_balance_before;
		assert_eq!(actual_received_amount, 99286252365528551);
		assert!(actual_received_amount > calculated_swap_return_with_negative_slippage);
		assert!(actual_received_amount < calculated_swap_return);
	})
}

#[test]
fn swap_with_expired_deadline_should_not_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();

		System::set_block_number(100);

		assert_noop!(
			StableAmm::swap(
				RawOrigin::Signed(BOB).into(),
				meta_pool_id,
				0,
				1,
				1e17 as Balance,
				0,
				BOB,
				99
			),
			Error::<Test>::Deadline
		);
	})
}

#[test]
fn swap_underlying_with_token_index_out_range_should_not_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();

		assert_noop!(
			StableAmm::swap(
				RawOrigin::Signed(BOB).into(),
				meta_pool_id,
				0,
				9,
				1e17 as Balance,
				0,
				BOB,
				99
			),
			Error::<Test>::CurrencyIndexOutRange
		);
	})
}

#[test]
fn swap_underlying_from_meta_to_base_between_same_decimal_token_should_work() {
	new_test_ext().execute_with(|| {
		let (base_pool_id, meta_pool_id) = setup_test_meta_pool();
		let meta_pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();
		let base_pool = StableAmm::pools(base_pool_id).unwrap().get_pool_info();

		let calculated_swap_return =
			StableAmm::calculate_meta_swap_underlying(meta_pool_id, 1e17 as Balance, 0, 1).unwrap();

		assert_eq!(calculated_swap_return, 99682616104034773);

		let token_from_balance_before = get_user_balance(meta_pool.currency_ids[0], &BOB);
		let token_to_balance_before = get_user_balance(base_pool.currency_ids[0], &BOB);

		let bob_unchanged_token = vec![
			meta_pool.currency_ids[1],
			base_pool.currency_ids[1],
			base_pool.currency_ids[2],
			meta_pool.lp_currency_id,
			base_pool.lp_currency_id,
		];

		let bob_unchanged_balance_before = get_user_token_balances(&bob_unchanged_token, &BOB);

		assert_ok!(StableAmm::swap_meta_pool_underlying(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			0,
			1,
			1e17 as Balance,
			calculated_swap_return,
			BOB,
			u64::MAX,
		));

		let token_from_balance_after = get_user_balance(meta_pool.currency_ids[0], &BOB);
		let token_to_balance_after = get_user_balance(base_pool.currency_ids[0], &BOB);
		let bob_unchanged_balance_after = get_user_token_balances(&bob_unchanged_token, &BOB);

		assert_eq!(token_from_balance_before - token_from_balance_after, 1e17 as Balance);
		assert_eq!(
			token_to_balance_after - token_to_balance_before,
			calculated_swap_return as Balance
		);

		assert_eq!(bob_unchanged_balance_after, bob_unchanged_balance_before);
	})
}

#[test]
fn swap_underlying_from_base_to_meta_between_different_decimal_token_should_work() {
	new_test_ext().execute_with(|| {
		let (base_pool_id, meta_pool_id) = setup_test_meta_pool();
		let meta_pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();
		let base_pool = StableAmm::pools(base_pool_id).unwrap().get_pool_info();

		let calculated_swap_return =
			StableAmm::calculate_meta_swap_underlying(meta_pool_id, 1e5 as Balance, 2, 0).unwrap();

		assert_eq!(calculated_swap_return, 99682656211218516);

		let min_return_with_negative_slippage = calculated_swap_return * 9998 / 10000;

		let token_from_balance_before = get_user_balance(base_pool.currency_ids[1], &ALICE);
		let token_to_balance_before = get_user_balance(meta_pool.currency_ids[0], &ALICE);

		let unchanged_token = vec![
			meta_pool.currency_ids[1],
			base_pool.currency_ids[0],
			base_pool.currency_ids[2],
			meta_pool.lp_currency_id,
			base_pool.lp_currency_id,
		];

		let unchanged_balance_before = get_user_token_balances(&unchanged_token, &ALICE);

		assert_ok!(StableAmm::swap_meta_pool_underlying(
			RawOrigin::Signed(ALICE).into(),
			meta_pool_id,
			2,
			0,
			1e5 as Balance,
			min_return_with_negative_slippage,
			ALICE,
			u64::MAX,
		));

		let token_from_balance_after = get_user_balance(base_pool.currency_ids[1], &ALICE);
		let token_to_balance_after = get_user_balance(meta_pool.currency_ids[0], &ALICE);

		let unchanged_balance_after = get_user_token_balances(&unchanged_token, &ALICE);

		assert_eq!(token_from_balance_before - token_from_balance_after, 1e5 as Balance);
		assert_eq!(token_to_balance_after - token_to_balance_before, 99683651227847339);

		assert_eq!(unchanged_balance_after, unchanged_balance_before);
	})
}

#[test]
fn swap_underlying_from_base_to_base_between_different_decimal_token_should_work() {
	new_test_ext().execute_with(|| {
		let (base_pool_id, meta_pool_id) = setup_test_meta_pool();
		let meta_pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();
		let base_pool = StableAmm::pools(base_pool_id).unwrap().get_pool_info();

		let calculated_swap_return =
			StableAmm::calculate_meta_swap_underlying(meta_pool_id, 1e17 as Balance, 1, 3).unwrap();

		assert_eq!(calculated_swap_return, 99959);

		let token_from_balance_before = get_user_balance(base_pool.currency_ids[0], &ALICE);
		let token_to_balance_before = get_user_balance(base_pool.currency_ids[2], &ALICE);

		let unchanged_token = vec![
			meta_pool.currency_ids[0],
			meta_pool.currency_ids[1],
			base_pool.currency_ids[1],
			meta_pool.lp_currency_id,
			base_pool.lp_currency_id,
		];

		let unchanged_balance_before = get_user_token_balances(&unchanged_token, &ALICE);

		assert_ok!(StableAmm::swap_meta_pool_underlying(
			RawOrigin::Signed(ALICE).into(),
			meta_pool_id,
			1,
			3,
			1e17 as Balance,
			calculated_swap_return,
			ALICE,
			u64::MAX,
		));

		let token_from_balance_after = get_user_balance(base_pool.currency_ids[0], &ALICE);
		let token_to_balance_after = get_user_balance(base_pool.currency_ids[2], &ALICE);
		let unchanged_balance_after = get_user_token_balances(&unchanged_token, &ALICE);

		assert_eq!(token_from_balance_before - token_from_balance_after, 1e17 as Balance);
		assert_eq!(token_to_balance_after - token_to_balance_before, calculated_swap_return);

		assert_eq!(unchanged_balance_before, unchanged_balance_after);
	})
}

#[test]
fn swap_underlying_from_meta_to_meta_should_work() {
	new_test_ext().execute_with(|| {})
}

#[test]
fn swap_underlying_not_reach_min_dy_due_to_front_running_should_revert() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();

		let calculated_swap_return =
			StableAmm::calculate_meta_swap_underlying(meta_pool_id, 1e17 as Balance, 0, 1).unwrap();

		assert_eq!(calculated_swap_return, 99682616104034773);

		assert_ok!(StableAmm::swap_meta_pool_underlying(
			RawOrigin::Signed(ALICE).into(),
			meta_pool_id,
			0,
			1,
			1e17 as Balance,
			0,
			ALICE,
			u64::MAX,
		));

		assert_noop!(
			StableAmm::swap_meta_pool_underlying(
				RawOrigin::Signed(ALICE).into(),
				meta_pool_id,
				0,
				1,
				1e17 as Balance,
				calculated_swap_return,
				ALICE,
				u64::MAX,
			),
			Error::<Test>::AmountSlippage
		);
	})
}

#[test]
fn swap_underlying_with_lower_min_dy_after_front_running_should_work() {
	new_test_ext().execute_with(|| {
		let (base_pool_id, meta_pool_id) = setup_test_meta_pool();

		let calculated_swap_return =
			StableAmm::calculate_meta_swap_underlying(meta_pool_id, 1e17 as Balance, 0, 1).unwrap();

		assert_eq!(calculated_swap_return, 99682616104034773);

		let calculated_swap_return_with_negative_slippage = calculated_swap_return * 99 / 100;

		let meta_pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();
		let base_pool = StableAmm::pools(base_pool_id).unwrap().get_pool_info();

		let token_from_balance_before = get_user_balance(meta_pool.currency_ids[0], &ALICE);
		let token_to_balance_before = get_user_balance(base_pool.currency_ids[0], &ALICE);

		assert_ok!(StableAmm::swap(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			0,
			1,
			1e17 as Balance,
			0,
			BOB,
			u64::MAX
		));

		assert_ok!(StableAmm::swap_meta_pool_underlying(
			RawOrigin::Signed(ALICE).into(),
			meta_pool_id,
			0,
			1,
			1e17 as Balance,
			calculated_swap_return_with_negative_slippage,
			ALICE,
			u64::MAX,
		));

		let token_from_balance_after = get_user_balance(meta_pool.currency_ids[0], &ALICE);
		let token_to_balance_after = get_user_balance(base_pool.currency_ids[0], &ALICE);

		assert_eq!(token_from_balance_before - token_from_balance_after, 1e17 as Balance);
		assert!(calculated_swap_return_with_negative_slippage < 99266340636749675);
		assert_eq!(token_to_balance_after - token_to_balance_before, 99266340636749675);
	})
}

#[test]
fn swap_underlying_with_expired_block_should_revert() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();
		System::set_block_number(100);
		assert_noop!(
			StableAmm::swap_meta_pool_underlying(
				RawOrigin::Signed(ALICE).into(),
				meta_pool_id,
				0,
				1,
				1e17 as Balance,
				0,
				ALICE,
				99
			),
			Error::<Test>::Deadline
		);
	})
}

#[test]
fn get_meta_virtual_price_after_first_deposit_should_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();

		let meta_virtual_price = StableAmm::get_virtual_price(meta_pool_id);

		assert_eq!(meta_virtual_price, 1e18 as Balance);
	})
}

#[test]
fn get_meta_virtual_price_after_swap_should_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();

		assert_ok!(StableAmm::swap(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			0,
			1,
			1e17 as Balance,
			0,
			BOB,
			u64::MAX
		));

		let meta_virtual_price = StableAmm::get_virtual_price(meta_pool_id);
		assert_eq!(meta_virtual_price, 1000050005862349911);

		assert_ok!(StableAmm::swap(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			0,
			1,
			1e17 as Balance,
			0,
			BOB,
			u64::MAX
		));
	})
}

#[test]
fn get_expected_meta_virtual_price_after_imbalance_withdrawal_should_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![1e18 as Balance, 1e18 as Balance],
			0,
			BOB,
			u64::MAX,
		));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(CHARLIE).into(),
			meta_pool_id,
			vec![1e18 as Balance, 1e18 as Balance],
			0,
			CHARLIE,
			u64::MAX,
		));

		assert_ok!(StableAmm::remove_liquidity_imbalance(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![1e18 as Balance, 0],
			2e18 as Balance,
			BOB,
			u64::MAX
		));

		let meta_virtual_price = StableAmm::get_virtual_price(meta_pool_id);
		assert_eq!(meta_virtual_price, 1000100094088440633);

		assert_ok!(StableAmm::remove_liquidity_imbalance(
			RawOrigin::Signed(CHARLIE).into(),
			meta_pool_id,
			vec![0, 1e18 as Balance],
			2e18 as Balance,
			CHARLIE,
			u64::MAX
		));

		let meta_virtual_price = StableAmm::get_virtual_price(meta_pool_id);
		assert_eq!(meta_virtual_price, 1000200154928939884);
	})
}

#[test]
fn meta_virtual_price_unchanged_after_balanced_deposit_should_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();

		assert_eq!(StableAmm::get_virtual_price(meta_pool_id), 1e18 as Balance);

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![1e18 as Balance, 1e18 as Balance],
			0,
			BOB,
			u64::MAX,
		));

		assert_eq!(StableAmm::get_virtual_price(meta_pool_id), 1e18 as Balance);

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(CHARLIE).into(),
			meta_pool_id,
			vec![2e18 as Balance, 0],
			0,
			CHARLIE,
			u64::MAX,
		));

		let meta_virtual_price = StableAmm::get_virtual_price(meta_pool_id);
		assert_eq!(meta_virtual_price, 1000167146429977312);

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(CHARLIE).into(),
			meta_pool_id,
			vec![2e18 as Balance, 1e18 as Balance],
			0,
			CHARLIE,
			u64::MAX,
		));

		let meta_virtual_price = StableAmm::get_virtual_price(meta_pool_id);
		assert_eq!(meta_virtual_price, 1000167146429977312);
	})
}

#[test]
fn meta_virtual_price_unchanged_after_balanced_withdrawal_should_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();

		assert_eq!(StableAmm::get_virtual_price(meta_pool_id), 1e18 as Balance);

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![1e18 as Balance, 1e18 as Balance],
			0,
			BOB,
			u64::MAX,
		));

		assert_eq!(StableAmm::get_virtual_price(meta_pool_id), 1e18 as Balance);

		assert_ok!(StableAmm::remove_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			1e18 as Balance,
			vec![0, 0],
			BOB,
			u64::MAX,
		));

		assert_eq!(StableAmm::get_virtual_price(meta_pool_id), 1e18 as Balance);
	})
}

#[test]
fn set_meta_fee_with_no_admin_should_revert() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();
		assert_noop!(
			StableAmm::set_swap_fee(
				RawOrigin::Signed(CHARLIE).into(),
				meta_pool_id,
				1e8 as Balance
			),
			BadOrigin
		);
	})
}

#[test]
fn set_meta_fee_in_limit_should_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();
		assert_ok!(StableAmm::set_swap_fee(RawOrigin::Root.into(), meta_pool_id, 1e8 as Balance,),);
		let meta_pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();
		assert_eq!(meta_pool.fee, 1e8 as Balance);
	})
}

#[test]
fn set_meta_admin_fee_with_no_admin_should_revert() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();
		assert_noop!(
			StableAmm::set_admin_fee(
				RawOrigin::Signed(CHARLIE).into(),
				meta_pool_id,
				1e8 as Balance
			),
			BadOrigin
		);
	})
}

#[test]
fn set_meta_admin_fee_exceed_limit_should_revert() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();
		assert_noop!(
			StableAmm::set_admin_fee(RawOrigin::Root.into(), meta_pool_id, (1e10 as Balance) + 1),
			Error::<Test>::ExceedThreshold
		);
	})
}

#[test]
fn set_meta_admin_fee_in_limit_should_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();
		assert_ok!(StableAmm::set_admin_fee(RawOrigin::Root.into(), meta_pool_id, 1e10 as Balance),);

		let meta_pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();
		assert_eq!(meta_pool.admin_fee, 1e10 as Balance);
	})
}

#[test]
fn get_meta_admin_balance_with_out_of_range_index_should_revert() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();
		assert_eq!(StableAmm::get_admin_balance(meta_pool_id, 3), None);
	})
}

#[test]
fn get_meta_admin_balance_is_zero_with_zero_admin_fee_should_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();

		assert_eq!(StableAmm::get_admin_balance(meta_pool_id, 0), Some(0));
		assert_eq!(StableAmm::get_admin_balance(meta_pool_id, 1), Some(0));

		assert_ok!(StableAmm::swap(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			0,
			1,
			1e17 as Balance,
			0,
			BOB,
			u64::MAX
		));

		assert_eq!(StableAmm::get_admin_balance(meta_pool_id, 0), Some(0));
		assert_eq!(StableAmm::get_admin_balance(meta_pool_id, 1), Some(0));
	})
}

#[test]
fn get_expected_admin_balance_after_set_admin_fee_should_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();

		assert_eq!(StableAmm::get_admin_balance(meta_pool_id, 0), Some(0));
		assert_eq!(StableAmm::get_admin_balance(meta_pool_id, 1), Some(0));

		assert_ok!(StableAmm::set_admin_fee(RawOrigin::Root.into(), meta_pool_id, 1e8 as Balance),);

		assert_ok!(StableAmm::swap(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			0,
			1,
			1e17 as Balance,
			0,
			BOB,
			u64::MAX
		));

		assert_ok!(StableAmm::swap(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			1,
			0,
			1e17 as Balance,
			0,
			BOB,
			u64::MAX
		));

		assert_eq!(StableAmm::get_admin_balance(meta_pool_id, 0), Some(1001973776101));
		assert_eq!(StableAmm::get_admin_balance(meta_pool_id, 1), Some(998024139765));
	})
}

#[test]
fn get_expected_admin_balance_after_remove_one_currency_should_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();

		assert_eq!(StableAmm::get_admin_balance(meta_pool_id, 0), Some(0));
		assert_eq!(StableAmm::get_admin_balance(meta_pool_id, 1), Some(0));

		assert_ok!(StableAmm::set_admin_fee(RawOrigin::Root.into(), meta_pool_id, 1e8 as Balance),);

		assert_ok!(StableAmm::remove_liquidity_one_currency(
			RawOrigin::Signed(ALICE).into(),
			meta_pool_id,
			1e16 as Balance,
			1,
			0,
			ALICE,
			u64::MAX,
		));

		assert_eq!(StableAmm::get_admin_balance(meta_pool_id, 0), Some(0));
		assert_eq!(StableAmm::get_admin_balance(meta_pool_id, 1), Some(49992612012));

		assert_ok!(StableAmm::remove_liquidity_one_currency(
			RawOrigin::Signed(ALICE).into(),
			meta_pool_id,
			1e16 as Balance,
			0,
			0,
			ALICE,
			u64::MAX,
		));

		assert_eq!(StableAmm::get_admin_balance(meta_pool_id, 0), Some(49751463774));
		assert_eq!(StableAmm::get_admin_balance(meta_pool_id, 1), Some(49992612012));
	})
}

#[test]
fn get_expected_admin_balance_after_remove_imbalance_should_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();

		assert_eq!(StableAmm::get_admin_balance(meta_pool_id, 0), Some(0));
		assert_eq!(StableAmm::get_admin_balance(meta_pool_id, 1), Some(0));

		assert_ok!(StableAmm::set_admin_fee(RawOrigin::Root.into(), meta_pool_id, 1e8 as Balance),);

		assert_ok!(StableAmm::remove_liquidity_imbalance(
			RawOrigin::Signed(ALICE).into(),
			meta_pool_id,
			vec![4000000000000000, 5000000000000000],
			Balance::MAX,
			ALICE,
			u64::MAX,
		));

		assert_eq!(StableAmm::get_admin_balance(meta_pool_id, 0), Some(2500012310));
		assert_eq!(StableAmm::get_admin_balance(meta_pool_id, 1), Some(2499987689));

		assert_ok!(StableAmm::remove_liquidity_imbalance(
			RawOrigin::Signed(ALICE).into(),
			meta_pool_id,
			vec![5000000000000000, 4000000000000000],
			Balance::MAX,
			ALICE,
			u64::MAX,
		));

		assert_eq!(StableAmm::get_admin_balance(meta_pool_id, 0), Some(4988723716));
		assert_eq!(StableAmm::get_admin_balance(meta_pool_id, 1), Some(4988674586));
	})
}

#[test]
fn get_expected_admin_balance_after_add_liquidity_should_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();

		assert_eq!(StableAmm::get_admin_balance(meta_pool_id, 0), Some(0));
		assert_eq!(StableAmm::get_admin_balance(meta_pool_id, 1), Some(0));

		assert_ok!(StableAmm::set_admin_fee(RawOrigin::Root.into(), meta_pool_id, 1e8 as Balance),);

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(ALICE).into(),
			meta_pool_id,
			vec![1e18 as Balance, 2e18 as Balance],
			0,
			ALICE,
			u64::MAX,
		));

		assert_eq!(StableAmm::get_admin_balance(meta_pool_id, 0), Some(2494899977135));
		assert_eq!(StableAmm::get_admin_balance(meta_pool_id, 1), Some(2505100022864));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(ALICE).into(),
			meta_pool_id,
			vec![2e18 as Balance, 1e18 as Balance],
			0,
			ALICE,
			u64::MAX,
		));
		assert_eq!(StableAmm::get_admin_balance(meta_pool_id, 0), Some(6488370760276));
		assert_eq!(StableAmm::get_admin_balance(meta_pool_id, 1), Some(6514897563944));
	})
}

#[test]
fn withdraw_admin_balance_with_zero_admin_fee_should_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();

		let meta_pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();

		let unchanged_token = vec![meta_pool.currency_ids[0], meta_pool.currency_ids[1]];

		let unchanged_balance_before = get_user_token_balances(&unchanged_token, &ALICE);

		assert_ok!(StableAmm::withdraw_admin_fee(RawOrigin::Signed(BOB).into(), meta_pool_id));

		let unchanged_balance_after = get_user_token_balances(&unchanged_token, &ALICE);

		assert_eq!(unchanged_balance_after, unchanged_balance_before);
	})
}

#[test]
fn withdraw_admin_balance_with_expected_amount_after_swap_should_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();

		assert_ok!(StableAmm::set_admin_fee(RawOrigin::Root.into(), meta_pool_id, 1e8 as Balance));

		let meta_pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();

		assert_ok!(StableAmm::swap(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			0,
			1,
			1e17 as Balance,
			0,
			BOB,
			u64::MAX
		));
		assert_ok!(StableAmm::swap(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			1,
			0,
			1e17 as Balance,
			0,
			BOB,
			u64::MAX
		));

		assert_eq!(StableAmm::get_admin_balance(meta_pool_id, 0), Some(1001973776101));
		assert_eq!(StableAmm::get_admin_balance(meta_pool_id, 1), Some(998024139765));

		let fee_tokens = vec![meta_pool.currency_ids[0], meta_pool.currency_ids[1]];

		let fee_tokens_balance_before = get_user_token_balances(&fee_tokens, &ALICE);

		assert_ok!(StableAmm::withdraw_admin_fee(RawOrigin::Signed(BOB).into(), meta_pool_id));

		let fee_tokens_balance_after = get_user_token_balances(&fee_tokens, &ALICE);

		assert_eq!(fee_tokens_balance_after[0] - fee_tokens_balance_before[0], 1001973776101);
		assert_eq!(fee_tokens_balance_after[1] - fee_tokens_balance_before[1], 998024139765);
	})
}

#[test]
fn withdraw_admin_balance_with_expected_amount_after_swap_underlying_should_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();

		assert_ok!(StableAmm::set_admin_fee(RawOrigin::Root.into(), meta_pool_id, 1e8 as Balance));

		let meta_pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();

		assert_ok!(StableAmm::swap(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			0,
			1,
			1e17 as Balance,
			0,
			BOB,
			u64::MAX
		));

		assert_ok!(StableAmm::swap(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			1,
			0,
			1e17 as Balance,
			0,
			BOB,
			u64::MAX
		));

		assert_eq!(StableAmm::get_admin_balance(meta_pool_id, 0), Some(1001973776101));
		assert_eq!(StableAmm::get_admin_balance(meta_pool_id, 1), Some(998024139765));

		let fee_tokens = vec![meta_pool.currency_ids[0], meta_pool.currency_ids[1]];

		let fee_tokens_balance_before = get_user_token_balances(&fee_tokens, &ALICE);

		assert_ok!(StableAmm::withdraw_admin_fee(RawOrigin::Signed(BOB).into(), meta_pool_id));

		let fee_tokens_balance_after = get_user_token_balances(&fee_tokens, &ALICE);

		assert_eq!(fee_tokens_balance_after[0] - fee_tokens_balance_before[0], 1001973776101);
		assert_eq!(fee_tokens_balance_after[1] - fee_tokens_balance_before[1], 998024139765);
	})
}

#[test]
fn withdraw_admin_balance_has_no_impact_on_user_withdrawal_should_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();

		assert_ok!(StableAmm::set_admin_fee(RawOrigin::Root.into(), meta_pool_id, 1e8 as Balance));

		let meta_pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();
		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			1,
			vec![1e18 as Balance, 1e18 as Balance],
			0,
			BOB,
			u64::MAX
		));

		for _i in 0..10 {
			assert_ok!(StableAmm::swap(
				RawOrigin::Signed(ALICE).into(),
				meta_pool_id,
				0,
				1,
				1e17 as Balance,
				0,
				ALICE,
				u64::MAX
			));
			assert_ok!(StableAmm::swap(
				RawOrigin::Signed(ALICE).into(),
				meta_pool_id,
				1,
				0,
				1e17 as Balance,
				0,
				ALICE,
				u64::MAX
			));
		}

		assert_ok!(StableAmm::withdraw_admin_fee(RawOrigin::Signed(BOB).into(), meta_pool_id));

		let impacted_tokens = vec![meta_pool.currency_ids[0], meta_pool.currency_ids[1]];
		let impacted_tokens_balance_before = get_user_token_balances(&impacted_tokens, &BOB);

		let lp_balance = get_user_balance(meta_pool.lp_currency_id, &BOB);
		assert_ok!(StableAmm::remove_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			lp_balance,
			vec![0, 0],
			BOB,
			u64::MAX,
		));

		let impacted_tokens_balance_after = get_user_token_balances(&impacted_tokens, &BOB);

		assert_eq!(
			impacted_tokens_balance_after[0] - impacted_tokens_balance_before[0],
			1000009516257264879
		);
		assert_eq!(
			impacted_tokens_balance_after[1] - impacted_tokens_balance_before[1],
			1000980987206499309
		);
	})
}

#[test]
fn ramp_meta_a_upwards_should_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();

		mine_block();

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![1e18 as Balance, 0],
			0,
			BOB,
			u64::MAX
		));

		mine_block();

		let end_timestamp = Timestamp::now() / 1000 + 14 * DAYS + 1;
		assert_ok!(StableAmm::ramp_a(
			RawOrigin::Root.into(),
			meta_pool_id,
			100,
			end_timestamp.into()
		));

		let pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();
		assert_eq!(StableAmm::get_a_precise(&pool), Some(5000));
		assert_eq!(StableAmm::get_virtual_price(meta_pool_id), 1000167146429977312);

		mine_block_with_timestamp(Timestamp::now() / 1000 + 100000);

		let pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();
		assert_eq!(StableAmm::get_a_precise(&pool), Some(5413));
		assert_eq!(StableAmm::get_virtual_price(meta_pool_id), 1000258443200231295);

		mine_block_with_timestamp(end_timestamp.into());
		let pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();
		assert_eq!(StableAmm::get_a_precise(&pool), Some(10000));
		assert_eq!(StableAmm::get_virtual_price(meta_pool_id), 1000771363829405068);
	})
}

#[test]
fn ramp_meta_a_downward_should_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();

		mine_block();

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![1e18 as Balance, 0],
			0,
			BOB,
			u64::MAX
		));

		mine_block();

		let end_timestamp = Timestamp::now() / 1000 + 14 * DAYS + 1;
		assert_ok!(StableAmm::ramp_a(
			RawOrigin::Root.into(),
			meta_pool_id,
			25,
			end_timestamp.into()
		));

		let pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();

		assert_eq!(StableAmm::get_a_precise(&pool), Some(5000));
		assert_eq!(StableAmm::get_virtual_price(meta_pool_id), 1000167146429977312);

		mine_block_with_timestamp(Timestamp::now() / 1000 + 100000);

		let pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();
		assert_eq!(StableAmm::get_a_precise(&pool), Some(4794));
		assert_eq!(StableAmm::get_virtual_price(meta_pool_id), 1000115870150391894);

		mine_block_with_timestamp(end_timestamp);
		let pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();
		assert_eq!(StableAmm::get_a_precise(&pool), Some(2500));
		assert_eq!(StableAmm::get_virtual_price(meta_pool_id), 998999574522335473);
	})
}

#[test]
fn ramp_meta_a_with_non_owner_should_not_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();

		mine_block();
		let end_timestamp = Timestamp::now() / 1000 + 14 * DAYS + 1;

		assert_noop!(
			StableAmm::ramp_a(
				RawOrigin::Signed(BOB).into(),
				meta_pool_id,
				55,
				end_timestamp.into()
			),
			BadOrigin
		);
	})
}

#[test]
fn ramp_meta_a_not_delay_should_not_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();
		mine_block();

		let end_timestamp = Timestamp::now() / 1000 + 14 * DAYS + 1;
		assert_ok!(StableAmm::ramp_a(
			RawOrigin::Root.into(),
			meta_pool_id,
			55,
			end_timestamp.into()
		));

		assert_noop!(
			StableAmm::ramp_a(RawOrigin::Root.into(), meta_pool_id, 55, end_timestamp.into()),
			Error::<Test>::RampADelay
		);
	})
}

#[test]
fn ramp_meta_a_out_of_range_should_not_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();
		mine_block();

		let end_timestamp = Timestamp::now() / 1000 + 14 * DAYS + 1;

		assert_noop!(
			StableAmm::ramp_a(RawOrigin::Root.into(), meta_pool_id, 0, end_timestamp.into()),
			Error::<Test>::ExceedThreshold
		);

		assert_noop!(
			StableAmm::ramp_a(RawOrigin::Root.into(), meta_pool_id, 501, end_timestamp.into()),
			Error::<Test>::ExceedMaxAChange
		);
	})
}

#[test]
fn stop_ramp_meta_a_should_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();
		mine_block();

		let end_timestamp = Timestamp::now() / 1000 + 14 * DAYS + 100;
		assert_ok!(StableAmm::ramp_a(
			RawOrigin::Root.into(),
			meta_pool_id,
			100,
			end_timestamp.into()
		));

		mine_block_with_timestamp(Timestamp::now() / 1000 + 100000);

		let pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();
		assert_eq!(StableAmm::get_a_precise(&pool), Some(5413));

		assert_ok!(StableAmm::stop_ramp_a(RawOrigin::Root.into(), meta_pool_id));
		let pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();
		assert_eq!(StableAmm::get_a_precise(&pool), Some(5413));

		mine_block_with_timestamp(end_timestamp);
		let pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();
		assert_eq!(StableAmm::get_a_precise(&pool), Some(5413));
	})
}

#[test]
fn stop_ramp_meta_a_repeat_should_not_work() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();
		mine_block();

		let end_timestamp = Timestamp::now() / 1000 + 14 * DAYS + 100;
		assert_ok!(StableAmm::ramp_a(
			RawOrigin::Root.into(),
			meta_pool_id,
			100,
			end_timestamp.into()
		));

		mine_block_with_timestamp(Timestamp::now() / 1000 + 100000);

		let pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();
		assert_eq!(StableAmm::get_a_precise(&pool), Some(5413));

		assert_ok!(StableAmm::stop_ramp_a(RawOrigin::Root.into(), meta_pool_id));
		let pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();
		assert_eq!(StableAmm::get_a_precise(&pool), Some(5413));

		assert_noop!(
			StableAmm::stop_ramp_a(RawOrigin::Root.into(), meta_pool_id),
			Error::<Test>::AlreadyStoppedRampA
		);
	})
}

#[test]
fn meta_pool_check_maximum_differences_in_a_and_virtual_price_when_time_manipulations_and_increasing_a(
) {
	new_test_ext().execute_with(|| {
		mine_block();

		let (_, meta_pool_id) = setup_test_meta_pool();

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(ALICE).into(),
			meta_pool_id,
			vec![1e18 as Balance, 0],
			0,
			ALICE,
			u64::MAX,
		));

		let pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();
		assert_eq!(StableAmm::get_a_precise(&pool), Some(5000));
		assert_eq!(StableAmm::get_virtual_price(meta_pool_id), 1000167146429977312);

		let end_timestamp = Timestamp::now() / 1000 + 14 * DAYS + 100;
		assert_ok!(StableAmm::ramp_a(
			RawOrigin::Root.into(),
			meta_pool_id,
			100,
			end_timestamp.into()
		));

		// Malicious miner skips 900 seconds
		set_block_timestamp(Timestamp::now() / 1000 + 900);

		let pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();
		assert_eq!(StableAmm::get_a_precise(&pool), Some(5003));
		assert_eq!(StableAmm::get_virtual_price(meta_pool_id), 1000167862696363286);
	})
}

#[test]
fn meta_check_maximum_differences_in_a_and_virtual_price_when_time_manipulations_and_decreasing_a()
{
	new_test_ext().execute_with(|| {
		mine_block();

		let (_, meta_pool_id) = setup_test_meta_pool();

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(ALICE).into(),
			meta_pool_id,
			vec![1e18 as Balance, 0],
			0,
			ALICE,
			u64::MAX,
		));

		let pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();
		assert_eq!(StableAmm::get_a_precise(&pool), Some(5000));
		assert_eq!(StableAmm::get_virtual_price(meta_pool_id), 1000167146429977312);

		let end_timestamp = Timestamp::now() / 1000 + 14 * DAYS + 100;

		assert_ok!(StableAmm::ramp_a(
			RawOrigin::Root.into(),
			meta_pool_id,
			25,
			end_timestamp.into()
		));

		// Malicious miner skips 900 seconds
		set_block_timestamp(Timestamp::now() / 1000 + 900);

		let pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();
		assert_eq!(StableAmm::get_a_precise(&pool), Some(4999));
		assert_eq!(StableAmm::get_virtual_price(meta_pool_id), 1000166907487883089);
	})
}

struct AttackContext {
	pub initial_attacker_balances: Vec<Balance>,
	pub initial_pool_balances: Vec<Balance>,
	pub pool_currencies: Vec<CurrencyId>,
	pub attacker: AccountId,
	pub pool_id: PoolId,
}

fn prepare_attack_meta_context(new_a: Balance) -> AttackContext {
	mine_block();

	let (_, meta_pool_id) = setup_test_meta_pool();
	let attacker = BOB;
	let pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();

	let mut attack_balances = Vec::new();
	for currency_id in pool.currency_ids.iter() {
		attack_balances
			.push(<Test as Config>::MultiCurrency::free_balance(*currency_id, &attacker));
	}

	assert_ok!(StableAmm::ramp_a(
		RawOrigin::Root.into(),
		meta_pool_id,
		new_a,
		(Timestamp::now() / 1000 + 14 * DAYS).into()
	));

	assert_eq!(attack_balances[0], 1e20 as Balance);
	assert_eq!(attack_balances[1], 3e20 as Balance);

	assert_eq!(pool.balances[0], 1e18 as Balance);
	assert_eq!(pool.balances[1], 1e18 as Balance);

	AttackContext {
		initial_attacker_balances: attack_balances,
		initial_pool_balances: pool.balances.clone(),
		pool_currencies: pool.currency_ids.clone(),
		attacker,
		pool_id: meta_pool_id,
	}
}

#[test]
fn check_when_ramp_a_upwards_and_tokens_price_equally() {
	new_test_ext().execute_with(|| {
		let context = prepare_attack_meta_context(100);

		// Swap 1e18 of firstToken to secondToken, causing massive imbalance in the pool
		assert_ok!(StableAmm::swap(
			RawOrigin::Signed(context.attacker).into(),
			context.pool_id,
			0,
			1,
			1e18 as Balance,
			0,
			context.attacker,
			u64::MAX
		));
		let second_token_output = <Test as Config>::MultiCurrency::free_balance(
			context.pool_currencies[1],
			&context.attacker,
		) - context.initial_attacker_balances[1];

		assert_eq!(second_token_output, 908591742545002306);

		// Pool is imbalanced! Now trades from secondToken -> firstToken may be profitable in small
		// sizes
		let pool = StableAmm::pools(context.pool_id).unwrap().get_pool_info();
		assert_eq!(pool.balances[0], 2e18 as Balance);
		assert_eq!(pool.balances[1], 91408257454997694);

		// Malicious miner skips 900 seconds
		set_block_timestamp(Timestamp::now() / 1000 + 900);

		assert_eq!(StableAmm::get_a_precise(&pool), Some(5003));

		let balances_before = get_user_token_balances(&context.pool_currencies, &context.attacker);

		assert_ok!(StableAmm::swap(
			RawOrigin::Signed(context.attacker).into(),
			context.pool_id,
			1,
			0,
			second_token_output,
			0,
			context.attacker,
			u64::MAX
		));

		let first_token_output = <Test as Config>::MultiCurrency::free_balance(
			context.pool_currencies[0],
			&context.attacker,
		) - balances_before[0];
		assert_eq!(first_token_output, 997214696574405737);

		let final_attacker_balances =
			get_user_token_balances(&context.pool_currencies, &context.attacker);

		assert!(final_attacker_balances[0] < context.initial_attacker_balances[0]);
		assert_eq!(final_attacker_balances[1], context.initial_attacker_balances[1]);
		assert_eq!(
			context.initial_attacker_balances[0] - final_attacker_balances[0],
			2785303425594263
		);
		assert_eq!(context.initial_attacker_balances[1] - final_attacker_balances[1], 0);

		// checked pool balance,
		let pool = StableAmm::pools(context.pool_id).unwrap().get_pool_info();
		assert!(pool.balances[0] > context.initial_pool_balances[0]);
		assert_eq!(pool.balances[1], context.initial_pool_balances[1]);

		assert_eq!(pool.balances[0] - context.initial_pool_balances[0], 2785303425594263);
		assert_eq!(pool.balances[1] - context.initial_pool_balances[1], 0);
	})
}

#[test]
fn meta_check_when_ramp_a_upwards_and_tokens_price_unequally() {
	new_test_ext().execute_with(|| {
		let mut context = prepare_attack_meta_context(100);

		// Set up pool to be imbalanced prior to the attack
		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(ALICE).into(),
			context.pool_id,
			vec![0, 2e18 as Balance],
			0,
			ALICE,
			u64::MAX,
		));

		let pool = StableAmm::pools(context.pool_id).unwrap().get_pool_info();
		assert_eq!(pool.balances[0], 1e18 as Balance);
		assert_eq!(pool.balances[1], 3e18 as Balance);

		// rewrite pool balances
		context.initial_pool_balances = pool.balances.clone();

		// Swap 1e18 of firstToken to secondToken, resolving imbalance in the pool
		assert_ok!(StableAmm::swap(
			RawOrigin::Signed(context.attacker).into(),
			context.pool_id,
			0,
			1,
			1e18 as Balance,
			0,
			context.attacker,
			u64::MAX
		));
		let second_token_output = <Test as Config>::MultiCurrency::free_balance(
			context.pool_currencies[1],
			&context.attacker,
		) - context.initial_attacker_balances[1];

		assert_eq!(second_token_output, 1011933251060681353);

		// Pool is imbalanced! Now trades from secondToken -> firstToken may be profitable in small
		// sizes
		let pool = StableAmm::pools(context.pool_id).unwrap().get_pool_info();
		assert_eq!(pool.balances[0], 2e18 as Balance);
		assert_eq!(pool.balances[1], 1988066748939318647);

		// Malicious miner skips 900 seconds
		set_block_timestamp(Timestamp::now() / 1000 + 900);
		assert_eq!(StableAmm::get_a_precise(&pool), Some(5003));

		let balances_before = get_user_token_balances(&context.pool_currencies, &context.attacker);

		assert_ok!(StableAmm::swap(
			RawOrigin::Signed(context.attacker).into(),
			context.pool_id,
			1,
			0,
			second_token_output,
			0,
			context.attacker,
			u64::MAX
		));

		let first_token_output = <Test as Config>::MultiCurrency::free_balance(
			context.pool_currencies[0],
			&context.attacker,
		) - balances_before[0];
		assert_eq!(first_token_output, 998017518949630644);

		let final_attacker_balances =
			get_user_token_balances(&context.pool_currencies, &context.attacker);

		assert!(final_attacker_balances[0] < context.initial_attacker_balances[0]);
		assert_eq!(final_attacker_balances[1], context.initial_attacker_balances[1]);
		assert_eq!(
			context.initial_attacker_balances[0] - final_attacker_balances[0],
			1982481050369356
		);
		assert_eq!(context.initial_attacker_balances[1] - final_attacker_balances[1], 0);

		// checked pool balance,
		let pool = StableAmm::pools(context.pool_id).unwrap().get_pool_info();
		assert!(pool.balances[0] > context.initial_pool_balances[0]);
		assert_eq!(pool.balances[1], context.initial_pool_balances[1]);

		assert_eq!(pool.balances[0] - context.initial_pool_balances[0], 1982481050369356);
		assert_eq!(pool.balances[1] - context.initial_pool_balances[1], 0);
	})
}

#[test]
fn meta_check_when_ramp_a_downwards_and_tokens_price_equally() {
	new_test_ext().execute_with(|| {
		let context = prepare_attack_meta_context(25);
		// Swap 1e18 of firstToken to secondToken, causing massive imbalance in the pool
		assert_ok!(StableAmm::swap(
			RawOrigin::Signed(context.attacker).into(),
			context.pool_id,
			0,
			1,
			1e18 as Balance,
			0,
			context.attacker,
			u64::MAX
		));
		let second_token_output = <Test as Config>::MultiCurrency::free_balance(
			context.pool_currencies[1],
			&context.attacker,
		) - context.initial_attacker_balances[1];

		assert_eq!(second_token_output, 908591742545002306);

		// Pool is imbalanced! Now trades from secondToken -> firstToken may be profitable in small
		// sizes
		let pool = StableAmm::pools(context.pool_id).unwrap().get_pool_info();
		assert_eq!(pool.balances[0], 2e18 as Balance);
		assert_eq!(pool.balances[1], 91408257454997694);

		// Malicious miner skips 900 seconds
		set_block_timestamp(Timestamp::now() / 1000 + 900);

		assert_eq!(StableAmm::get_a_precise(&pool), Some(4999));

		let balances_before = get_user_token_balances(&context.pool_currencies, &context.attacker);

		assert_ok!(StableAmm::swap(
			RawOrigin::Signed(context.attacker).into(),
			context.pool_id,
			1,
			0,
			second_token_output,
			0,
			context.attacker,
			u64::MAX
		));

		let first_token_output = <Test as Config>::MultiCurrency::free_balance(
			context.pool_currencies[0],
			&context.attacker,
		) - balances_before[0];
		assert_eq!(first_token_output, 997276754500361021);

		let final_attacker_balances =
			get_user_token_balances(&context.pool_currencies, &context.attacker);

		assert!(final_attacker_balances[0] < context.initial_attacker_balances[0]);
		assert_eq!(final_attacker_balances[1], context.initial_attacker_balances[1]);
		assert_eq!(
			context.initial_attacker_balances[0] - final_attacker_balances[0],
			2723245499638979
		);
		assert_eq!(context.initial_attacker_balances[1] - final_attacker_balances[1], 0);

		// checked pool balance,
		let pool = StableAmm::pools(context.pool_id).unwrap().get_pool_info();
		assert!(pool.balances[0] > context.initial_pool_balances[0]);
		assert_eq!(pool.balances[1], context.initial_pool_balances[1]);

		assert_eq!(pool.balances[0] - context.initial_pool_balances[0], 2723245499638979);
		assert_eq!(pool.balances[1] - context.initial_pool_balances[1], 0);
	})
}

#[test]
fn meta_check_when_ramp_a_downwards_and_tokens_price_unequally() {
	new_test_ext().execute_with(|| {
		let mut context = prepare_attack_meta_context(25);

		// Set up pool to be imbalanced prior to the attack
		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(ALICE).into(),
			context.pool_id,
			vec![0, 2e18 as Balance],
			0,
			ALICE,
			u64::MAX,
		));

		let pool = StableAmm::pools(context.pool_id).unwrap().get_pool_info();
		assert_eq!(pool.balances[0], 1e18 as Balance);
		assert_eq!(pool.balances[1], 3e18 as Balance);

		// rewrite pool balances
		context.initial_pool_balances = pool.balances.clone();

		// Swap 1e18 of firstToken to secondToken, resolving imbalance in the pool
		assert_ok!(StableAmm::swap(
			RawOrigin::Signed(context.attacker).into(),
			context.pool_id,
			0,
			1,
			1e18 as Balance,
			0,
			context.attacker,
			u64::MAX
		));
		let second_token_output = <Test as Config>::MultiCurrency::free_balance(
			context.pool_currencies[1],
			&context.attacker,
		) - context.initial_attacker_balances[1];

		assert_eq!(second_token_output, 1011933251060681353);

		// Pool is imbalanced! Now trades from secondToken -> firstToken may be profitable in small
		// sizes
		let pool = StableAmm::pools(context.pool_id).unwrap().get_pool_info();
		assert_eq!(pool.balances[0], 2e18 as Balance);
		assert_eq!(pool.balances[1], 1988066748939318647);

		// Malicious miner skips 900 seconds
		set_block_timestamp(Timestamp::now() / 1000 + 900);
		assert_eq!(StableAmm::get_a_precise(&pool), Some(4999));

		let balances_before = get_user_token_balances(&context.pool_currencies, &context.attacker);

		assert_ok!(StableAmm::swap(
			RawOrigin::Signed(context.attacker).into(),
			context.pool_id,
			1,
			0,
			second_token_output,
			0,
			context.attacker,
			u64::MAX
		));

		let first_token_output = <Test as Config>::MultiCurrency::free_balance(
			context.pool_currencies[0],
			&context.attacker,
		) - balances_before[0];
		assert_eq!(first_token_output, 998007711333645455);

		let final_attacker_balances =
			get_user_token_balances(&context.pool_currencies, &context.attacker);

		assert!(final_attacker_balances[0] < context.initial_attacker_balances[0]);
		assert_eq!(final_attacker_balances[1], context.initial_attacker_balances[1]);
		assert_eq!(
			context.initial_attacker_balances[0] - final_attacker_balances[0],
			1992288666354545
		);
		assert_eq!(context.initial_attacker_balances[1] - final_attacker_balances[1], 0);

		// checked pool balance,
		let pool = StableAmm::pools(context.pool_id).unwrap().get_pool_info();
		assert!(pool.balances[0] > context.initial_pool_balances[0]);
		assert_eq!(pool.balances[1], context.initial_pool_balances[1]);

		assert_eq!(pool.balances[0] - context.initial_pool_balances[0], 1992288666354545);
		assert_eq!(pool.balances[1] - context.initial_pool_balances[1], 0);
	})
}

#[test]
fn meta_check_arithmetic_in_add_liquidity_should_successfully() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();

		let pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();

		mint_more_currencies(
			vec![BOB, CHARLIE],
			pool.currency_ids,
			vec![1_000_000_000e18 as Balance, 1_000_000_000e18 as Balance],
		);

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![100000000000000000000000, 200000000000000000000000], // [100_000e18, 200_00018]
			0,
			BOB,
			u64::MAX,
		));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(CHARLIE).into(),
			meta_pool_id,
			vec![100000000000000000000000000, 300000000000000000000000000], /* [100_000_000e18,
			                                                                 * 300_000_000e18] */
			0,
			CHARLIE,
			u64::MAX,
		));

		assert_eq!(
			<Test as Config>::MultiCurrency::free_balance(pool.lp_currency_id, &ALICE),
			2e18 as Balance
		);
		assert_eq!(
			<Test as Config>::MultiCurrency::free_balance(pool.lp_currency_id, &BOB),
			299583613596961209609933
		);
		assert_eq!(
			<Test as Config>::MultiCurrency::free_balance(pool.lp_currency_id, &CHARLIE),
			398605324970970482408685465
		);

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![300000000000000000000000000, 100000000000000000000000000], /* [300_000_000e18,
			                                                                 * 100_000_000e18] */
			0,
			BOB,
			u64::MAX,
		));

		assert_eq!(
			<Test as Config>::MultiCurrency::free_balance(pool.lp_currency_id, &BOB),
			401292897411939364910247311
		);
	})
}

#[test]
fn meta_check_arithmetic_in_remove_liquidity_should_successfully() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();
		let pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();

		mint_more_currencies(
			vec![BOB, CHARLIE],
			pool.currency_ids.clone(),
			vec![10_000_000_000e18 as Balance, 10_000_000_000e18 as Balance],
		);

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![100000000000000000000000, 200000000000000000000000], // [100_000e18, 200_00018]
			0,
			BOB,
			u64::MAX,
		));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(CHARLIE).into(),
			meta_pool_id,
			vec![100000000000000000000000000, 300000000000000000000000000], /* [100_000_000e18,
			                                                                 * 300_000_000e18] */
			0,
			CHARLIE,
			u64::MAX,
		));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![300000000000000000000000000, 100000000000000000000000000], /* [300_000_000e18,
			                                                                 * 100_000_000e18] */
			0,
			BOB,
			u64::MAX,
		));

		let user1_pool_lp_balance_before =
			<Test as Config>::MultiCurrency::free_balance(pool.lp_currency_id, &BOB);
		let user1_token0_balance_before =
			<Test as Config>::MultiCurrency::free_balance(pool.currency_ids[0], &BOB);
		let user1_token1_balance_before =
			<Test as Config>::MultiCurrency::free_balance(pool.currency_ids[1], &BOB);

		assert_ok!(StableAmm::remove_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			user1_pool_lp_balance_before,
			vec![0, 0],
			BOB,
			u64::MAX
		));

		let user1_pool_lp_balance_after =
			<Test as Config>::MultiCurrency::free_balance(pool.lp_currency_id, &BOB);
		let user1_token0_balance_after =
			<Test as Config>::MultiCurrency::free_balance(pool.currency_ids[0], &BOB);
		let user1_token1_balance_after =
			<Test as Config>::MultiCurrency::free_balance(pool.currency_ids[1], &BOB);

		assert_eq!(user1_pool_lp_balance_after, 0);
		assert_eq!(
			user1_token0_balance_after - user1_token0_balance_before,
			200722146595179027183639390
		);
		assert_eq!(
			user1_token1_balance_after - user1_token1_balance_before,
			200772314589703770768227294
		);

		// user2 remove liquidity
		let user2_pool_lp_balance_before =
			<Test as Config>::MultiCurrency::free_balance(pool.lp_currency_id, &CHARLIE);
		let user2_token0_balance_before =
			<Test as Config>::MultiCurrency::free_balance(pool.currency_ids[0], &CHARLIE);
		let user2_token1_balance_before =
			<Test as Config>::MultiCurrency::free_balance(pool.currency_ids[1], &CHARLIE);

		assert_ok!(StableAmm::remove_liquidity(
			RawOrigin::Signed(CHARLIE).into(),
			meta_pool_id,
			user2_pool_lp_balance_before,
			vec![0, 0],
			CHARLIE,
			u64::MAX
		));

		let user2_pool_lp_balance_after =
			<Test as Config>::MultiCurrency::free_balance(pool.lp_currency_id, &CHARLIE);
		let user2_token0_balance_after =
			<Test as Config>::MultiCurrency::free_balance(pool.currency_ids[0], &CHARLIE);
		let user2_token1_balance_after =
			<Test as Config>::MultiCurrency::free_balance(pool.currency_ids[1], &CHARLIE);

		assert_eq!(user2_pool_lp_balance_after, 0);
		assert_eq!(
			user2_token0_balance_after - user2_token0_balance_before,
			199377853404443702798886871
		);
		assert_eq!(
			user2_token1_balance_after - user2_token1_balance_before,
			199427685409668927405371910
		);
	})
}

#[test]
fn meta_check_arithmetic_in_remove_liquidity_one_currency_should_successfully() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();
		let pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();
		mint_more_currencies(
			vec![BOB, CHARLIE],
			pool.currency_ids.clone(),
			vec![10_000_000_000e18 as Balance, 10_000_000_000e18 as Balance],
		);

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![100000000000000000000000, 200000000000000000000000], // [100_000e18, 200_00018]
			0,
			BOB,
			u64::MAX,
		));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(CHARLIE).into(),
			meta_pool_id,
			vec![100000000000000000000000000, 300000000000000000000000000], /* [100_000_000e18,
			                                                                 * 300_000_000e18] */
			0,
			CHARLIE,
			u64::MAX,
		));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![300000000000000000000000000, 100000000000000000000000000], /* [300_000_000e18,
			                                                                 * 100_000_000e18] */
			0,
			BOB,
			u64::MAX,
		));

		let user1_pool_lp_balance_before =
			<Test as Config>::MultiCurrency::free_balance(pool.lp_currency_id, &BOB);
		let user1_token0_balance_before =
			<Test as Config>::MultiCurrency::free_balance(pool.currency_ids[0], &BOB);
		let user1_token1_balance_before =
			<Test as Config>::MultiCurrency::free_balance(pool.currency_ids[1], &BOB);

		let user2_pool_lp_balance_before =
			<Test as Config>::MultiCurrency::free_balance(pool.lp_currency_id, &CHARLIE);
		let user2_token0_balance_before =
			<Test as Config>::MultiCurrency::free_balance(pool.currency_ids[0], &CHARLIE);
		let user2_token1_balance_before =
			<Test as Config>::MultiCurrency::free_balance(pool.currency_ids[1], &CHARLIE);

		assert_ok!(StableAmm::remove_liquidity_one_currency(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			user1_pool_lp_balance_before,
			0,
			0,
			BOB,
			u64::MAX
		));

		let user1_pool_lp_balance_after =
			<Test as Config>::MultiCurrency::free_balance(pool.lp_currency_id, &BOB);
		let user1_token0_balance_after =
			<Test as Config>::MultiCurrency::free_balance(pool.currency_ids[0], &BOB);
		let user1_token1_balance_after =
			<Test as Config>::MultiCurrency::free_balance(pool.currency_ids[1], &BOB);

		assert_eq!(user1_pool_lp_balance_after, 0);
		assert_eq!(
			user1_token0_balance_after - user1_token0_balance_before,
			382567648485687465509067831
		);
		assert_eq!(user1_token1_balance_after - user1_token1_balance_before, 0);

		assert_ok!(StableAmm::remove_liquidity_one_currency(
			RawOrigin::Signed(CHARLIE).into(),
			meta_pool_id,
			user2_pool_lp_balance_before,
			0,
			0,
			CHARLIE,
			u64::MAX
		));

		let user2_pool_lp_balance_after =
			<Test as Config>::MultiCurrency::free_balance(pool.lp_currency_id, &CHARLIE);
		let user2_token0_balance_after =
			<Test as Config>::MultiCurrency::free_balance(pool.currency_ids[0], &CHARLIE);
		let user2_token1_balance_after =
			<Test as Config>::MultiCurrency::free_balance(pool.currency_ids[1], &CHARLIE);

		assert_eq!(user2_pool_lp_balance_after, 0);
		assert_eq!(
			user2_token0_balance_after - user2_token0_balance_before,
			17532352514268550250562021
		);
		assert_eq!(user2_token1_balance_after - user2_token1_balance_before, 0);
	})
}

#[test]
fn meta_check_arithmetic_in_remove_liquidity_imbalance_should_successfully() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();
		let pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();
		mint_more_currencies(
			vec![BOB, CHARLIE],
			pool.currency_ids.clone(),
			vec![10_000_000_000e18 as Balance, 10_000_000_000e18 as Balance],
		);

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![100000000000000000000000, 200000000000000000000000], // [100_000e18, 200_00018]
			0,
			BOB,
			u64::MAX,
		));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(CHARLIE).into(),
			meta_pool_id,
			vec![100000000000000000000000000, 300000000000000000000000000], /* [100_000_000e18,
			                                                                 * 300_000_000e18] */
			0,
			CHARLIE,
			u64::MAX,
		));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![300000000000000000000000000, 100000000000000000000000000], /* [300_000_000e18,
			                                                                 * 100_000_000e18] */
			0,
			BOB,
			u64::MAX,
		));

		let user1_pool_lp_balance_before =
			<Test as Config>::MultiCurrency::free_balance(pool.lp_currency_id, &BOB);
		let user1_token0_balance_before =
			<Test as Config>::MultiCurrency::free_balance(pool.currency_ids[0], &BOB);
		let user1_token1_balance_before =
			<Test as Config>::MultiCurrency::free_balance(pool.currency_ids[1], &BOB);

		let user2_pool_lp_balance_before =
			<Test as Config>::MultiCurrency::free_balance(pool.lp_currency_id, &CHARLIE);
		let user2_token0_balance_before =
			<Test as Config>::MultiCurrency::free_balance(pool.currency_ids[0], &CHARLIE);
		let user2_token1_balance_before =
			<Test as Config>::MultiCurrency::free_balance(pool.currency_ids[1], &CHARLIE);

		assert_ok!(StableAmm::remove_liquidity_imbalance(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![300000000000000000000000000, 100000000000000000000000000],
			user1_pool_lp_balance_before,
			BOB,
			u64::MAX
		));

		let user1_pool_lp_balance_after =
			<Test as Config>::MultiCurrency::free_balance(pool.lp_currency_id, &BOB);
		let user1_token0_balance_after =
			<Test as Config>::MultiCurrency::free_balance(pool.currency_ids[0], &BOB);
		let user1_token1_balance_after =
			<Test as Config>::MultiCurrency::free_balance(pool.currency_ids[1], &BOB);

		assert_eq!(
			user1_pool_lp_balance_before - user1_pool_lp_balance_after,
			401193808332107345545123456
		);
		assert_eq!(
			user1_token0_balance_after - user1_token0_balance_before,
			300000000000000000000000000
		);
		assert_eq!(
			user1_token1_balance_after - user1_token1_balance_before,
			100000000000000000000000000
		);

		assert_ok!(StableAmm::remove_liquidity_imbalance(
			RawOrigin::Signed(CHARLIE).into(),
			meta_pool_id,
			vec![100000000000000000000000000, 300000000000000000000000000],
			user2_pool_lp_balance_before,
			CHARLIE,
			u64::MAX
		));

		let user2_pool_lp_balance_after =
			<Test as Config>::MultiCurrency::free_balance(pool.lp_currency_id, &CHARLIE);
		let user2_token0_balance_after =
			<Test as Config>::MultiCurrency::free_balance(pool.currency_ids[0], &CHARLIE);
		let user2_token1_balance_after =
			<Test as Config>::MultiCurrency::free_balance(pool.currency_ids[1], &CHARLIE);

		assert_eq!(user2_pool_lp_balance_after, 200293563918551832434667);
		assert_eq!(
			user2_token0_balance_after - user2_token0_balance_before,
			100000000000000000000000000
		);
		assert_eq!(
			user2_token1_balance_after - user2_token1_balance_before,
			300000000000000000000000000
		);
	})
}

#[test]
fn meta_check_arithmetic_in_swap_should_successfully() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();
		let pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();
		mint_more_currencies(
			vec![BOB, CHARLIE],
			pool.currency_ids.clone(),
			vec![10_000_000_000e18 as Balance, 10_000_000_000e18 as Balance],
		);

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![100000000000000000000000, 200000000000000000000000], // [100_000e18, 200_00018]
			0,
			BOB,
			u64::MAX,
		));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(CHARLIE).into(),
			meta_pool_id,
			vec![100000000000000000000000000, 300000000000000000000000000], /* [100_000_000e18,
			                                                                 * 300_000_000e18] */
			0,
			CHARLIE,
			u64::MAX,
		));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![300000000000000000000000000, 100000000000000000000000000], /* [300_000_000e18,
			                                                                 * 100_000_000e18] */
			0,
			BOB,
			u64::MAX,
		));

		let user1_pool_lp_balance_before =
			<Test as Config>::MultiCurrency::free_balance(pool.lp_currency_id, &BOB);
		let user1_token0_balance_before =
			<Test as Config>::MultiCurrency::free_balance(pool.currency_ids[0], &BOB);
		let user1_token1_balance_before =
			<Test as Config>::MultiCurrency::free_balance(pool.currency_ids[1], &BOB);

		let user2_pool_lp_balance_before =
			<Test as Config>::MultiCurrency::free_balance(pool.lp_currency_id, &CHARLIE);
		let user2_token0_balance_before =
			<Test as Config>::MultiCurrency::free_balance(pool.currency_ids[0], &CHARLIE);
		let user2_token1_balance_before =
			<Test as Config>::MultiCurrency::free_balance(pool.currency_ids[1], &CHARLIE);

		assert_ok!(StableAmm::swap(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			0,
			1,
			100000000000000000000000000,
			0,
			BOB,
			u64::MAX
		));

		let user1_pool_lp_balance_after =
			<Test as Config>::MultiCurrency::free_balance(pool.lp_currency_id, &BOB);
		let user1_token0_balance_after =
			<Test as Config>::MultiCurrency::free_balance(pool.currency_ids[0], &BOB);
		let user1_token1_balance_after =
			<Test as Config>::MultiCurrency::free_balance(pool.currency_ids[1], &BOB);

		assert_eq!(user1_pool_lp_balance_before, user1_pool_lp_balance_after);
		assert_eq!(
			user1_token0_balance_before - user1_token0_balance_after,
			100000000000000000000000000
		);
		assert_eq!(
			user1_token1_balance_after - user1_token1_balance_before,
			99382677941655828590709465
		);

		assert_ok!(StableAmm::swap(
			RawOrigin::Signed(CHARLIE).into(),
			meta_pool_id,
			1,
			0,
			100000000000000000000000000,
			0,
			CHARLIE,
			u64::MAX
		));

		let user2_pool_lp_balance_after =
			<Test as Config>::MultiCurrency::free_balance(pool.lp_currency_id, &CHARLIE);
		let user2_token0_balance_after =
			<Test as Config>::MultiCurrency::free_balance(pool.currency_ids[0], &CHARLIE);
		let user2_token1_balance_after =
			<Test as Config>::MultiCurrency::free_balance(pool.currency_ids[1], &CHARLIE);

		assert_eq!(user2_pool_lp_balance_after, user2_pool_lp_balance_before);
		assert_eq!(
			user2_token0_balance_after - user2_token0_balance_before,
			100416682007269587274140452
		);
		assert_eq!(
			user2_token1_balance_before - user2_token1_balance_after,
			100000000000000000000000000
		);

		let pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();

		// check pool balances
		assert_eq!(pool.balances[0], 399683318992730412725859548);
		assert_eq!(pool.balances[1], 400817323058344171409290535);

		let pool_token0_balance =
			<Test as Config>::MultiCurrency::free_balance(pool.currency_ids[0], &pool.account);
		let pool_token1_balance =
			<Test as Config>::MultiCurrency::free_balance(pool.currency_ids[1], &pool.account);
		assert_eq!(pool.balances[0], pool_token0_balance);
		assert_eq!(pool.balances[1], pool_token1_balance);
	})
}

#[test]
fn meta_check_arithmetic_in_add_liquidity_with_admin_fee_should_successfully() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();
		let pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();
		mint_more_currencies(
			vec![BOB, CHARLIE],
			pool.currency_ids.clone(),
			vec![10_000_000_000e18 as Balance, 10_000_000_000e18 as Balance],
		);

		assert_ok!(StableAmm::set_swap_fee(RawOrigin::Root.into(), meta_pool_id, SWAP_FEE,));
		assert_ok!(StableAmm::set_admin_fee(RawOrigin::Root.into(), meta_pool_id, MAX_ADMIN_FEE,));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![100000000000000000000000, 200000000000000000000000], // [100_000e18, 200_00018]
			0,
			BOB,
			u64::MAX,
		));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(CHARLIE).into(),
			meta_pool_id,
			vec![100000000000000000000000000, 300000000000000000000000000], /* [100_000_000e18,
			                                                                 * 300_000_000e18] */
			0,
			CHARLIE,
			u64::MAX,
		));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![300000000000000000000000000, 100000000000000000000000000], /* [300_000_000e18,
			                                                                 * 100_000_000e18] */
			0,
			BOB,
			u64::MAX,
		));

		let admin_balances = StableAmm::get_admin_balances(meta_pool_id);
		assert_eq!(admin_balances[0], 116218703966498771606127);
		assert_eq!(admin_balances[1], 117921007525488838747514);
	})
}

#[test]
fn meta_check_arithmetic_in_remove_liquidity_with_admin_fee_should_successfully() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();
		let pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();
		mint_more_currencies(
			vec![BOB, CHARLIE],
			pool.currency_ids.clone(),
			vec![10_000_000_000e18 as Balance, 10_000_000_000e18 as Balance],
		);

		assert_ok!(StableAmm::set_swap_fee(RawOrigin::Root.into(), meta_pool_id, SWAP_FEE,));
		assert_ok!(StableAmm::set_admin_fee(RawOrigin::Root.into(), meta_pool_id, MAX_ADMIN_FEE,));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![100000000000000000000000, 200000000000000000000000], // [100_000e18, 200_00018]
			0,
			BOB,
			u64::MAX,
		));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(CHARLIE).into(),
			meta_pool_id,
			vec![100000000000000000000000000, 300000000000000000000000000], /* [100_000_000e18,
			                                                                 * 300_000_000e18] */
			0,
			CHARLIE,
			u64::MAX,
		));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![300000000000000000000000000, 100000000000000000000000000], /* [300_000_000e18,
			                                                                 * 100_000_000e18] */
			0,
			BOB,
			u64::MAX,
		));

		let user1_pool_lp_balance_before =
			<Test as Config>::MultiCurrency::free_balance(pool.lp_currency_id, &BOB);

		assert_ok!(StableAmm::remove_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			user1_pool_lp_balance_before,
			vec![0, 0],
			BOB,
			u64::MAX
		));
		// user2 remove liquidity
		let user2_pool_lp_balance_before =
			<Test as Config>::MultiCurrency::free_balance(pool.lp_currency_id, &CHARLIE);

		assert_ok!(StableAmm::remove_liquidity(
			RawOrigin::Signed(CHARLIE).into(),
			meta_pool_id,
			user2_pool_lp_balance_before,
			vec![0, 0],
			CHARLIE,
			u64::MAX
		));

		let admin_balances = StableAmm::get_admin_balances(meta_pool_id);
		assert_eq!(admin_balances[0], 116218703966498771606127);
		assert_eq!(admin_balances[1], 117921007525488838747514);
	})
}

#[test]
fn meta_check_arithmetic_in_remove_liquidity_one_currency_with_admin_fee_should_successfully() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();
		let pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();
		mint_more_currencies(
			vec![BOB, CHARLIE],
			pool.currency_ids.clone(),
			vec![10_000_000_000e18 as Balance, 10_000_000_000e18 as Balance],
		);

		assert_ok!(StableAmm::set_swap_fee(RawOrigin::Root.into(), meta_pool_id, SWAP_FEE,));
		assert_ok!(StableAmm::set_admin_fee(RawOrigin::Root.into(), meta_pool_id, MAX_ADMIN_FEE,));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![100000000000000000000000, 200000000000000000000000], // [100_000e18, 200_00018]
			0,
			BOB,
			u64::MAX,
		));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(CHARLIE).into(),
			meta_pool_id,
			vec![100000000000000000000000000, 300000000000000000000000000], /* [100_000_000e18,
			                                                                 * 300_000_000e18] */
			0,
			CHARLIE,
			u64::MAX,
		));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![300000000000000000000000000, 100000000000000000000000000], /* [300_000_000e18,
			                                                                 * 100_000_000e18] */
			0,
			BOB,
			u64::MAX,
		));

		let user1_pool_lp_balance_before =
			<Test as Config>::MultiCurrency::free_balance(pool.lp_currency_id, &BOB);
		let user2_pool_lp_balance_before =
			<Test as Config>::MultiCurrency::free_balance(pool.lp_currency_id, &CHARLIE);

		assert_ok!(StableAmm::remove_liquidity_one_currency(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			user1_pool_lp_balance_before,
			0,
			0,
			BOB,
			u64::MAX
		));

		assert_ok!(StableAmm::remove_liquidity_one_currency(
			RawOrigin::Signed(CHARLIE).into(),
			meta_pool_id,
			user2_pool_lp_balance_before,
			1,
			0,
			CHARLIE,
			u64::MAX
		));

		let admin_balances = StableAmm::get_admin_balances(meta_pool_id);
		assert_eq!(admin_balances[0], 253156563645258671123072);
		assert_eq!(admin_balances[1], 117921008529025874896694);

		let balances = StableAMM::pools(meta_pool_id).unwrap().get_pool_info().balances;
		assert_eq!(balances[0], 17382152193711710630633607);
		assert_eq!(balances[1], 66);
	})
}

#[test]
fn meta_check_arithmetic_in_remove_liquidity_imbalance_with_admin_fee_should_successfully() {
	new_test_ext().execute_with(|| {
		let (_, meta_pool_id) = setup_test_meta_pool();
		let pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();
		mint_more_currencies(
			vec![BOB, CHARLIE],
			pool.currency_ids.clone(),
			vec![10_000_000_000e18 as Balance, 10_000_000_000e18 as Balance],
		);

		assert_ok!(StableAmm::set_swap_fee(RawOrigin::Root.into(), meta_pool_id, SWAP_FEE,));
		assert_ok!(StableAmm::set_admin_fee(RawOrigin::Root.into(), meta_pool_id, MAX_ADMIN_FEE,));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![100000000000000000000000, 200000000000000000000000], // [100_000e18, 200_00018]
			0,
			BOB,
			u64::MAX,
		));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(CHARLIE).into(),
			meta_pool_id,
			vec![100000000000000000000000000, 300000000000000000000000000], /* [100_000_000e18,
			                                                                 * 300_000_000e18] */
			0,
			CHARLIE,
			u64::MAX,
		));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![300000000000000000000000000, 100000000000000000000000000], /* [300_000_000e18,
			                                                                 * 100_000_000e18] */
			0,
			BOB,
			u64::MAX,
		));

		let user1_pool_lp_balance_before =
			<Test as Config>::MultiCurrency::free_balance(pool.lp_currency_id, &BOB);
		let user2_pool_lp_balance_before =
			<Test as Config>::MultiCurrency::free_balance(pool.lp_currency_id, &CHARLIE);

		assert_ok!(StableAmm::remove_liquidity_imbalance(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![200000000000000000000000000, 100000000000000000000000000],
			user1_pool_lp_balance_before,
			BOB,
			u64::MAX
		));

		assert_ok!(StableAmm::remove_liquidity_imbalance(
			RawOrigin::Signed(CHARLIE).into(),
			meta_pool_id,
			vec![100000000000000000000000000, 200000000000000000000000000],
			user2_pool_lp_balance_before,
			CHARLIE,
			u64::MAX
		));

		let admin_balances = StableAmm::get_admin_balances(meta_pool_id);
		assert_eq!(admin_balances[0], 151146217664745609762144);
		assert_eq!(admin_balances[1], 152991616465138594784072);

		let balances = StableAMM::pools(meta_pool_id).unwrap().get_pool_info().balances;
		assert_eq!(balances[0], 99948854782335254390237856);
		assert_eq!(balances[1], 100047009383534861405215928);
	})
}

#[test]
fn meta_check_arithmetic_in_swap_imbalance_with_admin_fee_should_successfully() {
	new_test_ext().execute_with(|| {
		let (meta_pool_id, _) = setup_test_base_pool();
		let pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();
		mint_more_currencies(
			vec![BOB, CHARLIE],
			pool.currency_ids.clone(),
			vec![10_000_000_000e18 as Balance, 10_000_000_000e18 as Balance],
		);

		assert_ok!(StableAmm::set_swap_fee(RawOrigin::Root.into(), meta_pool_id, SWAP_FEE,));
		assert_ok!(StableAmm::set_admin_fee(RawOrigin::Root.into(), meta_pool_id, MAX_ADMIN_FEE,));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![100000000000000000000000, 200000000000000000000000], // [100_000e18, 200_00018]
			0,
			BOB,
			u64::MAX,
		));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(CHARLIE).into(),
			meta_pool_id,
			vec![100000000000000000000000000, 300000000000000000000000000], /* [100_000_000e18,
			                                                                 * 300_000_000e18] */
			0,
			CHARLIE,
			u64::MAX,
		));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![300000000000000000000000000, 100000000000000000000000000], /* [300_000_000e18,
			                                                                 * 100_000_000e18] */
			0,
			BOB,
			u64::MAX,
		));

		assert_ok!(StableAmm::swap(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			0,
			1,
			100000000000000000000000000,
			0,
			BOB,
			u64::MAX
		));

		assert_ok!(StableAmm::swap(
			RawOrigin::Signed(CHARLIE).into(),
			meta_pool_id,
			1,
			0,
			100000000000000000000000000,
			0,
			CHARLIE,
			u64::MAX
		));

		let admin_balances = StableAmm::get_admin_balances(meta_pool_id);
		assert_eq!(admin_balances[0], 216736707365806476948490);
		assert_eq!(admin_balances[1], 217402988477687128132247);

		let balances = StableAMM::pools(meta_pool_id).unwrap().get_pool_info().balances;
		assert_eq!(balances[0], 399465778896725795886030548);
		assert_eq!(balances[1], 400600099040276221776518758);
	})
}

#[test]
fn add_pool_and_base_pool_liquidity_should_work() {
	new_test_ext().execute_with(|| {
		let (basic_pool_id, meta_pool_id) = setup_test_meta_pool();

		// remove all initial lp balance.
		assert_ok!(StableAmm::remove_liquidity(
			RawOrigin::Signed(ALICE).into(),
			meta_pool_id,
			2e18 as Balance,
			vec![0, 0],
			ALICE,
			u64::MAX,
		));

		let expected_mint_amount = StableAmm::calculate_currency_amount(
			meta_pool_id,
			vec![1e18 as Balance, 3e18 as Balance],
			true,
		)
		.unwrap();

		let pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();

		assert_ok!(StableAmm::add_pool_and_base_pool_liquidity(
			RawOrigin::Signed(ALICE).into(),
			meta_pool_id,
			basic_pool_id,
			vec![1e18 as Balance, 0],
			vec![1e18 as Balance, 1e6 as Balance, 1e6 as Balance],
			0,
			CHARLIE,
			u64::MAX
		));

		let lp_amount = get_user_balance(pool.lp_currency_id, &CHARLIE);

		assert_eq!(lp_amount, expected_mint_amount);
		assert_eq!(lp_amount, 3987053390609794133);
	})
}

#[test]
fn remove_pool_and_base_pool_liquidity_should_work() {
	new_test_ext().execute_with(|| {
		create_mock_base_pool();

		let base_pool_id = 0;
		let base_pool = StableAmm::pools(base_pool_id).unwrap().get_pool_info();

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			base_pool_id,
			vec![1e18 as Balance, 1e6 as Balance, 1e6 as Balance],
			0,
			BOB,
			u64::MAX
		));

		create_mock_meta_pool();
		let meta_pool_id = 1;
		let meta_pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![1e18 as Balance, 1e18 as Balance],
			0,
			BOB,
			u64::MAX
		));

		assert_ok!(StableAmm::remove_pool_and_base_pool_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			base_pool_id,
			1e18 as Balance,
			vec![0, 0],
			vec![0, 0, 0],
			BOB,
			u64::MAX
		));

		let balances_after = get_user_token_balances(
			&vec![
				base_pool.currency_ids[0],
				base_pool.currency_ids[1],
				base_pool.currency_ids[2],
				meta_pool.currency_ids[0],
				base_pool.lp_currency_id,
				meta_pool.lp_currency_id,
			],
			&BOB,
		);

		assert_eq!(balances_after[0], 99166666666666666666);
		assert_eq!(balances_after[1], 99166666);
		assert_eq!(balances_after[2], 99166666);
		assert_eq!(balances_after[3], 99500000000000000000);
		assert_eq!(balances_after[4], 2000000000000000000);
		assert_eq!(balances_after[5], 1000000000000000000);
	})
}

#[test]
fn remove_pool_and_base_pool_liquidity_one_currency_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(StableAmm::create_base_pool(
			RawOrigin::Root.into(),
			vec![Token(TOKEN1_SYMBOL), Token(TOKEN2_SYMBOL), Token(TOKEN3_SYMBOL)],
			vec![TOKEN1_DECIMAL, TOKEN2_DECIMAL, TOKEN3_DECIMAL],
			50,
			1e7 as Number,
			0,
			ALICE,
			Vec::from("base_pool_lp"),
		));

		let base_pool_id = 0;

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			base_pool_id,
			vec![1e18 as Balance, 1e18 as Balance, 1e6 as Balance],
			0,
			BOB,
			u64::MAX
		));

		let base_pool = StableAmm::pools(0).unwrap().get_pool_info();
		let base_pool_lp_currency = base_pool.lp_currency_id;

		assert_ok!(StableAmm::create_meta_pool(
			RawOrigin::Root.into(),
			vec![Token(TOKEN4_SYMBOL), base_pool_lp_currency],
			vec![TOKEN4_DECIMAL, STABLE_LP_DECIMAL],
			50,
			1e7 as Number,
			0,
			ALICE,
			Vec::from("meta_pool_lp"),
		));

		let meta_pool_id = 1;
		let meta_pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![1e6 as Balance, 1e18 as Balance],
			0,
			BOB,
			u64::MAX
		));

		assert_ok!(StableAmm::remove_pool_and_base_pool_liquidity_one_currency(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			base_pool_id,
			1e18 as Balance,
			0,
			0,
			BOB,
			u64::MAX
		));

		let balances_after = get_user_token_balances(
			&vec![
				base_pool.currency_ids[0],
				base_pool.currency_ids[1],
				base_pool.currency_ids[2],
				meta_pool.currency_ids[0],
				base_pool.lp_currency_id,
				meta_pool.lp_currency_id,
			],
			&BOB,
		);

		assert_eq!(balances_after[0], 99915975025371929634);
		assert_eq!(balances_after[1], 99000000000000000000);
		assert_eq!(balances_after[2], 99000000);
		assert_eq!(balances_after[3], 99000000);
		assert_eq!(balances_after[4], 2000000000000000000);
		assert_eq!(balances_after[5], 1000000000000000000);
	})
}

#[test]
fn swap_pool_from_base_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(StableAmm::create_base_pool(
			RawOrigin::Root.into(),
			vec![Token(TOKEN1_SYMBOL), Token(TOKEN2_SYMBOL), Token(TOKEN3_SYMBOL)],
			vec![TOKEN1_DECIMAL, TOKEN2_DECIMAL, TOKEN3_DECIMAL],
			50,
			1e7 as Number,
			0,
			ALICE,
			Vec::from("base_pool_lp"),
		));

		let base_pool_id = 0;
		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			base_pool_id,
			vec![1e18 as Balance, 1e18 as Balance, 1e6 as Balance],
			0,
			BOB,
			u64::MAX
		));

		let base_pool = StableAmm::pools(0).unwrap().get_pool_info();
		let base_pool_lp_currency = base_pool.lp_currency_id;

		assert_ok!(StableAmm::create_meta_pool(
			RawOrigin::Root.into(),
			vec![Token(TOKEN4_SYMBOL), base_pool_lp_currency],
			vec![TOKEN4_DECIMAL, STABLE_LP_DECIMAL],
			50,
			1e7 as Number,
			0,
			ALICE,
			Vec::from("meta_pool_lp"),
		));

		let meta_pool_id = 1;
		let meta_pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			vec![1e6 as Balance, 1e18 as Balance],
			0,
			BOB,
			u64::MAX
		));

		assert_ok!(StableAmm::swap_pool_from_base(
			RawOrigin::Signed(BOB).into(),
			meta_pool_id,
			base_pool_id,
			0,
			0,
			1e16 as Balance,
			0,
			BOB,
			u64::MAX
		));

		let balances_after = get_user_token_balances(
			&vec![
				Token(TOKEN1_SYMBOL),
				Token(TOKEN2_SYMBOL),
				Token(TOKEN3_SYMBOL),
				Token(TOKEN4_SYMBOL),
				base_pool.lp_currency_id,
				meta_pool.lp_currency_id,
			],
			&BOB,
		);

		assert_eq!(balances_after[0], 98990000000000000000);
		assert_eq!(balances_after[1], 99000000000000000000);
		assert_eq!(balances_after[2], 99000000);
		assert_eq!(balances_after[3], 99009982);
		assert_eq!(balances_after[4], 2000000000000000000);
		assert_eq!(balances_after[5], 2000000000000000000);
	})
}

#[test]
fn swap_pool_to_base_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(StableAmm::create_base_pool(
			RawOrigin::Root.into(),
			vec![Token(TOKEN1_SYMBOL), Token(TOKEN2_SYMBOL), Token(TOKEN3_SYMBOL)],
			vec![TOKEN1_DECIMAL, TOKEN2_DECIMAL, TOKEN3_DECIMAL],
			50,
			1e7 as Number,
			0,
			ALICE,
			Vec::from("base_pool_lp"),
		));

		let base_pool_id = 0;
		let base_pool = StableAmm::pools(base_pool_id).unwrap().get_pool_info();

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			base_pool_id,
			vec![1e18 as Balance, 1e18 as Balance, 1e6 as Balance],
			0,
			BOB,
			u64::MAX
		));

		assert_ok!(StableAmm::create_meta_pool(
			RawOrigin::Root.into(),
			vec![Token(TOKEN4_SYMBOL), base_pool.lp_currency_id],
			vec![TOKEN4_DECIMAL, STABLE_LP_DECIMAL],
			50,
			1e7 as Number,
			0,
			ALICE,
			Vec::from("meta_pool_lp"),
		));

		let meta_poo_id = 1;
		let meta_pool = StableAmm::pools(meta_poo_id).unwrap().get_pool_info();

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			meta_poo_id,
			vec![1e6 as Balance, 1e18 as Balance],
			0,
			BOB,
			u64::MAX
		));

		assert_ok!(StableAmm::swap_pool_to_base(
			RawOrigin::Signed(BOB).into(),
			meta_poo_id,
			base_pool_id,
			0,
			0,
			1e6 as Balance,
			0,
			BOB,
			u64::MAX
		));

		let balances_after = get_user_token_balances(
			&vec![
				Token(TOKEN1_SYMBOL),
				Token(TOKEN2_SYMBOL),
				Token(TOKEN3_SYMBOL),
				Token(TOKEN4_SYMBOL),
				base_pool.lp_currency_id,
				meta_pool.lp_currency_id,
			],
			&BOB,
		);

		assert_eq!(balances_after[0], 99881980616021312485);
		assert_eq!(balances_after[1], 99000000000000000000);
		assert_eq!(balances_after[2], 99000000);
		assert_eq!(balances_after[3], 98000000);
		assert_eq!(balances_after[4], 2000000000000000000);
		assert_eq!(balances_after[5], 2000000000000000000);
	})
}

#[test]
fn get_meta_virtual_price_impact_on_base_pool_should_work() {
	new_test_ext().execute_with(|| {
		let (base_pool_id, meta_pool_id) = setup_test_meta_pool();
		assert_eq!(StableAmm::get_virtual_price(base_pool_id), 1e18 as Balance);

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(ALICE).into(),
			base_pool_id,
			vec![2e20 as Balance, 1e8 as Balance, 1e8 as Balance],
			0,
			ALICE,
			u64::MAX,
		));

		assert_eq!(StableAmm::get_virtual_price(base_pool_id), 1000015381247123616);
		assert_eq!(StableAmm::get_virtual_price(meta_pool_id), 1e18 as Balance);

		set_block_timestamp(Timestamp::now() / 1000 + 11 * 3600);
		assert_eq!(StableAmm::get_virtual_price(meta_pool_id), 1000007690622981952);
	})
}

#[test]
fn meta_pool_add_liquidity_impact_on_base_pool_price_should_work() {
	new_test_ext().execute_with(|| {
		let (base_pool_id, meta_pool_id) = setup_test_meta_pool();

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(ALICE).into(),
			base_pool_id,
			vec![2e20 as Balance, 1e8 as Balance, 1e8 as Balance],
			0,
			ALICE,
			u64::MAX,
		));

		set_block_timestamp(Timestamp::now() / 1000 + 11 * 3600);
		assert_eq!(StableAmm::get_virtual_price(meta_pool_id), 1000007690622981952);

		let meta_pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();
		let meta_lp_currency_amount_before = get_user_balance(meta_pool.lp_currency_id, &ALICE);
		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(ALICE).into(),
			meta_pool_id,
			vec![1e18 as Balance, 3e18 as Balance],
			0,
			ALICE,
			u64::MAX,
		));
		let meta_lp_currency_amount_after = get_user_balance(meta_pool.lp_currency_id, &ALICE);

		assert_eq!(
			meta_lp_currency_amount_after - meta_lp_currency_amount_before,
			3991687236658238901
		);
	})
}

#[test]
fn meta_pool_remove_liquidity_impact_on_base_pool_price_should_work() {
	new_test_ext().execute_with(|| {
		let (base_pool_id, meta_pool_id) = setup_test_meta_pool();

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(ALICE).into(),
			base_pool_id,
			vec![2e20 as Balance, 1e8 as Balance, 1e8 as Balance],
			0,
			ALICE,
			u64::MAX,
		));

		set_block_timestamp(Timestamp::now() / 1000 + 11 * 3600);
		assert_eq!(StableAmm::get_virtual_price(meta_pool_id), 1000007690622981952);

		let meta_pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();
		let base_lp_currency_amount_before = get_user_balance(meta_pool.currency_ids[1], &ALICE);
		let meta_pool_currency_amount_before = get_user_balance(meta_pool.currency_ids[0], &ALICE);

		assert_ok!(StableAmm::remove_liquidity(
			RawOrigin::Signed(ALICE).into(),
			meta_pool_id,
			1e16 as Balance,
			vec![0, 0],
			ALICE,
			u64::MAX,
		));
		let base_lp_currency_amount_after = get_user_balance(meta_pool.currency_ids[1], &ALICE);
		let meta_pool_currency_amount_after = get_user_balance(meta_pool.currency_ids[0], &ALICE);

		// meta pool remove liquidity not effect by base price.
		assert_eq!(
			base_lp_currency_amount_after - base_lp_currency_amount_before,
			5000000000000000
		); //1e18 * (1e16 / 2e18)
		assert_eq!(
			meta_pool_currency_amount_after - meta_pool_currency_amount_before,
			5000000000000000
		);
	})
}

#[test]
fn meta_pool_remove_liquidity_one_currency_impact_on_base_pool_price_should_work() {
	new_test_ext().execute_with(|| {
		let (base_pool_id, meta_pool_id) = setup_test_meta_pool();

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(ALICE).into(),
			base_pool_id,
			vec![2e20 as Balance, 1e8 as Balance, 1e8 as Balance],
			0,
			ALICE,
			u64::MAX,
		));

		set_block_timestamp(Timestamp::now() / 1000 + 11 * 3600);
		assert_eq!(StableAmm::get_virtual_price(meta_pool_id), 1000007690622981952);

		let meta_pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();
		let base_lp_currency_amount_before = get_user_balance(meta_pool.currency_ids[1], &ALICE);
		let meta_pool_currency_amount_before = get_user_balance(meta_pool.currency_ids[0], &ALICE);

		assert_ok!(StableAmm::remove_liquidity_one_currency(
			RawOrigin::Signed(ALICE).into(),
			meta_pool_id,
			1e16 as Balance,
			0,
			0,
			ALICE,
			u64::MAX,
		));

		let base_lp_currency_amount_after = get_user_balance(meta_pool.currency_ids[1], &ALICE);
		let meta_pool_currency_amount_after = get_user_balance(meta_pool.currency_ids[0], &ALICE);

		assert_eq!(base_lp_currency_amount_after - base_lp_currency_amount_before, 0);
		// if the base price not changed, the result is 9994508116007042
		assert_eq!(
			meta_pool_currency_amount_after - meta_pool_currency_amount_before,
			9994583427668418
		);
	})
}

#[test]
fn meta_pool_remove_liquidity_imbalance_impact_on_base_pool_price_should_work() {
	new_test_ext().execute_with(|| {
		let (base_pool_id, meta_pool_id) = setup_test_meta_pool();

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(ALICE).into(),
			base_pool_id,
			vec![2e20 as Balance, 1e8 as Balance, 1e8 as Balance],
			0,
			ALICE,
			u64::MAX,
		));

		set_block_timestamp(Timestamp::now() / 1000 + 11 * 3600);
		assert_eq!(StableAmm::get_virtual_price(meta_pool_id), 1000007690622981952);

		let meta_pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();
		let base_lp_currency_amount_before = get_user_balance(meta_pool.currency_ids[1], &ALICE);
		let meta_pool_currency_amount_before = get_user_balance(meta_pool.currency_ids[0], &ALICE);
		let meta_lp_currency_amount_before = get_user_balance(meta_pool.lp_currency_id, &ALICE);

		assert_ok!(StableAmm::remove_liquidity_imbalance(
			RawOrigin::Signed(ALICE).into(),
			meta_pool_id,
			vec![5000000000000000, 5000000000000000],
			Balance::MAX,
			ALICE,
			u64::MAX,
		));

		let base_lp_currency_amount_after = get_user_balance(meta_pool.currency_ids[1], &ALICE);
		let meta_pool_currency_amount_after = get_user_balance(meta_pool.currency_ids[0], &ALICE);
		let meta_lp_currency_amount_after = get_user_balance(meta_pool.lp_currency_id, &ALICE);

		assert_eq!(
			base_lp_currency_amount_after - base_lp_currency_amount_before,
			5000000000000000
		);
		assert_eq!(
			meta_pool_currency_amount_after - meta_pool_currency_amount_before,
			5000000000000000
		);
		assert_eq!(
			meta_lp_currency_amount_before - meta_lp_currency_amount_after,
			10000000000000001
		);
	})
}

#[test]
fn meta_pool_swap_impact_on_base_pool_price_should_work() {
	new_test_ext().execute_with(|| {
		let (base_pool_id, meta_pool_id) = setup_test_meta_pool();

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(ALICE).into(),
			base_pool_id,
			vec![2e20 as Balance, 1e8 as Balance, 1e8 as Balance],
			0,
			ALICE,
			u64::MAX,
		));

		set_block_timestamp(Timestamp::now() / 1000 + 11 * 3600);
		assert_eq!(StableAmm::get_virtual_price(meta_pool_id), 1000007690622981952);

		let meta_pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();
		let base_lp_currency_amount_before = get_user_balance(meta_pool.currency_ids[1], &ALICE);
		let meta_pool_currency_amount_before = get_user_balance(meta_pool.currency_ids[0], &ALICE);
		let meta_lp_currency_amount_before = get_user_balance(meta_pool.lp_currency_id, &ALICE);

		assert_ok!(StableAmm::swap(
			RawOrigin::Signed(ALICE).into(),
			meta_pool_id,
			0,
			1,
			1e16 as Balance,
			0,
			ALICE,
			u64::MAX,
		));

		let base_lp_currency_amount_after = get_user_balance(meta_pool.currency_ids[1], &ALICE);
		let meta_pool_currency_amount_after = get_user_balance(meta_pool.currency_ids[0], &ALICE);
		let meta_lp_currency_amount_after = get_user_balance(meta_pool.lp_currency_id, &ALICE);

		// if base price no change, then get 9988041372295327 base lp after swap.
		assert_eq!(
			base_lp_currency_amount_after - base_lp_currency_amount_before,
			9987890773726400
		);
		assert_eq!(
			meta_pool_currency_amount_before - meta_pool_currency_amount_after,
			1e16 as Balance
		);
		// no impact on meta lp
		assert_eq!(meta_lp_currency_amount_before - meta_lp_currency_amount_after, 0);
	})
}

#[test]
fn meta_pool_swap_underlying_impact_on_base_pool_price_should_work() {
	new_test_ext().execute_with(|| {
		let (base_pool_id, meta_pool_id) = setup_test_meta_pool();

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(ALICE).into(),
			base_pool_id,
			vec![2e20 as Balance, 1e8 as Balance, 1e8 as Balance],
			0,
			ALICE,
			u64::MAX,
		));

		set_block_timestamp(Timestamp::now() / 1000 + 11 * 3600);
		assert_eq!(StableAmm::get_virtual_price(meta_pool_id), 1000007690622981952);

		let meta_pool = StableAmm::pools(meta_pool_id).unwrap().get_pool_info();
		let base_pool = StableAmm::pools(base_pool_id).unwrap().get_pool_info();

		let meta_pool_currency_amount_before = get_user_balance(meta_pool.currency_ids[0], &ALICE);
		let target_currency_amount_before = get_user_balance(base_pool.currency_ids[0], &ALICE);

		assert_ok!(StableAmm::swap_meta_pool_underlying(
			RawOrigin::Signed(ALICE).into(),
			meta_pool_id,
			0,
			1,
			1e16 as Balance,
			0,
			ALICE,
			u64::MAX,
		));

		let meta_pool_currency_amount_after = get_user_balance(meta_pool.currency_ids[0], &ALICE);
		let target_currency_amount_after = get_user_balance(base_pool.currency_ids[0], &ALICE);

		assert_eq!(
			meta_pool_currency_amount_before - meta_pool_currency_amount_after,
			1e16 as Balance
		);
		// if base price no change, get 9986043212788621 target amount.
		assert_eq!(target_currency_amount_after - target_currency_amount_before, 9993224247822464);
	})
}
