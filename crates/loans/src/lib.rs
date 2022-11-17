// Copyright 2022 Interlay.
// This file is part of Interlay.

// Copyright 2021 Parallel Finance Developer.
// This file is part of Parallel Finance.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! # Loans pallet
//!
//! ## Overview
//!
//! Loans pallet implement the lending protocol by using a pool-based strategy
//! that aggregates each user's supplied assets. The interest rate is dynamically
//! determined by the supply and demand.

#![cfg_attr(not(feature = "std"), no_std)]

pub use crate::rate_model::*;

use currency::Amount;
use frame_support::{
    log,
    pallet_prelude::*,
    require_transactional,
    traits::{
        tokens::fungibles::{Inspect, Mutate, Transfer},
        UnixTime,
    },
    transactional, PalletId,
};
use frame_system::pallet_prelude::*;
use num_traits::cast::ToPrimitive;
use orml_traits::{MultiCurrency, MultiReservableCurrency};
pub use pallet::*;
use primitives::{Balance, CurrencyId, Liquidity, Rate, Ratio, Shortfall, Timestamp};
use sp_runtime::{
    traits::{
        AccountIdConversion, CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, One, SaturatedConversion, Saturating,
        StaticLookup, Zero,
    },
    ArithmeticError, FixedPointNumber, FixedU128,
};
use sp_std::{marker, result::Result};
use traits::{ConvertToBigUint, LoansApi as LoansTrait, LoansMarketDataProvider, MarketInfo, MarketStatus};

pub use orml_traits::currency::{OnDeposit, OnSlash, OnTransfer};
use sp_io::hashing::blake2_256;
pub use types::{BorrowSnapshot, EarnedSnapshot, Market, MarketState, RewardMarketState};
pub use weights::WeightInfo;

mod benchmarking;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

mod farming;
mod interest;
#[cfg(test)]
mod lend_token;
mod rate_model;
mod types;

pub mod weights;

pub const REWARD_ACCOUNT_PREFIX: &[u8; 13] = b"loans/farming";
pub const INCENTIVE_ACCOUNT_PREFIX: &[u8; 15] = b"loans/incentive";

type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
type AssetIdOf<T> = <<T as Config>::Assets as Inspect<<T as frame_system::Config>::AccountId>>::AssetId;
type BalanceOf<T> = <<T as Config>::Assets as Inspect<<T as frame_system::Config>::AccountId>>::Balance;

pub struct OnSlashHook<T>(marker::PhantomData<T>);
// This implementation is not allowed to fail, so erors are logged instead of being propagated.
// If the slash-related FRAME traits are allowed to fail, this can be fixed.
// Opened a GitHub issue for this in the Substrate repo: https://github.com/paritytech/substrate/issues/12533
// TODO: Propagate error once the issue is resolved upstream
impl<T: Config> OnSlash<T::AccountId, AssetIdOf<T>, BalanceOf<T>> for OnSlashHook<T> {
    fn on_slash(currency_id: AssetIdOf<T>, account_id: &T::AccountId, amount: BalanceOf<T>) {
        if currency_id.is_lend_token() {
            let f = || -> DispatchResult {
                let underlying_id = Pallet::<T>::underlying_id(currency_id)?;
                Pallet::<T>::update_reward_supply_index(underlying_id)?;
                Pallet::<T>::distribute_supplier_reward(underlying_id, account_id)?;
                Ok(())
            };
            if let Err(e) = f() {
                log::trace!(
                    target: "loans::on_slash",
                    "error: {:?}, currency_id: {:?}, account_id: {:?}, amount: {:?}",
                    e,
                    currency_id,
                    account_id,
                    amount,
                );
            }
        }
    }
}

pub struct PreDeposit<T>(marker::PhantomData<T>);
impl<T: Config> OnDeposit<T::AccountId, AssetIdOf<T>, BalanceOf<T>> for PreDeposit<T> {
    fn on_deposit(currency_id: AssetIdOf<T>, account_id: &T::AccountId, _amount: BalanceOf<T>) -> DispatchResult {
        if currency_id.is_lend_token() {
            let underlying_id = Pallet::<T>::underlying_id(currency_id)?;
            Pallet::<T>::update_reward_supply_index(underlying_id)?;
            Pallet::<T>::distribute_supplier_reward(underlying_id, account_id)?;
        }
        Ok(())
    }
}

pub struct PostDeposit<T>(marker::PhantomData<T>);
impl<T: Config> OnDeposit<T::AccountId, AssetIdOf<T>, BalanceOf<T>> for PostDeposit<T> {
    fn on_deposit(currency_id: AssetIdOf<T>, account_id: &T::AccountId, amount: BalanceOf<T>) -> DispatchResult {
        if currency_id.is_lend_token() {
            Pallet::<T>::lock_if_account_deposited(account_id, currency_id, amount)?;
        }
        Ok(())
    }
}

pub struct PreTransfer<T>(marker::PhantomData<T>);
impl<T: Config> OnTransfer<T::AccountId, AssetIdOf<T>, BalanceOf<T>> for PreTransfer<T> {
    fn on_transfer(
        currency_id: AssetIdOf<T>,
        from: &T::AccountId,
        to: &T::AccountId,
        _amount: BalanceOf<T>,
    ) -> DispatchResult {
        if currency_id.is_lend_token() {
            let underlying_id = Pallet::<T>::underlying_id(currency_id)?;
            Pallet::<T>::update_reward_supply_index(underlying_id)?;
            Pallet::<T>::distribute_supplier_reward(underlying_id, from)?;
            Pallet::<T>::distribute_supplier_reward(underlying_id, to)?;
        }
        Ok(())
    }
}

pub struct PostTransfer<T>(marker::PhantomData<T>);
impl<T: Config> OnTransfer<T::AccountId, AssetIdOf<T>, BalanceOf<T>> for PostTransfer<T> {
    fn on_transfer(
        currency_id: AssetIdOf<T>,
        _from: &T::AccountId,
        to: &T::AccountId,
        amount: BalanceOf<T>,
    ) -> DispatchResult {
        if currency_id.is_lend_token() {
            Pallet::<T>::lock_if_account_deposited(to, currency_id, amount)?;
        }
        Ok(())
    }
}

/// Utility type for managing upgrades/migrations.
#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum Versions {
    V0,
}

#[frame_support::pallet]
pub mod pallet {

    use super::*;

    #[pallet::config]
    pub trait Config:
        frame_system::Config + currency::Config<Balance = BalanceOf<Self>, UnsignedFixedPoint = FixedU128>
    {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// The loan's module id, keep all collaterals of CDPs.
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// The origin which can add/reduce reserves.
        type ReserveOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// The origin which can update rate model, liquidate incentive and
        /// add/reduce reserves. Root can always do this.
        type UpdateOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;

        /// Unix time
        type UnixTime: UnixTime;

        /// Assets for deposit/withdraw collateral assets to/from loans module
        type Assets: Transfer<Self::AccountId, AssetId = CurrencyId, Balance = Balance>
            + Inspect<Self::AccountId, AssetId = CurrencyId, Balance = Balance>
            + Mutate<Self::AccountId, AssetId = CurrencyId, Balance = Balance>;

        /// Reward asset id.
        #[pallet::constant]
        type RewardAssetId: Get<AssetIdOf<Self>>;

        /// Reference currency for expressing asset prices. Example: USD, IBTC.
        #[pallet::constant]
        type ReferenceAssetId: Get<AssetIdOf<Self>>;
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Insufficient liquidity to borrow more or disable collateral
        InsufficientLiquidity,
        /// Insufficient deposit to redeem
        InsufficientDeposit,
        /// Repay amount greater than allowed
        TooMuchRepay,
        /// Asset already enabled/disabled collateral
        DuplicateOperation,
        /// No deposit asset
        NoDeposit,
        /// Repay amount more than collateral amount
        InsufficientCollateral,
        /// Liquidator is same as borrower
        LiquidatorIsBorrower,
        /// Deposits are not used as a collateral
        DepositsAreNotCollateral,
        /// Insufficient shortfall to repay
        InsufficientShortfall,
        /// Insufficient reserves
        InsufficientReserves,
        /// Invalid rate model params
        InvalidRateModelParam,
        /// Market not activated
        MarketNotActivated,
        /// Oracle price not ready
        PriceOracleNotReady,
        /// Oracle price is zero
        PriceIsZero,
        /// Invalid asset id
        InvalidCurrencyId,
        /// Invalid lend_token id
        InvalidLendTokenId,
        /// Market does not exist
        MarketDoesNotExist,
        /// Market already exists
        MarketAlreadyExists,
        /// New markets must have a pending state
        NewMarketMustHavePendingState,
        /// Upper bound of supplying is exceeded
        SupplyCapacityExceeded,
        /// Upper bound of borrowing is exceeded
        BorrowCapacityExceeded,
        /// Insufficient cash in the pool
        InsufficientCash,
        /// The factor should be greater than 0% and less than 100%
        InvalidFactor,
        /// The supply cap cannot be zero
        InvalidSupplyCap,
        /// The exchange rate should be greater than 0.02 and less than 1
        InvalidExchangeRate,
        /// Amount cannot be zero
        InvalidAmount,
        /// Locking collateral failed. The account has no `free` tokens.
        DepositAllCollateralFailed,
        /// Unlocking collateral failed. The account has no `reserved` tokens.
        WithdrawAllCollateralFailed,
        /// Tokens already locked for a different purpose than borrow collateral
        TokensAlreadyLocked,
        /// Payer cannot be signer
        PayerIsSigner,
        /// Codec error
        CodecError,
        /// Collateral is reserved and cannot be liquidated
        CollateralReserved,
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub (crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Enable collateral for certain asset
        /// [sender, asset_id]
        DepositCollateral(T::AccountId, AssetIdOf<T>, BalanceOf<T>),
        /// Disable collateral for certain asset
        /// [sender, asset_id]
        WithdrawCollateral(T::AccountId, AssetIdOf<T>, BalanceOf<T>),
        /// Event emitted when assets are deposited
        /// [sender, asset_id, amount]
        Deposited(T::AccountId, AssetIdOf<T>, BalanceOf<T>),
        /// Event emitted when assets are redeemed
        /// [sender, asset_id, amount]
        Redeemed(T::AccountId, AssetIdOf<T>, BalanceOf<T>),
        /// Event emitted when cash is borrowed
        /// [sender, asset_id, amount]
        Borrowed(T::AccountId, AssetIdOf<T>, BalanceOf<T>),
        /// Event emitted when a borrow is repaid
        /// [sender, asset_id, amount]
        RepaidBorrow(T::AccountId, AssetIdOf<T>, BalanceOf<T>),
        /// Event emitted when a borrow is liquidated
        /// [liquidator, borrower, liquidation_asset_id, collateral_asset_id, repay_amount, collateral_amount]
        LiquidatedBorrow(
            T::AccountId,
            T::AccountId,
            AssetIdOf<T>,
            AssetIdOf<T>,
            BalanceOf<T>,
            BalanceOf<T>,
        ),
        /// Event emitted when the reserves are reduced
        /// [admin, asset_id, reduced_amount, total_reserves]
        ReservesReduced(T::AccountId, AssetIdOf<T>, BalanceOf<T>, BalanceOf<T>),
        /// Event emitted when the reserves are added
        /// [admin, asset_id, added_amount, total_reserves]
        ReservesAdded(T::AccountId, AssetIdOf<T>, BalanceOf<T>, BalanceOf<T>),
        /// New market is set
        /// [new_interest_rate_model]
        NewMarket(AssetIdOf<T>, Market<BalanceOf<T>>),
        /// Event emitted when a market is activated
        /// [admin, asset_id]
        ActivatedMarket(AssetIdOf<T>),
        /// New market parameters is updated
        /// [admin, asset_id]
        UpdatedMarket(AssetIdOf<T>, Market<BalanceOf<T>>),
        /// Reward added
        RewardAdded(T::AccountId, BalanceOf<T>),
        /// Reward withdrawed
        RewardWithdrawn(T::AccountId, BalanceOf<T>),
        /// Event emitted when market reward speed updated.
        MarketRewardSpeedUpdated(AssetIdOf<T>, BalanceOf<T>, BalanceOf<T>),
        /// Deposited when Reward is distributed to a supplier
        DistributedSupplierReward(AssetIdOf<T>, T::AccountId, BalanceOf<T>, BalanceOf<T>),
        /// Deposited when Reward is distributed to a borrower
        DistributedBorrowerReward(AssetIdOf<T>, T::AccountId, BalanceOf<T>, BalanceOf<T>),
        /// Reward Paid for user
        RewardPaid(T::AccountId, BalanceOf<T>),
        /// Event emitted when the incentive reserves are redeemed and transfer to receiver's account
        /// [receive_account_id, asset_id, reduced_amount]
        IncentiveReservesReduced(T::AccountId, AssetIdOf<T>, BalanceOf<T>),
    }

    /// The timestamp of the last calculation of accrued interest
    #[pallet::storage]
    #[pallet::getter(fn last_accrued_interest_time)]
    pub type LastAccruedInterestTime<T: Config> = StorageMap<_, Blake2_128Concat, AssetIdOf<T>, Timestamp, ValueQuery>;

    /// Total amount of outstanding borrows of the underlying in this market
    /// CurrencyId -> Balance
    #[pallet::storage]
    #[pallet::getter(fn total_borrows)]
    pub type TotalBorrows<T: Config> = StorageMap<_, Blake2_128Concat, AssetIdOf<T>, BalanceOf<T>, ValueQuery>;

    /// Total amount of reserves of the underlying held in this market
    /// CurrencyId -> Balance
    #[pallet::storage]
    #[pallet::getter(fn total_reserves)]
    pub type TotalReserves<T: Config> = StorageMap<_, Blake2_128Concat, AssetIdOf<T>, BalanceOf<T>, ValueQuery>;

    /// Mapping of account addresses to outstanding borrow balances
    /// CurrencyId -> Owner -> BorrowSnapshot
    #[pallet::storage]
    #[pallet::getter(fn account_borrows)]
    pub type AccountBorrows<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        AssetIdOf<T>,
        Blake2_128Concat,
        T::AccountId,
        BorrowSnapshot<BalanceOf<T>>,
        ValueQuery,
    >;

    /// Mapping of account addresses to deposit details
    /// CollateralType -> Owner -> Deposits
    #[pallet::storage]
    #[pallet::getter(fn account_deposits)]
    pub type AccountDeposits<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, AssetIdOf<T>, Blake2_128Concat, T::AccountId, BalanceOf<T>, ValueQuery>;

    /// Mapping of account addresses to total deposit interest accrual
    /// CurrencyId -> Owner -> EarnedSnapshot
    #[pallet::storage]
    #[pallet::getter(fn account_earned)]
    pub type AccountEarned<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        AssetIdOf<T>,
        Blake2_128Concat,
        T::AccountId,
        EarnedSnapshot<BalanceOf<T>>,
        ValueQuery,
    >;

    /// Accumulator of the total earned interest rate since the opening of the market
    /// CurrencyId -> u128
    #[pallet::storage]
    #[pallet::getter(fn borrow_index)]
    pub type BorrowIndex<T: Config> = StorageMap<_, Blake2_128Concat, AssetIdOf<T>, Rate, ValueQuery>;

    /// The exchange rate from the underlying to the internal collateral
    #[pallet::storage]
    #[pallet::getter(fn exchange_rate)]
    pub type ExchangeRate<T: Config> = StorageMap<_, Blake2_128Concat, AssetIdOf<T>, Rate, ValueQuery>;

    /// Mapping of borrow rate to currency type
    #[pallet::storage]
    #[pallet::getter(fn borrow_rate)]
    pub type BorrowRate<T: Config> = StorageMap<_, Blake2_128Concat, AssetIdOf<T>, Rate, ValueQuery>;

    /// Mapping of supply rate to currency type
    #[pallet::storage]
    #[pallet::getter(fn supply_rate)]
    pub type SupplyRate<T: Config> = StorageMap<_, Blake2_128Concat, AssetIdOf<T>, Rate, ValueQuery>;

    /// Borrow utilization ratio
    #[pallet::storage]
    #[pallet::getter(fn utilization_ratio)]
    pub type UtilizationRatio<T: Config> = StorageMap<_, Blake2_128Concat, AssetIdOf<T>, Ratio, ValueQuery>;

    /// Mapping of asset id to its market
    #[pallet::storage]
    pub type Markets<T: Config> = StorageMap<_, Blake2_128Concat, AssetIdOf<T>, Market<BalanceOf<T>>>;

    /// Mapping of lend_token id to asset id
    /// `lend_token id`: voucher token id
    /// `asset id`: underlying token id
    #[pallet::storage]
    pub type UnderlyingAssetId<T: Config> = StorageMap<_, Blake2_128Concat, AssetIdOf<T>, AssetIdOf<T>>;

    /// Mapping of token id to supply reward speed
    #[pallet::storage]
    #[pallet::getter(fn reward_supply_speed)]
    pub type RewardSupplySpeed<T: Config> = StorageMap<_, Blake2_128Concat, AssetIdOf<T>, BalanceOf<T>, ValueQuery>;

    /// Mapping of token id to borrow reward speed
    #[pallet::storage]
    #[pallet::getter(fn reward_borrow_speed)]
    pub type RewardBorrowSpeed<T: Config> = StorageMap<_, Blake2_128Concat, AssetIdOf<T>, BalanceOf<T>, ValueQuery>;

    /// The Reward market supply state for each market
    #[pallet::storage]
    #[pallet::getter(fn reward_supply_state)]
    pub type RewardSupplyState<T: Config> =
        StorageMap<_, Blake2_128Concat, AssetIdOf<T>, RewardMarketState<T::BlockNumber, BalanceOf<T>>, ValueQuery>;

    /// The Reward market borrow state for each market
    #[pallet::storage]
    #[pallet::getter(fn reward_borrow_state)]
    pub type RewardBorrowState<T: Config> =
        StorageMap<_, Blake2_128Concat, AssetIdOf<T>, RewardMarketState<T::BlockNumber, BalanceOf<T>>, ValueQuery>;

    ///  The Reward index for each market for each supplier as of the last time they accrued Reward
    #[pallet::storage]
    #[pallet::getter(fn reward_supplier_index)]
    pub type RewardSupplierIndex<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, AssetIdOf<T>, Blake2_128Concat, T::AccountId, BalanceOf<T>, ValueQuery>;

    ///  The Reward index for each market for each borrower as of the last time they accrued Reward
    #[pallet::storage]
    #[pallet::getter(fn reward_borrower_index)]
    pub type RewardBorrowerIndex<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, AssetIdOf<T>, Blake2_128Concat, T::AccountId, BalanceOf<T>, ValueQuery>;

    /// The reward accrued but not yet transferred to each user.
    #[pallet::storage]
    #[pallet::getter(fn reward_accrued)]
    pub type RewardAccrued<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, BalanceOf<T>, ValueQuery>;

    /// The maximum allowed exchange rate for a market.
    #[pallet::storage]
    #[pallet::getter(fn max_exchange_rate)]
    pub type MaxExchangeRate<T: Config> = StorageValue<_, Rate, ValueQuery>;

    /// The minimum allowed exchange rate for a market.
    #[pallet::storage]
    #[pallet::getter(fn min_exchange_rate)]
    pub type MinExchangeRate<T: Config> = StorageValue<_, Rate, ValueQuery>;

    /// DefaultVersion is using for initialize the StorageVersion
    #[pallet::type_value]
    pub(super) fn DefaultVersion<T: Config>() -> Versions {
        Versions::V0
    }

    /// Storage version of the pallet.
    #[pallet::storage]
    pub(crate) type StorageVersion<T: Config> = StorageValue<_, Versions, ValueQuery, DefaultVersion<T>>;

    #[pallet::genesis_config]
    pub struct GenesisConfig {
        pub max_exchange_rate: Rate,
        pub min_exchange_rate: Rate,
    }

    #[cfg(feature = "std")]
    impl Default for GenesisConfig {
        fn default() -> Self {
            Self {
                max_exchange_rate: Default::default(),
                min_exchange_rate: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig {
        fn build(&self) {
            MaxExchangeRate::<T>::put(&self.max_exchange_rate);
            MinExchangeRate::<T>::put(&self.min_exchange_rate);
        }
    }

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Stores a new market and its related currency. Returns `Err` if a currency
        /// is not attached to an existent market.
        ///
        /// All provided market states must be `Pending`, otherwise an error will be returned.
        ///
        /// If a currency is already attached to a market, then the market will be replaced
        /// by the new provided value.
        ///
        /// The lend_token id and asset id are bound, the lend_token id of new provided market cannot
        /// be duplicated with the existing one, otherwise it will return `InvalidLendTokenId`.
        ///
        /// - `asset_id`: Market related currency
        /// - `market`: The market that is going to be stored
        #[pallet::weight(<T as Config>::WeightInfo::add_market())]
        #[transactional]
        pub fn add_market(
            origin: OriginFor<T>,
            asset_id: AssetIdOf<T>,
            market: Market<BalanceOf<T>>,
        ) -> DispatchResultWithPostInfo {
            T::UpdateOrigin::ensure_origin(origin)?;
            ensure!(!Markets::<T>::contains_key(asset_id), Error::<T>::MarketAlreadyExists);
            ensure!(
                market.state == MarketState::Pending,
                Error::<T>::NewMarketMustHavePendingState
            );
            ensure!(market.rate_model.check_model(), Error::<T>::InvalidRateModelParam);
            ensure!(
                market.collateral_factor >= Ratio::zero() && market.collateral_factor < Ratio::one(),
                Error::<T>::InvalidFactor,
            );
            ensure!(
                market.liquidation_threshold < Ratio::one() && market.liquidation_threshold >= market.collateral_factor,
                Error::<T>::InvalidFactor
            );
            ensure!(
                market.reserve_factor > Ratio::zero() && market.reserve_factor < Ratio::one(),
                Error::<T>::InvalidFactor,
            );
            ensure!(
                market.liquidate_incentive_reserved_factor > Ratio::zero()
                    && market.liquidate_incentive_reserved_factor < Ratio::one(),
                Error::<T>::InvalidFactor,
            );
            ensure!(market.supply_cap > Zero::zero(), Error::<T>::InvalidSupplyCap,);

            // Ensures a given `lend_token_id` not exists on the `Market` and `UnderlyingAssetId`.
            Self::ensure_lend_token(market.lend_token_id)?;
            // Update storage of `Market` and `UnderlyingAssetId`
            Markets::<T>::insert(asset_id, market.clone());
            UnderlyingAssetId::<T>::insert(market.lend_token_id, asset_id);

            // Init the ExchangeRate and BorrowIndex for asset
            ExchangeRate::<T>::insert(asset_id, Self::min_exchange_rate());
            BorrowIndex::<T>::insert(asset_id, Rate::one());

            Self::deposit_event(Event::<T>::NewMarket(asset_id, market));
            Ok(().into())
        }

        /// Activates a market. Returns `Err` if the market currency does not exist.
        ///
        /// If the market is already activated, does nothing.
        ///
        /// - `asset_id`: Market related currency
        #[pallet::weight(<T as Config>::WeightInfo::activate_market())]
        #[transactional]
        pub fn activate_market(origin: OriginFor<T>, asset_id: AssetIdOf<T>) -> DispatchResultWithPostInfo {
            T::UpdateOrigin::ensure_origin(origin)?;
            Self::mutate_market(asset_id, |stored_market| {
                if let MarketState::Active = stored_market.state {
                    return stored_market.clone();
                }
                stored_market.state = MarketState::Active;
                stored_market.clone()
            })?;
            Self::deposit_event(Event::<T>::ActivatedMarket(asset_id));
            Ok(().into())
        }

        /// Updates the rate model of a stored market. Returns `Err` if the market
        /// currency does not exist or the rate model is invalid.
        ///
        /// - `asset_id`: Market related currency
        /// - `rate_model`: The new rate model to be updated
        #[pallet::weight(<T as Config>::WeightInfo::update_rate_model())]
        #[transactional]
        pub fn update_rate_model(
            origin: OriginFor<T>,
            asset_id: AssetIdOf<T>,
            rate_model: InterestRateModel,
        ) -> DispatchResultWithPostInfo {
            T::UpdateOrigin::ensure_origin(origin)?;
            ensure!(rate_model.check_model(), Error::<T>::InvalidRateModelParam);
            let market = Self::mutate_market(asset_id, |stored_market| {
                stored_market.rate_model = rate_model;
                stored_market.clone()
            })?;
            Self::deposit_event(Event::<T>::UpdatedMarket(asset_id, market));

            Ok(().into())
        }

        /// Updates a stored market. Returns `Err` if the market currency does not exist.
        ///
        /// - `asset_id`: market related currency
        /// - `collateral_factor`: the collateral utilization ratio
        /// - `reserve_factor`: fraction of interest currently set aside for reserves
        /// - `close_factor`: maximum liquidation ratio at one time
        /// - `liquidate_incentive`: liquidation incentive ratio
        /// - `cap`: market capacity
        #[pallet::weight(<T as Config>::WeightInfo::update_market())]
        #[transactional]
        pub fn update_market(
            origin: OriginFor<T>,
            asset_id: AssetIdOf<T>,
            collateral_factor: Option<Ratio>,
            liquidation_threshold: Option<Ratio>,
            reserve_factor: Option<Ratio>,
            close_factor: Option<Ratio>,
            liquidate_incentive_reserved_factor: Option<Ratio>,
            liquidate_incentive: Option<Rate>,
            supply_cap: Option<BalanceOf<T>>,
            borrow_cap: Option<BalanceOf<T>>,
        ) -> DispatchResultWithPostInfo {
            T::UpdateOrigin::ensure_origin(origin)?;

            let market = Self::market(asset_id)?;

            let collateral_factor = collateral_factor.unwrap_or(market.collateral_factor);
            let liquidation_threshold = liquidation_threshold.unwrap_or(market.liquidation_threshold);
            let reserve_factor = reserve_factor.unwrap_or(market.reserve_factor);
            let close_factor = close_factor.unwrap_or(market.close_factor);
            let liquidate_incentive_reserved_factor =
                liquidate_incentive_reserved_factor.unwrap_or(market.liquidate_incentive_reserved_factor);
            let liquidate_incentive = liquidate_incentive.unwrap_or(market.liquidate_incentive);
            let supply_cap = supply_cap.unwrap_or(market.supply_cap);
            let borrow_cap = borrow_cap.unwrap_or(market.borrow_cap);

            ensure!(
                collateral_factor >= Ratio::zero() && collateral_factor < Ratio::one(),
                Error::<T>::InvalidFactor
            );
            ensure!(
                liquidation_threshold >= collateral_factor && liquidation_threshold < Ratio::one(),
                Error::<T>::InvalidFactor
            );
            ensure!(
                reserve_factor > Ratio::zero() && reserve_factor < Ratio::one(),
                Error::<T>::InvalidFactor
            );
            ensure!(supply_cap > Zero::zero(), Error::<T>::InvalidSupplyCap);

            let market = Self::mutate_market(asset_id, |stored_market| {
                *stored_market = Market {
                    state: stored_market.state,
                    lend_token_id: stored_market.lend_token_id,
                    rate_model: stored_market.rate_model,
                    collateral_factor,
                    liquidation_threshold,
                    reserve_factor,
                    close_factor,
                    liquidate_incentive,
                    liquidate_incentive_reserved_factor,
                    supply_cap,
                    borrow_cap,
                };
                stored_market.clone()
            })?;
            Self::deposit_event(Event::<T>::UpdatedMarket(asset_id, market));

            Ok(().into())
        }

        /// Force updates a stored market. Returns `Err` if the market currency
        /// does not exist.
        ///
        /// - `asset_id`: market related currency
        /// - `market`: the new market parameters
        #[pallet::weight(<T as Config>::WeightInfo::force_update_market())]
        #[transactional]
        pub fn force_update_market(
            origin: OriginFor<T>,
            asset_id: AssetIdOf<T>,
            market: Market<BalanceOf<T>>,
        ) -> DispatchResultWithPostInfo {
            T::UpdateOrigin::ensure_origin(origin)?;
            ensure!(market.rate_model.check_model(), Error::<T>::InvalidRateModelParam);
            if UnderlyingAssetId::<T>::contains_key(market.lend_token_id) {
                ensure!(
                    Self::underlying_id(market.lend_token_id)? == asset_id,
                    Error::<T>::InvalidLendTokenId
                );
            }
            UnderlyingAssetId::<T>::insert(market.lend_token_id, asset_id);
            let updated_market = Self::mutate_market(asset_id, |stored_market| {
                *stored_market = market;
                stored_market.clone()
            })?;

            Self::deposit_event(Event::<T>::UpdatedMarket(asset_id, updated_market));
            Ok(().into())
        }

        /// Add reward for the pallet account.
        ///
        /// - `amount`: Reward amount added
        #[pallet::weight(<T as Config>::WeightInfo::add_reward())]
        #[transactional]
        pub fn add_reward(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            ensure!(!amount.is_zero(), Error::<T>::InvalidAmount);

            let reward_asset = T::RewardAssetId::get();
            let pool_account = Self::reward_account_id()?;

            let amount_to_transfer: Amount<T> = Amount::new(amount, reward_asset);
            amount_to_transfer.transfer(&who, &pool_account)?;

            Self::deposit_event(Event::<T>::RewardAdded(who, amount));

            Ok(().into())
        }

        /// Withdraw reward token from pallet account.
        ///
        /// The origin must conform to `UpdateOrigin`.
        ///
        /// - `target_account`: account receive reward token.
        /// - `amount`: Withdraw amount
        #[pallet::weight(<T as Config>::WeightInfo::withdraw_missing_reward())]
        #[transactional]
        pub fn withdraw_missing_reward(
            origin: OriginFor<T>,
            target_account: <T::Lookup as StaticLookup>::Source,
            amount: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            T::UpdateOrigin::ensure_origin(origin)?;
            ensure!(!amount.is_zero(), Error::<T>::InvalidAmount);

            let reward_asset = T::RewardAssetId::get();
            let pool_account = Self::reward_account_id()?;
            let target_account = T::Lookup::lookup(target_account)?;

            let amount_to_transfer: Amount<T> = Amount::new(amount, reward_asset);
            amount_to_transfer.transfer(&pool_account, &target_account)?;

            Self::deposit_event(Event::<T>::RewardWithdrawn(target_account, amount));

            Ok(().into())
        }

        /// Updates reward speed for the specified market
        ///
        /// The origin must conform to `UpdateOrigin`.
        ///
        /// - `asset_id`: Market related currency
        /// - `reward_per_block`: reward amount per block.
        #[pallet::weight(<T as Config>::WeightInfo::update_market_reward_speed())]
        #[transactional]
        pub fn update_market_reward_speed(
            origin: OriginFor<T>,
            asset_id: AssetIdOf<T>,
            supply_reward_per_block: Option<BalanceOf<T>>,
            borrow_reward_per_block: Option<BalanceOf<T>>,
        ) -> DispatchResultWithPostInfo {
            T::UpdateOrigin::ensure_origin(origin)?;
            Self::ensure_active_market(asset_id)?;

            let current_supply_speed = RewardSupplySpeed::<T>::get(asset_id);
            let current_borrow_speed = RewardBorrowSpeed::<T>::get(asset_id);

            let supply_reward_per_block = supply_reward_per_block.unwrap_or(current_supply_speed);
            let borrow_reward_per_block = borrow_reward_per_block.unwrap_or(current_borrow_speed);

            if supply_reward_per_block != current_supply_speed {
                Self::update_reward_supply_index(asset_id)?;
                RewardSupplySpeed::<T>::try_mutate(asset_id, |current_speed| -> DispatchResult {
                    *current_speed = supply_reward_per_block;
                    Ok(())
                })?;
            }

            if borrow_reward_per_block != current_borrow_speed {
                Self::update_reward_borrow_index(asset_id)?;
                RewardBorrowSpeed::<T>::try_mutate(asset_id, |current_speed| -> DispatchResult {
                    *current_speed = borrow_reward_per_block;
                    Ok(())
                })?;
            }

            Self::deposit_event(Event::<T>::MarketRewardSpeedUpdated(
                asset_id,
                supply_reward_per_block,
                borrow_reward_per_block,
            ));
            Ok(().into())
        }

        /// Claim reward from all market.
        #[pallet::weight(<T as Config>::WeightInfo::claim_reward())]
        #[transactional]
        pub fn claim_reward(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            for asset_id in Markets::<T>::iter_keys() {
                Self::collect_market_reward(asset_id, &who)?;
            }

            Self::pay_reward(&who)?;

            Ok(().into())
        }

        /// Claim reward from the specified market.
        ///
        /// - `asset_id`: Market related currency
        #[pallet::weight(<T as Config>::WeightInfo::claim_reward_for_market())]
        #[transactional]
        pub fn claim_reward_for_market(origin: OriginFor<T>, asset_id: AssetIdOf<T>) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            Self::collect_market_reward(asset_id, &who)?;

            Self::pay_reward(&who)?;

            Ok(().into())
        }

        /// Sender supplies assets into the market and receives internal supplies in exchange.
        ///
        /// - `asset_id`: the asset to be deposited.
        /// - `mint_amount`: the amount to be deposited.
        #[pallet::weight(<T as Config>::WeightInfo::mint())]
        #[transactional]
        pub fn mint(
            origin: OriginFor<T>,
            asset_id: AssetIdOf<T>,
            #[pallet::compact] mint_amount: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            ensure!(!mint_amount.is_zero(), Error::<T>::InvalidAmount);
            Self::do_mint(&who, asset_id, mint_amount)?;

            Ok(().into())
        }

        /// Sender redeems some of internal supplies in exchange for the underlying asset.
        ///
        /// - `asset_id`: the asset to be redeemed.
        /// - `redeem_amount`: the amount to be redeemed.
        #[pallet::weight(<T as Config>::WeightInfo::redeem())]
        #[transactional]
        pub fn redeem(
            origin: OriginFor<T>,
            asset_id: AssetIdOf<T>,
            #[pallet::compact] redeem_amount: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            ensure!(!redeem_amount.is_zero(), Error::<T>::InvalidAmount);

            let lend_token_id = Self::lend_token_id(asset_id)?;
            // if the receiver has collateral locked
            let deposit = Pallet::<T>::account_deposits(lend_token_id, &who);
            if deposit > 0 {
                // Withdraw the `lend_tokens` from the borrow collateral, so they are redeemable.
                // This assumes that a user cannot have both `free` and `locked` lend tokens at
                // the same time (for the purposes of lending and borrowing).
                let amount = Amount::<T>::new(redeem_amount, asset_id);
                let collateral = Self::recompute_collateral_amount(&amount)?;
                Self::do_withdraw_collateral(&who, collateral.currency(), collateral.amount())?;
            }
            Self::do_redeem(&who, asset_id, redeem_amount)?;

            Ok(().into())
        }

        /// Sender redeems all of internal supplies in exchange for the underlying asset.
        ///
        /// - `asset_id`: the asset to be redeemed.
        #[pallet::weight(<T as Config>::WeightInfo::redeem_all())]
        #[transactional]
        pub fn redeem_all(origin: OriginFor<T>, asset_id: AssetIdOf<T>) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin.clone())?;
            // This function is an almost 1:1 duplicate of the logic in `do_redeem`.
            // It could be refactored to compute the redeemable underlying
            // with `Self::recompute_underlying_amount(&Self::free_lend_tokens(asset_id, &who)?)?`
            // but that would cause the `accrue_interest_works_after_redeem_all` unit test to fail with
            // left: `1000000000003607`,
            // right: `1000000000003608`'

            // Chaining `calc_underlying_amount` and `calc_collateral_amount` continuously decreases
            // an amount because of rounding down, and having the current function call `do_redeem`
            // would perform three conversions: lend_token -> token -> lend_token -> token.
            // Calling `do_redeem_voucher` directly only performs one conversion: lend_token -> token,
            // avoiding this edge case.
            // TODO: investigate whether it is possible to implement the conversion functions
            // with guarantees that this always holds:
            // `calc_underlying_amount(calc_collateral_amount(x)) = calc_collateral_amount(calc_underlying_amount(x))`
            // Use the `converting_to_and_from_collateral_should_not_change_results` unit test to achieve this.
            // If there are leftover lend_tokens after a `redeem_all` (because of rounding down), it would make it
            // impossible to enforce "collateral toggle" state transitions.
            Self::ensure_active_market(asset_id)?;

            let lend_token_id = Self::lend_token_id(asset_id)?;
            // if the receiver has collateral locked
            let deposit = Pallet::<T>::account_deposits(lend_token_id, &who);
            if deposit > 0 {
                // then withdraw all collateral
                Self::withdraw_all_collateral(origin, asset_id)?;
            }

            // `do_redeem()` logic duplicate:
            Self::accrue_interest(asset_id)?;
            let exchange_rate = Self::exchange_rate_stored(asset_id)?;
            Self::update_earned_stored(&who, asset_id, exchange_rate)?;
            let lend_tokens = Self::free_lend_tokens(asset_id, &who)?;
            ensure!(!lend_tokens.is_zero(), Error::<T>::InvalidAmount);
            let redeem_amount = Self::do_redeem_voucher(&who, asset_id, lend_tokens.amount())?;
            Self::deposit_event(Event::<T>::Redeemed(who, asset_id, redeem_amount));

            Ok(().into())
        }

        /// Sender borrows assets from the protocol to their own address.
        ///
        /// - `asset_id`: the asset to be borrowed.
        /// - `borrow_amount`: the amount to be borrowed.
        #[pallet::weight(<T as Config>::WeightInfo::borrow())]
        #[transactional]
        pub fn borrow(
            origin: OriginFor<T>,
            asset_id: AssetIdOf<T>,
            #[pallet::compact] borrow_amount: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            ensure!(!borrow_amount.is_zero(), Error::<T>::InvalidAmount);
            Self::do_borrow(&who, asset_id, borrow_amount)?;

            Ok(().into())
        }

        /// Sender repays some of their debts.
        ///
        /// - `asset_id`: the asset to be repaid.
        /// - `repay_amount`: the amount to be repaid.
        #[pallet::weight(<T as Config>::WeightInfo::repay_borrow())]
        #[transactional]
        pub fn repay_borrow(
            origin: OriginFor<T>,
            asset_id: AssetIdOf<T>,
            #[pallet::compact] repay_amount: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            ensure!(!repay_amount.is_zero(), Error::<T>::InvalidAmount);
            Self::do_repay_borrow(&who, asset_id, repay_amount)?;

            Ok(().into())
        }

        /// Sender repays all of their debts.
        ///
        /// - `asset_id`: the asset to be repaid.
        #[pallet::weight(<T as Config>::WeightInfo::repay_borrow_all())]
        #[transactional]
        pub fn repay_borrow_all(origin: OriginFor<T>, asset_id: AssetIdOf<T>) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            Self::ensure_active_market(asset_id)?;
            Self::accrue_interest(asset_id)?;
            let account_borrows = Self::current_borrow_balance(&who, asset_id)?;
            ensure!(!account_borrows.is_zero(), Error::<T>::InvalidAmount);
            Self::do_repay_borrow(&who, asset_id, account_borrows)?;

            Ok(().into())
        }

        #[pallet::weight(<T as Config>::WeightInfo::collateral_asset())]
        #[transactional]
        pub fn deposit_all_collateral(origin: OriginFor<T>, asset_id: AssetIdOf<T>) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            let free_lend_tokens = Self::free_lend_tokens(asset_id, &who)?;
            ensure!(!free_lend_tokens.is_zero(), Error::<T>::DepositAllCollateralFailed);
            let reserved_lend_tokens = Self::reserved_lend_tokens(asset_id, &who)?;
            // This check could fail if `withdraw_all_collateral()` leaves leftover lend_tokens locked.
            // However the current implementation is guaranteed to withdraw everything.
            ensure!(reserved_lend_tokens.is_zero(), Error::<T>::TokensAlreadyLocked);
            Self::do_deposit_collateral(&who, free_lend_tokens.currency(), free_lend_tokens.amount())?;
            Ok(().into())
        }

        #[pallet::weight(<T as Config>::WeightInfo::collateral_asset())]
        #[transactional]
        pub fn withdraw_all_collateral(origin: OriginFor<T>, asset_id: AssetIdOf<T>) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            let lend_token_id = Self::lend_token_id(asset_id)?;
            let collateral = Self::account_deposits(lend_token_id, who.clone());
            ensure!(!collateral.is_zero(), Error::<T>::WithdrawAllCollateralFailed);
            Self::do_withdraw_collateral(&who, lend_token_id, collateral)?;
            Ok(().into())
        }

        /// The sender liquidates the borrower's collateral.
        ///
        /// - `borrower`: the borrower to be liquidated.
        /// - `liquidation_asset_id`: the assert to be liquidated.
        /// - `repay_amount`: the amount to be repaid borrow.
        /// - `collateral_asset_id`: The collateral to seize from the borrower.
        #[pallet::weight(<T as Config>::WeightInfo::liquidate_borrow())]
        #[transactional]
        pub fn liquidate_borrow(
            origin: OriginFor<T>,
            borrower: T::AccountId,
            liquidation_asset_id: AssetIdOf<T>,
            #[pallet::compact] repay_amount: BalanceOf<T>,
            collateral_asset_id: AssetIdOf<T>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            Self::accrue_interest(liquidation_asset_id)?;
            Self::accrue_interest(collateral_asset_id)?;
            ensure!(!repay_amount.is_zero(), Error::<T>::InvalidAmount);
            Self::do_liquidate_borrow(who, borrower, liquidation_asset_id, repay_amount, collateral_asset_id)?;
            Ok(().into())
        }

        /// Add reserves by transferring from payer.
        ///
        /// May only be called from `T::ReserveOrigin`.
        ///
        /// - `payer`: the payer account.
        /// - `asset_id`: the assets to be added.
        /// - `add_amount`: the amount to be added.
        #[pallet::weight(<T as Config>::WeightInfo::add_reserves())]
        #[transactional]
        pub fn add_reserves(
            origin: OriginFor<T>,
            payer: <T::Lookup as StaticLookup>::Source,
            asset_id: AssetIdOf<T>,
            #[pallet::compact] add_amount: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            T::ReserveOrigin::ensure_origin(origin)?;
            let payer = T::Lookup::lookup(payer)?;
            Self::ensure_active_market(asset_id)?;

            ensure!(!add_amount.is_zero(), Error::<T>::InvalidAmount);
            let amount_to_transfer: Amount<T> = Amount::new(add_amount, asset_id);
            amount_to_transfer.transfer(&payer, &Self::account_id())?;
            let total_reserves = Self::total_reserves(asset_id);
            let total_reserves_new = total_reserves
                .checked_add(add_amount)
                .ok_or(ArithmeticError::Overflow)?;
            TotalReserves::<T>::insert(asset_id, total_reserves_new);

            Self::deposit_event(Event::<T>::ReservesAdded(
                payer,
                asset_id,
                add_amount,
                total_reserves_new,
            ));

            Ok(().into())
        }

        /// Reduces reserves by transferring to receiver.
        ///
        /// May only be called from `T::ReserveOrigin`.
        ///
        /// - `receiver`: the receiver account.
        /// - `asset_id`: the assets to be reduced.
        /// - `reduce_amount`: the amount to be reduced.
        #[pallet::weight(<T as Config>::WeightInfo::reduce_reserves())]
        #[transactional]
        pub fn reduce_reserves(
            origin: OriginFor<T>,
            receiver: <T::Lookup as StaticLookup>::Source,
            asset_id: AssetIdOf<T>,
            #[pallet::compact] reduce_amount: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            T::ReserveOrigin::ensure_origin(origin)?;
            let receiver = T::Lookup::lookup(receiver)?;
            Self::ensure_active_market(asset_id)?;
            ensure!(!reduce_amount.is_zero(), Error::<T>::InvalidAmount);

            let total_reserves = Self::total_reserves(asset_id);
            if reduce_amount > total_reserves {
                return Err(Error::<T>::InsufficientReserves.into());
            }
            let total_reserves_new = total_reserves
                .checked_sub(reduce_amount)
                .ok_or(ArithmeticError::Underflow)?;
            TotalReserves::<T>::insert(asset_id, total_reserves_new);

            let amount_to_transfer: Amount<T> = Amount::new(reduce_amount, asset_id);
            amount_to_transfer.transfer(&Self::account_id(), &receiver)?;

            Self::deposit_event(Event::<T>::ReservesReduced(
                receiver,
                asset_id,
                reduce_amount,
                total_reserves_new,
            ));

            Ok(().into())
        }

        /// Sender redeems some of internal supplies in exchange for the underlying asset.
        ///
        /// - `asset_id`: the asset to be redeemed.
        /// - `redeem_amount`: the amount to be redeemed.
        #[pallet::weight(<T as Config>::WeightInfo::redeem()+<T as Config>::WeightInfo::reduce_reserves())]
        #[transactional]
        pub fn reduce_incentive_reserves(
            origin: OriginFor<T>,
            receiver: <T::Lookup as StaticLookup>::Source,
            asset_id: AssetIdOf<T>,
            #[pallet::compact] redeem_amount: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            T::ReserveOrigin::ensure_origin(origin)?;
            ensure!(!redeem_amount.is_zero(), Error::<T>::InvalidAmount);
            let receiver = T::Lookup::lookup(receiver)?;
            let from = Self::incentive_reward_account_id()?;
            Self::ensure_active_market(asset_id)?;
            let exchange_rate = Self::exchange_rate_stored(asset_id)?;
            let voucher_amount = Self::calc_collateral_amount(redeem_amount, exchange_rate)?;
            let redeem_amount = Self::do_redeem_voucher(&from, asset_id, voucher_amount)?;

            let amount_to_transfer: Amount<T> = Amount::new(redeem_amount, asset_id);
            amount_to_transfer.transfer(&from, &receiver)?;

            Self::deposit_event(Event::<T>::IncentiveReservesReduced(receiver, asset_id, redeem_amount));
            Ok(().into())
        }
    }
}

impl<T: Config> Pallet<T> {
    pub fn account_id() -> T::AccountId {
        T::PalletId::get().into_account_truncating()
    }

    pub fn get_account_liquidity(account: &T::AccountId) -> Result<(Liquidity, Shortfall), DispatchError> {
        let total_borrow_value = Self::total_borrowed_value(account)?;
        let total_collateral_value = Self::total_collateral_value(account)?;

        log::trace!(
            target: "loans::get_account_liquidity",
            "account: {:?}, total_borrow_value: {:?}, total_collateral_value: {:?}",
            account,
            total_borrow_value.into_inner(),
            total_collateral_value.into_inner(),
        );
        if total_collateral_value > total_borrow_value {
            Ok((
                total_collateral_value
                    .checked_sub(&total_borrow_value)
                    .ok_or(ArithmeticError::Underflow)?,
                FixedU128::zero(),
            ))
        } else {
            Ok((
                FixedU128::zero(),
                total_borrow_value
                    .checked_sub(&total_collateral_value)
                    .ok_or(ArithmeticError::Underflow)?,
            ))
        }
    }

    pub fn get_account_liquidation_threshold_liquidity(
        account: &T::AccountId,
    ) -> Result<(Liquidity, Shortfall), DispatchError> {
        let total_borrow_value = Self::total_borrowed_value(account)?;
        let total_collateral_value = Self::total_liquidation_threshold_value(account)?;

        log::trace!(
            target: "loans::get_account_liquidation_threshold_liquidity",
            "account: {:?}, total_borrow_value: {:?}, total_collateral_value: {:?}",
            account,
            total_borrow_value.into_inner(),
            total_collateral_value.into_inner(),
        );
        if total_collateral_value > total_borrow_value {
            Ok((total_collateral_value - total_borrow_value, FixedU128::zero()))
        } else {
            Ok((FixedU128::zero(), total_borrow_value - total_collateral_value))
        }
    }

    fn total_borrowed_value(borrower: &T::AccountId) -> Result<FixedU128, DispatchError> {
        let mut total_borrow_value: FixedU128 = FixedU128::zero();
        for (asset_id, _) in Self::active_markets() {
            let currency_borrow_amount = Self::current_borrow_balance(borrower, asset_id)?;
            if currency_borrow_amount.is_zero() {
                continue;
            }
            total_borrow_value = Self::get_asset_value(asset_id, currency_borrow_amount)?
                .checked_add(&total_borrow_value)
                .ok_or(ArithmeticError::Overflow)?;
        }

        Ok(total_borrow_value)
    }

    fn collateral_balance(
        asset_id: AssetIdOf<T>,
        lend_token_amount: BalanceOf<T>,
    ) -> Result<BalanceOf<T>, DispatchError> {
        let exchange_rate = Self::exchange_rate_stored(asset_id)?;
        let underlying_amount = Self::calc_underlying_amount(lend_token_amount, exchange_rate)?;
        let market = Self::market(asset_id)?;
        let effects_amount = market.collateral_factor.mul_ceil(underlying_amount);

        Ok(BalanceOf::<T>::saturated_from(effects_amount))
    }

    fn collateral_amount_value(
        asset_id: AssetIdOf<T>,
        lend_token_amount: BalanceOf<T>,
    ) -> Result<FixedU128, DispatchError> {
        let effects_amount = Self::collateral_balance(asset_id, lend_token_amount)?;
        Self::get_asset_value(asset_id, effects_amount)
    }

    fn collateral_asset_value(supplier: &T::AccountId, asset_id: AssetIdOf<T>) -> Result<FixedU128, DispatchError> {
        let lend_token_id = Self::lend_token_id(asset_id)?;
        if !AccountDeposits::<T>::contains_key(lend_token_id, supplier) {
            return Ok(FixedU128::zero());
        }
        let deposits = Self::account_deposits(lend_token_id, supplier);
        if deposits.is_zero() {
            return Ok(FixedU128::zero());
        }
        Self::collateral_amount_value(asset_id, deposits)
    }

    fn liquidation_threshold_asset_value(
        borrower: &T::AccountId,
        asset_id: AssetIdOf<T>,
    ) -> Result<FixedU128, DispatchError> {
        let lend_token_id = Self::lend_token_id(asset_id)?;
        if !AccountDeposits::<T>::contains_key(lend_token_id, borrower) {
            return Ok(FixedU128::zero());
        }
        let deposits = Self::account_deposits(lend_token_id, borrower);
        if deposits.is_zero() {
            return Ok(FixedU128::zero());
        }
        let exchange_rate = Self::exchange_rate_stored(asset_id)?;
        let underlying_amount = Self::calc_underlying_amount(deposits, exchange_rate)?;
        let market = Self::market(asset_id)?;
        let effects_amount = market.liquidation_threshold.mul_ceil(underlying_amount);

        Self::get_asset_value(asset_id, effects_amount)
    }

    fn total_collateral_value(supplier: &T::AccountId) -> Result<FixedU128, DispatchError> {
        let mut total_asset_value: FixedU128 = FixedU128::zero();
        for (asset_id, _market) in Self::active_markets() {
            total_asset_value = total_asset_value
                .checked_add(&Self::collateral_asset_value(supplier, asset_id)?)
                .ok_or(ArithmeticError::Overflow)?;
        }

        Ok(total_asset_value)
    }

    fn total_liquidation_threshold_value(borrower: &T::AccountId) -> Result<FixedU128, DispatchError> {
        let mut total_asset_value: FixedU128 = FixedU128::zero();
        for (asset_id, _market) in Self::active_markets() {
            total_asset_value = total_asset_value
                .checked_add(&Self::liquidation_threshold_asset_value(borrower, asset_id)?)
                .ok_or(ArithmeticError::Overflow)?;
        }

        Ok(total_asset_value)
    }

    /// Checks if the redeemer should be allowed to redeem tokens in given market.
    /// Takes into account both `free` and `locked` (i.e. deposited as collateral) lend_tokens.
    fn redeem_allowed(asset_id: AssetIdOf<T>, redeemer: &T::AccountId, voucher_amount: BalanceOf<T>) -> DispatchResult {
        log::trace!(
            target: "loans::redeem_allowed",
            "asset_id: {:?}, redeemer: {:?}, voucher_amount: {:?}",
            asset_id,
            redeemer,
            voucher_amount,
        );
        let lend_token_id = Self::lend_token_id(asset_id)?;
        if Self::balance(lend_token_id, redeemer) < voucher_amount {
            return Err(Error::<T>::InsufficientDeposit.into());
        }

        // Ensure there is enough cash in the market
        let exchange_rate = Self::exchange_rate_stored(asset_id)?;
        let redeem_amount = Self::calc_underlying_amount(voucher_amount, exchange_rate)?;
        Self::ensure_enough_cash(asset_id, redeem_amount)?;

        // Ensure that withdrawing deposited collateral doesn't leave the user undercollateralized.
        let collateral_amount = voucher_amount.saturating_sub(Self::free_lend_tokens(asset_id, redeemer)?.amount());
        let collateral_underlying_amount = Self::calc_underlying_amount(collateral_amount, exchange_rate)?;
        let market = Self::market(asset_id)?;
        let effects_amount = market.collateral_factor.mul_ceil(collateral_underlying_amount);
        let redeem_effects_value = Self::get_asset_value(asset_id, effects_amount)?;
        log::trace!(
            target: "loans::redeem_allowed",
            "redeem_amount: {:?}, redeem_effects_value: {:?}",
            redeem_amount,
            redeem_effects_value.into_inner(),
        );

        Self::ensure_liquidity(redeemer, redeem_effects_value)?;

        Ok(())
    }

    #[require_transactional]
    pub fn do_redeem_voucher(
        who: &T::AccountId,
        asset_id: AssetIdOf<T>,
        voucher_amount: BalanceOf<T>,
    ) -> Result<BalanceOf<T>, DispatchError> {
        Self::redeem_allowed(asset_id, who, voucher_amount)?;
        Self::update_reward_supply_index(asset_id)?;
        Self::distribute_supplier_reward(asset_id, who)?;

        let exchange_rate = Self::exchange_rate_stored(asset_id)?;
        let redeem_amount = Self::calc_underlying_amount(voucher_amount, exchange_rate)?;

        let lend_token_id = Self::lend_token_id(asset_id)?;
        let lend_token_amount: Amount<T> = Amount::new(voucher_amount, lend_token_id);

        // Need to first `lock_on` in order to `burn_from` because:
        // 1) only the `free` lend_tokens are redeemable
        // 2) `burn_from` can only be called on locked tokens.
        lend_token_amount.lock_on(who)?;
        lend_token_amount.burn_from(who)?;

        let amount_to_transfer: Amount<T> = Amount::new(redeem_amount, asset_id);
        amount_to_transfer
            .transfer(&Self::account_id(), who)
            .map_err(|_| Error::<T>::InsufficientCash)?;

        Ok(redeem_amount)
    }

    /// Borrower shouldn't borrow more than his total collateral value
    fn borrow_allowed(asset_id: AssetIdOf<T>, borrower: &T::AccountId, borrow_amount: BalanceOf<T>) -> DispatchResult {
        Self::ensure_under_borrow_cap(asset_id, borrow_amount)?;
        Self::ensure_enough_cash(asset_id, borrow_amount)?;
        let borrow_value = Self::get_asset_value(asset_id, borrow_amount)?;
        Self::ensure_liquidity(borrower, borrow_value)?;

        Ok(())
    }

    #[require_transactional]
    fn do_repay_borrow_with_amount(
        borrower: &T::AccountId,
        asset_id: AssetIdOf<T>,
        account_borrows: BalanceOf<T>,
        repay_amount: BalanceOf<T>,
    ) -> DispatchResult {
        if account_borrows < repay_amount {
            return Err(Error::<T>::TooMuchRepay.into());
        }
        Self::update_reward_borrow_index(asset_id)?;
        Self::distribute_borrower_reward(asset_id, borrower)?;

        let amount_to_transfer: Amount<T> = Amount::new(repay_amount, asset_id);
        amount_to_transfer.transfer(borrower, &Self::account_id())?;

        let account_borrows_new = account_borrows
            .checked_sub(repay_amount)
            .ok_or(ArithmeticError::Underflow)?;
        let total_borrows = Self::total_borrows(asset_id);
        // NOTE : total_borrows use a different way to calculate interest
        // so when user repays all borrows, total_borrows can be less than account_borrows
        // which will cause it to fail with `ArithmeticError::Underflow`
        //
        // Change it back to checked_sub will cause Underflow
        let total_borrows_new = total_borrows.saturating_sub(repay_amount);
        AccountBorrows::<T>::insert(
            asset_id,
            borrower,
            BorrowSnapshot {
                principal: account_borrows_new,
                borrow_index: Self::borrow_index(asset_id),
            },
        );
        TotalBorrows::<T>::insert(asset_id, total_borrows_new);

        Ok(())
    }

    // Calculates and returns the most recent amount of borrowed balance of `currency_id`
    // for `who`.
    pub fn current_borrow_balance(who: &T::AccountId, asset_id: AssetIdOf<T>) -> Result<BalanceOf<T>, DispatchError> {
        let snapshot: BorrowSnapshot<BalanceOf<T>> = Self::account_borrows(asset_id, who);
        if snapshot.principal.is_zero() || snapshot.borrow_index.is_zero() {
            return Ok(Zero::zero());
        }
        // Calculate new borrow balance using the interest index:
        // recent_borrow_balance = snapshot.principal * borrow_index / snapshot.borrow_index
        let recent_borrow_balance = Self::borrow_index(asset_id)
            .checked_div(&snapshot.borrow_index)
            .and_then(|r| r.checked_mul_int(snapshot.principal))
            .ok_or(ArithmeticError::Overflow)?;

        Ok(recent_borrow_balance)
    }

    #[require_transactional]
    fn update_earned_stored(who: &T::AccountId, asset_id: AssetIdOf<T>, exchange_rate: Rate) -> DispatchResult {
        let deposits = AccountDeposits::<T>::get(asset_id, who);
        let account_earned = AccountEarned::<T>::get(asset_id, who);
        let total_earned_prior_new = exchange_rate
            .checked_sub(&account_earned.exchange_rate_prior)
            .and_then(|r| r.checked_mul_int(deposits))
            .and_then(|r| r.checked_add(account_earned.total_earned_prior))
            .ok_or(ArithmeticError::Overflow)?;

        AccountEarned::<T>::insert(
            asset_id,
            who,
            EarnedSnapshot {
                exchange_rate_prior: exchange_rate,
                total_earned_prior: total_earned_prior_new,
            },
        );

        Ok(())
    }

    /// Checks if the liquidation should be allowed to occur
    fn liquidate_borrow_allowed(
        borrower: &T::AccountId,
        liquidation_asset_id: AssetIdOf<T>,
        repay_amount: BalanceOf<T>,
        market: &Market<BalanceOf<T>>,
    ) -> DispatchResult {
        log::trace!(
            target: "loans::liquidate_borrow_allowed",
            "borrower: {:?}, liquidation_asset_id {:?}, repay_amount {:?}, market: {:?}",
            borrower,
            liquidation_asset_id,
            repay_amount,
            market
        );
        let (_, shortfall) = Self::get_account_liquidation_threshold_liquidity(borrower)?;

        // C_other >= B_other + B_dot_over
        // C_other >= B_other
        // C_other >= B_other + B_dot - B_dot
        // C_all - B_all >= 0
        // shortfall == 0
        if shortfall.is_zero() {
            return Err(Error::<T>::InsufficientShortfall.into());
        }

        // The liquidator may not repay more than 50%(close_factor) of the borrower's borrow balance.
        let account_borrows = Self::current_borrow_balance(borrower, liquidation_asset_id)?;
        let account_borrows_value = Self::get_asset_value(liquidation_asset_id, account_borrows)?;
        let repay_value = Self::get_asset_value(liquidation_asset_id, repay_amount)?;

        if market.close_factor.mul_ceil(account_borrows_value.into_inner()) < repay_value.into_inner() {
            return Err(Error::<T>::TooMuchRepay.into());
        }

        Ok(())
    }

    /// Note:
    /// - liquidation_asset_id is borrower's debt asset.
    /// - collateral_asset_id is borrower's collateral asset.
    /// - repay_amount is amount of liquidation_asset_id
    ///
    /// The liquidator will repay a certain amount of liquidation_asset_id from own
    /// account for borrower. Then the protocol will reduce borrower's debt
    /// and liquidator will receive collateral_asset_id(as voucher amount) from
    /// borrower.
    #[require_transactional]
    pub fn do_liquidate_borrow(
        liquidator: T::AccountId,
        borrower: T::AccountId,
        liquidation_asset_id: AssetIdOf<T>,
        repay_amount: BalanceOf<T>,
        collateral_asset_id: AssetIdOf<T>,
    ) -> DispatchResult {
        Self::ensure_active_market(liquidation_asset_id)?;
        Self::ensure_active_market(collateral_asset_id)?;

        let market = Self::market(liquidation_asset_id)?;

        if borrower == liquidator {
            return Err(Error::<T>::LiquidatorIsBorrower.into());
        }
        Self::liquidate_borrow_allowed(&borrower, liquidation_asset_id, repay_amount, &market)?;

        let lend_token_id = Self::lend_token_id(collateral_asset_id)?;
        let deposits = AccountDeposits::<T>::get(lend_token_id, &borrower);
        let exchange_rate = Self::exchange_rate_stored(collateral_asset_id)?;
        let borrower_deposit_amount = exchange_rate
            .checked_mul_int(deposits)
            .ok_or(ArithmeticError::Overflow)?;

        let collateral_value = Self::get_asset_value(collateral_asset_id, borrower_deposit_amount)?;
        // liquidate_value contains the incentive of liquidator and the punishment of the borrower
        let liquidate_value = Self::get_asset_value(liquidation_asset_id, repay_amount)?
            .checked_mul(&market.liquidate_incentive)
            .ok_or(ArithmeticError::Overflow)?;

        if collateral_value < liquidate_value {
            return Err(Error::<T>::InsufficientCollateral.into());
        }

        // Calculate the collateral will get
        let liquidate_value_amount =
            Amount::<T>::from_unsigned_fixed_point(liquidate_value, T::ReferenceAssetId::get())?;
        let real_collateral_underlying_amount = liquidate_value_amount.convert_to(collateral_asset_id)?.amount();

        //inside transfer token
        Self::liquidated_transfer(
            &liquidator,
            &borrower,
            liquidation_asset_id,
            collateral_asset_id,
            repay_amount,
            real_collateral_underlying_amount,
            &market,
        )?;

        Ok(())
    }

    #[require_transactional]
    fn liquidated_transfer(
        liquidator: &T::AccountId,
        borrower: &T::AccountId,
        liquidation_asset_id: AssetIdOf<T>,
        collateral_asset_id: AssetIdOf<T>,
        repay_amount: BalanceOf<T>,
        collateral_underlying_amount: BalanceOf<T>,
        market: &Market<BalanceOf<T>>,
    ) -> DispatchResult {
        log::trace!(
            target: "loans::liquidated_transfer",
            "liquidator: {:?}, borrower: {:?}, liquidation_asset_id: {:?},
                collateral_asset_id: {:?}, repay_amount: {:?}, collateral_underlying_amount: {:?}",
            liquidator,
            borrower,
            liquidation_asset_id,
            collateral_asset_id,
            repay_amount,
            collateral_underlying_amount
        );

        // update borrow index after accrue interest.
        Self::update_reward_borrow_index(liquidation_asset_id)?;
        Self::distribute_borrower_reward(liquidation_asset_id, liquidator)?;

        // 1.liquidator repay borrower's debt,
        // transfer from liquidator to module account
        let amount_to_transfer: Amount<T> = Amount::new(repay_amount, liquidation_asset_id);
        amount_to_transfer.transfer(liquidator, &Self::account_id())?;

        // 2.the system reduce borrower's debt
        let account_borrows = Self::current_borrow_balance(borrower, liquidation_asset_id)?;
        let account_borrows_new = account_borrows
            .checked_sub(repay_amount)
            .ok_or(ArithmeticError::Underflow)?;
        let total_borrows = Self::total_borrows(liquidation_asset_id);
        let total_borrows_new = total_borrows
            .checked_sub(repay_amount)
            .ok_or(ArithmeticError::Underflow)?;
        AccountBorrows::<T>::insert(
            liquidation_asset_id,
            borrower,
            BorrowSnapshot {
                principal: account_borrows_new,
                borrow_index: Self::borrow_index(liquidation_asset_id),
            },
        );
        TotalBorrows::<T>::insert(liquidation_asset_id, total_borrows_new);

        // update supply index before modify supply balance.
        Self::update_reward_supply_index(collateral_asset_id)?;
        Self::distribute_supplier_reward(collateral_asset_id, liquidator)?;
        Self::distribute_supplier_reward(collateral_asset_id, borrower)?;
        Self::distribute_supplier_reward(collateral_asset_id, &Self::incentive_reward_account_id()?)?;

        // 3.the liquidator will receive voucher token from borrower
        let exchange_rate = Self::exchange_rate_stored(collateral_asset_id)?;
        let collateral_amount = Self::calc_collateral_amount(collateral_underlying_amount, exchange_rate)?;
        let lend_token_id = Self::lend_token_id(collateral_asset_id)?;
        let incentive_reserved = market.liquidate_incentive_reserved_factor.mul_floor(
            FixedU128::from_inner(collateral_amount)
                .checked_div(&market.liquidate_incentive)
                .map(|r| r.into_inner())
                .ok_or(ArithmeticError::Underflow)?,
        );

        // Unlock this balance to make it transferrable
        let amount_to_liquidate: Amount<T> = Amount::new(collateral_amount, lend_token_id);
        amount_to_liquidate.unlock_on(borrower)?;

        // increase liquidator's voucher_balance
        let liquidator_amount_u128 = collateral_amount
            .checked_sub(incentive_reserved)
            .ok_or(ArithmeticError::Underflow)?;
        let liquidator_amount: Amount<T> = Amount::new(liquidator_amount_u128, lend_token_id);
        liquidator_amount.transfer(borrower, liquidator)?;

        // increase reserve's voucher_balance
        let incentive_reserved_amount: Amount<T> = Amount::new(incentive_reserved, lend_token_id);
        incentive_reserved_amount.transfer(borrower, &Self::incentive_reward_account_id()?)?;

        Self::deposit_event(Event::<T>::LiquidatedBorrow(
            liquidator.clone(),
            borrower.clone(),
            liquidation_asset_id,
            collateral_asset_id,
            repay_amount,
            collateral_underlying_amount,
        ));

        Ok(())
    }

    pub fn lock_if_account_deposited(
        account_id: &T::AccountId,
        lend_token_id: AssetIdOf<T>,
        incoming_amount: BalanceOf<T>,
    ) -> DispatchResult {
        // if the receiver already has their collateral deposited
        let deposit = Pallet::<T>::account_deposits(lend_token_id, account_id);
        if deposit > 0 {
            // then any incoming `lend_tokens` must automatically be deposited as collateral
            // to enforce the "collateral toggle"
            Self::do_deposit_collateral(account_id, lend_token_id, incoming_amount)?;
        }
        Ok(())
    }

    // Ensures a given `asset_id` is an active market.
    fn ensure_active_market(asset_id: AssetIdOf<T>) -> Result<Market<BalanceOf<T>>, DispatchError> {
        Self::active_markets()
            .find(|(id, _)| id == &asset_id)
            .map(|(_, market)| market)
            .ok_or_else(|| Error::<T>::MarketNotActivated.into())
    }

    /// Ensure market is enough to supply `amount` asset.
    fn ensure_under_supply_cap(asset_id: AssetIdOf<T>, amount: BalanceOf<T>) -> DispatchResult {
        let market = Self::market(asset_id)?;
        // Assets holded by market currently.
        let current_cash = Self::balance(asset_id, &Self::account_id());
        let total_cash = current_cash.checked_add(amount).ok_or(ArithmeticError::Overflow)?;
        ensure!(total_cash <= market.supply_cap, Error::<T>::SupplyCapacityExceeded);

        Ok(())
    }

    /// Make sure the borrowing under the borrow cap
    fn ensure_under_borrow_cap(asset_id: AssetIdOf<T>, amount: BalanceOf<T>) -> DispatchResult {
        let market = Self::market(asset_id)?;
        let total_borrows = Self::total_borrows(asset_id);
        let new_total_borrows = total_borrows.checked_add(amount).ok_or(ArithmeticError::Overflow)?;
        ensure!(
            new_total_borrows <= market.borrow_cap,
            Error::<T>::BorrowCapacityExceeded
        );

        Ok(())
    }

    /// Make sure there is enough cash available in the pool
    fn ensure_enough_cash(asset_id: AssetIdOf<T>, amount: BalanceOf<T>) -> DispatchResult {
        let reducible_cash = Self::get_total_cash(asset_id)
            .checked_sub(Self::total_reserves(asset_id))
            .ok_or(ArithmeticError::Underflow)?;
        if reducible_cash < amount {
            return Err(Error::<T>::InsufficientCash.into());
        }

        Ok(())
    }

    // Ensures a given `lend_token_id` is unique in `Markets` and `UnderlyingAssetId`.
    fn ensure_lend_token(lend_token_id: CurrencyId) -> DispatchResult {
        // The lend_token id is unique, cannot be repeated
        ensure!(
            !UnderlyingAssetId::<T>::contains_key(lend_token_id),
            Error::<T>::InvalidLendTokenId
        );

        // The lend_token id should not be the same as the id of any asset in markets
        ensure!(
            !Markets::<T>::contains_key(lend_token_id),
            Error::<T>::InvalidLendTokenId
        );

        Ok(())
    }

    // Ensures that `account` have sufficient liquidity to move your assets
    // Returns `Err` If InsufficientLiquidity
    // `account`: account that need a liquidity check
    // `reduce_amount`: values that will have an impact on liquidity
    fn ensure_liquidity(account: &T::AccountId, reduce_amount: FixedU128) -> DispatchResult {
        let (total_liquidity, _) = Self::get_account_liquidity(account)?;
        if total_liquidity >= reduce_amount {
            return Ok(());
        }
        Err(Error::<T>::InsufficientLiquidity.into())
    }

    pub fn calc_underlying_amount(
        voucher_amount: BalanceOf<T>,
        exchange_rate: Rate,
    ) -> Result<BalanceOf<T>, DispatchError> {
        Ok(exchange_rate
            .checked_mul_int(voucher_amount)
            .ok_or(ArithmeticError::Overflow)?)
    }

    pub fn calc_collateral_amount(
        underlying_amount: BalanceOf<T>,
        exchange_rate: Rate,
    ) -> Result<BalanceOf<T>, DispatchError> {
        Ok(FixedU128::from_inner(underlying_amount)
            .checked_div(&exchange_rate)
            .map(|r| r.into_inner())
            .ok_or(ArithmeticError::Underflow)?)
    }

    fn get_total_cash(asset_id: AssetIdOf<T>) -> BalanceOf<T> {
        orml_tokens::Pallet::<T>::reducible_balance(asset_id, &Self::account_id(), true)
    }

    /// Get the total balance of `who`.
    /// Ignores any frozen balance of this account.
    fn balance(asset_id: AssetIdOf<T>, who: &T::AccountId) -> BalanceOf<T> {
        <orml_tokens::Pallet<T> as MultiCurrency<T::AccountId>>::total_balance(asset_id, who)
    }

    /// Total supply of lending tokens (lend_tokens), given the underlying
    pub fn total_supply(asset_id: AssetIdOf<T>) -> Result<BalanceOf<T>, DispatchError> {
        let lend_token_id = Self::lend_token_id(asset_id)?;
        Ok(orml_tokens::Pallet::<T>::total_issuance(lend_token_id))
    }

    /// Free lending tokens (lend_tokens) of an account, given the underlying
    pub fn free_lend_tokens(asset_id: AssetIdOf<T>, account_id: &T::AccountId) -> Result<Amount<T>, DispatchError> {
        let lend_token_id = Self::lend_token_id(asset_id)?;
        let amount = Amount::new(
            orml_tokens::Pallet::<T>::free_balance(lend_token_id, account_id),
            lend_token_id,
        );
        Ok(amount)
    }

    /// Reserved lending tokens (lend_tokens) of an account, given the underlying
    pub fn reserved_lend_tokens(asset_id: AssetIdOf<T>, account_id: &T::AccountId) -> Result<Amount<T>, DispatchError> {
        let lend_token_id = Self::lend_token_id(asset_id)?;
        let amount = Amount::new(
            orml_tokens::Pallet::<T>::reserved_balance(lend_token_id, account_id),
            lend_token_id,
        );
        Ok(amount)
    }

    // Returns the value of the asset, in the reference currency.
    // Returns `Err` if oracle price not ready or arithmetic error.
    pub fn get_asset_value(asset_id: AssetIdOf<T>, amount: BalanceOf<T>) -> Result<FixedU128, DispatchError> {
        let asset_amount = Amount::<T>::new(amount, asset_id);
        let reference_amount = asset_amount.convert_to(T::ReferenceAssetId::get())?;
        reference_amount.to_unsigned_fixed_point()
    }

    // Returns a stored Market.
    //
    // Returns `Err` if market does not exist.
    pub fn market(asset_id: AssetIdOf<T>) -> Result<Market<BalanceOf<T>>, DispatchError> {
        Markets::<T>::try_get(asset_id).map_err(|_err| Error::<T>::MarketDoesNotExist.into())
    }

    // Mutates a stored Market.
    //
    // Returns `Err` if market does not exist.
    pub(crate) fn mutate_market<F>(asset_id: AssetIdOf<T>, cb: F) -> Result<Market<BalanceOf<T>>, DispatchError>
    where
        F: FnOnce(&mut Market<BalanceOf<T>>) -> Market<BalanceOf<T>>,
    {
        Markets::<T>::try_mutate(asset_id, |opt| -> Result<Market<BalanceOf<T>>, DispatchError> {
            if let Some(market) = opt {
                return Ok(cb(market));
            }
            Err(Error::<T>::MarketDoesNotExist.into())
        })
    }

    // All markets that are `MarketStatus::Active`.
    fn active_markets() -> impl Iterator<Item = (AssetIdOf<T>, Market<BalanceOf<T>>)> {
        Markets::<T>::iter().filter(|(_, market)| market.state == MarketState::Active)
    }

    // Returns the lend_token_id of the related asset
    //
    // Returns `Err` if market does not exist.
    pub fn lend_token_id(asset_id: AssetIdOf<T>) -> Result<AssetIdOf<T>, DispatchError> {
        if let Ok(market) = Self::market(asset_id) {
            Ok(market.lend_token_id)
        } else {
            Err(Error::<T>::MarketDoesNotExist.into())
        }
    }

    // Returns the incentive reward account
    pub fn incentive_reward_account_id() -> Result<T::AccountId, DispatchError> {
        let account_id: T::AccountId = T::PalletId::get().into_account_truncating();
        let entropy = (INCENTIVE_ACCOUNT_PREFIX, &[account_id]).using_encoded(blake2_256);
        Ok(T::AccountId::decode(&mut &entropy[..]).map_err(|_| Error::<T>::CodecError)?)
    }
}

impl<T: Config> LoansTrait<AssetIdOf<T>, AccountIdOf<T>, BalanceOf<T>, Amount<T>> for Pallet<T> {
    fn do_mint(supplier: &AccountIdOf<T>, asset_id: AssetIdOf<T>, amount: BalanceOf<T>) -> Result<(), DispatchError> {
        Self::ensure_active_market(asset_id)?;
        Self::ensure_under_supply_cap(asset_id, amount)?;

        Self::accrue_interest(asset_id)?;

        // update supply index before modify supply balance.
        Self::update_reward_supply_index(asset_id)?;
        Self::distribute_supplier_reward(asset_id, supplier)?;

        let exchange_rate = Self::exchange_rate_stored(asset_id)?;
        Self::update_earned_stored(supplier, asset_id, exchange_rate)?;
        let voucher_amount = Self::calc_collateral_amount(amount, exchange_rate)?;
        ensure!(!voucher_amount.is_zero(), Error::<T>::InvalidExchangeRate);

        let amount_to_transfer: Amount<T> = Amount::new(amount, asset_id);
        amount_to_transfer.transfer(supplier, &Self::account_id())?;

        let lend_token_id = Self::lend_token_id(asset_id)?;
        let lend_tokens_to_mint: Amount<T> = Amount::new(voucher_amount, lend_token_id);
        lend_tokens_to_mint.mint_to(supplier)?;

        Self::deposit_event(Event::<T>::Deposited(supplier.clone(), asset_id, amount));
        Ok(())
    }

    fn do_borrow(borrower: &AccountIdOf<T>, asset_id: AssetIdOf<T>, amount: BalanceOf<T>) -> Result<(), DispatchError> {
        Self::ensure_active_market(asset_id)?;

        Self::accrue_interest(asset_id)?;
        Self::borrow_allowed(asset_id, borrower, amount)?;

        // update borrow index after accrue interest.
        Self::update_reward_borrow_index(asset_id)?;
        Self::distribute_borrower_reward(asset_id, borrower)?;

        let account_borrows = Self::current_borrow_balance(borrower, asset_id)?;
        let account_borrows_new = account_borrows.checked_add(amount).ok_or(ArithmeticError::Overflow)?;
        let total_borrows = Self::total_borrows(asset_id);
        let total_borrows_new = total_borrows.checked_add(amount).ok_or(ArithmeticError::Overflow)?;
        AccountBorrows::<T>::insert(
            asset_id,
            borrower,
            BorrowSnapshot {
                principal: account_borrows_new,
                borrow_index: Self::borrow_index(asset_id),
            },
        );
        TotalBorrows::<T>::insert(asset_id, total_borrows_new);
        let amount_to_transfer: Amount<T> = Amount::new(amount, asset_id);
        amount_to_transfer.transfer(&Self::account_id(), borrower)?;

        Self::deposit_event(Event::<T>::Borrowed(borrower.clone(), asset_id, amount));
        Ok(())
    }

    fn do_deposit_collateral(
        supplier: &AccountIdOf<T>,
        asset_id: AssetIdOf<T>,
        amount: BalanceOf<T>,
    ) -> Result<(), DispatchError> {
        let lend_token_amount: Amount<T> = Amount::new(amount, asset_id);
        // If the given asset_id is not a valid lend_token, fetching the underlying will fail
        let underlying_id = Self::underlying_id(lend_token_amount.currency())?;
        Self::ensure_active_market(underlying_id)?;

        // Will fail if supplier has insufficient free tokens
        lend_token_amount.lock_on(supplier)?;

        // Increase the amount of collateral deposited
        let deposit = Self::account_deposits(lend_token_amount.currency(), supplier);
        let new_deposit = deposit
            .checked_add(lend_token_amount.amount())
            .ok_or(ArithmeticError::Overflow)?;
        AccountDeposits::<T>::insert(lend_token_amount.currency(), supplier, new_deposit);

        Self::deposit_event(Event::<T>::DepositCollateral(
            supplier.clone(),
            lend_token_amount.currency(),
            lend_token_amount.amount(),
        ));
        Ok(())
    }

    fn do_withdraw_collateral(
        supplier: &AccountIdOf<T>,
        asset_id: AssetIdOf<T>,
        amount: BalanceOf<T>,
    ) -> Result<(), DispatchError> {
        let lend_token_amount: Amount<T> = Amount::new(amount, asset_id);
        // If the given asset_id is not a valid lend_token, fetching the underlying will fail
        let underlying_id = Self::underlying_id(lend_token_amount.currency())?;
        Self::ensure_active_market(underlying_id)?;

        let total_collateral_value = Self::total_collateral_value(supplier)?;
        let collateral_amount_value = Self::collateral_amount_value(underlying_id, lend_token_amount.amount())?;
        let total_borrowed_value = Self::total_borrowed_value(supplier)?;
        log::trace!(
            target: "loans::collateral_asset",
            "total_collateral_value: {:?}, collateral_asset_value: {:?}, total_borrowed_value: {:?}",
            total_collateral_value.into_inner(),
            collateral_amount_value.into_inner(),
            total_borrowed_value.into_inner(),
        );

        if total_collateral_value
            < total_borrowed_value
                .checked_add(&collateral_amount_value)
                .ok_or(ArithmeticError::Overflow)?
        {
            return Err(Error::<T>::InsufficientLiquidity.into());
        }

        lend_token_amount.unlock_on(supplier)?;

        // Decrease the amount of collateral deposited
        AccountDeposits::<T>::try_mutate_exists(asset_id, supplier, |deposits| -> DispatchResult {
            let d = deposits
                .unwrap_or_default()
                .checked_sub(lend_token_amount.amount())
                .ok_or(ArithmeticError::Underflow)?;
            if d.is_zero() {
                // remove deposits storage if zero balance
                *deposits = None;
            } else {
                *deposits = Some(d);
            }
            Ok(())
        })?;

        Self::deposit_event(Event::<T>::WithdrawCollateral(
            supplier.clone(),
            lend_token_amount.currency(),
            lend_token_amount.amount(),
        ));
        Ok(())
    }

    fn do_repay_borrow(
        borrower: &AccountIdOf<T>,
        asset_id: AssetIdOf<T>,
        amount: BalanceOf<T>,
    ) -> Result<(), DispatchError> {
        Self::ensure_active_market(asset_id)?;
        Self::accrue_interest(asset_id)?;
        let account_borrows = Self::current_borrow_balance(borrower, asset_id)?;
        Self::do_repay_borrow_with_amount(borrower, asset_id, account_borrows, amount)?;
        Self::deposit_event(Event::<T>::RepaidBorrow(borrower.clone(), asset_id, amount));
        Ok(())
    }

    fn do_redeem(supplier: &AccountIdOf<T>, asset_id: AssetIdOf<T>, amount: BalanceOf<T>) -> Result<(), DispatchError> {
        Self::ensure_active_market(asset_id)?;
        Self::accrue_interest(asset_id)?;
        let exchange_rate = Self::exchange_rate_stored(asset_id)?;
        Self::update_earned_stored(supplier, asset_id, exchange_rate)?;
        let voucher_amount = Self::calc_collateral_amount(amount, exchange_rate)?;
        let redeem_amount = Self::do_redeem_voucher(supplier, asset_id, voucher_amount)?;
        Self::deposit_event(Event::<T>::Redeemed(supplier.clone(), asset_id, redeem_amount));
        Ok(())
    }

    fn recompute_underlying_amount(lend_tokens: &Amount<T>) -> Result<Amount<T>, DispatchError> {
        // This function could be called externally to this pallet, with interest
        // possibly not having accrued for a few blocks. This would result in using an
        // outdated exchange rate. Call `accrue_interest` to avoid this.
        let underlying_id = Self::underlying_id(lend_tokens.currency())?;
        Self::ensure_active_market(underlying_id)?;
        Self::accrue_interest(underlying_id)?;
        let exchange_rate = Self::exchange_rate_stored(underlying_id)?;
        let underlying_amount = Self::calc_underlying_amount(lend_tokens.amount(), exchange_rate)?;
        Ok(Amount::new(underlying_amount, underlying_id))
    }

    // Returns a stored asset_id
    //
    // Returns `Err` if asset_id does not exist, it also means that lend_token_id is invalid.
    fn underlying_id(lend_token_id: AssetIdOf<T>) -> Result<AssetIdOf<T>, DispatchError> {
        UnderlyingAssetId::<T>::try_get(lend_token_id).map_err(|_err| Error::<T>::InvalidLendTokenId.into())
    }

    fn recompute_collateral_amount(underlying: &Amount<T>) -> Result<Amount<T>, DispatchError> {
        // This function could be called externally to this pallet, with interest
        // possibly not having accrued for a few blocks. This would result in using an
        // outdated exchange rate. Call `accrue_interest` to avoid this.
        Self::ensure_active_market(underlying.currency())?;
        Self::accrue_interest(underlying.currency())?;
        let exchange_rate = Self::exchange_rate_stored(underlying.currency())?;
        let underlying_amount = Self::calc_collateral_amount(underlying.amount(), exchange_rate)?;
        let lend_token_id = Self::lend_token_id(underlying.currency())?;
        Ok(Amount::new(underlying_amount, lend_token_id))
    }
}

impl<T: Config> LoansMarketDataProvider<AssetIdOf<T>, BalanceOf<T>> for Pallet<T> {
    fn get_market_info(asset_id: AssetIdOf<T>) -> Result<MarketInfo, DispatchError> {
        let market = Self::market(asset_id)?;
        let full_rate = Self::get_full_interest_rate(asset_id).ok_or(Error::<T>::InvalidRateModelParam)?;
        Ok(MarketInfo {
            collateral_factor: market.collateral_factor,
            liquidation_threshold: market.liquidation_threshold,
            reserve_factor: market.reserve_factor,
            close_factor: market.close_factor,
            full_rate,
        })
    }

    fn get_market_status(asset_id: AssetIdOf<T>) -> Result<MarketStatus<Balance>, DispatchError> {
        let (borrow_rate, supply_rate, exchange_rate, utilization, total_borrows, total_reserves, borrow_index) =
            Self::get_market_status(asset_id)?;
        Ok(MarketStatus {
            borrow_rate,
            supply_rate,
            exchange_rate,
            utilization,
            total_borrows,
            total_reserves,
            borrow_index,
        })
    }

    fn get_full_interest_rate(asset_id: AssetIdOf<T>) -> Option<Rate> {
        if let Ok(market) = Self::market(asset_id) {
            let rate = match market.rate_model {
                InterestRateModel::Jump(jump) => Some(jump.full_rate),
                _ => None,
            };
            return rate;
        }
        None
    }
}
