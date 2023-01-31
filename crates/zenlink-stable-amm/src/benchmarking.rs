// Copyright 2021-2022 Zenlink.
// Licensed under Apache 2.0.

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::Pallet as StablePallet;

use frame_benchmarking::{benchmarks, whitelisted_caller};
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

fn token1<CurrencyId: TryFrom<u64> + Default>() -> CurrencyId {
	CurrencyId::try_from(513u64).unwrap_or_default()
}

fn token2<CurrencyId: TryFrom<u64> + Default>() -> CurrencyId {
	CurrencyId::try_from(514u64).unwrap_or_default()
}

fn token3<CurrencyId: TryFrom<u64> + Default>() -> CurrencyId {
	CurrencyId::try_from(515u64).unwrap_or_default()
}

fn stable_pool_0<CurrencyId: TryFrom<u64> + Default>() -> CurrencyId {
	CurrencyId::try_from(1024u64).unwrap_or_default()
}

benchmarks! {

	where_clause { where T::CurrencyId: TryFrom<u64> + Default}

	create_base_pool{
		let admin_fee_receiver: T::AccountId = whitelisted_caller();
	}:_(RawOrigin::Root,
		[token1::<T::CurrencyId>(), token2::<T::CurrencyId>()].to_vec(),
		[12,12].to_vec(),
		INITIAL_A_VALUE,
		SWAP_FEE,
		ADMIN_FEE,
		admin_fee_receiver,
		Vec::from("stable_pool_lp_0")
	)

	create_meta_pool{
		let caller: T::AccountId = whitelisted_caller();
		assert_ok!(StablePallet::<T>::create_base_pool(
			(RawOrigin::Root).into(),
			[token1::<T::CurrencyId>(), token2::<T::CurrencyId>()].to_vec(),
			[12,12].to_vec(),
			INITIAL_A_VALUE,
			SWAP_FEE,
			ADMIN_FEE,
			caller.clone(),
			Vec::from("stable_pool_lp_0")
		));

		assert_ok!(T::MultiCurrency::deposit(token1::<T::CurrencyId>(), &caller, UNIT * 1000));
		assert_ok!(T::MultiCurrency::deposit(token2::<T::CurrencyId>(), &caller, UNIT * 1000));

		assert_ok!(
			StablePallet::<T>::add_liquidity(
				RawOrigin::Signed(caller.clone()).into(),
				0u32.into(),
				[10*UNIT, 10*UNIT].to_vec(),
				0,
				caller.clone(),
				1000u32.into()
			)
		);

	}:_(RawOrigin::Root,
		[token3::<T::CurrencyId>(), stable_pool_0::<T::CurrencyId>()].to_vec(),
		[12,18].to_vec(),
		INITIAL_A_VALUE,
		SWAP_FEE,
		ADMIN_FEE,
		caller,
		Vec::from("stable_pool_lp_1")
	)

	add_liquidity{
		let caller: T::AccountId = whitelisted_caller();
		assert_ok!(StablePallet::<T>::create_base_pool(
			(RawOrigin::Root).into(),
			[token1::<T::CurrencyId>(), token2::<T::CurrencyId>()].to_vec(),
			[12,12].to_vec(),
			INITIAL_A_VALUE,
			SWAP_FEE,
			ADMIN_FEE,
			caller.clone(),
			Vec::from("stable_pool_lp_0")
		));

		assert_ok!(T::MultiCurrency::deposit(token1::<T::CurrencyId>(), &caller, UNIT * 1000));
		assert_ok!(T::MultiCurrency::deposit(token2::<T::CurrencyId>(), &caller, UNIT * 1000));

	}:_(RawOrigin::Signed(caller.clone()), 0u32.into(), [10*UNIT, 10*UNIT].to_vec(), 0, caller.clone(),1000u32.into())

	swap{
		let caller: T::AccountId = whitelisted_caller();
		assert_ok!(StablePallet::<T>::create_base_pool(
			(RawOrigin::Root).into(),
			[token1::<T::CurrencyId>(), token2::<T::CurrencyId>()].to_vec(),
			[12,12].to_vec(),
			INITIAL_A_VALUE,
			SWAP_FEE,
			ADMIN_FEE,
			caller.clone(),
			Vec::from("stable_pool_lp_0")
		));

		assert_ok!(T::MultiCurrency::deposit(token1::<T::CurrencyId>(), &caller, UNIT * 1000));
		assert_ok!(T::MultiCurrency::deposit(token2::<T::CurrencyId>(), &caller, UNIT * 1000));

		assert_ok!(
			StablePallet::<T>::add_liquidity(
				RawOrigin::Signed(caller.clone()).into(),
				0u32.into(),
				[10*UNIT, 10*UNIT].to_vec(),
				0,
				caller.clone(),
				1000u32.into()
			)
		);

	}:_(RawOrigin::Signed(caller.clone()),
		0u32.into(),
		0u32,
		1u32,
		1 * UNIT,
		0,
		caller.clone(),
		1000u32.into()
	)

	remove_liquidity{
		let caller: T::AccountId = whitelisted_caller();
		assert_ok!(StablePallet::<T>::create_base_pool(
			(RawOrigin::Root).into(),
			[token1::<T::CurrencyId>(), token2::<T::CurrencyId>()].to_vec(),
			[12,12].to_vec(),
			INITIAL_A_VALUE,
			SWAP_FEE,
			ADMIN_FEE,
			caller.clone(),
			Vec::from("stable_pool_lp_0")
		));

		assert_ok!(T::MultiCurrency::deposit(token1::<T::CurrencyId>(), &caller, UNIT * 1000));
		assert_ok!(T::MultiCurrency::deposit(token2::<T::CurrencyId>(), &caller, UNIT * 1000));

		assert_ok!(
			StablePallet::<T>::add_liquidity(
				RawOrigin::Signed(caller.clone()).into(),
				0u32.into(),
				[10*UNIT, 10*UNIT].to_vec(),
				0,
				caller.clone(),
				1000u32.into()
			)
		);

	}:_(RawOrigin::Signed(caller.clone()),
		0u32.into(),
		1 * UNIT,
		[0,0].to_vec(),
		caller.clone(),
		1000u32.into()
	)

	remove_liquidity_one_currency{
		let caller: T::AccountId = whitelisted_caller();
		assert_ok!(StablePallet::<T>::create_base_pool(
			(RawOrigin::Root).into(),
			[token1::<T::CurrencyId>(), token2::<T::CurrencyId>()].to_vec(),
			[12,12].to_vec(),
			INITIAL_A_VALUE,
			SWAP_FEE,
			ADMIN_FEE,
			caller.clone(),
			Vec::from("stable_pool_lp_0")
		));

		assert_ok!(T::MultiCurrency::deposit(token1::<T::CurrencyId>(), &caller, UNIT * 1000));
		assert_ok!(T::MultiCurrency::deposit(token2::<T::CurrencyId>(), &caller, UNIT * 1000));

		assert_ok!(
			StablePallet::<T>::add_liquidity(
				RawOrigin::Signed(caller.clone()).into(),
				0u32.into(),
				[10*UNIT, 10*UNIT].to_vec(),
				0,
				caller.clone(),
				1000u32.into()
			)
		);

	}:_(RawOrigin::Signed(caller.clone()),
		0u32.into(),
		1 * UNIT,
		1,
		0,
		caller.clone(),
		1000u32.into()
	)

	remove_liquidity_imbalance{
		let caller: T::AccountId = whitelisted_caller();
		assert_ok!(StablePallet::<T>::create_base_pool(
			(RawOrigin::Root).into(),
			[token1::<T::CurrencyId>(), token2::<T::CurrencyId>()].to_vec(),
			[12,12].to_vec(),
			INITIAL_A_VALUE,
			SWAP_FEE,
			ADMIN_FEE,
			caller.clone(),
			Vec::from("stable_pool_lp_0")
		));

		assert_ok!(T::MultiCurrency::deposit(token1::<T::CurrencyId>(), &caller, UNIT * 1000));
		assert_ok!(T::MultiCurrency::deposit(token2::<T::CurrencyId>(), &caller, UNIT * 1000));

		assert_ok!(
			StablePallet::<T>::add_liquidity(
				RawOrigin::Signed(caller.clone()).into(),
				0u32.into(),
				[100*UNIT, 100*UNIT].to_vec(),
				0,
				caller.clone(),
				1000u32.into()
			)
		);

	}:_(RawOrigin::Signed(caller.clone()),
		0u32.into(),
		[10 * UNIT, 1*UNIT].to_vec(),
		20 * LP_UNIT,
		caller.clone(),
		1000u32.into()
	)

	add_pool_and_base_pool_liquidity{
		let caller: T::AccountId = whitelisted_caller();
		assert_ok!(StablePallet::<T>::create_base_pool(
			(RawOrigin::Root).into(),
			[token1::<T::CurrencyId>(), token2::<T::CurrencyId>()].to_vec(),
			[12,12].to_vec(),
			INITIAL_A_VALUE,
			SWAP_FEE,
			ADMIN_FEE,
			caller.clone(),
			Vec::from("stable_pool_lp_0")
		));

		assert_ok!(T::MultiCurrency::deposit(token1::<T::CurrencyId>(), &caller, UNIT * 1000));
		assert_ok!(T::MultiCurrency::deposit(token2::<T::CurrencyId>(), &caller, UNIT * 1000));
		assert_ok!(T::MultiCurrency::deposit(token3::<T::CurrencyId>(), &caller, UNIT * 1000));

		assert_ok!(
			StablePallet::<T>::add_liquidity(
				RawOrigin::Signed(caller.clone()).into(),
				0u32.into(),
				[10*UNIT, 10*UNIT].to_vec(),
				0,
				caller.clone(),
				1000u32.into()
			)
		);

		assert_ok!(StablePallet::<T>::create_meta_pool(
			(RawOrigin::Root).into(),
			[token3::<T::CurrencyId>(), stable_pool_0::<T::CurrencyId>()].to_vec(),
			[12,18].to_vec(),
			INITIAL_A_VALUE,
			SWAP_FEE,
			ADMIN_FEE,
			caller.clone(),
			Vec::from("stable_pool_lp_1")
		));

	}:_(
		RawOrigin::Signed(caller.clone()),
		1u32.into(),
		0u32.into(),
		[9*UNIT, 1* UNIT].to_vec(),
		[10* UNIT, 10 * UNIT].to_vec(),
		0,
		caller.clone(),
		1000u32.into()
	)

	remove_pool_and_base_pool_liquidity{
		let caller: T::AccountId = whitelisted_caller();
		assert_ok!(StablePallet::<T>::create_base_pool(
			(RawOrigin::Root).into(),
			[token1::<T::CurrencyId>(), token2::<T::CurrencyId>()].to_vec(),
			[12,12].to_vec(),
			INITIAL_A_VALUE,
			SWAP_FEE,
			ADMIN_FEE,
			caller.clone(),
			Vec::from("stable_pool_lp_0")
		));

		assert_ok!(T::MultiCurrency::deposit(token1::<T::CurrencyId>(), &caller, UNIT * 1000));
		assert_ok!(T::MultiCurrency::deposit(token2::<T::CurrencyId>(), &caller, UNIT * 1000));
		assert_ok!(T::MultiCurrency::deposit(token3::<T::CurrencyId>(), &caller, UNIT * 1000));

		assert_ok!(
			StablePallet::<T>::add_liquidity(
				RawOrigin::Signed(caller.clone()).into(),
				0u32.into(),
				[10*UNIT, 10*UNIT].to_vec(),
				0,
				caller.clone(),
				1000u32.into()
			)
		);

		assert_ok!(StablePallet::<T>::create_meta_pool(
			(RawOrigin::Root).into(),
			[token3::<T::CurrencyId>(), stable_pool_0::<T::CurrencyId>()].to_vec(),
			[12,18].to_vec(),
			INITIAL_A_VALUE,
			SWAP_FEE,
			ADMIN_FEE,
			caller.clone(),
			Vec::from("stable_pool_lp_1")
		));

		assert_ok!(
			StablePallet::<T>::add_liquidity(
				RawOrigin::Signed(caller.clone()).into(),
				1u32.into(),
				[5*UNIT, 5*LP_UNIT].to_vec(),
				0,
				caller.clone(),
				1000u32.into()
			)
		);

	}:_(
		RawOrigin::Signed(caller.clone()),
		1u32.into(),
		0u32.into(),
		10 * LP_UNIT,
		[2*UNIT, 2* UNIT].to_vec(),
		[2* UNIT, 2 * UNIT].to_vec(),
		caller.clone(),
		1000u32.into()
	)

	remove_pool_and_base_pool_liquidity_one_currency{
		let caller: T::AccountId = whitelisted_caller();
		assert_ok!(StablePallet::<T>::create_base_pool(
			(RawOrigin::Root).into(),
			[token1::<T::CurrencyId>(), token2::<T::CurrencyId>()].to_vec(),
			[12,12].to_vec(),
			INITIAL_A_VALUE,
			SWAP_FEE,
			ADMIN_FEE,
			caller.clone(),
			Vec::from("stable_pool_lp_0")
		));

		assert_ok!(T::MultiCurrency::deposit(token1::<T::CurrencyId>(), &caller, UNIT * 1000));
		assert_ok!(T::MultiCurrency::deposit(token2::<T::CurrencyId>(), &caller, UNIT * 1000));
		assert_ok!(T::MultiCurrency::deposit(token3::<T::CurrencyId>(), &caller, UNIT * 1000));

		assert_ok!(
			StablePallet::<T>::add_liquidity(
				RawOrigin::Signed(caller.clone()).into(),
				0u32.into(),
				[10*UNIT, 10*UNIT].to_vec(),
				0,
				caller.clone(),
				1000u32.into()
			)
		);

		assert_ok!(StablePallet::<T>::create_meta_pool(
			(RawOrigin::Root).into(),
			[token3::<T::CurrencyId>(), stable_pool_0::<T::CurrencyId>()].to_vec(),
			[12,18].to_vec(),
			INITIAL_A_VALUE,
			SWAP_FEE,
			ADMIN_FEE,
			caller.clone(),
			Vec::from("stable_pool_lp_1")
		));

		assert_ok!(
			StablePallet::<T>::add_liquidity(
				RawOrigin::Signed(caller.clone()).into(),
				1u32.into(),
				[5*UNIT, 5*LP_UNIT].to_vec(),
				0,
				caller.clone(),
				1000u32.into()
			)
		);

	}:_(
		RawOrigin::Signed(caller.clone()),
		1u32.into(),
		0u32.into(),
		10 * LP_UNIT,
		0,
		0,
		caller.clone(),
		1000u32.into()
	)

	swap_pool_from_base{
		let caller: T::AccountId = whitelisted_caller();
		assert_ok!(StablePallet::<T>::create_base_pool(
			(RawOrigin::Root).into(),
			[token1::<T::CurrencyId>(), token2::<T::CurrencyId>()].to_vec(),
			[12,12].to_vec(),
			INITIAL_A_VALUE,
			SWAP_FEE,
			ADMIN_FEE,
			caller.clone(),
			Vec::from("stable_pool_lp_0")
		));

		assert_ok!(T::MultiCurrency::deposit(token1::<T::CurrencyId>(), &caller, UNIT * 1000));
		assert_ok!(T::MultiCurrency::deposit(token2::<T::CurrencyId>(), &caller, UNIT * 1000));
		assert_ok!(T::MultiCurrency::deposit(token3::<T::CurrencyId>(), &caller, UNIT * 1000));

		assert_ok!(
			StablePallet::<T>::add_liquidity(
				RawOrigin::Signed(caller.clone()).into(),
				0u32.into(),
				[10*UNIT, 10*UNIT].to_vec(),
				0,
				caller.clone(),
				1000u32.into()
			)
		);

		assert_ok!(StablePallet::<T>::create_meta_pool(
			(RawOrigin::Root).into(),
			[token3::<T::CurrencyId>(), stable_pool_0::<T::CurrencyId>()].to_vec(),
			[12,18].to_vec(),
			INITIAL_A_VALUE,
			SWAP_FEE,
			ADMIN_FEE,
			caller.clone(),
			Vec::from("stable_pool_lp_1")
		));

		assert_ok!(
			StablePallet::<T>::add_liquidity(
				RawOrigin::Signed(caller.clone()).into(),
				1u32.into(),
				[5*UNIT, 5*LP_UNIT].to_vec(),
				0,
				caller.clone(),
				1000u32.into()
			)
		);
	}:_(
		RawOrigin::Signed(caller.clone()),
		1u32.into(),
		0u32.into(),
		0,
		0,
		1*UNIT,
		0,
		caller.clone(),
		1000u32.into()
	)

	swap_pool_to_base{
		let caller: T::AccountId = whitelisted_caller();
		assert_ok!(StablePallet::<T>::create_base_pool(
			(RawOrigin::Root).into(),
			[token1::<T::CurrencyId>(), token2::<T::CurrencyId>()].to_vec(),
			[12,12].to_vec(),
			INITIAL_A_VALUE,
			SWAP_FEE,
			ADMIN_FEE,
			caller.clone(),
			Vec::from("stable_pool_lp_0")
		));

		assert_ok!(T::MultiCurrency::deposit(token1::<T::CurrencyId>(), &caller, UNIT * 1000));
		assert_ok!(T::MultiCurrency::deposit(token2::<T::CurrencyId>(), &caller, UNIT * 1000));
		assert_ok!(T::MultiCurrency::deposit(token3::<T::CurrencyId>(), &caller, UNIT * 1000));

		assert_ok!(
			StablePallet::<T>::add_liquidity(
				RawOrigin::Signed(caller.clone()).into(),
				0u32.into(),
				[10*UNIT, 10*UNIT].to_vec(),
				0,
				caller.clone(),
				1000u32.into()
			)
		);

		assert_ok!(StablePallet::<T>::create_meta_pool(
			(RawOrigin::Root).into(),
			[token3::<T::CurrencyId>(), stable_pool_0::<T::CurrencyId>()].to_vec(),
			[12,18].to_vec(),
			INITIAL_A_VALUE,
			SWAP_FEE,
			ADMIN_FEE,
			caller.clone(),
			Vec::from("stable_pool_lp_1")
		));

		assert_ok!(
			StablePallet::<T>::add_liquidity(
				RawOrigin::Signed(caller.clone()).into(),
				1u32.into(),
				[5*UNIT, 5*LP_UNIT].to_vec(),
				0,
				caller.clone(),
				1000u32.into()
			)
		);
	}:_(
		RawOrigin::Signed(caller.clone()),
		1u32.into(),
		0u32.into(),
		0,
		0,
		1*UNIT,
		0,
		caller.clone(),
		1000u32.into()
	)

	swap_meta_pool_underlying{
		let caller: T::AccountId = whitelisted_caller();
		assert_ok!(StablePallet::<T>::create_base_pool(
			(RawOrigin::Root).into(),
			[token1::<T::CurrencyId>(), token2::<T::CurrencyId>()].to_vec(),
			[12,12].to_vec(),
			INITIAL_A_VALUE,
			SWAP_FEE,
			ADMIN_FEE,
			caller.clone(),
			Vec::from("stable_pool_lp_0")
		));

		assert_ok!(T::MultiCurrency::deposit(token1::<T::CurrencyId>(), &caller, UNIT * 1000));
		assert_ok!(T::MultiCurrency::deposit(token2::<T::CurrencyId>(), &caller, UNIT * 1000));
		assert_ok!(T::MultiCurrency::deposit(token3::<T::CurrencyId>(), &caller, UNIT * 1000));

		assert_ok!(
			StablePallet::<T>::add_liquidity(
				RawOrigin::Signed(caller.clone()).into(),
				0u32.into(),
				[10*UNIT, 10*UNIT].to_vec(),
				0,
				caller.clone(),
				1000u32.into()
			)
		);

		assert_ok!(StablePallet::<T>::create_meta_pool(
			(RawOrigin::Root).into(),
			[token3::<T::CurrencyId>(), stable_pool_0::<T::CurrencyId>()].to_vec(),
			[12,18].to_vec(),
			INITIAL_A_VALUE,
			SWAP_FEE,
			ADMIN_FEE,
			caller.clone(),
			Vec::from("stable_pool_lp_1")
		));

		assert_ok!(
			StablePallet::<T>::add_liquidity(
				RawOrigin::Signed(caller.clone()).into(),
				1u32.into(),
				[5*UNIT, 5*LP_UNIT].to_vec(),
				0,
				caller.clone(),
				1000u32.into()
			)
		);
	}:_(
		RawOrigin::Signed(caller.clone()),
		1u32.into(),
		0,
		2,
		1*UNIT,
		0,
		caller.clone(),
		1000u32.into()
	)

	withdraw_admin_fee{
		let caller: T::AccountId = whitelisted_caller();
		assert_ok!(StablePallet::<T>::create_base_pool(
			(RawOrigin::Root).into(),
			[token1::<T::CurrencyId>(), token2::<T::CurrencyId>()].to_vec(),
			[12,12].to_vec(),
			INITIAL_A_VALUE,
			SWAP_FEE,
			100000000,
			caller.clone(),
			Vec::from("stable_pool_lp_0")
		));

		assert_ok!(T::MultiCurrency::deposit(token1::<T::CurrencyId>(), &caller, UNIT * 1000));
		assert_ok!(T::MultiCurrency::deposit(token2::<T::CurrencyId>(), &caller, UNIT * 1000));

		assert_ok!(
			StablePallet::<T>::add_liquidity(
				RawOrigin::Signed(caller.clone()).into(),
				0u32.into(),
				[10*UNIT, 10*UNIT].to_vec(),
				0,
				caller.clone(),
				1000u32.into()
			)
		);

		assert_ok!(StablePallet::<T>::swap(
			RawOrigin::Signed(caller.clone()).into(),
			0u32.into(),
			0u32,
			1u32,
			1 * UNIT,
			0,
			caller.clone(),
			1000u32.into()
		));

	}:_(
		RawOrigin::Signed(caller.clone()),
		0u32.into()
	)
}
