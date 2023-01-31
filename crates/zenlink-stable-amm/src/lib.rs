// Copyright 2021-2022 Zenlink.
// Licensed under Apache 2.0.

//! # Stable AMM Pallet
//!
//! Based on the Curve V1 StableSwap architecture.
//!
//! There are two categories of pool:
//! - Basic: pairs two or more stablecoins
//! - Meta: pairs stablecoins with the LP token of another base pool
//!
//! ## Overview
//!
//! This pallet provides functionality for:
//!
//! - Creating pools
//! - Adding / removing liquidity
//! - Swapping currencies
//! - Ramping of A
//!
//! ### Terminology
//!
//! - **Amplification Coefficient:** This determines a pool's tolerance for imbalance.
//!
//! - **Swap Fee:** The fee taken from the output currency.
//!
//! - **Admin Fee:** The percentage of the fee taken from the swap fee, claimable by the pool owner.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]

pub mod rpc;
pub mod traits;

#[cfg(test)]
mod base_pool_tests;
#[cfg(any(feature = "runtime-benchmarks", test))]
pub mod benchmarking;
#[cfg(test)]
mod meta_pool_tests;
#[cfg(test)]
mod mock;

mod base_pool;
mod meta_pool;
mod primitives;
mod utils;
mod weights;

use frame_support::{
	dispatch::{Codec, DispatchResult},
	pallet_prelude::*,
	traits::UnixTime,
	transactional, PalletId,
};
use orml_traits::MultiCurrency;
use sp_arithmetic::traits::{checked_pow, AtLeast32BitUnsigned, CheckedAdd, One, Zero};
use sp_core::U256;
use sp_runtime::traits::{AccountIdConversion, StaticLookup};
use sp_std::{ops::Sub, vec, vec::Vec};

pub use pallet::*;
use primitives::*;
use traits::{StablePoolLpCurrencyIdGenerate, ValidateCurrency};
pub use weights::WeightInfo;

#[allow(type_alias_bounds)]
type AccountIdOf<T: Config> = <T as frame_system::Config>::AccountId;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_system::pallet_prelude::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The currency ID type
		type CurrencyId: Parameter
			+ Member
			+ Copy
			+ MaybeSerializeDeserialize
			+ Ord
			+ TypeInfo
			+ MaxEncodedLen;

		/// The trait control all currencies
		type MultiCurrency: MultiCurrency<
			AccountIdOf<Self>,
			CurrencyId = Self::CurrencyId,
			Balance = Balance,
		>;

		/// The pool ID type
		type PoolId: Parameter + Codec + Copy + Ord + AtLeast32BitUnsigned + Zero + One + Default;

		/// The trait verify currency for some scenes.
		type EnsurePoolAsset: ValidateCurrency<Self::CurrencyId>;

		type LpGenerate: StablePoolLpCurrencyIdGenerate<Self::CurrencyId, Self::PoolId>;

		/// The trait get timestamp of chain.
		type TimeProvider: UnixTime;

		#[pallet::constant]
		type PoolCurrencySymbolLimit: Get<u32>;

		/// This pallet ID.
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// The id of next pool
	#[pallet::storage]
	#[pallet::getter(fn next_pool_id)]
	pub type NextPoolId<T: Config> = StorageValue<_, T::PoolId, ValueQuery>;

	/// Info of a pool.
	#[pallet::storage]
	#[pallet::getter(fn pools)]
	pub type Pools<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::PoolId,
		Pool<T::PoolId, T::CurrencyId, T::AccountId, BoundedVec<u8, T::PoolCurrencySymbolLimit>>,
	>;

	/// The pool id corresponding to lp currency
	#[pallet::storage]
	#[pallet::getter(fn lp_currencies)]
	pub type LpCurrencies<T: Config> = StorageMap<_, Blake2_128Concat, T::CurrencyId, T::PoolId>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A pool was created.
		CreatePool {
			pool_id: T::PoolId,
			currency_ids: Vec<T::CurrencyId>,
			lp_currency_id: T::CurrencyId,
			a: Number,
			account: T::AccountId,
			admin_fee_receiver: T::AccountId,
		},
		/// A pool's admin_fee_receiver was updated.
		UpdateAdminFeeReceiver { pool_id: T::PoolId, admin_fee_receiver: T::AccountId },
		/// Supply some liquidity to a pool.
		AddLiquidity {
			pool_id: T::PoolId,
			who: T::AccountId,
			to: T::AccountId,
			supply_amounts: Vec<Balance>,
			fees: Vec<Balance>,
			new_d: Balance,
			mint_amount: Balance,
		},
		/// Swap a amounts of currency to get other.
		CurrencyExchange {
			pool_id: T::PoolId,
			who: T::AccountId,
			to: T::AccountId,
			in_index: u32,
			in_amount: Balance,
			out_index: u32,
			out_amount: Balance,
		},
		/// Remove some liquidity from a pool.
		RemoveLiquidity {
			pool_id: T::PoolId,
			who: T::AccountId,
			to: T::AccountId,
			amounts: Vec<Balance>,
			fees: Vec<Balance>,
			new_total_supply: Balance,
		},
		/// Remove some liquidity from a pool to get only one currency.
		RemoveLiquidityOneCurrency {
			pool_id: T::PoolId,
			who: T::AccountId,
			to: T::AccountId,
			out_index: u32,
			burn_amount: Balance,
			out_amount: Balance,
		},
		/// Remove liquidity from a pool with specify the amounts of currencies to be obtained.
		RemoveLiquidityImbalance {
			pool_id: T::PoolId,
			who: T::AccountId,
			to: T::AccountId,
			amounts: Vec<Balance>,
			fees: Vec<Balance>,
			new_d: Balance,
			new_total_supply: Balance,
		},
		/// A pool's swap fee parameters was updated
		NewSwapFee { pool_id: T::PoolId, new_swap_fee: Number },
		/// A pool's admin fee parameters was updated
		NewAdminFee { pool_id: T::PoolId, new_admin_fee: Number },
		/// A pool's 'A' was ramped.
		RampA {
			pool_id: T::PoolId,
			initial_a_precise: Number,
			future_a_precise: Number,
			now: Number,
			future_a_time: Number,
		},
		/// A pool's ramping A was stopped.
		StopRampA { pool_id: T::PoolId, current_a: Number, now: Number },
		/// A pool's admin fee was collected.
		CollectProtocolFee { pool_id: T::PoolId, currency_id: T::CurrencyId, fee_amount: Balance },

		CurrencyExchangeUnderlying {
			pool_id: T::PoolId,
			account: T::AccountId,
			in_amount: Balance,
			out_amount: Balance,
			currency_index_from: u32,
			currency_index_to: u32,
			to: T::AccountId,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The currency id can't join stable amm pool.
		InvalidPooledCurrency,
		/// The currency id can't become the lp currency id of stable amm pool.
		InvalidLpCurrency,
		/// The parameters of a call are contradictory.
		MismatchParameter,
		/// The decimal of currency is invalid when create pool.
		InvalidCurrencyDecimal,
		/// The pool id is invalid.
		InvalidPoolId,
		/// The base pool mismatch this pool.
		InvalidBasePool,
		/// The error generate by some arithmetic function.
		Arithmetic,
		/// The call already expired.
		Deadline,
		/// The caller does not have enough currencies.
		InsufficientSupply,
		/// The pool does not have enough currencies.
		InsufficientReserve,
		/// The new d below then older.
		CheckDFailed,
		/// Slippage is too large.
		AmountSlippage,
		/// Forbid swap same currency.
		SwapSameCurrency,
		/// The index of currency id bigger the length of pool's currencies;
		CurrencyIndexOutRange,
		/// The pool does not have enough lp currency.
		InsufficientLpReserve,
		/// The setting value exceed threshold.
		ExceedThreshold,
		/// The A of this pool is already ramped in current period.
		RampADelay,
		/// The value of feature_a_time is too small.
		MinRampTime,
		/// Forbid change A of a pool bigger than MAX_A.
		ExceedMaxAChange,
		/// The ramping A of this pool is already stopped.
		AlreadyStoppedRampA,
		/// The fee parameter exceeds MAX_SWAP_FEE when create pool.
		ExceedMaxFee,
		/// The admin fee parameter exceeds MAX_ADMIN_FEE when create pool.
		ExceedMaxAdminFee,
		/// The A parameter exceed MAX_A when create pool.
		ExceedMaxA,
		/// The lp currency id is already used when create pool.
		LpCurrencyAlreadyUsed,
		/// Require all currencies of this pool when first supply.
		RequireAllCurrencies,
		/// The symbol of created pool maybe exceed length limit.
		BadPoolCurrencySymbol,
		/// The transaction change nothing.
		InvalidTransaction,
		/// The base pool lp currency is invalid when create meta pool.
		InvalidBasePoolLpCurrency,
		/// The token index out of range.
		TokenIndexOutOfRange,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Create a stable amm pool.
		///
		/// Only admin can create pool.
		///
		/// # Argument
		///
		/// - `currency_ids`: The currencies will be join the created pool.
		/// - `currency_decimals`: The currencies corresponding decimals.
		/// - `lp_currency_id`: The specify lp currency id of the created pool.
		/// - `a`: The initial A of created pool.
		/// - `fee`: The swap fee of created pool.
		/// - `admin_fee`: The admin fee of created pool.
		/// - `admin_fee_receiver`: The admin fee receiver of created pool.
		/// - `lp_currency_symbol`: The symbol of created pool lp currency.
		/// - `lp_currency_decimal`: The decimal of created pool lp currency.
		#[pallet::weight(T::WeightInfo::create_base_pool())]
		#[transactional]
		pub fn create_base_pool(
			origin: OriginFor<T>,
			currency_ids: Vec<T::CurrencyId>,
			currency_decimals: Vec<u32>,
			a: Number,
			fee: Number,
			admin_fee: Number,
			admin_fee_receiver: T::AccountId,
			lp_currency_symbol: Vec<u8>,
		) -> DispatchResult {
			ensure_root(origin)?;

			let (new_pool, pool_id) = Self::inner_create_base_pool(
				&currency_ids,
				currency_decimals,
				a,
				fee,
				admin_fee,
				&admin_fee_receiver,
				lp_currency_symbol,
			)?;

			LpCurrencies::<T>::insert(new_pool.lp_currency_id, pool_id);

			NextPoolId::<T>::try_mutate(|pool_id| -> DispatchResult {
				*pool_id = pool_id.checked_add(&One::one()).ok_or(Error::<T>::Arithmetic)?;
				Ok(())
			})?;

			Pools::<T>::try_mutate(pool_id, |pool_info| -> DispatchResult {
				ensure!(pool_info.is_none(), Error::<T>::InvalidPoolId);
				let lp_currency_id = new_pool.lp_currency_id;
				let pool_account = new_pool.account.clone();

				*pool_info = Some(Pool::Base(new_pool));

				Self::deposit_event(Event::CreatePool {
					pool_id,
					currency_ids,
					lp_currency_id,
					a,
					account: pool_account,
					admin_fee_receiver,
				});

				Ok(())
			})
		}

		/// Create a stable amm meta pool.
		///
		/// Only admin can create pool.
		///
		/// # Argument
		///
		/// - `currency_ids`: The currencies will be join the created pool.
		/// - `currency_decimals`: The currencies corresponding decimals.
		/// - `lp_currency_id`: The specify lp currency id of the created pool.
		/// - `a`: The initial A of created pool.
		/// - `fee`: The swap fee of created pool.
		/// - `admin_fee`: The admin fee of created pool.
		/// - `admin_fee_receiver`: The admin fee receiver of created pool.
		/// - `lp_currency_symbol`: The symbol of created pool lp currency.
		/// - `lp_currency_decimal`: The decimal of created pool lp currency.
		#[pallet::weight(T::WeightInfo::create_meta_pool())]
		#[transactional]
		pub fn create_meta_pool(
			origin: OriginFor<T>,
			currency_ids: Vec<T::CurrencyId>,
			currency_decimals: Vec<u32>,
			a: Number,
			fee: Number,
			admin_fee: Number,
			admin_fee_receiver: T::AccountId,
			lp_currency_symbol: Vec<u8>,
		) -> DispatchResult {
			ensure_root(origin)?;
			let base_pool_lp_currency = currency_ids.last().ok_or(Error::<T>::MismatchParameter)?;
			let base_pool_id = Self::lp_currencies(base_pool_lp_currency)
				.ok_or(Error::<T>::InvalidBasePoolLpCurrency)?;

			let (meta_pool_info, pool_id) = Self::inner_create_base_pool(
				&currency_ids,
				currency_decimals,
				a,
				fee,
				admin_fee,
				&admin_fee_receiver,
				lp_currency_symbol,
			)?;

			let base_pool =
				Self::pools(base_pool_id).ok_or(Error::<T>::InvalidBasePoolLpCurrency)?;

			let base_pool_virtual_price =
				Self::get_pool_virtual_price(&base_pool).ok_or(Error::<T>::Arithmetic)?;

			let meta_pool = MetaPool {
				base_pool_id,
				base_virtual_price: base_pool_virtual_price,
				base_cache_last_updated: T::TimeProvider::now().as_secs(),
				base_currencies: base_pool.get_currency_ids(),
				info: meta_pool_info,
			};

			LpCurrencies::<T>::insert(meta_pool.info.lp_currency_id, pool_id);

			NextPoolId::<T>::try_mutate(|pool_id| -> DispatchResult {
				*pool_id = pool_id.checked_add(&One::one()).ok_or(Error::<T>::Arithmetic)?;
				Ok(())
			})?;

			Pools::<T>::try_mutate(pool_id, |pool_info| -> DispatchResult {
				ensure!(pool_info.is_none(), Error::<T>::InvalidPoolId);
				let lp_currency_id = meta_pool.info.lp_currency_id;
				let pool_account = meta_pool.info.account.clone();

				*pool_info = Some(Pool::Meta(meta_pool));

				Self::deposit_event(Event::CreatePool {
					pool_id,
					currency_ids,
					lp_currency_id,
					a,
					account: pool_account,
					admin_fee_receiver,
				});
				Ok(())
			})
		}

		/// Supply amounts of currencies to the pool.
		///
		/// # Argument
		///
		/// - `pool_id`: The id of pool.
		/// - `amounts`: Supply amounts of currencies.
		/// - `min_mint_amount`: The min amount of lp currency get.
		/// - `deadline`: Height of the cutoff block of this transaction
		#[pallet::weight(T::WeightInfo::add_liquidity())]
		#[transactional]
		pub fn add_liquidity(
			origin: OriginFor<T>,
			pool_id: T::PoolId,
			amounts: Vec<Balance>,
			min_mint_amount: Balance,
			to: T::AccountId,
			deadline: T::BlockNumber,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let now = frame_system::Pallet::<T>::block_number();
			ensure!(deadline > now, Error::<T>::Deadline);

			Self::inner_add_liquidity(&who, pool_id, &amounts, min_mint_amount, &to)?;

			Ok(())
		}

		/// Swap a amounts of currencies to get other.
		///
		/// # Argument
		///
		/// - `pool_id`: The id of pool.
		/// - `from_index`: The index of swap currency id.
		/// - `to_index`: The index of receive currency id.
		/// - `in_amount`: The amounts of currencies swap.
		/// - `min_mint_amount`: The min amount of receive currency.
		/// - `deadline`: Height of the cutoff block of this transaction
		#[pallet::weight(T::WeightInfo::swap())]
		#[transactional]
		pub fn swap(
			origin: OriginFor<T>,
			poo_id: T::PoolId,
			from_index: u32,
			to_index: u32,
			in_amount: Balance,
			min_out_amount: Balance,
			to: T::AccountId,
			deadline: T::BlockNumber,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let now = frame_system::Pallet::<T>::block_number();
			ensure!(deadline > now, Error::<T>::Deadline);

			Self::inner_swap(
				&who,
				poo_id,
				from_index as usize,
				to_index as usize,
				in_amount,
				min_out_amount,
				&to,
			)?;

			Ok(())
		}

		/// Remove liquidity from a pool.
		///
		/// # Argument
		///
		/// - `pool_id`: The id of pool.
		/// - `lp_amount`: The amounts of lp currency.
		/// - `min_amounts`: The min amounts of pool's currencies to get.
		/// - `deadline`: Height of the cutoff block of this transaction
		#[pallet::weight(T::WeightInfo::remove_liquidity())]
		#[transactional]
		pub fn remove_liquidity(
			origin: OriginFor<T>,
			poo_id: T::PoolId,
			lp_amount: Balance,
			min_amounts: Vec<Balance>,
			to: T::AccountId,
			deadline: T::BlockNumber,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let now = frame_system::Pallet::<T>::block_number();
			ensure!(deadline > now, Error::<T>::Deadline);

			Self::inner_remove_liquidity(poo_id, &who, lp_amount, &min_amounts, &to)?;

			Ok(())
		}

		/// Remove liquidity from a pool to get one currency.
		///
		/// # Argument
		///
		/// - `pool_id`: The id of pool.
		/// - `lp_amount`: The amounts of lp currency.
		/// - `index`: The index of receive currency.
		/// - `min_amount`: The min amounts of received currency;
		/// - `deadline`: Height of the cutoff block of this transaction
		#[pallet::weight(T::WeightInfo::remove_liquidity_one_currency())]
		#[transactional]
		pub fn remove_liquidity_one_currency(
			origin: OriginFor<T>,
			poo_id: T::PoolId,
			lp_amount: Balance,
			index: u32,
			min_amount: Balance,
			to: T::AccountId,
			deadline: T::BlockNumber,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let now = frame_system::Pallet::<T>::block_number();
			ensure!(deadline > now, Error::<T>::Deadline);

			Self::inner_remove_liquidity_one_currency(
				poo_id, &who, lp_amount, index, min_amount, &to,
			)?;

			Ok(())
		}

		/// Remove liquidity from a pool to the specify amounts of currencies.
		///
		/// # Argument
		///
		/// - `pool_id`: The id of pool.
		/// - `amounts`: The specify amounts of receive currencies.
		/// - `max_burn_amount`: The max amount of burned lp currency.
		/// - `deadline`: Height of the cutoff block of this transaction
		#[pallet::weight(T::WeightInfo::remove_liquidity_imbalance())]
		#[transactional]
		pub fn remove_liquidity_imbalance(
			origin: OriginFor<T>,
			pool_id: T::PoolId,
			amounts: Vec<Balance>,
			max_burn_amount: Balance,
			to: T::AccountId,
			deadline: T::BlockNumber,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let now = frame_system::Pallet::<T>::block_number();
			ensure!(deadline > now, Error::<T>::Deadline);

			Self::inner_remove_liquidity_imbalance(&who, pool_id, &amounts, max_burn_amount, &to)?;

			Ok(())
		}

		/// Supply amounts of currencies to the pool which contains the lp currency of the base
		/// pool.
		///
		/// # Argument
		///
		/// - `pool_id`: The id of pool.
		/// - `base_pool_id`: The id of base pool.
		/// - `meta_amounts`: Supply amounts of currencies to pool. The last element must be zero.
		/// - `base_amounts`: Supply amounts of currencies to base pool.
		/// - `min_to_mint`: The min amount of pool lp currency get.
		/// - `deadline`: Height of the cutoff block of this transaction.
		#[pallet::weight(T::WeightInfo::add_pool_and_base_pool_liquidity())]
		#[transactional]
		pub fn add_pool_and_base_pool_liquidity(
			origin: OriginFor<T>,
			pool_id: T::PoolId,
			base_pool_id: T::PoolId,
			meta_amounts: Vec<Balance>,
			base_amounts: Vec<Balance>,
			min_to_mint: Balance,
			to: T::AccountId,
			deadline: T::BlockNumber,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let now = frame_system::Pallet::<T>::block_number();
			ensure!(deadline > now, Error::<T>::Deadline);

			Self::inner_add_pool_and_base_pool_liquidity(
				&who,
				pool_id,
				base_pool_id,
				meta_amounts,
				&base_amounts,
				min_to_mint,
				&to,
			)?;

			Ok(())
		}

		/// Remove liquidity from a pool which contains the lp currency of the base pool.
		///
		/// # Argument
		///
		/// - `pool_id`: The id of pool.
		/// - `base_pool_id`: The id of base pool.
		/// - `amount`: The amounts of lp currency to burn.
		/// - `min_amounts_meta`: The min amounts of pool's currencies to get.
		/// - `min_amounts_base`: The min amounts of basic pool's currencies to get.
		/// - `deadline`: Height of the cutoff block of this transaction.
		#[pallet::weight(T::WeightInfo::remove_pool_and_base_pool_liquidity())]
		#[transactional]
		pub fn remove_pool_and_base_pool_liquidity(
			origin: OriginFor<T>,
			pool_id: T::PoolId,
			base_pool_id: T::PoolId,
			amount: Balance,
			min_amounts_meta: Vec<Balance>,
			min_amounts_base: Vec<Balance>,
			to: T::AccountId,
			deadline: T::BlockNumber,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let now = frame_system::Pallet::<T>::block_number();
			ensure!(deadline > now, Error::<T>::Deadline);

			Self::inner_remove_pool_and_base_pool_liquidity(
				&who,
				pool_id,
				base_pool_id,
				amount,
				&min_amounts_meta,
				&min_amounts_base,
				&to,
			)?;

			Ok(())
		}

		/// Remove liquidity from a pool which contains the lp currency of the base pool
		/// to get one currency.
		///
		/// # Argument
		///
		/// - `pool_id`: The id of pool.
		/// - `base_pool_id`: The id of base pool.
		/// - `amount`: The amounts of lp currency to burn.
		/// - `i`: The index of target currency in basic pool.
		/// - `min_amount`: The min amounts of received currency.
		/// - `deadline`: Height of the cutoff block of this transaction.
		#[pallet::weight(T::WeightInfo::remove_pool_and_base_pool_liquidity_one_currency())]
		#[transactional]
		pub fn remove_pool_and_base_pool_liquidity_one_currency(
			origin: OriginFor<T>,
			pool_id: T::PoolId,
			base_pool_id: T::PoolId,
			amount: Balance,
			i: u32,
			min_amount: Balance,
			to: T::AccountId,
			deadline: T::BlockNumber,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let now = frame_system::Pallet::<T>::block_number();
			ensure!(deadline > now, Error::<T>::Deadline);

			Self::inner_remove_pool_and_base_pool_liquidity_one_currency(
				&who,
				pool_id,
				base_pool_id,
				amount,
				i,
				min_amount,
				&to,
			)?;

			Ok(())
		}

		/// Swap the currency from basic pool to get amounts of other currency in pool.
		/// to get one currency.
		///
		/// # Argument
		///
		/// - `pool_id`: The id of pool.
		/// - `base_pool_id`: The id of base pool.
		/// - `in_index`: The index of swap currency in basic pool.
		/// - `out_index`: The index of target currency in pool.
		/// - `dx`: The amounts of swap currency.
		/// - `min_dy`: The min amounts of target currency.
		/// - `deadline`: Height of the cutoff block of this transaction.
		#[pallet::weight(T::WeightInfo::swap_pool_from_base())]
		#[transactional]
		pub fn swap_pool_from_base(
			origin: OriginFor<T>,
			pool_id: T::PoolId,
			base_pool_id: T::PoolId,
			in_index: u32,
			out_index: u32,
			dx: Balance,
			min_dy: Balance,
			to: T::AccountId,
			deadline: T::BlockNumber,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let now = frame_system::Pallet::<T>::block_number();
			ensure!(deadline > now, Error::<T>::Deadline);

			Self::inner_swap_pool_from_base(
				&who,
				pool_id,
				base_pool_id,
				in_index,
				out_index,
				dx,
				min_dy,
				&to,
			)?;

			Ok(())
		}

		/// Swap the currency from pool to get amounts of other currency in basic pool.
		/// to get one currency.
		///
		/// # Argument
		///
		/// - `pool_id`: The id of pool.
		/// - `base_pool_id`: The id of base pool.
		/// - `in_index`: The index of swap currency in basic pool.
		/// - `out_index`: The index of target currency in pool.
		/// - `dx`: The amounts of swap currency.
		/// - `min_dy`: The min amounts of target currency.
		/// - `deadline`: Height of the cutoff block of this transaction.
		#[pallet::weight(T::WeightInfo::swap_pool_to_base())]
		#[transactional]
		pub fn swap_pool_to_base(
			origin: OriginFor<T>,
			pool_id: T::PoolId,
			base_pool_id: T::PoolId,
			in_index: u32,
			out_index: u32,
			dx: Balance,
			min_dy: Balance,
			to: T::AccountId,
			deadline: T::BlockNumber,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let now = frame_system::Pallet::<T>::block_number();
			ensure!(deadline > now, Error::<T>::Deadline);

			Self::inner_swap_pool_to_base(
				&who,
				pool_id,
				base_pool_id,
				in_index,
				out_index,
				dx,
				min_dy,
				&to,
			)?;

			Ok(())
		}

		#[pallet::weight(T::WeightInfo::swap_meta_pool_underlying())]
		#[transactional]
		pub fn swap_meta_pool_underlying(
			origin: OriginFor<T>,
			pool_id: T::PoolId,
			in_index: u32,
			out_index: u32,
			dx: Balance,
			min_dy: Balance,
			to: T::AccountId,
			deadline: T::BlockNumber,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let now = frame_system::Pallet::<T>::block_number();
			ensure!(deadline > now, Error::<T>::Deadline);

			Pools::<T>::try_mutate_exists(
				pool_id,
				|optioned_pool| -> Result<Balance, DispatchError> {
					let pool = optioned_pool.as_mut().ok_or(Error::<T>::InvalidPoolId)?;
					match pool {
						Pool::Meta(mp) => Self::meta_pool_swap_underlying(
							mp,
							pool_id,
							&who,
							&to,
							dx,
							min_dy,
							in_index as usize,
							out_index as usize,
						),
						_ => Err(Error::<T>::InvalidPoolId.into()),
					}
				},
			)?;

			Ok(())
		}

		/// Update admin fee receiver of the pool.
		///
		/// Only called by admin.
		///
		/// # Argument
		///
		/// - `pool_id`: The id of pool.
		/// - `fee_receiver`: The new admin fee receiver of this pool.
		#[pallet::weight(1_000_000)]
		#[transactional]
		pub fn update_fee_receiver(
			origin: OriginFor<T>,
			pool_id: T::PoolId,
			fee_receiver: <T::Lookup as StaticLookup>::Source,
		) -> DispatchResult {
			ensure_root(origin)?;
			let admin_fee_receiver = T::Lookup::lookup(fee_receiver)?;
			Pools::<T>::try_mutate_exists(pool_id, |optioned_pool| -> DispatchResult {
				let pool = optioned_pool.as_mut().ok_or(Error::<T>::InvalidPoolId)?;
				pool.set_admin_fee_receiver(admin_fee_receiver.clone());

				Self::deposit_event(Event::UpdateAdminFeeReceiver { pool_id, admin_fee_receiver });
				Ok(())
			})
		}

		/// Update fee of the pool.
		///
		/// Only called by admin.
		///
		/// # Argument
		///
		/// - `pool_id`: The id of pool.
		/// - `new_swap_fee`: The new swap fee of this pool.
		#[pallet::weight(1_000_000)]
		#[transactional]
		pub fn set_swap_fee(
			origin: OriginFor<T>,
			pool_id: T::PoolId,
			new_swap_fee: Number,
		) -> DispatchResult {
			ensure_root(origin)?;
			Pools::<T>::try_mutate_exists(pool_id, |optioned_pool| -> DispatchResult {
				let pool = optioned_pool.as_mut().ok_or(Error::<T>::InvalidPoolId)?;
				ensure!(new_swap_fee <= MAX_SWAP_FEE, Error::<T>::ExceedThreshold);

				pool.set_fee(new_swap_fee);

				Self::deposit_event(Event::NewSwapFee { pool_id, new_swap_fee });
				Ok(())
			})
		}

		/// Update admin fee of the pool.
		///
		/// Only called by admin.
		///
		/// # Argument
		///
		/// - `pool_id`: The id of pool.
		/// - `new_admin_fee`: The new admin fee of this pool.
		#[pallet::weight(1_000_000)]
		#[transactional]
		pub fn set_admin_fee(
			origin: OriginFor<T>,
			pool_id: T::PoolId,
			new_admin_fee: Number,
		) -> DispatchResult {
			ensure_root(origin)?;
			Pools::<T>::try_mutate_exists(pool_id, |optioned_pool| -> DispatchResult {
				let pool = optioned_pool.as_mut().ok_or(Error::<T>::InvalidPoolId)?;
				ensure!(new_admin_fee <= MAX_ADMIN_FEE, Error::<T>::ExceedThreshold);

				pool.set_admin_fee(new_admin_fee);

				Self::deposit_event(Event::NewAdminFee { pool_id, new_admin_fee });
				Ok(())
			})
		}

		/// Start ramping up or down A parameter towards given future_a and future_a_time
		///
		/// Only called by admin.
		/// Checks if the change is too rapid, and commits the new A value only when it falls under
		/// the limit range.
		///
		/// # Argument
		///
		/// - `pool_id`: The id of pool.
		/// - `future_a`: The new A to ramp towards.
		/// - `future_a_time`: Timestamp when the new A should be reached
		#[pallet::weight(1_000_000)]
		#[transactional]
		pub fn ramp_a(
			origin: OriginFor<T>,
			pool_id: T::PoolId,
			future_a: Number,
			future_a_time: Number,
		) -> DispatchResult {
			ensure_root(origin)?;
			let now = T::TimeProvider::now().as_secs() as Number;
			Pools::<T>::try_mutate_exists(pool_id, |optioned_pool| -> DispatchResult {
				let general_pool = optioned_pool.as_mut().ok_or(Error::<T>::InvalidPoolId)?;
				let pool = match general_pool {
					Pool::Base(bp) => bp,
					Pool::Meta(mp) => &mut mp.info,
				};
				ensure!(
					now >= pool
						.initial_a_time
						.checked_add(Number::from(DAY))
						.ok_or(Error::<T>::Arithmetic)?,
					Error::<T>::RampADelay
				);

				ensure!(
					future_a_time >=
						now.checked_add(Number::from(MIN_RAMP_TIME))
							.ok_or(Error::<T>::Arithmetic)?,
					Error::<T>::MinRampTime
				);

				ensure!(future_a > Zero::zero() && future_a < MAX_A, Error::<T>::ExceedThreshold);

				let (initial_a_precise, future_a_precise) = Self::get_a_precise(pool)
					.and_then(|initial_a_precise| -> Option<(Number, Number)> {
						let future_a_precise = future_a.checked_mul(A_PRECISION)?;
						Some((initial_a_precise, future_a_precise))
					})
					.ok_or(Error::<T>::Arithmetic)?;

				let max_a_change = Number::from(MAX_A_CHANGE);

				if future_a_precise < initial_a_precise {
					ensure!(
						future_a_precise.checked_mul(max_a_change).ok_or(Error::<T>::Arithmetic)? >=
							initial_a_precise,
						Error::<T>::ExceedMaxAChange
					);
				} else {
					ensure!(
						future_a_precise <=
							initial_a_precise
								.checked_mul(max_a_change)
								.ok_or(Error::<T>::Arithmetic)?,
						Error::<T>::ExceedMaxAChange
					);
				}

				pool.initial_a = initial_a_precise;
				pool.future_a = future_a_precise;
				pool.initial_a_time = now;
				pool.future_a_time = future_a_time;

				Self::deposit_event(Event::RampA {
					pool_id,
					initial_a_precise,
					future_a_precise,
					now,
					future_a_time,
				});

				Ok(())
			})
		}

		/// Stop ramping A parameter.
		///
		/// Only called by admin.
		///
		/// # Argument
		///
		/// - `pool_id`: The id of pool.
		#[pallet::weight(1_000_000)]
		#[transactional]
		pub fn stop_ramp_a(origin: OriginFor<T>, pool_id: T::PoolId) -> DispatchResult {
			ensure_root(origin)?;
			Pools::<T>::try_mutate_exists(pool_id, |optioned_pool| -> DispatchResult {
				let general_pool = optioned_pool.as_mut().ok_or(Error::<T>::InvalidPoolId)?;
				let pool = match general_pool {
					Pool::Base(bp) => bp,
					Pool::Meta(mp) => &mut mp.info,
				};

				let now = T::TimeProvider::now().as_secs() as Number;
				ensure!(pool.future_a_time > now, Error::<T>::AlreadyStoppedRampA);

				let current_a = Self::get_a_precise(pool).ok_or(Error::<T>::Arithmetic)?;

				pool.initial_a = current_a;
				pool.future_a = current_a;
				pool.initial_a_time = now;
				pool.future_a_time = now;

				Self::deposit_event(Event::StopRampA { pool_id, current_a, now });
				Ok(())
			})
		}

		/// Withdraw the admin fee from pool to admin fee receiver.
		///
		/// Can called by anyone.
		///
		/// # Argument
		///
		/// - `pool_id`: The id of pool.
		#[pallet::weight(T::WeightInfo::withdraw_admin_fee())]
		#[transactional]
		pub fn withdraw_admin_fee(origin: OriginFor<T>, pool_id: T::PoolId) -> DispatchResult {
			ensure_signed(origin)?;

			Pools::<T>::try_mutate_exists(pool_id, |optioned_pool| -> DispatchResult {
				let general_pool = optioned_pool.as_mut().ok_or(Error::<T>::InvalidPoolId)?;
				let pool = match general_pool {
					Pool::Base(bp) => bp,
					Pool::Meta(mp) => &mut mp.info,
				};

				for (i, reserve) in pool.balances.iter().enumerate() {
					let balance =
						T::MultiCurrency::free_balance(pool.currency_ids[i], &pool.account)
							.checked_sub(*reserve)
							.ok_or(Error::<T>::Arithmetic)?;

					if !balance.is_zero() {
						T::MultiCurrency::transfer(
							pool.currency_ids[i],
							&pool.account,
							&pool.admin_fee_receiver,
							balance,
						)?;
					}
					Self::deposit_event(Event::CollectProtocolFee {
						pool_id,
						currency_id: pool.currency_ids[i],
						fee_amount: balance,
					});
				}
				Ok(())
			})
		}
	}
}

impl<T: Config> Pallet<T> {
	fn inner_add_liquidity(
		who: &T::AccountId,
		pool_id: T::PoolId,
		amounts: &[Balance],
		min_mint_amount: Balance,
		to: &T::AccountId,
	) -> Result<Balance, DispatchError> {
		Pools::<T>::try_mutate_exists(pool_id, |optioned_pool| -> Result<Balance, DispatchError> {
			let pool = optioned_pool.as_mut().ok_or(Error::<T>::InvalidPoolId)?;
			match pool {
				Pool::Base(bp) =>
					Self::base_pool_add_liquidity(who, pool_id, bp, amounts, min_mint_amount, to),
				Pool::Meta(mp) =>
					Self::meta_pool_add_liquidity(who, pool_id, mp, amounts, min_mint_amount, to),
			}
		})
	}

	fn inner_swap(
		who: &T::AccountId,
		pool_id: T::PoolId,
		i: usize,
		j: usize,
		in_amount: Balance,
		out_min_amount: Balance,
		to: &T::AccountId,
	) -> Result<Balance, DispatchError> {
		ensure!(i != j, Error::<T>::SwapSameCurrency);

		Pools::<T>::try_mutate_exists(pool_id, |optioned_pool| -> Result<Balance, DispatchError> {
			let pool = optioned_pool.as_mut().ok_or(Error::<T>::InvalidPoolId)?;
			match pool {
				Pool::Base(bp) =>
					Self::base_pool_swap(who, pool_id, bp, i, j, in_amount, out_min_amount, to),
				Pool::Meta(mp) =>
					Self::meta_pool_swap(who, pool_id, mp, i, j, in_amount, out_min_amount, to),
			}
		})
	}

	fn inner_remove_liquidity(
		pool_id: T::PoolId,
		who: &T::AccountId,
		lp_amount: Balance,
		min_amounts: &[Balance],
		to: &T::AccountId,
	) -> DispatchResult {
		Pools::<T>::try_mutate_exists(pool_id, |optioned_pool| -> DispatchResult {
			ensure!(!lp_amount.is_zero(), Error::<T>::InvalidTransaction);
			let global_pool = optioned_pool.as_mut().ok_or(Error::<T>::InvalidPoolId)?;
			let pool = match global_pool {
				Pool::Base(bp) => bp,
				Pool::Meta(mp) => &mut mp.info,
			};

			let lp_total_supply = T::MultiCurrency::total_issuance(pool.lp_currency_id);

			ensure!(lp_total_supply >= lp_amount, Error::<T>::InsufficientReserve);

			let currencies_length = pool.currency_ids.len();
			let min_amounts_length = min_amounts.len();
			ensure!(currencies_length == min_amounts_length, Error::<T>::MismatchParameter);

			let fees: Vec<Balance> = vec![Zero::zero(); currencies_length];
			let amounts = Self::calculate_base_remove_liquidity(pool, lp_amount)
				.ok_or(Error::<T>::Arithmetic)?;

			for (i, amount) in amounts.iter().enumerate() {
				ensure!(*amount >= min_amounts[i], Error::<T>::AmountSlippage);
				pool.balances[i] =
					pool.balances[i].checked_sub(*amount).ok_or(Error::<T>::Arithmetic)?;
				T::MultiCurrency::transfer(pool.currency_ids[i], &pool.account, to, *amount)?;
			}

			T::MultiCurrency::withdraw(pool.lp_currency_id, who, lp_amount)?;
			Self::deposit_event(Event::RemoveLiquidity {
				pool_id,
				who: who.clone(),
				to: to.clone(),
				amounts,
				fees,
				new_total_supply: lp_total_supply - lp_amount,
			});
			Ok(())
		})
	}

	fn inner_remove_liquidity_one_currency(
		pool_id: T::PoolId,
		who: &T::AccountId,
		lp_amount: Balance,
		index: u32,
		min_amount: Balance,
		to: &T::AccountId,
	) -> Result<Balance, DispatchError> {
		Pools::<T>::try_mutate_exists(pool_id, |optioned_pool| -> Result<Balance, DispatchError> {
			ensure!(!lp_amount.is_zero(), Error::<T>::InvalidTransaction);
			let pool = optioned_pool.as_mut().ok_or(Error::<T>::InvalidPoolId)?;
			match pool {
				Pool::Base(bp) => Self::base_pool_remove_liquidity_one_currency(
					pool_id, bp, who, lp_amount, index, min_amount, to,
				),
				Pool::Meta(mp) => Self::meta_pool_remove_liquidity_one_currency(
					pool_id, mp, who, lp_amount, index, min_amount, to,
				),
			}
		})
	}

	fn inner_remove_liquidity_imbalance(
		who: &T::AccountId,
		pool_id: T::PoolId,
		amounts: &[Balance],
		max_burn_amount: Balance,
		to: &T::AccountId,
	) -> DispatchResult {
		Pools::<T>::try_mutate_exists(pool_id, |optioned_pool| -> DispatchResult {
			let pool = optioned_pool.as_mut().ok_or(Error::<T>::InvalidPoolId)?;
			match pool {
				Pool::Base(bp) => Self::base_pool_remove_liquidity_imbalance(
					who,
					pool_id,
					bp,
					amounts,
					max_burn_amount,
					to,
				),
				Pool::Meta(mp) => Self::meta_pool_remove_liquidity_imbalance(
					who,
					pool_id,
					mp,
					amounts,
					max_burn_amount,
					to,
				),
			}
		})
	}

	fn inner_add_pool_and_base_pool_liquidity(
		who: &T::AccountId,
		pool_id: T::PoolId,
		base_pool_id: T::PoolId,
		mut meta_amounts: Vec<Balance>,
		base_amounts: &[Balance],
		min_to_mint: Balance,
		to: &T::AccountId,
	) -> DispatchResult {
		let base_pool = Self::pools(base_pool_id).ok_or(Error::<T>::InvalidPoolId)?;
		let meta_pool = Self::pools(pool_id).ok_or(Error::<T>::InvalidPoolId)?;
		match meta_pool {
			Pool::Base(_) => Err(Error::<T>::InvalidPoolId),
			Pool::Meta(ref mp) => {
				ensure!(mp.base_pool_id == base_pool_id, Error::<T>::MismatchParameter);
				Ok(())
			},
		}?;

		let base_pool_lp_currency = base_pool.get_lp_currency();
		let meta_pool_currencies = meta_pool.get_currency_ids();

		let mut deposit_base = false;
		for amount in base_amounts.iter() {
			if *amount > Zero::zero() {
				deposit_base = true;
				break
			}
		}
		let mut base_lp_received: Balance = Balance::default();
		if deposit_base {
			base_lp_received = Self::inner_add_liquidity(who, base_pool_id, base_amounts, 0, who)?;
		}
		let base_lp_prior = <T as Config>::MultiCurrency::free_balance(base_pool_lp_currency, who);

		for (i, c) in meta_pool_currencies.iter().enumerate() {
			if *c == base_pool_lp_currency {
				meta_amounts[i] = base_lp_received;
			}
		}

		Self::inner_add_liquidity(who, pool_id, &meta_amounts, min_to_mint, to)?;

		if deposit_base {
			let base_lp_after =
				<T as Config>::MultiCurrency::free_balance(base_pool_lp_currency, who);
			ensure!(
				base_lp_after
					.checked_add(base_lp_received)
					.and_then(|n| n.checked_sub(base_lp_prior))
					.ok_or(Error::<T>::Arithmetic)? ==
					Zero::zero(),
				Error::<T>::AmountSlippage
			)
		}

		Ok(())
	}

	fn inner_remove_pool_and_base_pool_liquidity(
		who: &T::AccountId,
		pool_id: T::PoolId,
		base_pool_id: T::PoolId,
		amount: Balance,
		min_amounts_meta: &[Balance],
		min_amounts_base: &[Balance],
		to: &T::AccountId,
	) -> DispatchResult {
		let base_pool = Self::pools(base_pool_id).ok_or(Error::<T>::InvalidPoolId)?;
		let base_pool_lp_currency = base_pool.get_lp_currency();
		let base_lp_amount_before =
			<T as Config>::MultiCurrency::free_balance(base_pool_lp_currency, who);

		Self::inner_remove_liquidity(pool_id, who, amount, min_amounts_meta, who)?;

		let base_lp_amount_after =
			<T as Config>::MultiCurrency::free_balance(base_pool_lp_currency, who);
		let base_lp_amount = base_lp_amount_after
			.checked_sub(base_lp_amount_before)
			.ok_or(Error::<T>::Arithmetic)?;

		Self::inner_remove_liquidity(base_pool_id, who, base_lp_amount, min_amounts_base, to)?;

		Ok(())
	}

	fn inner_remove_pool_and_base_pool_liquidity_one_currency(
		who: &T::AccountId,
		pool_id: T::PoolId,
		base_pool_id: T::PoolId,
		amount: Balance,
		i: u32,
		min_amount: Balance,
		to: &T::AccountId,
	) -> DispatchResult {
		let base_pool = Self::pools(base_pool_id).ok_or(Error::<T>::InvalidPoolId)?;
		let meta_pool = Self::pools(pool_id).ok_or(Error::<T>::InvalidPoolId)?;
		match meta_pool {
			Pool::Base(_) => Err(Error::<T>::InvalidPoolId),
			Pool::Meta(ref mp) => {
				ensure!(mp.base_pool_id == base_pool_id, Error::<T>::MismatchParameter);
				Ok(())
			},
		}?;

		let base_pool_lp_currency = base_pool.get_lp_currency();
		let meta_pool_currencies = meta_pool.get_currency_ids();

		let mut base_pool_currency_index: Option<u32> = None;
		for (i, c) in meta_pool_currencies.iter().enumerate() {
			if *c == base_pool_lp_currency {
				base_pool_currency_index = Some(i as u32)
			}
		}

		let base_pool_currency_index =
			base_pool_currency_index.ok_or(Error::<T>::InvalidBasePool)?;

		let base_pool_currency_before =
			<T as Config>::MultiCurrency::free_balance(base_pool_lp_currency, who);
		Self::inner_remove_liquidity_one_currency(
			pool_id,
			who,
			amount,
			base_pool_currency_index,
			0,
			who,
		)?;
		let base_pool_currency_after =
			<T as Config>::MultiCurrency::free_balance(base_pool_lp_currency, who);

		let base_pool_currency_amount = base_pool_currency_after
			.checked_sub(base_pool_currency_before)
			.ok_or(Error::<T>::Arithmetic)?;

		Self::inner_remove_liquidity_one_currency(
			base_pool_id,
			who,
			base_pool_currency_amount,
			i,
			min_amount,
			to,
		)?;

		Ok(())
	}

	fn inner_swap_pool_from_base(
		who: &T::AccountId,
		meta_pool_id: T::PoolId,
		base_pool_id: T::PoolId,
		in_index: u32,
		out_index: u32,
		dx: Balance,
		min_dy: Balance,
		to: &T::AccountId,
	) -> Result<Balance, DispatchError> {
		let base_pool = Self::pools(base_pool_id).ok_or(Error::<T>::InvalidBasePool)?.info();
		let meta_pool = Self::pools(meta_pool_id).ok_or(Error::<T>::InvalidPoolId)?;

		match meta_pool {
			Pool::Base(_) => Err(Error::<T>::InvalidPoolId),
			Pool::Meta(ref mp) => {
				ensure!(mp.base_pool_id == base_pool_id, Error::<T>::MismatchParameter);
				Ok(())
			},
		}?;

		let meta_pool = meta_pool.info();

		let base_pool_currency = base_pool.lp_currency_id;
		let mut base_pool_lp_currency_in_meta_index = None;

		for (i, c) in meta_pool.currency_ids.iter().enumerate() {
			if *c == base_pool_currency {
				base_pool_lp_currency_in_meta_index = Some(i)
			}
		}
		// ensure meta pool currencies contains the lp currency of base pool
		let base_pool_lp_currency_in_meta_index =
			base_pool_lp_currency_in_meta_index.ok_or(Error::<T>::MismatchParameter)?;

		let base_pool_len = base_pool.currency_ids.len();

		let mut base_amounts = vec![Balance::default(); base_pool_len];
		base_amounts[in_index as usize] = dx;

		let base_lp_amount = Self::inner_add_liquidity(who, base_pool_id, &base_amounts, 0, who)?;

		let mut out_amount: Balance = 0;
		if base_pool_lp_currency_in_meta_index != (out_index as usize) {
			out_amount = Self::inner_swap(
				who,
				meta_pool_id,
				base_pool_lp_currency_in_meta_index,
				out_index as usize,
				base_lp_amount,
				min_dy,
				to,
			)?;
		}

		Ok(out_amount)
	}

	fn inner_swap_pool_to_base(
		who: &T::AccountId,
		pool_id: T::PoolId,
		base_pool_id: T::PoolId,
		in_index: u32,
		out_index: u32,
		dx: Balance,
		min_dy: Balance,
		to: &T::AccountId,
	) -> Result<Balance, DispatchError> {
		let base_pool_currency =
			Self::get_lp_currency(base_pool_id).ok_or(Error::<T>::InvalidPoolId)?;
		let base_pool_currency_index = Self::get_currency_index(pool_id, base_pool_currency)
			.ok_or(Error::<T>::InvalidBasePool)?;

		let mut base_lp_amount = Balance::default();
		if base_pool_currency_index != in_index {
			base_lp_amount = Self::inner_swap(
				who,
				pool_id,
				in_index as usize,
				base_pool_currency_index as usize,
				dx,
				0,
				who,
			)?;
		}
		let out_amount = Self::inner_remove_liquidity_one_currency(
			base_pool_id,
			who,
			base_lp_amount,
			out_index,
			min_dy,
			to,
		)?;

		Ok(out_amount)
	}

	pub(crate) fn calculate_currency_amount(
		pool_id: T::PoolId,
		amounts: Vec<Balance>,
		deposit: bool,
	) -> Result<Balance, DispatchError> {
		if let Some(pool) = Self::pools(pool_id) {
			match pool {
				Pool::Base(bp) => Self::calculate_base_currency_amount(&bp, amounts, deposit),
				Pool::Meta(mp) => Self::calculate_meta_currency_amount(&mp, amounts, deposit),
			}
		} else {
			Err(Error::<T>::InvalidPoolId.into())
		}
	}

	pub(crate) fn get_admin_balance(pool_id: T::PoolId, currency_index: usize) -> Option<Balance> {
		if let Some(general_pool) = Self::pools(pool_id) {
			let pool = match general_pool {
				Pool::Base(bp) => bp,
				Pool::Meta(mp) => mp.info,
			};
			let currencies_len = pool.currency_ids.len();
			if currency_index >= currencies_len {
				return None
			}

			let balance =
				T::MultiCurrency::free_balance(pool.currency_ids[currency_index], &pool.account);

			balance.checked_sub(pool.balances[currency_index])
		} else {
			None
		}
	}
}
