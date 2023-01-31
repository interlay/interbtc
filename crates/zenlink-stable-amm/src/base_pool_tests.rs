// Copyright 2021-2022 Zenlink.
// Licensed under Apache 2.0.

use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;
use sp_runtime::DispatchError::BadOrigin;

use super::{
	mock::{CurrencyId::*, *},
	*,
};

const INITIAL_A_VALUE: Balance = 50;
const SWAP_FEE: Balance = 1e7 as Balance;
const ADMIN_FEE: Balance = 0;

pub fn setup_test_base_pool() -> (PoolId, CurrencyId) {
	assert_ok!(StableAmm::create_base_pool(
		RawOrigin::Root.into(),
		vec![Token(TOKEN1_SYMBOL), Token(TOKEN2_SYMBOL),],
		vec![TOKEN1_DECIMAL, TOKEN2_DECIMAL],
		INITIAL_A_VALUE,
		SWAP_FEE,
		ADMIN_FEE,
		ALICE,
		Vec::from("stable_pool_lp"),
	));

	let pool_id = StableAmm::next_pool_id() - 1;
	let lp_currency_id = StableAmm::pools(pool_id).unwrap().get_lp_currency();

	assert_ok!(StableAmm::add_liquidity(
		RawOrigin::Signed(ALICE).into(),
		0,
		vec![1e18 as Balance, 1e18 as Balance],
		0,
		ALICE,
		u64::MAX,
	));
	(0, lp_currency_id)
}

#[test]
fn create_pool_with_incorrect_parameter_should_not_work() {
	new_test_ext().execute_with(|| {
		// only root can create pool
		assert_noop!(
			StableAmm::create_base_pool(
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
		assert_eq!(StableAmm::next_pool_id(), 0);
		assert_eq!(StableAmm::pools(0), None);

		//create mismatch parameter should not work
		assert_noop!(
			StableAmm::create_base_pool(
				RawOrigin::Root.into(),
				vec![Token(TOKEN1_SYMBOL), Token(TOKEN2_SYMBOL), Token(TOKEN3_SYMBOL),],
				vec![TOKEN1_DECIMAL, TOKEN2_DECIMAL, TOKEN3_DECIMAL, TOKEN4_DECIMAL],
				0,
				0,
				0,
				ALICE,
				Vec::from("stable_pool_lp"),
			),
			Error::<Test>::MismatchParameter
		);
		assert_eq!(StableAmm::next_pool_id(), 0);
		assert_eq!(StableAmm::pools(0), None);

		// create with forbidden token should not work
		assert_noop!(
			StableAmm::create_base_pool(
				RawOrigin::Root.into(),
				vec![
					Forbidden(TOKEN1_SYMBOL),
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
			Error::<Test>::InvalidPooledCurrency
		);
		assert_eq!(StableAmm::next_pool_id(), 0);
		assert_eq!(StableAmm::pools(0), None);

		// Create with invalid decimal should not work
		assert_noop!(
			StableAmm::create_base_pool(
				RawOrigin::Root.into(),
				vec![Token(TOKEN1_SYMBOL), Token(TOKEN2_SYMBOL), Token(TOKEN3_SYMBOL),],
				vec![TOKEN1_DECIMAL, 20, TOKEN3_DECIMAL],
				0,
				0,
				0,
				ALICE,
				Vec::from("stable_pool_lp"),
			),
			Error::<Test>::InvalidCurrencyDecimal
		);
		assert_eq!(StableAmm::next_pool_id(), 0);
		assert_eq!(StableAmm::pools(0), None);
	});
}

#[test]
fn create_pool_with_parameters_exceed_threshold_should_not_work() {
	new_test_ext().execute_with(|| {
		// exceed max swap fee
		assert_noop!(
			StableAmm::create_base_pool(
				RawOrigin::Root.into(),
				vec![
					Token(TOKEN1_SYMBOL),
					Token(TOKEN2_SYMBOL),
					Token(TOKEN3_SYMBOL),
					Token(TOKEN4_SYMBOL)
				],
				vec![TOKEN1_DECIMAL, TOKEN2_DECIMAL, TOKEN3_DECIMAL, TOKEN4_DECIMAL],
				0,
				(MAX_SWAP_FEE + 1).into(),
				0,
				ALICE,
				Vec::from("stable_pool_lp"),
			),
			Error::<Test>::ExceedMaxFee
		);
		assert_eq!(StableAmm::next_pool_id(), 0);
		assert_eq!(StableAmm::pools(0), None);

		// exceed max admin fee
		assert_noop!(
			StableAmm::create_base_pool(
				RawOrigin::Root.into(),
				vec![
					Token(TOKEN1_SYMBOL),
					Token(TOKEN2_SYMBOL),
					Token(TOKEN3_SYMBOL),
					Token(TOKEN4_SYMBOL)
				],
				vec![TOKEN1_DECIMAL, TOKEN2_DECIMAL, TOKEN3_DECIMAL, TOKEN4_DECIMAL],
				0,
				(MAX_SWAP_FEE).into(),
				(MAX_ADMIN_FEE + 1).into(),
				ALICE,
				Vec::from("stable_pool_lp"),
			),
			Error::<Test>::ExceedMaxAdminFee
		);
		assert_eq!(StableAmm::next_pool_id(), 0);
		assert_eq!(StableAmm::pools(0), None);

		// exceed max a
		assert_noop!(
			StableAmm::create_base_pool(
				RawOrigin::Root.into(),
				vec![
					Token(TOKEN1_SYMBOL),
					Token(TOKEN2_SYMBOL),
					Token(TOKEN3_SYMBOL),
					Token(TOKEN4_SYMBOL)
				],
				vec![TOKEN1_DECIMAL, TOKEN2_DECIMAL, TOKEN3_DECIMAL, TOKEN4_DECIMAL],
				MAX_A.into(),
				(MAX_SWAP_FEE - 1).into(),
				(MAX_ADMIN_FEE - 1).into(),
				ALICE,
				Vec::from("stable_pool_lp"),
			),
			Error::<Test>::ExceedMaxA
		);
		assert_eq!(StableAmm::next_pool_id(), 0);
		assert_eq!(StableAmm::pools(0), None);
	});
}

#[test]
fn create_pool_should_work() {
	new_test_ext().execute_with(|| {
		let lp_currency_id = CurrencyId::StableLPV2(0);
		assert_eq!(StableAmm::lp_currencies(lp_currency_id), None);

		assert_ok!(StableAmm::create_base_pool(
			RawOrigin::Root.into(),
			vec![
				Token(TOKEN1_SYMBOL),
				Token(TOKEN2_SYMBOL),
				Token(TOKEN3_SYMBOL),
				Token(TOKEN4_SYMBOL)
			],
			vec![TOKEN1_DECIMAL, TOKEN2_DECIMAL, TOKEN3_DECIMAL, TOKEN4_DECIMAL],
			INITIAL_A_VALUE,
			SWAP_FEE,
			ADMIN_FEE,
			ALICE,
			Vec::from("stable_pool_lp"),
		));

		assert_eq!(StableAmm::next_pool_id(), 1);

		assert_eq!(
			StableAmm::pools(0),
			Some(MockPool::Base(BasePool {
				currency_ids: vec![
					Token(TOKEN1_SYMBOL),
					Token(TOKEN2_SYMBOL),
					Token(TOKEN3_SYMBOL),
					Token(TOKEN4_SYMBOL),
				],
				lp_currency_id,
				token_multipliers: vec![
					checked_pow(10, (POOL_TOKEN_COMMON_DECIMALS - TOKEN1_DECIMAL) as usize)
						.unwrap(),
					checked_pow(10, (POOL_TOKEN_COMMON_DECIMALS - TOKEN2_DECIMAL) as usize)
						.unwrap(),
					checked_pow(10, (POOL_TOKEN_COMMON_DECIMALS - TOKEN3_DECIMAL) as usize)
						.unwrap(),
					checked_pow(10, (POOL_TOKEN_COMMON_DECIMALS - TOKEN4_DECIMAL) as usize)
						.unwrap(),
				],
				balances: vec![Zero::zero(); 4],
				fee: SWAP_FEE,
				admin_fee: ADMIN_FEE,
				initial_a: INITIAL_A_VALUE * (A_PRECISION as Balance),
				future_a: INITIAL_A_VALUE * (A_PRECISION as Balance),
				initial_a_time: 0,
				future_a_time: 0,
				account: POOL0ACCOUNTID,
				admin_fee_receiver: ALICE,
				lp_currency_symbol: BoundedVec::<u8, PoolCurrencySymbolLimit>::try_from(Vec::from(
					"stable_pool_lp"
				))
				.unwrap(),
				lp_currency_decimal: 18,
			}))
		);

		assert_eq!(StableAmm::lp_currencies(lp_currency_id), Some(0))
	});
}

#[test]
fn add_liquidity_with_incorrect_params_should_not_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(StableAmm::create_base_pool(
			RawOrigin::Root.into(),
			vec![Token(TOKEN1_SYMBOL), Token(TOKEN2_SYMBOL),],
			vec![TOKEN1_DECIMAL, TOKEN2_DECIMAL],
			INITIAL_A_VALUE,
			SWAP_FEE,
			ADMIN_FEE,
			ALICE,
			Vec::from("stable_pool_lp"),
		));

		// case0: add_liquidity with incorrect pool id
		assert_noop!(
			StableAmm::add_liquidity(
				RawOrigin::Signed(BOB).into(),
				1,
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
				0,
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
				0,
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
fn add_liquidity_should_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, lp_currency_id) = setup_test_base_pool();

		assert_eq!(
			<Test as Config>::MultiCurrency::free_balance(lp_currency_id, &ALICE),
			2e18 as Balance
		);

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			pool_id,
			vec![1e18 as Balance, 3e18 as Balance],
			0,
			CHARLIE,
			u64::MAX,
		));
		assert_eq!(
			<Test as Config>::MultiCurrency::free_balance(lp_currency_id, &CHARLIE),
			3991672211258372957
		);
	});
}

#[test]
fn add_liquidity_with_expected_amount_lp_token_should_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, lp_currency_id) = setup_test_base_pool();

		assert_eq!(
			<Test as Config>::MultiCurrency::free_balance(lp_currency_id, &ALICE),
			2e18 as Balance
		);
		let calculated_lp_token_amount = StableAmm::calculate_currency_amount(
			pool_id,
			vec![1e18 as Balance, 3e18 as Balance],
			true,
		)
		.unwrap_or_default();
		assert_eq!(calculated_lp_token_amount, 3992673697878079065);

		let calculated_lp_token_amount_with_slippage = calculated_lp_token_amount * 999 / 1000;

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			pool_id,
			vec![1e18 as Balance, 3e18 as Balance],
			calculated_lp_token_amount_with_slippage,
			CHARLIE,
			u64::MAX,
		));
		assert_eq!(
			<Test as Config>::MultiCurrency::free_balance(lp_currency_id, &CHARLIE),
			3991672211258372957
		);
	});
}

#[test]
fn add_liquidity_lp_token_amount_has_small_slippage_should_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, lp_currency_id) = setup_test_base_pool();

		let calculated_lp_token_amount = StableAmm::calculate_currency_amount(
			pool_id,
			vec![1e18 as Balance, 3e18 as Balance],
			true,
		)
		.unwrap_or_default();

		let calculated_lp_token_amount_with_negative_slippage =
			calculated_lp_token_amount * 999 / 1000;
		let calculated_lp_token_amount_with_positive_slippage =
			calculated_lp_token_amount * 1001 / 1000;
		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			pool_id,
			vec![1e18 as Balance, 3e18 as Balance],
			calculated_lp_token_amount_with_negative_slippage,
			CHARLIE,
			u64::MAX,
		));

		let lp_token_balance =
			<Test as Config>::MultiCurrency::free_balance(lp_currency_id, &CHARLIE);
		assert!(lp_token_balance > calculated_lp_token_amount_with_negative_slippage);
		assert!(lp_token_balance < calculated_lp_token_amount_with_positive_slippage);
	})
}

#[test]
fn add_liquidity_update_pool_balance_should_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, _) = setup_test_base_pool();

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			pool_id,
			vec![1e18 as Balance, 3e18 as Balance],
			0,
			CHARLIE,
			u64::MAX,
		));

		assert_eq!(
			<Test as Config>::MultiCurrency::free_balance(Token(TOKEN1_SYMBOL), &POOL0ACCOUNTID),
			2e18 as Balance
		);

		assert_eq!(
			<Test as Config>::MultiCurrency::free_balance(Token(TOKEN2_SYMBOL), &POOL0ACCOUNTID),
			4e18 as Balance
		);
	})
}

#[test]
fn add_liquidity_when_mint_amount_not_reach_due_to_front_running_should_not_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, _) = setup_test_base_pool();

		let calculated_lp_token_amount = StableAmm::calculate_currency_amount(
			pool_id,
			vec![1e18 as Balance, 3e18 as Balance],
			true,
		)
		.unwrap_or_default();
		let calculated_lp_token_amount_with_slippage = calculated_lp_token_amount * 999 / 1000;
		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(ALICE).into(),
			pool_id,
			vec![1e18 as Balance, 3e18 as Balance],
			0,
			ALICE,
			u64::MAX,
		));

		assert_noop!(
			StableAmm::add_liquidity(
				RawOrigin::Signed(BOB).into(),
				pool_id,
				vec![1e18 as Balance, 3e18 as Balance],
				calculated_lp_token_amount_with_slippage,
				ALICE,
				u64::MAX,
			),
			Error::<Test>::AmountSlippage
		);
	})
}

#[test]
fn add_liquidity_with_expired_deadline_should_not_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(StableAmm::create_base_pool(
			RawOrigin::Root.into(),
			vec![Token(TOKEN1_SYMBOL), Token(TOKEN2_SYMBOL),],
			vec![TOKEN1_DECIMAL, TOKEN2_DECIMAL],
			INITIAL_A_VALUE,
			SWAP_FEE,
			ADMIN_FEE,
			ALICE,
			Vec::from("stable_pool_lp"),
		));

		System::set_block_number(100);

		assert_noop!(
			StableAmm::add_liquidity(
				RawOrigin::Signed(ALICE).into(),
				0,
				vec![1e18 as Balance, 1e18 as Balance],
				0,
				ALICE,
				99,
			),
			Error::<Test>::Deadline
		);
	})
}

#[test]
fn remove_liquidity_exceed_total_supply_should_not_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, _) = setup_test_base_pool();
		let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();
		assert!(StableAmm::calculate_base_remove_liquidity(&pool, u128::MAX) == None)
	})
}

#[test]
fn remove_liquidity_with_incorrect_min_amounts_length_should_not_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, _) = setup_test_base_pool();
		assert_noop!(
			StableAmm::remove_liquidity(
				RawOrigin::Signed(ALICE).into(),
				pool_id,
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
fn remove_liquidity_should_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, lp_currency_id) = setup_test_base_pool();

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			pool_id,
			vec![2e18 as Balance, 1e16 as Balance],
			0,
			BOB,
			u64::MAX,
		));

		assert_ok!(StableAmm::remove_liquidity(
			RawOrigin::Signed(ALICE).into(),
			pool_id,
			2e18 as Balance,
			vec![0, 0],
			ALICE,
			u64::MAX
		));

		let current_bob_balance =
			<Test as Config>::MultiCurrency::free_balance(lp_currency_id, &BOB);
		assert_eq!(current_bob_balance, 1996275270169644725);

		assert_ok!(StableAmm::remove_liquidity(
			RawOrigin::Signed(BOB).into(),
			pool_id,
			current_bob_balance,
			vec![0, 0],
			BOB,
			u64::MAX
		));
		assert_eq!(
			<Test as Config>::MultiCurrency::free_balance(Token(TOKEN1_SYMBOL), &POOL0ACCOUNTID),
			0
		);
		assert_eq!(
			<Test as Config>::MultiCurrency::free_balance(Token(TOKEN2_SYMBOL), &POOL0ACCOUNTID),
			0
		);
	})
}

#[test]
fn remove_liquidity_with_expected_return_amount_underlying_currency_should_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, lp_currency_id) = setup_test_base_pool();

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			pool_id,
			vec![2e18 as Balance, 1e16 as Balance],
			0,
			CHARLIE,
			u64::MAX,
		));
		let first_token_balance_before =
			<Test as Config>::MultiCurrency::free_balance(Token(TOKEN1_SYMBOL), &CHARLIE);
		let second_token_balance_before =
			<Test as Config>::MultiCurrency::free_balance(Token(TOKEN2_SYMBOL), &CHARLIE);
		let pool_token_balance_before =
			<Test as Config>::MultiCurrency::free_balance(lp_currency_id, &CHARLIE);

		assert_eq!(pool_token_balance_before, 1996275270169644725);
		let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();
		let expected_balances =
			StableAmm::calculate_base_remove_liquidity(&pool, pool_token_balance_before).unwrap();
		assert_eq!(expected_balances[0], 1498601924450190405);
		assert_eq!(expected_balances[1], 504529314564897436);

		assert_ok!(StableAmm::remove_liquidity(
			RawOrigin::Signed(CHARLIE).into(),
			pool_id,
			pool_token_balance_before,
			expected_balances.clone(),
			CHARLIE,
			u64::MAX
		));

		let first_token_balance_after =
			<Test as Config>::MultiCurrency::free_balance(Token(TOKEN1_SYMBOL), &CHARLIE);
		let second_token_balance_after =
			<Test as Config>::MultiCurrency::free_balance(Token(TOKEN2_SYMBOL), &CHARLIE);

		assert_eq!(first_token_balance_after - first_token_balance_before, expected_balances[0]);
		assert_eq!(second_token_balance_after - second_token_balance_before, expected_balances[1]);
	})
}

#[test]
fn remove_liquidity_exceed_own_lp_tokens_should_not_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, lp_currency_id) = setup_test_base_pool();

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			pool_id,
			vec![2e18 as Balance, 1e16 as Balance],
			0,
			BOB,
			u64::MAX,
		));

		let pool_token_balance =
			<Test as Config>::MultiCurrency::free_balance(lp_currency_id, &BOB);
		assert_eq!(pool_token_balance, 1996275270169644725);
		assert_noop!(
			StableAmm::remove_liquidity(
				RawOrigin::Signed(BOB).into(),
				pool_id,
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
fn remove_liquidity_when_min_amounts_not_reached_due_to_front_running_should_not_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, lp_currency_id) = setup_test_base_pool();

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			pool_id,
			vec![2e18 as Balance, 1e16 as Balance],
			0,
			BOB,
			u64::MAX,
		));

		let pool_token_balance =
			<Test as Config>::MultiCurrency::free_balance(lp_currency_id, &BOB);
		assert_eq!(pool_token_balance, 1996275270169644725);

		let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();
		let expected_balances =
			StableAmm::calculate_base_remove_liquidity(&pool, pool_token_balance).unwrap();
		assert_eq!(expected_balances[0], 1498601924450190405);
		assert_eq!(expected_balances[1], 504529314564897436);

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(CHARLIE).into(),
			pool_id,
			vec![1e16 as Balance, 2e18 as Balance],
			0,
			CHARLIE,
			u64::MAX,
		));

		assert_noop!(
			StableAmm::remove_liquidity(
				RawOrigin::Signed(BOB).into(),
				pool_id,
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
fn remove_liquidity_with_expired_deadline_should_not_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, lp_currency_id) = setup_test_base_pool();

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			pool_id,
			vec![2e18 as Balance, 1e16 as Balance],
			0,
			BOB,
			u64::MAX,
		));
		let pool_token_balance =
			<Test as Config>::MultiCurrency::free_balance(lp_currency_id, &BOB);

		System::set_block_number(100);

		assert_noop!(
			StableAmm::remove_liquidity(
				RawOrigin::Signed(BOB).into(),
				pool_id,
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
fn remove_liquidity_imbalance_with_mismatch_amounts_should_not_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, _) = setup_test_base_pool();
		assert_noop!(
			StableAmm::remove_liquidity_imbalance(
				RawOrigin::Signed(ALICE).into(),
				pool_id,
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
fn remove_liquidity_imbalance_when_withdraw_more_than_available_should_not_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, _) = setup_test_base_pool();
		assert_noop!(
			StableAmm::remove_liquidity_imbalance(
				RawOrigin::Signed(ALICE).into(),
				pool_id,
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
fn remove_liquidity_imbalance_with_max_burn_lp_token_amount_range_should_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, lp_currency_id) = setup_test_base_pool();
		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			pool_id,
			vec![2e18 as Balance, 1e16 as Balance],
			0,
			BOB,
			u64::MAX,
		));

		// calculates amount of pool token to be burned
		let max_pool_token_amount_to_be_burned = StableAmm::calculate_currency_amount(
			pool_id,
			vec![1e18 as Balance, 1e16 as Balance],
			false,
		)
		.unwrap();
		assert_eq!(1000688044155287276, max_pool_token_amount_to_be_burned);

		let max_pool_token_amount_to_be_burned_negative_slippage =
			max_pool_token_amount_to_be_burned * 1001 / 1000;
		let max_pool_token_amount_to_be_burned_positive_slippage =
			max_pool_token_amount_to_be_burned * 999 / 1000;
		let balance_before = get_user_token_balances(
			&[Token(TOKEN1_SYMBOL), Token(TOKEN2_SYMBOL), lp_currency_id],
			&BOB,
		);

		assert_ok!(StableAmm::remove_liquidity_imbalance(
			RawOrigin::Signed(BOB).into(),
			pool_id,
			vec![1e18 as Balance, 1e16 as Balance],
			max_pool_token_amount_to_be_burned_negative_slippage,
			BOB,
			u64::MAX
		));

		let balance_after = get_user_token_balances(
			&[Token(TOKEN1_SYMBOL), Token(TOKEN2_SYMBOL), lp_currency_id],
			&BOB,
		);

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
fn remove_liquidity_imbalance_exceed_own_lp_token_amount_should_not_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, lp_currency_id) = setup_test_base_pool();
		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			pool_id,
			vec![2e18 as Balance, 1e16 as Balance],
			0,
			BOB,
			u64::MAX,
		));

		let current_balance = <Test as Config>::MultiCurrency::free_balance(lp_currency_id, &BOB);
		assert_eq!(current_balance, 1996275270169644725);

		assert_noop!(
			StableAmm::remove_liquidity_imbalance(
				RawOrigin::Signed(BOB).into(),
				pool_id,
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
fn remove_liquidity_imbalance_when_min_amounts_of_underlying_tokens_not_reached_should_not_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, _) = setup_test_base_pool();
		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			pool_id,
			vec![2e18 as Balance, 1e16 as Balance],
			0,
			BOB,
			u64::MAX,
		));

		let max_pool_token_amount_to_be_burned = StableAmm::calculate_currency_amount(
			pool_id,
			vec![1e18 as Balance, 1e16 as Balance],
			false,
		)
		.unwrap();

		let max_pool_token_amount_to_be_burned_negative_slippage =
			max_pool_token_amount_to_be_burned * 1001 / 1000;

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(CHARLIE).into(),
			pool_id,
			vec![1e16 as Balance, 2e20 as Balance],
			0,
			ALICE,
			u64::MAX,
		));

		assert_noop!(
			StableAmm::remove_liquidity_imbalance(
				RawOrigin::Signed(BOB).into(),
				pool_id,
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
fn remove_liquidity_imbalance_with_expired_deadline_should_not_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, lp_currency_id) = setup_test_base_pool();
		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			pool_id,
			vec![2e18 as Balance, 1e16 as Balance],
			0,
			BOB,
			u64::MAX,
		));
		let current_balance = <Test as Config>::MultiCurrency::free_balance(lp_currency_id, &BOB);
		System::set_block_number(100);

		assert_noop!(
			StableAmm::remove_liquidity_imbalance(
				RawOrigin::Signed(BOB).into(),
				pool_id,
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
fn remove_liquidity_one_currency_with_currency_index_out_range_should_not_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, _) = setup_test_base_pool();
		let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();
		assert_eq!(StableAmm::calculate_base_remove_liquidity_one_token(&pool, 1, 5), None);
	})
}

#[test]
fn remove_liquidity_one_currency_calculation_should_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, lp_currency_id) = setup_test_base_pool();
		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			pool_id,
			vec![2e18 as Balance, 1e16 as Balance],
			0,
			BOB,
			u64::MAX,
		));

		let pool_token_balance =
			<Test as Config>::MultiCurrency::free_balance(lp_currency_id, &BOB);
		assert_eq!(pool_token_balance, 1996275270169644725);
		let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();
		assert_eq!(
			StableAmm::calculate_base_remove_liquidity_one_token(&pool, 2 * pool_token_balance, 0)
				.unwrap()
				.0,
			2999998601797183633
		);
	})
}

#[test]
fn remove_liquidity_one_currency_calculated_amount_as_min_amount_should_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, lp_currency_id) = setup_test_base_pool();
		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			pool_id,
			vec![2e18 as Balance, 1e16 as Balance],
			0,
			BOB,
			u64::MAX,
		));

		let pool_token_balance =
			<Test as Config>::MultiCurrency::free_balance(lp_currency_id, &BOB);
		assert_eq!(pool_token_balance, 1996275270169644725);
		let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();
		let calculated_first_token_amount =
			StableAmm::calculate_base_remove_liquidity_one_token(&pool, pool_token_balance, 0)
				.unwrap();
		assert_eq!(calculated_first_token_amount.0, 2008990034631583696);

		let before = <Test as Config>::MultiCurrency::free_balance(Token(TOKEN1_SYMBOL), &BOB);

		assert_ok!(StableAmm::remove_liquidity_one_currency(
			RawOrigin::Signed(BOB).into(),
			pool_id,
			pool_token_balance,
			0,
			calculated_first_token_amount.0,
			BOB,
			u64::MAX
		));

		let after = <Test as Config>::MultiCurrency::free_balance(Token(TOKEN1_SYMBOL), &BOB);
		assert_eq!(after - before, 2008990034631583696);
	})
}

#[test]
fn remove_liquidity_one_currency_with_lp_token_amount_exceed_own_should_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, lp_currency_id) = setup_test_base_pool();
		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			pool_id,
			vec![2e18 as Balance, 1e16 as Balance],
			0,
			BOB,
			u64::MAX,
		));

		let pool_token_balance =
			<Test as Config>::MultiCurrency::free_balance(lp_currency_id, &BOB);
		assert_eq!(pool_token_balance, 1996275270169644725);

		assert_noop!(
			StableAmm::remove_liquidity_one_currency(
				RawOrigin::Signed(BOB).into(),
				pool_id,
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
		let (pool_id, lp_currency_id) = setup_test_base_pool();
		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			pool_id,
			vec![2e18 as Balance, 1e16 as Balance],
			0,
			BOB,
			u64::MAX,
		));

		let pool_token_balance =
			<Test as Config>::MultiCurrency::free_balance(lp_currency_id, &BOB);
		assert_eq!(pool_token_balance, 1996275270169644725);

		let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();
		let calculated_first_token_amount =
			StableAmm::calculate_base_remove_liquidity_one_token(&pool, pool_token_balance, 0)
				.unwrap();
		assert_eq!(calculated_first_token_amount.0, 2008990034631583696);

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(CHARLIE).into(),
			pool_id,
			vec![1e16 as Balance, 1e20 as Balance],
			0,
			BOB,
			u64::MAX,
		));

		assert_noop!(
			StableAmm::remove_liquidity_one_currency(
				RawOrigin::Signed(BOB).into(),
				pool_id,
				pool_token_balance,
				0,
				calculated_first_token_amount.0,
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
		let (pool_id, lp_currency_id) = setup_test_base_pool();
		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			pool_id,
			vec![2e18 as Balance, 1e16 as Balance],
			0,
			BOB,
			u64::MAX,
		));

		let pool_token_balance =
			<Test as Config>::MultiCurrency::free_balance(lp_currency_id, &BOB);

		System::set_block_number(100);

		assert_noop!(
			StableAmm::remove_liquidity_one_currency(
				RawOrigin::Signed(BOB).into(),
				pool_id,
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
		let (pool_id, _) = setup_test_base_pool();
		let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();
		assert_eq!(StableAmm::calculate_base_swap_amount(&pool, 0, 9, 1e17 as Balance), None);
	})
}

#[test]
fn swap_with_currency_amount_exceed_own_should_not_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, _) = setup_test_base_pool();
		assert_noop!(
			StableAmm::swap(
				RawOrigin::Signed(BOB).into(),
				pool_id,
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
		let (pool_id, _) = setup_test_base_pool();
		let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();

		let calculated_swap_return =
			StableAmm::calculate_base_swap_amount(&pool, 0, 1, 1e17 as Balance).unwrap();
		assert_eq!(calculated_swap_return, 99702611562565288);

		let token_from_balance_before =
			<Test as Config>::MultiCurrency::free_balance(Token(TOKEN1_SYMBOL), &BOB);
		let token_to_balance_before =
			<Test as Config>::MultiCurrency::free_balance(Token(TOKEN2_SYMBOL), &CHARLIE);

		assert_ok!(StableAmm::swap(
			RawOrigin::Signed(BOB).into(),
			pool_id,
			0,
			1,
			1e17 as Balance,
			calculated_swap_return,
			CHARLIE,
			u64::MAX
		));
		let token_from_balance_after =
			<Test as Config>::MultiCurrency::free_balance(Token(TOKEN1_SYMBOL), &BOB);
		let token_to_balance_after =
			<Test as Config>::MultiCurrency::free_balance(Token(TOKEN2_SYMBOL), &CHARLIE);

		assert_eq!(token_from_balance_before - token_from_balance_after, 1e17 as Balance);
		assert_eq!(token_to_balance_after - token_to_balance_before, 99702611562565289);
	})
}

#[test]
fn swap_when_min_amount_receive_not_reached_due_to_front_running_should_not_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, _) = setup_test_base_pool();
		let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();
		let calculated_swap_return =
			StableAmm::calculate_base_swap_amount(&pool, 0, 1, 1e17 as Balance).unwrap();
		assert_eq!(calculated_swap_return, 99702611562565288);

		assert_ok!(StableAmm::swap(
			RawOrigin::Signed(CHARLIE).into(),
			pool_id,
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
				pool_id,
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
		let (pool_id, _) = setup_test_base_pool();
		let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();

		let token_from_balance_before =
			<Test as Config>::MultiCurrency::free_balance(Token(TOKEN1_SYMBOL), &BOB);
		let token_to_balance_before =
			<Test as Config>::MultiCurrency::free_balance(Token(TOKEN2_SYMBOL), &BOB);

		// BOB calculates how much token to receive with 1% slippage
		let calculated_swap_return =
			StableAmm::calculate_base_swap_amount(&pool, 0, 1, 1e17 as Balance).unwrap();
		assert_eq!(calculated_swap_return, 99702611562565288);
		let calculated_swap_return_with_negative_slippage = calculated_swap_return * 99 / 100;

		// CHARLIE swaps before User 1 does
		assert_ok!(StableAmm::swap(
			RawOrigin::Signed(CHARLIE).into(),
			pool_id,
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
			pool_id,
			0,
			1,
			1e17 as Balance,
			calculated_swap_return_with_negative_slippage,
			BOB,
			u64::MAX
		));

		let token_from_balance_after =
			<Test as Config>::MultiCurrency::free_balance(Token(TOKEN1_SYMBOL), &BOB);
		let token_to_balance_after =
			<Test as Config>::MultiCurrency::free_balance(Token(TOKEN2_SYMBOL), &BOB);

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
		let (pool_id, _) = setup_test_base_pool();

		System::set_block_number(100);

		assert_noop!(
			StableAmm::swap(
				RawOrigin::Signed(BOB).into(),
				pool_id,
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
fn calculate_virtual_price_should_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, _) = setup_test_base_pool();
		assert_eq!(StableAmm::get_virtual_price(pool_id), 1e18 as Balance);
	})
}

#[test]
fn calculate_virtual_price_after_swap_should_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, _) = setup_test_base_pool();
		assert_ok!(StableAmm::swap(
			RawOrigin::Signed(BOB).into(),
			pool_id,
			0,
			1,
			1e17 as Balance,
			0,
			BOB,
			u64::MAX
		));
		assert_eq!(StableAmm::get_virtual_price(pool_id), 1000050005862349911);

		assert_ok!(StableAmm::swap(
			RawOrigin::Signed(BOB).into(),
			pool_id,
			1,
			0,
			1e17 as Balance,
			0,
			BOB,
			u64::MAX
		));

		assert_eq!(StableAmm::get_virtual_price(pool_id), 1000100104768517937);
	})
}

#[test]
fn calculate_virtual_price_after_imbalanced_withdrawal_should_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, _) = setup_test_base_pool();
		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			pool_id,
			vec![1e18 as Balance, 1e18 as Balance],
			0,
			BOB,
			u64::MAX,
		));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(CHARLIE).into(),
			pool_id,
			vec![1e18 as Balance, 1e18 as Balance],
			0,
			CHARLIE,
			u64::MAX,
		));

		assert_eq!(StableAmm::get_virtual_price(pool_id), 1e18 as Balance);

		assert_ok!(StableAmm::remove_liquidity_imbalance(
			RawOrigin::Signed(BOB).into(),
			pool_id,
			vec![1e18 as Balance, 0],
			2e18 as Balance,
			BOB,
			u64::MAX
		));

		assert_eq!(StableAmm::get_virtual_price(pool_id), 1000100094088440633 as Balance);

		assert_ok!(StableAmm::remove_liquidity_imbalance(
			RawOrigin::Signed(CHARLIE).into(),
			pool_id,
			vec![0, 1e18 as Balance],
			2e18 as Balance,
			CHARLIE,
			u64::MAX
		));
		assert_eq!(StableAmm::get_virtual_price(pool_id), 1000200154928939884);
	})
}

#[test]
fn calculate_virtual_price_value_unchanged_after_deposits_should_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, _) = setup_test_base_pool();
		// pool is 1:1 ratio
		assert_eq!(StableAmm::get_virtual_price(pool_id), 1e18 as Balance);

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(CHARLIE).into(),
			pool_id,
			vec![1e18 as Balance, 1e18 as Balance],
			0,
			CHARLIE,
			u64::MAX,
		));
		assert_eq!(StableAmm::get_virtual_price(pool_id), 1e18 as Balance);

		// pool change 2:1 ratio, virtual_price also change
		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(CHARLIE).into(),
			pool_id,
			vec![2e18 as Balance, 0],
			0,
			CHARLIE,
			u64::MAX,
		));
		assert_eq!(StableAmm::get_virtual_price(pool_id), 1000167146429977312 as Balance);

		// keep 2:1 ratio after deposit, virtual not change.
		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			pool_id,
			vec![2e18 as Balance, 1e18 as Balance],
			0,
			BOB,
			u64::MAX,
		));
		assert_eq!(StableAmm::get_virtual_price(pool_id), 1000167146429977312);
	})
}

#[test]
fn calculate_virtual_price_value_not_change_after_balanced_withdrawal_should_not_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, _) = setup_test_base_pool();
		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			pool_id,
			vec![1e18 as Balance, 1e18 as Balance],
			0,
			BOB,
			u64::MAX,
		));

		assert_ok!(StableAmm::remove_liquidity(
			RawOrigin::Signed(BOB).into(),
			pool_id,
			1e18 as Balance,
			vec![0, 0],
			BOB,
			u64::MAX
		));

		assert_eq!(StableAmm::get_virtual_price(pool_id), 1e18 as Balance);
	})
}

#[test]
fn set_fee_with_non_owner_account_should_not_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, _) = setup_test_base_pool();
		assert_noop!(
			StableAmm::set_swap_fee(RawOrigin::Signed(BOB).into(), pool_id, 0,),
			BadOrigin
		);

		assert_noop!(
			StableAmm::set_swap_fee(RawOrigin::Signed(CHARLIE).into(), pool_id, 1e18 as Balance),
			BadOrigin
		);
	})
}

#[test]
fn set_fee_with_exceed_threshold_should_not_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, _) = setup_test_base_pool();

		assert_noop!(
			StableAmm::set_swap_fee(RawOrigin::Root.into(), pool_id, (1e8 as Balance) + 1),
			Error::<Test>::ExceedThreshold
		);
	})
}

#[test]
fn set_fee_should_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, _) = setup_test_base_pool();

		assert_ok!(StableAmm::set_swap_fee(RawOrigin::Root.into(), pool_id, 1e8 as Balance));

		let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();

		assert_eq!(pool.fee, 1e8 as Balance);
	})
}

#[test]
fn set_admin_fee_with_non_owner_should_not_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, _) = setup_test_base_pool();

		assert_noop!(
			StableAmm::set_admin_fee(RawOrigin::Signed(BOB).into(), pool_id, 0,),
			BadOrigin
		);
		assert_noop!(
			StableAmm::set_swap_fee(RawOrigin::Signed(CHARLIE).into(), pool_id, 1e10 as Balance,),
			BadOrigin
		);
	})
}

#[test]
fn set_admin_fee_with_exceed_threshold_should_not_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, _) = setup_test_base_pool();

		assert_noop!(
			StableAmm::set_admin_fee(RawOrigin::Root.into(), pool_id, (1e10 as Balance) + 1,),
			Error::<Test>::ExceedThreshold
		);
	})
}

#[test]
fn set_admin_fee_should_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, _) = setup_test_base_pool();

		assert_ok!(StableAmm::set_admin_fee(RawOrigin::Root.into(), pool_id, 1e10 as Balance,));

		let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();
		assert_eq!(pool.admin_fee, 1e10 as Balance);
	})
}

#[test]
fn get_admin_balance_with_index_out_of_range_should_not_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, _) = setup_test_base_pool();

		assert_eq!(StableAmm::get_admin_balance(pool_id, 3), None);
	})
}

#[test]
fn get_admin_balance_always_zero_when_admin_fee_equal_zero() {
	new_test_ext().execute_with(|| {
		let (pool_id, _) = setup_test_base_pool();
		assert_eq!(StableAmm::get_admin_balance(pool_id, 0), Some(Zero::zero()));
		assert_eq!(StableAmm::get_admin_balance(pool_id, 1), Some(Zero::zero()));

		assert_ok!(StableAmm::swap(
			RawOrigin::Signed(BOB).into(),
			pool_id,
			0,
			1,
			1e17 as Balance,
			0,
			BOB,
			u64::MAX
		));

		assert_eq!(StableAmm::get_admin_balance(pool_id, 0), Some(Zero::zero()));
		assert_eq!(StableAmm::get_admin_balance(pool_id, 1), Some(Zero::zero()));
	})
}

#[test]
fn get_admin_balance_with_expected_amount_after_swap_should_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, _) = setup_test_base_pool();
		assert_ok!(StableAmm::set_swap_fee(RawOrigin::Root.into(), pool_id, 1e7 as Balance,));

		assert_ok!(StableAmm::set_admin_fee(RawOrigin::Root.into(), pool_id, 1e8 as Balance,));

		assert_ok!(StableAmm::swap(
			RawOrigin::Signed(BOB).into(),
			pool_id,
			0,
			1,
			1e17 as Balance,
			0,
			BOB,
			u64::MAX
		));
		assert_eq!(StableAmm::get_admin_balance(pool_id, 0), Some(Zero::zero()));
		assert_eq!(StableAmm::get_admin_balance(pool_id, 1), Some(998024139765));

		assert_ok!(StableAmm::swap(
			RawOrigin::Signed(BOB).into(),
			pool_id,
			1,
			0,
			1e17 as Balance,
			0,
			BOB,
			u64::MAX
		));

		assert_eq!(StableAmm::get_admin_balance(pool_id, 0), Some(1001973776101));
		assert_eq!(StableAmm::get_admin_balance(pool_id, 1), Some(998024139765));
	})
}

#[test]
fn withdraw_admin_fee_when_no_admin_fee_should_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, _) = setup_test_base_pool();
		let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();
		assert_ok!(StableAmm::set_swap_fee(RawOrigin::Root.into(), pool_id, 1e7 as Balance,));
		assert_ok!(StableAmm::set_admin_fee(RawOrigin::Root.into(), pool_id, 1e8 as Balance));

		let first_token_balance_before = <Test as Config>::MultiCurrency::free_balance(
			Token(TOKEN1_SYMBOL),
			&pool.admin_fee_receiver,
		);
		let second_token_balance_before = <Test as Config>::MultiCurrency::free_balance(
			Token(TOKEN2_SYMBOL),
			&pool.admin_fee_receiver,
		);

		assert_ok!(StableAmm::withdraw_admin_fee(RawOrigin::Signed(BOB).into(), pool_id));

		let first_token_balance_after = <Test as Config>::MultiCurrency::free_balance(
			Token(TOKEN1_SYMBOL),
			&pool.admin_fee_receiver,
		);
		let second_token_balance_after = <Test as Config>::MultiCurrency::free_balance(
			Token(TOKEN2_SYMBOL),
			&pool.admin_fee_receiver,
		);

		assert_eq!(first_token_balance_before, first_token_balance_after);
		assert_eq!(second_token_balance_before, second_token_balance_after);
	})
}

#[test]
fn withdraw_admin_fee_with_expected_amount_of_fees_should_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, _) = setup_test_base_pool();
		let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();

		assert_ok!(StableAmm::set_swap_fee(RawOrigin::Root.into(), pool_id, 1e7 as Balance,));
		assert_ok!(StableAmm::set_admin_fee(RawOrigin::Root.into(), pool_id, 1e8 as Balance,));

		assert_ok!(StableAmm::swap(
			RawOrigin::Signed(BOB).into(),
			pool_id,
			0,
			1,
			1e17 as Balance,
			0,
			BOB,
			u64::MAX
		));

		assert_ok!(StableAmm::swap(
			RawOrigin::Signed(BOB).into(),
			pool_id,
			1,
			0,
			1e17 as Balance,
			0,
			BOB,
			u64::MAX
		));

		assert_eq!(StableAmm::get_admin_balance(pool_id, 0), Some(1001973776101));
		assert_eq!(StableAmm::get_admin_balance(pool_id, 1), Some(998024139765));

		let first_token_balance_before = <Test as Config>::MultiCurrency::free_balance(
			Token(TOKEN1_SYMBOL),
			&pool.admin_fee_receiver,
		);
		let second_token_balance_before = <Test as Config>::MultiCurrency::free_balance(
			Token(TOKEN2_SYMBOL),
			&pool.admin_fee_receiver,
		);

		assert_ok!(StableAmm::withdraw_admin_fee(RawOrigin::Signed(BOB).into(), pool_id));

		let first_token_balance_after = <Test as Config>::MultiCurrency::free_balance(
			Token(TOKEN1_SYMBOL),
			&pool.admin_fee_receiver,
		);
		let second_token_balance_after = <Test as Config>::MultiCurrency::free_balance(
			Token(TOKEN2_SYMBOL),
			&pool.admin_fee_receiver,
		);

		assert_eq!(first_token_balance_after - first_token_balance_before, 1001973776101);
		assert_eq!(second_token_balance_after - second_token_balance_before, 998024139765);
	})
}

#[test]
fn withdraw_admin_fee_has_no_impact_on_user_withdrawal() {
	new_test_ext().execute_with(|| {
		let (pool_id, _) = setup_test_base_pool();
		let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();

		assert_ok!(StableAmm::set_swap_fee(RawOrigin::Root.into(), pool_id, 1e7 as Balance,));
		assert_ok!(StableAmm::set_admin_fee(RawOrigin::Root.into(), pool_id, 1e8 as Balance,));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			pool_id,
			vec![1e18 as Balance, 1e18 as Balance],
			0,
			BOB,
			u64::MAX
		));

		for _i in 0..10 {
			assert_ok!(StableAmm::swap(
				RawOrigin::Signed(CHARLIE).into(),
				pool_id,
				0,
				1,
				1e17 as Balance,
				0,
				CHARLIE,
				u64::MAX
			));

			assert_ok!(StableAmm::swap(
				RawOrigin::Signed(CHARLIE).into(),
				pool_id,
				1,
				0,
				1e17 as Balance,
				0,
				CHARLIE,
				u64::MAX
			));
		}

		assert_ok!(StableAmm::withdraw_admin_fee(RawOrigin::Signed(BOB).into(), pool_id));

		let first_token_balance_before =
			<Test as Config>::MultiCurrency::free_balance(Token(TOKEN1_SYMBOL), &BOB);
		let second_token_balance_before =
			<Test as Config>::MultiCurrency::free_balance(Token(TOKEN2_SYMBOL), &BOB);

		let pool_token_balance =
			<Test as Config>::MultiCurrency::free_balance(pool.lp_currency_id, &BOB);

		assert_ok!(StableAmm::withdraw_admin_fee(RawOrigin::Signed(BOB).into(), pool_id));

		assert_ok!(StableAmm::remove_liquidity(
			RawOrigin::Signed(BOB).into(),
			pool_id,
			pool_token_balance,
			vec![0, 0],
			BOB,
			u64::MAX,
		));

		let first_token_balance_after =
			<Test as Config>::MultiCurrency::free_balance(Token(TOKEN1_SYMBOL), &BOB);
		let second_token_balance_after =
			<Test as Config>::MultiCurrency::free_balance(Token(TOKEN2_SYMBOL), &BOB);

		assert_eq!(first_token_balance_after - first_token_balance_before, 1000009516257264879);
		assert_eq!(second_token_balance_after - second_token_balance_before, 1000980987206499309);
	})
}

#[test]
fn ramp_a_upwards_should_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, _) = setup_test_base_pool();

		mine_block();

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			pool_id,
			vec![1e18 as Balance, 0],
			0,
			BOB,
			u64::MAX
		));

		mine_block();

		let end_timestamp = Timestamp::now() / 1000 + 14 * DAYS + 1;
		assert_ok!(StableAmm::ramp_a(RawOrigin::Root.into(), pool_id, 100, end_timestamp.into()));

		let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();
		assert_eq!(StableAmm::get_a_precise(&pool), Some(5000));
		assert_eq!(StableAmm::get_virtual_price(pool_id), 1000167146429977312);

		mine_block_with_timestamp(Timestamp::now() / 1000 + 100000);

		let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();
		assert_eq!(StableAmm::get_a_precise(&pool), Some(5413));
		assert_eq!(StableAmm::get_virtual_price(pool_id), 1000258443200231295);

		mine_block_with_timestamp(end_timestamp.into());
		let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();
		assert_eq!(StableAmm::get_a_precise(&pool), Some(10000));
		assert_eq!(StableAmm::get_virtual_price(pool_id), 1000771363829405068);
	})
}

#[test]
fn ramp_a_downward_should_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, _) = setup_test_base_pool();

		mine_block();

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			pool_id,
			vec![1e18 as Balance, 0],
			0,
			BOB,
			u64::MAX
		));

		mine_block();

		let end_timestamp = Timestamp::now() / 1000 + 14 * DAYS + 1;
		assert_ok!(StableAmm::ramp_a(RawOrigin::Root.into(), pool_id, 25, end_timestamp.into()));

		let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();

		assert_eq!(StableAmm::get_a_precise(&pool), Some(5000));
		assert_eq!(StableAmm::get_virtual_price(pool_id), 1000167146429977312);

		mine_block_with_timestamp(Timestamp::now() / 1000 + 100000);

		let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();
		assert_eq!(StableAmm::get_a_precise(&pool), Some(4794));
		assert_eq!(StableAmm::get_virtual_price(pool_id), 1000115870150391894);

		mine_block_with_timestamp(end_timestamp);
		let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();
		assert_eq!(StableAmm::get_a_precise(&pool), Some(2500));
		assert_eq!(StableAmm::get_virtual_price(pool_id), 998999574522335473);
	})
}

#[test]
fn ramp_a_with_non_owner_should_not_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, _) = setup_test_base_pool();

		mine_block();
		let end_timestamp = Timestamp::now() / 1000 + 14 * DAYS + 1;

		assert_noop!(
			StableAmm::ramp_a(RawOrigin::Signed(BOB).into(), pool_id, 55, end_timestamp.into()),
			BadOrigin
		);
	})
}

#[test]
fn ramp_a_not_delay_should_not_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, _) = setup_test_base_pool();
		mine_block();

		let end_timestamp = Timestamp::now() / 1000 + 14 * DAYS + 1;
		assert_ok!(StableAmm::ramp_a(RawOrigin::Root.into(), pool_id, 55, end_timestamp.into()));

		assert_noop!(
			StableAmm::ramp_a(RawOrigin::Root.into(), pool_id, 55, end_timestamp.into()),
			Error::<Test>::RampADelay
		);
	})
}

#[test]
fn ramp_a_out_of_range_should_not_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, _) = setup_test_base_pool();
		mine_block();

		let end_timestamp = Timestamp::now() / 1000 + 14 * DAYS + 1;

		assert_noop!(
			StableAmm::ramp_a(RawOrigin::Root.into(), pool_id, 0, end_timestamp.into()),
			Error::<Test>::ExceedThreshold
		);

		assert_noop!(
			StableAmm::ramp_a(RawOrigin::Root.into(), pool_id, 501, end_timestamp.into()),
			Error::<Test>::ExceedMaxAChange
		);
	})
}

#[test]
fn stop_ramp_a_should_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, _) = setup_test_base_pool();
		mine_block();

		let end_timestamp = Timestamp::now() / 1000 + 14 * DAYS + 100;
		assert_ok!(StableAmm::ramp_a(RawOrigin::Root.into(), pool_id, 100, end_timestamp.into()));

		mine_block_with_timestamp(Timestamp::now() / 1000 + 100000);

		let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();
		assert_eq!(StableAmm::get_a_precise(&pool), Some(5413));

		assert_ok!(StableAmm::stop_ramp_a(RawOrigin::Root.into(), pool_id));
		let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();
		assert_eq!(StableAmm::get_a_precise(&pool), Some(5413));

		mine_block_with_timestamp(end_timestamp);
		let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();
		assert_eq!(StableAmm::get_a_precise(&pool), Some(5413));
	})
}

#[test]
fn stop_ramp_a_repeat_should_not_work() {
	new_test_ext().execute_with(|| {
		let (pool_id, _) = setup_test_base_pool();
		mine_block();

		let end_timestamp = Timestamp::now() / 1000 + 14 * DAYS + 100;
		assert_ok!(StableAmm::ramp_a(RawOrigin::Root.into(), pool_id, 100, end_timestamp.into()));

		mine_block_with_timestamp(Timestamp::now() / 1000 + 100000);

		let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();
		assert_eq!(StableAmm::get_a_precise(&pool), Some(5413));

		assert_ok!(StableAmm::stop_ramp_a(RawOrigin::Root.into(), pool_id));
		let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();
		assert_eq!(StableAmm::get_a_precise(&pool), Some(5413));

		assert_noop!(
			StableAmm::stop_ramp_a(RawOrigin::Root.into(), pool_id),
			Error::<Test>::AlreadyStoppedRampA
		);
	})
}

#[test]
fn check_maximum_differences_in_a_and_virtual_price_when_time_manipulations_and_increasing_a() {
	new_test_ext().execute_with(|| {
		mine_block();

		let (pool_id, _) = setup_test_base_pool();

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(ALICE).into(),
			pool_id,
			vec![1e18 as Balance, 0],
			0,
			ALICE,
			u64::MAX,
		));

		let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();
		assert_eq!(StableAmm::get_a_precise(&pool), Some(5000));
		assert_eq!(StableAmm::get_virtual_price(pool_id), 1000167146429977312);

		let end_timestamp = Timestamp::now() / 1000 + 14 * DAYS + 100;
		assert_ok!(StableAmm::ramp_a(RawOrigin::Root.into(), pool_id, 100, end_timestamp.into()));

		// Malicious miner skips 900 seconds
		set_block_timestamp(Timestamp::now() / 1000 + 900);

		let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();
		assert_eq!(StableAmm::get_a_precise(&pool), Some(5003));
		assert_eq!(StableAmm::get_virtual_price(pool_id), 1000167862696363286);
	})
}

#[test]
fn check_maximum_differences_in_a_and_virtual_price_when_time_manipulations_and_decreasing_a() {
	new_test_ext().execute_with(|| {
		mine_block();

		let (pool_id, _) = setup_test_base_pool();

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(ALICE).into(),
			pool_id,
			vec![1e18 as Balance, 0],
			0,
			ALICE,
			u64::MAX,
		));

		let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();
		assert_eq!(StableAmm::get_a_precise(&pool), Some(5000));
		assert_eq!(StableAmm::get_virtual_price(pool_id), 1000167146429977312);

		let end_timestamp = Timestamp::now() / 1000 + 14 * DAYS + 100;

		assert_ok!(StableAmm::ramp_a(RawOrigin::Root.into(), pool_id, 25, end_timestamp.into()));

		// Malicious miner skips 900 seconds
		set_block_timestamp(Timestamp::now() / 1000 + 900);

		let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();
		assert_eq!(StableAmm::get_a_precise(&pool), Some(4999));
		assert_eq!(StableAmm::get_virtual_price(pool_id), 1000166907487883089);
	})
}

struct AttackContext {
	pub initial_attacker_balances: Vec<Balance>,
	pub initial_pool_balances: Vec<Balance>,
	pub pool_currencies: Vec<CurrencyId>,
	pub attacker: AccountId,
	pub pool_id: PoolId,
}

fn prepare_attack_context(new_a: Balance) -> AttackContext {
	mine_block();

	let (pool_id, _) = setup_test_base_pool();
	let attacker = BOB;
	let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();

	let mut attack_balances = Vec::new();
	for currency_id in pool.currency_ids.iter() {
		attack_balances
			.push(<Test as Config>::MultiCurrency::free_balance(*currency_id, &attacker));
	}

	assert_ok!(StableAmm::ramp_a(
		RawOrigin::Root.into(),
		pool_id,
		new_a,
		(Timestamp::now() / 1000 + 14 * DAYS).into()
	));

	assert_eq!(attack_balances[0], 1e20 as Balance);
	assert_eq!(attack_balances[1], 1e20 as Balance);

	assert_eq!(pool.balances[0], 1e18 as Balance);
	assert_eq!(pool.balances[1], 1e18 as Balance);

	AttackContext {
		initial_attacker_balances: attack_balances,
		initial_pool_balances: pool.balances.clone(),
		pool_currencies: pool.currency_ids.clone(),
		attacker,
		pool_id,
	}
}

#[test]
fn check_when_ramp_a_upwards_and_tokens_price_equally() {
	new_test_ext().execute_with(|| {
		let context = prepare_attack_context(100);

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
fn check_when_ramp_a_upwards_and_tokens_price_unequally() {
	new_test_ext().execute_with(|| {
		let mut context = prepare_attack_context(100);

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
fn check_when_ramp_a_downwards_and_tokens_price_equally() {
	new_test_ext().execute_with(|| {
		let context = prepare_attack_context(25);
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
fn check_when_ramp_a_downwards_and_tokens_price_unequally() {
	new_test_ext().execute_with(|| {
		let mut context = prepare_attack_context(25);

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
fn check_arithmetic_in_add_liquidity_should_successfully() {
	new_test_ext().execute_with(|| {
		let (pool_id, _) = setup_test_base_pool();
		let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();

		mint_more_currencies(
			vec![BOB, CHARLIE],
			pool.currency_ids,
			vec![1_000_000_000e18 as Balance, 1_000_000_000e18 as Balance],
		);

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			0,
			vec![100000000000000000000000, 200000000000000000000000], // [100_000e18, 200_00018]
			0,
			BOB,
			u64::MAX,
		));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(CHARLIE).into(),
			0,
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
			0,
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
fn check_arithmetic_in_remove_liquidity_should_successfully() {
	new_test_ext().execute_with(|| {
		let (pool_id, _) = setup_test_base_pool();
		let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();

		mint_more_currencies(
			vec![BOB, CHARLIE],
			pool.currency_ids.clone(),
			vec![10_000_000_000e18 as Balance, 10_000_000_000e18 as Balance],
		);

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			0,
			vec![100000000000000000000000, 200000000000000000000000], // [100_000e18, 200_00018]
			0,
			BOB,
			u64::MAX,
		));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(CHARLIE).into(),
			0,
			vec![100000000000000000000000000, 300000000000000000000000000], /* [100_000_000e18,
			                                                                 * 300_000_000e18] */
			0,
			CHARLIE,
			u64::MAX,
		));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			0,
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
			pool_id,
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
			pool_id,
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
fn check_arithmetic_in_remove_liquidity_one_currency_should_successfully() {
	new_test_ext().execute_with(|| {
		let (pool_id, _) = setup_test_base_pool();
		let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();
		mint_more_currencies(
			vec![BOB, CHARLIE],
			pool.currency_ids.clone(),
			vec![10_000_000_000e18 as Balance, 10_000_000_000e18 as Balance],
		);

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			0,
			vec![100000000000000000000000, 200000000000000000000000], // [100_000e18, 200_00018]
			0,
			BOB,
			u64::MAX,
		));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(CHARLIE).into(),
			0,
			vec![100000000000000000000000000, 300000000000000000000000000], /* [100_000_000e18,
			                                                                 * 300_000_000e18] */
			0,
			CHARLIE,
			u64::MAX,
		));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			0,
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
			pool_id,
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
			pool_id,
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
fn check_arithmetic_in_remove_liquidity_imbalance_should_successfully() {
	new_test_ext().execute_with(|| {
		let (pool_id, _) = setup_test_base_pool();
		let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();
		mint_more_currencies(
			vec![BOB, CHARLIE],
			pool.currency_ids.clone(),
			vec![10_000_000_000e18 as Balance, 10_000_000_000e18 as Balance],
		);

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			0,
			vec![100000000000000000000000, 200000000000000000000000], // [100_000e18, 200_00018]
			0,
			BOB,
			u64::MAX,
		));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(CHARLIE).into(),
			0,
			vec![100000000000000000000000000, 300000000000000000000000000], /* [100_000_000e18,
			                                                                 * 300_000_000e18] */
			0,
			CHARLIE,
			u64::MAX,
		));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			0,
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
			pool_id,
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
			pool_id,
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
fn check_arithmetic_in_swap_should_successfully() {
	new_test_ext().execute_with(|| {
		let (pool_id, _) = setup_test_base_pool();
		let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();
		mint_more_currencies(
			vec![BOB, CHARLIE],
			pool.currency_ids.clone(),
			vec![10_000_000_000e18 as Balance, 10_000_000_000e18 as Balance],
		);

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			0,
			vec![100000000000000000000000, 200000000000000000000000], // [100_000e18, 200_00018]
			0,
			BOB,
			u64::MAX,
		));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(CHARLIE).into(),
			0,
			vec![100000000000000000000000000, 300000000000000000000000000], /* [100_000_000e18,
			                                                                 * 300_000_000e18] */
			0,
			CHARLIE,
			u64::MAX,
		));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			0,
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
			pool_id,
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
			pool_id,
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

		let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();

		// check pool balances
		assert_eq!(pool.balances[0], 399683318992730412725859548);
		assert_eq!(pool.balances[1], 400817323058344171409290535);

		let pool_token0_balance =
			<Test as Config>::MultiCurrency::free_balance(pool.currency_ids[0], &POOL0ACCOUNTID);
		let pool_token1_balance =
			<Test as Config>::MultiCurrency::free_balance(pool.currency_ids[1], &POOL0ACCOUNTID);
		assert_eq!(pool.balances[0], pool_token0_balance);
		assert_eq!(pool.balances[1], pool_token1_balance);
	})
}

#[test]
fn check_arithmetic_in_add_liquidity_with_admin_fee_should_successfully() {
	new_test_ext().execute_with(|| {
		let (pool_id, _) = setup_test_base_pool();
		let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();
		mint_more_currencies(
			vec![BOB, CHARLIE],
			pool.currency_ids.clone(),
			vec![10_000_000_000e18 as Balance, 10_000_000_000e18 as Balance],
		);

		assert_ok!(StableAmm::set_swap_fee(RawOrigin::Root.into(), pool_id, SWAP_FEE,));
		assert_ok!(StableAmm::set_admin_fee(RawOrigin::Root.into(), pool_id, MAX_ADMIN_FEE,));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			0,
			vec![100000000000000000000000, 200000000000000000000000], // [100_000e18, 200_00018]
			0,
			BOB,
			u64::MAX,
		));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(CHARLIE).into(),
			0,
			vec![100000000000000000000000000, 300000000000000000000000000], /* [100_000_000e18,
			                                                                 * 300_000_000e18] */
			0,
			CHARLIE,
			u64::MAX,
		));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			0,
			vec![300000000000000000000000000, 100000000000000000000000000], /* [300_000_000e18,
			                                                                 * 100_000_000e18] */
			0,
			BOB,
			u64::MAX,
		));

		let admin_balances = StableAmm::get_admin_balances(pool_id);
		assert_eq!(admin_balances[0], 116218703966498771606127);
		assert_eq!(admin_balances[1], 117921007525488838747514);
	})
}

#[test]
fn check_arithmetic_in_remove_liquidity_with_admin_fee_should_successfully() {
	new_test_ext().execute_with(|| {
		let (pool_id, _) = setup_test_base_pool();
		let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();
		mint_more_currencies(
			vec![BOB, CHARLIE],
			pool.currency_ids.clone(),
			vec![10_000_000_000e18 as Balance, 10_000_000_000e18 as Balance],
		);

		assert_ok!(StableAmm::set_swap_fee(RawOrigin::Root.into(), pool_id, SWAP_FEE,));
		assert_ok!(StableAmm::set_admin_fee(RawOrigin::Root.into(), pool_id, MAX_ADMIN_FEE,));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			0,
			vec![100000000000000000000000, 200000000000000000000000], // [100_000e18, 200_00018]
			0,
			BOB,
			u64::MAX,
		));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(CHARLIE).into(),
			0,
			vec![100000000000000000000000000, 300000000000000000000000000], /* [100_000_000e18,
			                                                                 * 300_000_000e18] */
			0,
			CHARLIE,
			u64::MAX,
		));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			0,
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
			pool_id,
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
			pool_id,
			user2_pool_lp_balance_before,
			vec![0, 0],
			CHARLIE,
			u64::MAX
		));

		let admin_balances = StableAmm::get_admin_balances(pool_id);
		assert_eq!(admin_balances[0], 116218703966498771606127);
		assert_eq!(admin_balances[1], 117921007525488838747514);
	})
}

#[test]
fn check_arithmetic_in_remove_liquidity_one_currency_with_admin_fee_should_successfully() {
	new_test_ext().execute_with(|| {
		let (pool_id, _) = setup_test_base_pool();
		let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();
		mint_more_currencies(
			vec![BOB, CHARLIE],
			pool.currency_ids.clone(),
			vec![10_000_000_000e18 as Balance, 10_000_000_000e18 as Balance],
		);

		assert_ok!(StableAmm::set_swap_fee(RawOrigin::Root.into(), pool_id, SWAP_FEE,));
		assert_ok!(StableAmm::set_admin_fee(RawOrigin::Root.into(), pool_id, MAX_ADMIN_FEE,));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			0,
			vec![100000000000000000000000, 200000000000000000000000], // [100_000e18, 200_00018]
			0,
			BOB,
			u64::MAX,
		));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(CHARLIE).into(),
			0,
			vec![100000000000000000000000000, 300000000000000000000000000], /* [100_000_000e18,
			                                                                 * 300_000_000e18] */
			0,
			CHARLIE,
			u64::MAX,
		));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			0,
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
			pool_id,
			user1_pool_lp_balance_before,
			0,
			0,
			BOB,
			u64::MAX
		));

		assert_ok!(StableAmm::remove_liquidity_one_currency(
			RawOrigin::Signed(CHARLIE).into(),
			pool_id,
			user2_pool_lp_balance_before,
			1,
			0,
			CHARLIE,
			u64::MAX
		));

		let admin_balances = StableAmm::get_admin_balances(pool_id);
		assert_eq!(admin_balances[0], 253156563645258671123072);
		assert_eq!(admin_balances[1], 117921008529025874896694);

		let balances = StableAMM::pools(pool_id).unwrap().get_pool_info().balances;
		assert_eq!(balances[0], 17382152193711710630633607);
		assert_eq!(balances[1], 66);
	})
}

#[test]
fn check_arithmetic_in_remove_liquidity_imbalance_with_admin_fee_should_successfully() {
	new_test_ext().execute_with(|| {
		let (pool_id, _) = setup_test_base_pool();
		let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();
		mint_more_currencies(
			vec![BOB, CHARLIE],
			pool.currency_ids.clone(),
			vec![10_000_000_000e18 as Balance, 10_000_000_000e18 as Balance],
		);

		assert_ok!(StableAmm::set_swap_fee(RawOrigin::Root.into(), pool_id, SWAP_FEE,));
		assert_ok!(StableAmm::set_admin_fee(RawOrigin::Root.into(), pool_id, MAX_ADMIN_FEE,));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			0,
			vec![100000000000000000000000, 200000000000000000000000], // [100_000e18, 200_00018]
			0,
			BOB,
			u64::MAX,
		));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(CHARLIE).into(),
			0,
			vec![100000000000000000000000000, 300000000000000000000000000], /* [100_000_000e18,
			                                                                 * 300_000_000e18] */
			0,
			CHARLIE,
			u64::MAX,
		));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			0,
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
			pool_id,
			vec![200000000000000000000000000, 100000000000000000000000000],
			user1_pool_lp_balance_before,
			BOB,
			u64::MAX
		));

		assert_ok!(StableAmm::remove_liquidity_imbalance(
			RawOrigin::Signed(CHARLIE).into(),
			pool_id,
			vec![100000000000000000000000000, 200000000000000000000000000],
			user2_pool_lp_balance_before,
			CHARLIE,
			u64::MAX
		));

		let admin_balances = StableAmm::get_admin_balances(pool_id);
		assert_eq!(admin_balances[0], 151146217664745609762144);
		assert_eq!(admin_balances[1], 152991616465138594784072);

		let balances = StableAMM::pools(pool_id).unwrap().get_pool_info().balances;
		assert_eq!(balances[0], 99948854782335254390237856);
		assert_eq!(balances[1], 100047009383534861405215928);
	})
}

#[test]
fn check_arithmetic_in_swap_imbalance_with_admin_fee_should_successfully() {
	new_test_ext().execute_with(|| {
		let (pool_id, _) = setup_test_base_pool();
		let pool = StableAmm::pools(pool_id).unwrap().get_pool_info();
		mint_more_currencies(
			vec![BOB, CHARLIE],
			pool.currency_ids.clone(),
			vec![10_000_000_000e18 as Balance, 10_000_000_000e18 as Balance],
		);

		assert_ok!(StableAmm::set_swap_fee(RawOrigin::Root.into(), pool_id, SWAP_FEE,));
		assert_ok!(StableAmm::set_admin_fee(RawOrigin::Root.into(), pool_id, MAX_ADMIN_FEE,));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			0,
			vec![100000000000000000000000, 200000000000000000000000], // [100_000e18, 200_00018]
			0,
			BOB,
			u64::MAX,
		));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(CHARLIE).into(),
			0,
			vec![100000000000000000000000000, 300000000000000000000000000], /* [100_000_000e18,
			                                                                 * 300_000_000e18] */
			0,
			CHARLIE,
			u64::MAX,
		));

		assert_ok!(StableAmm::add_liquidity(
			RawOrigin::Signed(BOB).into(),
			0,
			vec![300000000000000000000000000, 100000000000000000000000000], /* [300_000_000e18,
			                                                                 * 100_000_000e18] */
			0,
			BOB,
			u64::MAX,
		));

		assert_ok!(StableAmm::swap(
			RawOrigin::Signed(BOB).into(),
			pool_id,
			0,
			1,
			100000000000000000000000000,
			0,
			BOB,
			u64::MAX
		));

		assert_ok!(StableAmm::swap(
			RawOrigin::Signed(CHARLIE).into(),
			pool_id,
			1,
			0,
			100000000000000000000000000,
			0,
			CHARLIE,
			u64::MAX
		));

		let admin_balances = StableAmm::get_admin_balances(pool_id);
		assert_eq!(admin_balances[0], 216736707365806476948490);
		assert_eq!(admin_balances[1], 217402988477687128132247);

		let balances = StableAMM::pools(pool_id).unwrap().get_pool_info().balances;
		assert_eq!(balances[0], 399465778896725795886030548);
		assert_eq!(balances[1], 400600099040276221776518758);
	})
}
