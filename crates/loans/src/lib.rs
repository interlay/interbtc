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
//! The loans pallet implements a Compound V2-style lending protocol by using a pool-based strategy
//! that aggregates each user's supplied assets. The interest rate is dynamically
//! determined by the supply and demand. Lending positions are tokenized and can thus have their
//! ownership transferred.

#![cfg_attr(not(feature = "std"), no_std)]

pub use crate::rate_model::*;
use crate::types::AccountLiquidity;

use currency::{Amount, Rounding};
use frame_support::{
    log,
    pallet_prelude::*,
    require_transactional,
    traits::{tokens::fungibles::Inspect, OnRuntimeUpgrade, UnixTime},
    transactional, PalletId,
};
use frame_system::pallet_prelude::*;
use num_traits::cast::ToPrimitive;
use orml_traits::{MultiCurrency, MultiReservableCurrency};
pub use pallet::*;
use primitives::{Balance, Rate, Ratio, Timestamp};
use sp_runtime::{
    traits::{
        AccountIdConversion, CheckedAdd, CheckedDiv, CheckedMul, One, SaturatedConversion, Saturating, StaticLookup,
        Zero,
    },
    ArithmeticError, FixedPointNumber, FixedU128,
};
use sp_std::{marker, result::Result};

use traits::{
    ConvertToBigUint, LoansApi as LoansTrait, LoansMarketDataProvider, MarketInfo, MarketStatus, OnExchangeRateChange,
};

pub use default_weights::WeightInfo;
pub use orml_traits::currency::{OnDeposit, OnSlash, OnTransfer};
pub use types::{BorrowSnapshot, EarnedSnapshot, Market, MarketState, RewardMarketState};

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod migration;

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

mod default_weights;

#[cfg(test)]
use mocktopus::macros::mockable;

pub const REWARD_SUB_ACCOUNT: &[u8; 7] = b"farming";
pub const INCENTIVE_SUB_ACCOUNT: &[u8; 9] = b"incentive";

pub const DEFAULT_MAX_EXCHANGE_RATE: u128 = 1_000_000_000_000_000_000; // 1
pub const DEFAULT_MIN_EXCHANGE_RATE: u128 = 20_000_000_000_000_000; // 0.02

type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
type CurrencyId<T> = <T as orml_tokens::Config>::CurrencyId;
type BalanceOf<T> = <T as currency::Config>::Balance;

/// Lending-specific methods on Amount
#[cfg_attr(test, mockable)]
trait LendingAmountExt {
    fn to_lend_token(&self) -> Result<Self, DispatchError>
    where
        Self: Sized;
    fn to_underlying(&self) -> Result<Self, DispatchError>
    where
        Self: Sized;
}

#[cfg_attr(test, mockable)]
impl<T: Config> LendingAmountExt for Amount<T> {
    fn to_lend_token(&self) -> Result<Self, DispatchError> {
        let lend_token_id = Pallet::<T>::lend_token_id(self.currency())?;
        self.convert_to(lend_token_id)
    }

    fn to_underlying(&self) -> Result<Self, DispatchError> {
        let underlying_id = Pallet::<T>::underlying_id(self.currency())?;
        self.convert_to(underlying_id)
    }
}

pub struct OnSlashHook<T>(marker::PhantomData<T>);
// This implementation is not allowed to fail, so erors are logged instead of being propagated.
// If the slash-related FRAME traits are allowed to fail, this can be fixed.
// Opened a GitHub issue for this in the Substrate repo: https://github.com/paritytech/substrate/issues/12533
// TODO: Propagate error once the issue is resolved upstream
impl<T: Config> OnSlash<T::AccountId, CurrencyId<T>, BalanceOf<T>> for OnSlashHook<T> {
    /// Whenever a lend_token balance is mutated, the supplier incentive rewards accumulated up to that point
    /// have to be distributed.
    fn on_slash(currency_id: CurrencyId<T>, account_id: &T::AccountId, amount: BalanceOf<T>) {
        if currency_id.is_lend_token() {
            // Note that wherever `on_slash` is called in the lending pallet, and the `account_id` has non-zero
            // `AccountDeposits`, the storage item needs to be decreased by `amount` to reflect the reduction
            // in collateral.
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
impl<T: Config> OnDeposit<T::AccountId, CurrencyId<T>, BalanceOf<T>> for PreDeposit<T> {
    /// Whenever a lend_token balance is mutated, the supplier incentive rewards accumulated up to that point
    /// have to be distributed.
    fn on_deposit(currency_id: CurrencyId<T>, account_id: &T::AccountId, _amount: BalanceOf<T>) -> DispatchResult {
        if currency_id.is_lend_token() {
            let underlying_id = Pallet::<T>::underlying_id(currency_id)?;
            Pallet::<T>::update_reward_supply_index(underlying_id)?;
            Pallet::<T>::distribute_supplier_reward(underlying_id, account_id)?;
        }
        Ok(())
    }
}

pub struct PostDeposit<T>(marker::PhantomData<T>);
impl<T: Config> OnDeposit<T::AccountId, CurrencyId<T>, BalanceOf<T>> for PostDeposit<T> {
    fn on_deposit(currency_id: CurrencyId<T>, account_id: &T::AccountId, amount: BalanceOf<T>) -> DispatchResult {
        if currency_id.is_lend_token() {
            Pallet::<T>::lock_if_account_deposited(account_id, &Amount::new(amount, currency_id))?;
        }
        Ok(())
    }
}

pub struct PreTransfer<T>(marker::PhantomData<T>);
impl<T: Config> OnTransfer<T::AccountId, CurrencyId<T>, BalanceOf<T>> for PreTransfer<T> {
    /// Whenever a lend_token balance is mutated, the supplier incentive rewards accumulated up to that point
    /// have to be distributed.
    fn on_transfer(
        currency_id: CurrencyId<T>,
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
impl<T: Config> OnTransfer<T::AccountId, CurrencyId<T>, BalanceOf<T>> for PostTransfer<T> {
    /// If an account has locked their lend_token balance as collateral, any incoming lend_tokens
    /// have to be automatically locked as well, in order to enforce a "collateral toggle" that
    /// offers the same UX as Compound V2's lending protocol implementation.
    fn on_transfer(
        currency_id: CurrencyId<T>,
        _from: &T::AccountId,
        to: &T::AccountId,
        amount: BalanceOf<T>,
    ) -> DispatchResult {
        if currency_id.is_lend_token() {
            Pallet::<T>::lock_if_account_deposited(to, &Amount::new(amount, currency_id))?;
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
        frame_system::Config + currency::Config<Balance = Balance, UnsignedFixedPoint = FixedU128>
    {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// The loan's module id, used to derive the account that holds the liquidity in all markets.
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// The origin which can add/reduce reserves.
        type ReserveOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// The origin which can add or update markets (including interest rate model, liquidate incentive and
        /// collateral ratios).
        type UpdateOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;

        /// Unix time
        type UnixTime: UnixTime;

        /// Incentive reward asset id.
        #[pallet::constant]
        type RewardAssetId: Get<CurrencyId<Self>>;

        /// Reference currency for expressing asset prices. Example: USD, IBTC.
        #[pallet::constant]
        type ReferenceAssetId: Get<CurrencyId<Self>>;

        /// Hook for exchangerate changes.
        type OnExchangeRateChange: OnExchangeRateChange<CurrencyId<Self>>;
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Insufficient liquidity to borrow more or disable collateral
        InsufficientLiquidity,
        /// Insufficient deposit to redeem
        InsufficientDeposit,
        /// Repay amount greater than allowed (either repays more than the existing debt, or
        /// exceeds the close factor)
        TooMuchRepay,
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
        /// The exchange rate should be a value between `MinExchangeRate` and `MaxExchangeRate`
        InvalidExchangeRate,
        /// Amount cannot be zero
        InvalidAmount,
        /// Locking collateral failed. The account has no `free` tokens.
        DepositAllCollateralFailed,
        /// Unlocking collateral failed. The account has no `reserved` tokens.
        WithdrawAllCollateralFailed,
        /// Tokens already locked for a different purpose than borrow collateral
        TokensAlreadyLocked,
        /// Only free lend tokens are redeemable
        LockedTokensCannotBeRedeemed,
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub (crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Enable collateral for certain asset
        DepositCollateral {
            account_id: T::AccountId,
            currency_id: CurrencyId<T>,
            amount: BalanceOf<T>,
        },
        /// Disable collateral for certain asset
        WithdrawCollateral {
            account_id: T::AccountId,
            currency_id: CurrencyId<T>,
            amount: BalanceOf<T>,
        },
        /// Event emitted when assets are deposited
        Deposited {
            account_id: T::AccountId,
            currency_id: CurrencyId<T>,
            amount: BalanceOf<T>,
        },
        /// Event emitted when assets are redeemed
        Redeemed {
            account_id: T::AccountId,
            currency_id: CurrencyId<T>,
            amount: BalanceOf<T>,
        },
        /// Event emitted when cash is borrowed
        Borrowed {
            account_id: T::AccountId,
            currency_id: CurrencyId<T>,
            amount: BalanceOf<T>,
        },
        /// Event emitted when a borrow is repaid
        RepaidBorrow {
            account_id: T::AccountId,
            currency_id: CurrencyId<T>,
            amount: BalanceOf<T>,
        },
        /// Event emitted when a borrow is liquidated
        LiquidatedBorrow {
            liquidator: T::AccountId,
            borrower: T::AccountId,
            liquidation_currency_id: CurrencyId<T>,
            collateral_currency_id: CurrencyId<T>,
            repay_amount: BalanceOf<T>,
            collateral_underlying_amount: BalanceOf<T>,
        },
        /// Event emitted when the reserves are reduced
        ReservesReduced {
            receiver: T::AccountId,
            currency_id: CurrencyId<T>,
            amount: BalanceOf<T>,
            new_reserve_amount: BalanceOf<T>,
        },
        /// Event emitted when the reserves are added
        ReservesAdded {
            payer: T::AccountId,
            currency_id: CurrencyId<T>,
            amount: BalanceOf<T>,
            new_reserve_amount: BalanceOf<T>,
        },
        /// New market is set
        NewMarket {
            underlying_currency_id: CurrencyId<T>,
            market: Market<BalanceOf<T>>,
        },
        /// Event emitted when a market is activated
        ActivatedMarket { underlying_currency_id: CurrencyId<T> },
        /// New market parameters is updated
        UpdatedMarket {
            underlying_currency_id: CurrencyId<T>,
            market: Market<BalanceOf<T>>,
        },
        /// Reward added
        RewardAdded { payer: T::AccountId, amount: BalanceOf<T> },
        /// Reward withdrawed
        RewardWithdrawn {
            receiver: T::AccountId,
            amount: BalanceOf<T>,
        },
        /// Event emitted when market reward speed updated.
        MarketRewardSpeedUpdated {
            underlying_currency_id: CurrencyId<T>,
            supply_reward_per_block: BalanceOf<T>,
            borrow_reward_per_block: BalanceOf<T>,
        },
        /// Deposited when Reward is distributed to a supplier
        DistributedSupplierReward {
            underlying_currency_id: CurrencyId<T>,
            supplier: T::AccountId,
            reward_delta: BalanceOf<T>,
            supply_reward_index: BalanceOf<T>,
        },
        /// Deposited when Reward is distributed to a borrower
        DistributedBorrowerReward {
            underlying_currency_id: CurrencyId<T>,
            borrower: T::AccountId,
            reward_delta: BalanceOf<T>,
            borrow_reward_index: BalanceOf<T>,
        },
        /// Reward Paid for user
        RewardPaid {
            receiver: T::AccountId,
            amount: BalanceOf<T>,
        },
        /// Event emitted when the incentive reserves are redeemed and transfer to receiver's account
        IncentiveReservesReduced {
            receiver: T::AccountId,
            currency_id: CurrencyId<T>,
            amount: BalanceOf<T>,
        },
        /// Event emitted when interest has been accrued for a market
        InterestAccrued {
            underlying_currency_id: CurrencyId<T>,
            total_borrows: BalanceOf<T>,
            total_reserves: BalanceOf<T>,
            borrow_index: FixedU128,
            utilization_ratio: Ratio,
            borrow_rate: Rate,
            supply_rate: Rate,
            exchange_rate: Rate,
        },
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
        fn on_runtime_upgrade() -> frame_support::weights::Weight {
            migration::collateral_toggle::Migration::<T>::on_runtime_upgrade()
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade(pre_upgrade_state: sp_std::vec::Vec<u8>) -> Result<(), &'static str> {
            frame_support::assert_ok!(migration::collateral_toggle::Migration::<T>::post_upgrade(
                pre_upgrade_state
            ));
            Ok(())
        }
    }

    /// The timestamp of the last calculation of accrued interest
    #[pallet::storage]
    #[pallet::getter(fn last_accrued_interest_time)]
    pub type LastAccruedInterestTime<T: Config> = StorageMap<_, Blake2_128Concat, CurrencyId<T>, Timestamp, ValueQuery>;

    /// Total amount of outstanding borrows of the underlying in this market
    /// CurrencyId -> Balance
    #[pallet::storage]
    pub type TotalBorrows<T: Config> = StorageMap<_, Blake2_128Concat, CurrencyId<T>, BalanceOf<T>, ValueQuery>;

    /// Total amount of reserves of the underlying held in this market
    /// CurrencyId -> Balance
    #[pallet::storage]
    pub type TotalReserves<T: Config> = StorageMap<_, Blake2_128Concat, CurrencyId<T>, BalanceOf<T>, ValueQuery>;

    /// Mapping of account addresses to outstanding borrow balances
    /// CurrencyId -> Owner -> BorrowSnapshot
    #[pallet::storage]
    #[pallet::getter(fn account_borrows)]
    pub type AccountBorrows<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        CurrencyId<T>,
        Blake2_128Concat,
        T::AccountId,
        BorrowSnapshot<BalanceOf<T>>,
        ValueQuery,
    >;

    /// Mapping of account addresses to collateral deposit details
    /// CollateralType -> Owner -> Collateral Deposits
    ///
    /// # Remarks
    ///
    /// Differently from Parallel Finance's implementation of lending, `AccountDeposits` only
    /// represents Lend Tokens locked as collateral rather than the entire Lend Token balance of an account.
    /// If an account minted without also locking their balance as collateral, their corresponding entry
    /// in this map will be zero.
    #[pallet::storage]
    pub type AccountDeposits<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, CurrencyId<T>, Blake2_128Concat, T::AccountId, BalanceOf<T>, ValueQuery>;

    /// Accumulator of the total earned interest rate since the opening of the market
    /// CurrencyId -> u128
    #[pallet::storage]
    #[pallet::getter(fn borrow_index)]
    pub type BorrowIndex<T: Config> = StorageMap<_, Blake2_128Concat, CurrencyId<T>, Rate, ValueQuery>;

    /// The internal exchange rate from the associated lend token to the underlying currency.
    #[pallet::storage]
    #[pallet::getter(fn exchange_rate)]
    pub type ExchangeRate<T: Config> = StorageMap<_, Blake2_128Concat, CurrencyId<T>, Rate, ValueQuery>;

    /// Mapping of borrow rate to currency type
    #[pallet::storage]
    #[pallet::getter(fn borrow_rate)]
    pub type BorrowRate<T: Config> = StorageMap<_, Blake2_128Concat, CurrencyId<T>, Rate, ValueQuery>;

    /// Mapping of supply rate to currency type
    #[pallet::storage]
    #[pallet::getter(fn supply_rate)]
    pub type SupplyRate<T: Config> = StorageMap<_, Blake2_128Concat, CurrencyId<T>, Rate, ValueQuery>;

    /// Borrow utilization ratio
    #[pallet::storage]
    #[pallet::getter(fn utilization_ratio)]
    pub type UtilizationRatio<T: Config> = StorageMap<_, Blake2_128Concat, CurrencyId<T>, Ratio, ValueQuery>;

    /// Mapping of underlying currency id to its market
    #[pallet::storage]
    pub type Markets<T: Config> = StorageMap<_, Blake2_128Concat, CurrencyId<T>, Market<BalanceOf<T>>>;

    /// Mapping of lend_token id to underlying currency id
    /// `lend_token id`: voucher token id
    /// `asset id`: underlying token id
    #[pallet::storage]
    pub type UnderlyingAssetId<T: Config> = StorageMap<_, Blake2_128Concat, CurrencyId<T>, CurrencyId<T>>;

    /// Mapping of underlying currency id to supply reward speed
    #[pallet::storage]
    #[pallet::getter(fn reward_supply_speed)]
    pub type RewardSupplySpeed<T: Config> = StorageMap<_, Blake2_128Concat, CurrencyId<T>, BalanceOf<T>, ValueQuery>;

    /// Mapping of underlying currency id to borrow reward speed
    #[pallet::storage]
    #[pallet::getter(fn reward_borrow_speed)]
    pub type RewardBorrowSpeed<T: Config> = StorageMap<_, Blake2_128Concat, CurrencyId<T>, BalanceOf<T>, ValueQuery>;

    /// The Reward market supply state for each market
    #[pallet::storage]
    #[pallet::getter(fn reward_supply_state)]
    pub type RewardSupplyState<T: Config> =
        StorageMap<_, Blake2_128Concat, CurrencyId<T>, RewardMarketState<T::BlockNumber, BalanceOf<T>>, ValueQuery>;

    /// The Reward market borrow state for each market
    #[pallet::storage]
    #[pallet::getter(fn reward_borrow_state)]
    pub type RewardBorrowState<T: Config> =
        StorageMap<_, Blake2_128Concat, CurrencyId<T>, RewardMarketState<T::BlockNumber, BalanceOf<T>>, ValueQuery>;

    /// The incentive reward index for each market for each supplier as of the last time they accrued Reward
    #[pallet::storage]
    #[pallet::getter(fn reward_supplier_index)]
    pub type RewardSupplierIndex<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, CurrencyId<T>, Blake2_128Concat, T::AccountId, BalanceOf<T>, ValueQuery>;

    /// The incentive reward index for each market for each borrower as of the last time they accrued Reward
    #[pallet::storage]
    #[pallet::getter(fn reward_borrower_index)]
    pub type RewardBorrowerIndex<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, CurrencyId<T>, Blake2_128Concat, T::AccountId, BalanceOf<T>, ValueQuery>;

    /// The incentive reward accrued but not yet transferred to each user.
    #[pallet::storage]
    #[pallet::getter(fn reward_accrued)]
    pub type RewardAccrued<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, BalanceOf<T>, ValueQuery>;

    /// The maximum allowed exchange rate for a market.
    #[pallet::storage]
    #[pallet::getter(fn max_exchange_rate)]
    pub type MaxExchangeRate<T: Config> = StorageValue<_, Rate, ValueQuery>;

    /// The minimum allowed exchange rate for a market. This is the starting rate when a market is first set up.
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
                max_exchange_rate: Rate::from_inner(DEFAULT_MAX_EXCHANGE_RATE),
                min_exchange_rate: Rate::from_inner(DEFAULT_MIN_EXCHANGE_RATE),
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
        /// Creates a new lending market for a given currency. Returns `Err` if a market already
        /// exists for the given currency.
        ///
        /// All provided market states must be `Pending`, otherwise an error will be returned.
        ///
        /// The lend_token id specified in the Market struct has to be unique, and cannot be later reused
        /// when creating a new market.
        ///
        /// - `asset_id`: Currency to enable lending and borrowing for.
        /// - `market`: Configuration of the new lending market
        #[pallet::call_index(0)]
        #[pallet::weight(<T as Config>::WeightInfo::add_market())]
        #[transactional]
        pub fn add_market(
            origin: OriginFor<T>,
            asset_id: CurrencyId<T>,
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
                market.liquidate_incentive_reserved_factor < Ratio::one(),
                Error::<T>::InvalidFactor,
            );
            ensure!(market.supply_cap > Zero::zero(), Error::<T>::InvalidSupplyCap,);

            // Ensures a given `lend_token_id` does not exist either in a `Market` or as an `UnderlyingAssetId`.
            Self::ensure_lend_token(market.lend_token_id)?;
            // Update storage of `Market` and `UnderlyingAssetId`
            Markets::<T>::insert(asset_id, market.clone());
            UnderlyingAssetId::<T>::insert(market.lend_token_id, asset_id);

            // Init the ExchangeRate and BorrowIndex for asset
            let initial_exchange_rate = Self::min_exchange_rate();
            let initial_borrow_index = Rate::one();
            ExchangeRate::<T>::insert(asset_id, initial_exchange_rate);
            BorrowIndex::<T>::insert(asset_id, initial_borrow_index);

            // Emit an `InterestAccrued` event so event subscribers can see the
            // initial exchange rate.
            Self::deposit_event(Event::<T>::InterestAccrued {
                underlying_currency_id: asset_id,
                total_borrows: Balance::zero(),
                total_reserves: Balance::zero(),
                borrow_index: initial_borrow_index,
                utilization_ratio: Ratio::zero(),
                borrow_rate: Rate::zero(),
                supply_rate: Rate::zero(),
                exchange_rate: initial_exchange_rate,
            });
            Self::deposit_event(Event::<T>::NewMarket {
                underlying_currency_id: asset_id,
                market,
            });
            Ok(().into())
        }

        /// Activates a market. Returns `Err` if the market does not exist.
        ///
        /// If the market is already active, does nothing.
        ///
        /// - `asset_id`: Currency to enable lending and borrowing for.
        #[pallet::call_index(1)]
        #[pallet::weight(<T as Config>::WeightInfo::activate_market())]
        #[transactional]
        pub fn activate_market(origin: OriginFor<T>, asset_id: CurrencyId<T>) -> DispatchResultWithPostInfo {
            T::UpdateOrigin::ensure_origin(origin)?;
            // TODO: if the market is already active throw an error,
            // to avoid emitting the `ActivatedMarket` event again.
            Self::mutate_market(asset_id, |stored_market| {
                if let MarketState::Active = stored_market.state {
                    return stored_market.clone();
                }
                stored_market.state = MarketState::Active;
                stored_market.clone()
            })?;
            Self::deposit_event(Event::<T>::ActivatedMarket {
                underlying_currency_id: asset_id,
            });
            Ok(().into())
        }

        /// Updates the rate model of a stored market. Returns `Err` if the market
        /// currency does not exist or the rate model is invalid.
        ///
        /// - `asset_id`: Market currency
        /// - `rate_model`: The new rate model to set
        #[pallet::call_index(2)]
        #[pallet::weight(<T as Config>::WeightInfo::update_rate_model())]
        #[transactional]
        pub fn update_rate_model(
            origin: OriginFor<T>,
            asset_id: CurrencyId<T>,
            rate_model: InterestRateModel,
        ) -> DispatchResultWithPostInfo {
            T::UpdateOrigin::ensure_origin(origin)?;
            ensure!(rate_model.check_model(), Error::<T>::InvalidRateModelParam);
            let market = Self::mutate_market(asset_id, |stored_market| {
                stored_market.rate_model = rate_model;
                stored_market.clone()
            })?;
            Self::deposit_event(Event::<T>::UpdatedMarket {
                underlying_currency_id: asset_id,
                market,
            });

            Ok(().into())
        }

        /// Updates a stored market. Returns `Err` if the market currency does not exist.
        ///
        /// - `asset_id`: market related currency
        /// - `collateral_factor`: the collateral utilization ratio
        /// - `liquidation_threshold`: The collateral ratio when a borrower can be liquidated
        /// - `reserve_factor`: fraction of interest set aside for reserves
        /// - `close_factor`: max percentage of debt that can be liquidated in a single transaction
        /// - `liquidate_incentive_reserved_factor`: liquidation share set aside for reserves
        /// - `liquidate_incentive`: liquidation incentive ratio
        /// - `supply_cap`: Upper bound of supplying
        /// - `borrow_cap`: Upper bound of borrowing
        #[pallet::call_index(3)]
        #[pallet::weight(<T as Config>::WeightInfo::update_market())]
        #[transactional]
        pub fn update_market(
            origin: OriginFor<T>,
            asset_id: CurrencyId<T>,
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
            ensure!(
                market.liquidate_incentive_reserved_factor < Ratio::one(),
                Error::<T>::InvalidFactor,
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
            Self::deposit_event(Event::<T>::UpdatedMarket {
                underlying_currency_id: asset_id,
                market,
            });

            Ok(().into())
        }

        /// Force updates a stored market. Returns `Err` if the market currency
        /// does not exist.
        ///
        /// - `asset_id`: market related currency
        /// - `market`: Configuration of the new lending market
        #[pallet::call_index(4)]
        #[pallet::weight(<T as Config>::WeightInfo::force_update_market())]
        #[transactional]
        pub fn force_update_market(
            origin: OriginFor<T>,
            asset_id: CurrencyId<T>,
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

            Self::deposit_event(Event::<T>::UpdatedMarket {
                underlying_currency_id: asset_id,
                market: updated_market,
            });
            Ok(().into())
        }

        /// Deposit incentive reward currency into the pallet account.
        ///
        /// - `amount`: Reward amount added
        #[pallet::call_index(5)]
        #[pallet::weight(<T as Config>::WeightInfo::add_reward())]
        #[transactional]
        pub fn add_reward(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            ensure!(!amount.is_zero(), Error::<T>::InvalidAmount);

            let reward_asset = T::RewardAssetId::get();
            let pool_account = Self::reward_account_id();

            let amount_to_transfer: Amount<T> = Amount::new(amount, reward_asset);
            amount_to_transfer.transfer(&who, &pool_account)?;

            Self::deposit_event(Event::<T>::RewardAdded { payer: who, amount });

            Ok(().into())
        }

        /// Updates reward speed for the specified market
        ///
        /// The origin must conform to `UpdateOrigin`.
        ///
        /// - `asset_id`: Market related currency
        /// - `supply_reward_per_block`: supply reward amount per block.
        /// - `borrow_reward_per_block`: borrow reward amount per block.
        #[pallet::call_index(6)]
        #[pallet::weight(<T as Config>::WeightInfo::update_market_reward_speed())]
        #[transactional]
        pub fn update_market_reward_speed(
            origin: OriginFor<T>,
            asset_id: CurrencyId<T>,
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

            Self::deposit_event(Event::<T>::MarketRewardSpeedUpdated {
                underlying_currency_id: asset_id,
                supply_reward_per_block,
                borrow_reward_per_block,
            });
            Ok(().into())
        }

        /// Claim incentive rewards for all markets.
        #[pallet::call_index(7)]
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

        /// Claim inceitve reward for the specified market.
        ///
        /// - `asset_id`: Market related currency
        #[pallet::call_index(8)]
        #[pallet::weight(<T as Config>::WeightInfo::claim_reward_for_market())]
        #[transactional]
        pub fn claim_reward_for_market(origin: OriginFor<T>, asset_id: CurrencyId<T>) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            Self::collect_market_reward(asset_id, &who)?;

            Self::pay_reward(&who)?;

            Ok(().into())
        }

        /// The caller supplies (lends) assets into the market and receives a corresponding amount
        /// of lend tokens, at the current internal exchange rate.
        ///
        /// - `asset_id`: the asset to be deposited.
        /// - `mint_amount`: the amount to be deposited.
        #[pallet::call_index(9)]
        #[pallet::weight(<T as Config>::WeightInfo::mint())]
        #[transactional]
        pub fn mint(
            origin: OriginFor<T>,
            asset_id: CurrencyId<T>,
            #[pallet::compact] mint_amount: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            ensure!(!mint_amount.is_zero(), Error::<T>::InvalidAmount);
            Self::do_mint(&who, &Amount::new(mint_amount, asset_id))?;

            Ok(().into())
        }

        /// The caller redeems lend tokens for the underlying asset, at the current
        /// internal exchange rate.
        ///
        /// - `asset_id`: the asset to be redeemed
        /// - `redeem_amount`: the amount to be redeemed, expressed in the underyling currency (`asset_id`)
        #[pallet::call_index(10)]
        #[pallet::weight(<T as Config>::WeightInfo::redeem())]
        #[transactional]
        pub fn redeem(
            origin: OriginFor<T>,
            asset_id: CurrencyId<T>,
            #[pallet::compact] redeem_amount: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            ensure!(!redeem_amount.is_zero(), Error::<T>::InvalidAmount);

            let underlying = Amount::<T>::new(redeem_amount, asset_id);
            let voucher = underlying.to_lend_token()?;

            Self::do_redeem(&who, &underlying, &voucher)?;

            Ok(().into())
        }

        /// The caller redeems their entire lend token balance in exchange for the underlying asset.
        /// Note: this will fail if the account needs some of the collateral for backing open borrows,
        /// or if any of the lend tokens are used by other pallets (e.g. used as vault collateral)
        ///
        /// - `asset_id`: the asset to be redeemed.
        #[pallet::call_index(11)]
        #[pallet::weight(<T as Config>::WeightInfo::redeem_all())]
        #[transactional]
        pub fn redeem_all(origin: OriginFor<T>, asset_id: CurrencyId<T>) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin.clone())?;

            let voucher = Self::balance(Self::lend_token_id(asset_id)?, &who);
            let underlying = &voucher.to_underlying()?;

            // note: we use total balance rather than underlying.to_lend_token, s.t. the account
            // is left neatly with 0 balance
            Self::do_redeem(&who, &underlying, &voucher)?;

            Ok(().into())
        }

        /// The caller borrows `borrow_amount` of `asset_id` from the protocol, using their
        /// supplied assets as collateral.
        ///
        /// - `asset_id`: the asset to be borrowed.
        /// - `borrow_amount`: the amount to be borrowed.
        #[pallet::call_index(12)]
        #[pallet::weight(<T as Config>::WeightInfo::borrow())]
        #[transactional]
        pub fn borrow(
            origin: OriginFor<T>,
            asset_id: CurrencyId<T>,
            #[pallet::compact] borrow_amount: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            ensure!(!borrow_amount.is_zero(), Error::<T>::InvalidAmount);
            Self::do_borrow(&who, &Amount::new(borrow_amount, asset_id))?;

            Ok(().into())
        }

        /// The caller repays some of their debts.
        ///
        /// - `asset_id`: the asset to be repaid.
        /// - `repay_amount`: the amount to be repaid, in the underlying currency (`asset_id`).
        #[pallet::call_index(13)]
        #[pallet::weight(<T as Config>::WeightInfo::repay_borrow())]
        #[transactional]
        pub fn repay_borrow(
            origin: OriginFor<T>,
            asset_id: CurrencyId<T>,
            #[pallet::compact] repay_amount: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            ensure!(!repay_amount.is_zero(), Error::<T>::InvalidAmount);
            Self::do_repay_borrow(&who, &Amount::new(repay_amount, asset_id))?;

            Ok(().into())
        }

        /// The caller repays all of their debts.
        ///
        /// - `asset_id`: the asset to be repaid.
        #[pallet::call_index(14)]
        #[pallet::weight(<T as Config>::WeightInfo::repay_borrow_all())]
        #[transactional]
        pub fn repay_borrow_all(origin: OriginFor<T>, asset_id: CurrencyId<T>) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            Self::ensure_active_market(asset_id)?;
            Self::accrue_interest(asset_id)?;
            let account_borrows = Self::current_borrow_balance(&who, asset_id)?;
            ensure!(!account_borrows.is_zero(), Error::<T>::InvalidAmount);
            Self::do_repay_borrow(&who, &account_borrows)?;

            Ok(().into())
        }

        /// Caller enables their lend token balance as borrow collateral. This operation locks
        /// the lend tokens, so they are no longer transferrable.
        /// Any incoming lend tokens into the caller's account (either by direct transfer or minting)
        /// are automatically locked as well, such that locking and unlocking borrow collateral is
        /// an atomic state (a "collateral toggle").
        /// If any of the caller's lend token balance is locked elsewhere (for instance, as bridge vault
        /// collateral), this operation will fail.
        /// If this operation is successful, the caller's maximum allowed debt increases.
        ///
        /// - `asset_id`: the underlying asset denoting the market whose lend tokens are to be
        /// enabled as collateral.
        #[pallet::call_index(15)]
        #[pallet::weight(<T as Config>::WeightInfo::deposit_all_collateral())]
        #[transactional]
        pub fn deposit_all_collateral(origin: OriginFor<T>, asset_id: CurrencyId<T>) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            let free_lend_tokens = Self::free_lend_tokens(asset_id, &who)?;
            ensure!(!free_lend_tokens.is_zero(), Error::<T>::DepositAllCollateralFailed);
            let reserved_lend_tokens = Self::reserved_lend_tokens(asset_id, &who)?;
            // This check could fail if `withdraw_all_collateral()` leaves leftover lend_tokens locked.
            // However the current implementation is guaranteed to withdraw everything.
            ensure!(reserved_lend_tokens.is_zero(), Error::<T>::TokensAlreadyLocked);
            Self::do_deposit_collateral(&who, &free_lend_tokens)?;
            Ok(().into())
        }

        /// Caller disables their lend token balance as borrow collateral. This operation unlocks
        /// the lend tokens, so they become transferrable.
        /// This operation can only succeed if the caller's debt is backed by sufficient collateral
        /// excluding this currency.
        ///
        /// - `asset_id`: the underlying asset denoting the market whose lend tokens are to be
        /// disabled as collateral.
        #[pallet::call_index(16)]
        #[pallet::weight(<T as Config>::WeightInfo::withdraw_all_collateral())]
        #[transactional]
        pub fn withdraw_all_collateral(origin: OriginFor<T>, asset_id: CurrencyId<T>) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            let lend_token_id = Self::lend_token_id(asset_id)?;
            let collateral = Self::account_deposits(lend_token_id, &who.clone());
            ensure!(!collateral.is_zero(), Error::<T>::WithdrawAllCollateralFailed);
            Self::do_withdraw_collateral(&who, &collateral)?;
            Ok(().into())
        }

        /// The caller liquidates the borrower's collateral. This extrinsic may need to be called multiple
        /// times to completely clear the borrower's bad debt, because of the `close_factor` parameter in
        /// the market. See the `close_factor_may_require_multiple_liquidations_to_clear_bad_debt` unit
        /// test for an example of this.
        ///
        /// - `borrower`: the borrower to be liquidated.
        /// - `liquidation_asset_id`: the underlying asset to be liquidated.
        /// - `repay_amount`: the amount of `liquidation_asset_id` to be repaid. This parameter can only
        /// be as large as the `close_factor` market parameter allows
        /// (`close_factor * borrower_debt_in_liquidation_asset`).
        /// - `collateral_asset_id`: The underlying currency whose lend tokens to seize from the borrower.
        /// Note that the liquidator has to redeem the received lend tokens from the market to convert them
        /// to `collateral_asset_id`.
        #[pallet::call_index(17)]
        #[pallet::weight(<T as Config>::WeightInfo::liquidate_borrow())]
        #[transactional]
        pub fn liquidate_borrow(
            origin: OriginFor<T>,
            borrower: T::AccountId,
            liquidation_asset_id: CurrencyId<T>,
            #[pallet::compact] repay_amount: BalanceOf<T>,
            collateral_asset_id: CurrencyId<T>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            Self::accrue_interest(liquidation_asset_id)?;
            Self::accrue_interest(collateral_asset_id)?;
            ensure!(!repay_amount.is_zero(), Error::<T>::InvalidAmount);
            let liquidation = Amount::new(repay_amount, liquidation_asset_id);
            Self::do_liquidate_borrow(who, borrower, &liquidation, collateral_asset_id)?;
            Ok(().into())
        }

        /// Add reserves by transferring from payer.
        /// TODO: This extrinsic currently does nothing useful. See the TODO comment
        /// of the `ensure_enough_cash` function for more details. Based on that
        /// TODO, decide whether this extrinsic should be kept.
        ///
        /// May only be called from `T::ReserveOrigin`.
        ///
        /// - `payer`: the payer account.
        /// - `asset_id`: the assets to be added.
        /// - `add_amount`: the amount to be added.
        #[pallet::call_index(18)]
        #[pallet::weight(<T as Config>::WeightInfo::add_reserves())]
        #[transactional]
        pub fn add_reserves(
            origin: OriginFor<T>,
            payer: <T::Lookup as StaticLookup>::Source,
            asset_id: CurrencyId<T>,
            #[pallet::compact] add_amount: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let amount_to_transfer = Amount::new(add_amount, asset_id);
            T::ReserveOrigin::ensure_origin(origin)?;
            let payer = T::Lookup::lookup(payer)?;
            Self::ensure_active_market(asset_id)?;
            Self::accrue_interest(asset_id)?;

            ensure!(!amount_to_transfer.is_zero(), Error::<T>::InvalidAmount);
            amount_to_transfer.transfer(&payer, &Self::account_id())?;
            let total_reserves = Self::total_reserves(asset_id);
            let total_reserves_new = total_reserves.checked_add(&amount_to_transfer)?;
            TotalReserves::<T>::insert(asset_id, total_reserves_new.amount());

            Self::deposit_event(Event::<T>::ReservesAdded {
                payer,
                currency_id: asset_id,
                amount: amount_to_transfer.amount(),
                new_reserve_amount: total_reserves_new.amount(),
            });

            Ok(().into())
        }

        /// Reduces reserves (treasury's share of accrued interest) by transferring to receiver.
        ///
        /// May only be called from `T::ReserveOrigin`.
        ///
        /// - `receiver`: the receiver account.
        /// - `asset_id`: the assets to be reduced.
        /// - `reduce_amount`: the amount to be reduced.
        #[pallet::call_index(19)]
        #[pallet::weight(<T as Config>::WeightInfo::reduce_reserves())]
        #[transactional]
        pub fn reduce_reserves(
            origin: OriginFor<T>,
            receiver: <T::Lookup as StaticLookup>::Source,
            asset_id: CurrencyId<T>,
            #[pallet::compact] reduce_amount: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            T::ReserveOrigin::ensure_origin(origin)?;
            let receiver = T::Lookup::lookup(receiver)?;
            Self::ensure_active_market(asset_id)?;
            Self::accrue_interest(asset_id)?;

            let amount_to_transfer = Amount::new(reduce_amount, asset_id);

            ensure!(!amount_to_transfer.is_zero(), Error::<T>::InvalidAmount);
            let total_reserves = Self::total_reserves(asset_id);
            if amount_to_transfer.gt(&total_reserves)? {
                return Err(Error::<T>::InsufficientReserves.into());
            }
            let total_reserves_new = total_reserves.checked_sub(&amount_to_transfer)?;
            TotalReserves::<T>::insert(asset_id, total_reserves_new.amount());

            amount_to_transfer.transfer(&Self::account_id(), &receiver)?;

            Self::deposit_event(Event::<T>::ReservesReduced {
                receiver,
                currency_id: asset_id,
                amount: amount_to_transfer.amount(),
                new_reserve_amount: total_reserves_new.amount(),
            });

            Ok(().into())
        }

        /// Sender redeems some of internal supplies in exchange for the underlying asset.
        ///
        /// - `asset_id`: the asset to be redeemed.
        /// - `redeem_amount`: the amount to be redeemed.
        #[pallet::call_index(20)]
        #[pallet::weight(<T as Config>::WeightInfo::reduce_incentive_reserves())]
        #[transactional]
        pub fn reduce_incentive_reserves(
            origin: OriginFor<T>,
            receiver: <T::Lookup as StaticLookup>::Source,
            asset_id: CurrencyId<T>,
            #[pallet::compact] redeem_amount: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            T::ReserveOrigin::ensure_origin(origin)?;
            ensure!(!redeem_amount.is_zero(), Error::<T>::InvalidAmount);
            let receiver = T::Lookup::lookup(receiver)?;
            let from = Self::incentive_reward_account_id();
            Self::ensure_active_market(asset_id)?;
            Self::accrue_interest(asset_id)?;

            let underlying = Amount::new(redeem_amount, asset_id);

            Self::do_redeem(&from, &underlying, &underlying.to_lend_token()?)?;

            underlying.transfer(&from, &receiver)?;

            Self::deposit_event(Event::<T>::IncentiveReservesReduced {
                receiver,
                currency_id: asset_id,
                amount: underlying.amount(),
            });
            Ok(().into())
        }
    }
}

impl<T: Config> Pallet<T> {
    #[cfg_attr(any(test, feature = "integration-tests"), visibility::make(pub))]
    fn account_deposits(lend_token_id: CurrencyId<T>, supplier: &T::AccountId) -> Amount<T> {
        Amount::new(AccountDeposits::<T>::get(lend_token_id, supplier), lend_token_id)
    }

    #[cfg_attr(any(test, feature = "integration-tests"), visibility::make(pub))]
    fn total_borrows(asset_id: CurrencyId<T>) -> Amount<T> {
        Amount::new(TotalBorrows::<T>::get(asset_id), asset_id)
    }

    #[cfg_attr(any(test, feature = "integration-tests"), visibility::make(pub))]
    fn total_reserves(asset_id: CurrencyId<T>) -> Amount<T> {
        Amount::new(TotalReserves::<T>::get(asset_id), asset_id)
    }

    pub fn account_id() -> T::AccountId {
        T::PalletId::get().into_account_truncating()
    }

    pub fn get_account_liquidity(account: &T::AccountId) -> Result<AccountLiquidity<T>, DispatchError> {
        let total_collateral_value = Self::total_collateral_value(account)?;
        let total_borrow_value = Self::total_borrowed_value(account)?;
        log::trace!(
            target: "loans::get_account_liquidity",
            "account: {:?}, total_borrow_value: {:?}, total_collateral_value: {:?}",
            account,
            total_borrow_value.amount(),
            total_collateral_value.amount(),
        );
        AccountLiquidity::from_collateral_and_debt(total_collateral_value, total_borrow_value)
    }

    pub fn get_account_liquidation_threshold_liquidity(
        account: &T::AccountId,
    ) -> Result<AccountLiquidity<T>, DispatchError> {
        let total_collateral_value = Self::total_liquidation_threshold_value(account)?;
        let total_borrow_value = Self::total_borrowed_value(account)?;
        log::trace!(
            target: "loans::get_account_liquidation_threshold_liquidity",
            "account: {:?}, total_borrow_value: {:?}, total_collateral_value: {:?}",
            account,
            total_borrow_value.amount(),
            total_collateral_value.amount(),
        );
        AccountLiquidity::from_collateral_and_debt(total_collateral_value, total_borrow_value)
    }

    fn total_borrowed_value(borrower: &T::AccountId) -> Result<Amount<T>, DispatchError> {
        let mut total_borrow_value = Amount::<T>::zero(T::ReferenceAssetId::get());
        for (asset_id, _) in Self::active_markets() {
            let borrow = Self::current_borrow_balance(borrower, asset_id)?;
            if borrow.is_zero() {
                continue;
            }
            let value = Self::get_asset_value(&borrow)?;
            total_borrow_value.checked_accrue(&value)?;
        }

        Ok(total_borrow_value)
    }

    fn collateral_amount_value(voucher: &Amount<T>) -> Result<Amount<T>, DispatchError> {
        let underlying = voucher.to_underlying()?;
        let market = Self::market(underlying.currency())?;
        let effects = underlying.map(|x| market.collateral_factor.mul_ceil(x));

        Self::get_asset_value(&effects)
    }

    fn collateral_asset_value(supplier: &T::AccountId, asset_id: CurrencyId<T>) -> Result<Amount<T>, DispatchError> {
        let lend_token_id = Self::lend_token_id(asset_id)?;
        let deposits = Self::account_deposits(lend_token_id, supplier);
        if deposits.is_zero() {
            return Ok(Amount::<T>::zero(T::ReferenceAssetId::get()));
        }
        Self::collateral_amount_value(&deposits)
    }

    fn liquidation_threshold_asset_value(
        borrower: &T::AccountId,
        asset_id: CurrencyId<T>,
    ) -> Result<Amount<T>, DispatchError> {
        let lend_token_id = Self::lend_token_id(asset_id)?;
        if !AccountDeposits::<T>::contains_key(lend_token_id, borrower) {
            return Ok(Amount::<T>::zero(T::ReferenceAssetId::get()));
        }
        let deposits = Self::account_deposits(lend_token_id, borrower);
        if deposits.is_zero() {
            return Ok(Amount::<T>::zero(T::ReferenceAssetId::get()));
        }
        let underlying_amount = deposits.to_underlying()?;
        let market = Self::market(asset_id)?;
        let effects_amount = underlying_amount.map(|x| market.liquidation_threshold.mul_ceil(x));

        Self::get_asset_value(&effects_amount)
    }

    fn total_collateral_value(supplier: &T::AccountId) -> Result<Amount<T>, DispatchError> {
        let mut total_asset_value = Amount::<T>::zero(T::ReferenceAssetId::get());
        for (asset_id, _market) in Self::active_markets() {
            total_asset_value = total_asset_value.checked_add(&Self::collateral_asset_value(supplier, asset_id)?)?;
        }

        Ok(total_asset_value)
    }

    fn total_liquidation_threshold_value(borrower: &T::AccountId) -> Result<Amount<T>, DispatchError> {
        let mut total_asset_value = Amount::<T>::zero(T::ReferenceAssetId::get());
        for (asset_id, _market) in Self::active_markets() {
            total_asset_value =
                total_asset_value.checked_add(&Self::liquidation_threshold_asset_value(borrower, asset_id)?)?;
        }

        Ok(total_asset_value)
    }

    /// Checks if the redeemer should be allowed to redeem tokens in given market.
    /// Takes into account both `free` and `locked` (i.e. deposited as collateral) lend_tokens of the redeemer.
    fn redeem_allowed(redeemer: &T::AccountId, voucher: &Amount<T>) -> DispatchResult {
        let asset_id = Self::underlying_id(voucher.currency())?;
        log::trace!(
            target: "loans::redeem_allowed",
            "asset_id: {:?}, redeemer: {:?}, voucher_amount: {:?}",
            asset_id,
            redeemer,
            voucher.amount(),
        );

        ensure!(!voucher.is_zero(), Error::<T>::InvalidAmount);

        if Self::balance(voucher.currency(), redeemer).lt(&voucher)? {
            return Err(Error::<T>::InsufficientDeposit.into());
        }

        // Ensure there is enough cash in the market
        let redeem_amount = voucher.to_underlying()?;
        Self::ensure_enough_cash(&redeem_amount)?;

        // Only free tokens are redeemable. If the account has enough liquidity, the lend tokens
        // must first be withdrawn from collateral (this happens automatically in the `redeem` and
        // `redeem_all` extrinsics)
        if voucher.gt(&Self::free_lend_tokens(asset_id, redeemer)?)? {
            return Err(Error::<T>::LockedTokensCannotBeRedeemed.into());
        }
        Ok(())
    }

    /// Borrower shouldn't borrow more than their total collateral value allows
    fn borrow_allowed(borrower: &T::AccountId, borrow: &Amount<T>) -> DispatchResult {
        Self::ensure_under_borrow_cap(borrow)?;
        Self::ensure_enough_cash(borrow)?;
        let borrow_value = Self::get_asset_value(borrow)?;
        Self::ensure_liquidity(borrower, borrow_value)?;

        Ok(())
    }

    #[require_transactional]
    fn do_repay_borrow_with_amount(
        borrower: &T::AccountId,
        asset_id: CurrencyId<T>,
        account_borrows: &Amount<T>,
        repay_amount: &Amount<T>,
    ) -> DispatchResult {
        if account_borrows.lt(repay_amount)? {
            return Err(Error::<T>::TooMuchRepay.into());
        }
        Self::update_reward_borrow_index(asset_id)?;
        Self::distribute_borrower_reward(asset_id, borrower)?;

        repay_amount.transfer(borrower, &Self::account_id())?;

        let account_borrows_new = account_borrows.checked_sub(&repay_amount)?;
        let total_borrows = Self::total_borrows(asset_id);
        // Use `saturating_sub` here, because it's intended for `total_borrows` to be rounded down,
        // such that it is less than or equal to the actual borrower debt.
        let total_borrows_new = total_borrows.saturating_sub(&repay_amount)?;
        AccountBorrows::<T>::insert(
            asset_id,
            borrower,
            BorrowSnapshot {
                principal: account_borrows_new.amount(),
                borrow_index: Self::borrow_index(asset_id),
            },
        );
        TotalBorrows::<T>::insert(asset_id, total_borrows_new.amount());

        Ok(())
    }

    // Calculates and returns the most recent amount of borrowed balance of `currency_id`
    // for `who`.
    pub fn current_borrow_balance(who: &T::AccountId, asset_id: CurrencyId<T>) -> Result<Amount<T>, DispatchError> {
        let snapshot: BorrowSnapshot<BalanceOf<T>> = Self::account_borrows(asset_id, who);
        if snapshot.principal.is_zero() || snapshot.borrow_index.is_zero() {
            return Ok(Amount::zero(asset_id));
        }
        let principal_amount = Amount::<T>::new(snapshot.principal, asset_id);
        // Round borrower debt up to avoid interest-free loans
        Self::borrow_balance_from_old_and_new_index(
            &snapshot.borrow_index,
            &Self::borrow_index(asset_id),
            principal_amount,
            Rounding::Up,
        )
    }

    pub fn borrow_balance_from_old_and_new_index(
        old_index: &FixedU128,
        new_index: &FixedU128,
        amount: Amount<T>,
        rounding: Rounding,
    ) -> Result<Amount<T>, DispatchError> {
        // Calculate new borrow balance using the interest index:
        // recent_borrow_balance = snapshot.principal * borrow_index / snapshot.borrow_index
        let borrow_index_increase = new_index.checked_div(&old_index).ok_or(ArithmeticError::Underflow)?;
        amount.checked_rounded_mul(&borrow_index_increase, rounding)
    }

    /// Checks if the liquidation should be allowed to occur
    fn liquidate_borrow_allowed(
        borrower: &T::AccountId,
        underlying: &Amount<T>,
        market: &Market<BalanceOf<T>>,
    ) -> DispatchResult {
        log::trace!(
            target: "loans::liquidate_borrow_allowed",
            "borrower: {:?}, liquidation_asset_id {:?}, repay_amount {:?}, market: {:?}",
            borrower,
            underlying.currency(),
            underlying.amount(),
            market
        );
        // The account's shortfall, as calculated using the liquidation threshold, should be non-zero
        if Self::get_account_liquidation_threshold_liquidity(borrower)?
            .shortfall()
            .is_zero()
        {
            return Err(Error::<T>::InsufficientShortfall.into());
        }

        // The liquidator may not repay more than 50% (close_factor) of the borrower's borrow balance.
        let account_borrows = Self::current_borrow_balance(borrower, underlying.currency())?;
        let account_borrows_value = Self::get_asset_value(&account_borrows)?;
        let repay_value = Self::get_asset_value(&underlying)?;

        if account_borrows_value
            .map(|x| market.close_factor.mul_ceil(x))
            .lt(&repay_value)?
        {
            return Err(Error::<T>::TooMuchRepay.into());
        }

        Ok(())
    }

    /// Note:
    /// - `liquidation_asset_id` is borrower's debt asset.
    /// - `collateral_asset_id` is borrower's collateral asset.
    /// - `repay_amount` is amount of liquidation_asset_id
    ///
    /// The liquidator will repay a certain amount of liquidation_asset_id from own
    /// account for borrower. Then the protocol will reduce borrower's debt
    /// and liquidator will receive collateral_asset_id (as voucher amount) from
    /// borrower.
    #[require_transactional]
    pub fn do_liquidate_borrow(
        liquidator: T::AccountId,
        borrower: T::AccountId,
        repayment_underlying: &Amount<T>,
        collateral_asset_id: CurrencyId<T>,
    ) -> DispatchResult {
        let liquidation_asset_id = repayment_underlying.currency();
        Self::ensure_active_market(liquidation_asset_id)?;
        Self::ensure_active_market(collateral_asset_id)?;

        let market = Self::market(liquidation_asset_id)?;

        if borrower == liquidator {
            return Err(Error::<T>::LiquidatorIsBorrower.into());
        }
        Self::liquidate_borrow_allowed(&borrower, repayment_underlying, &market)?;

        let lend_token_id = Self::lend_token_id(collateral_asset_id)?;
        let deposits = Self::account_deposits(lend_token_id, &borrower);
        ensure!(!deposits.is_zero(), Error::<T>::DepositsAreNotCollateral);
        let borrower_deposits = deposits.to_underlying()?;

        let collateral_value = Self::get_asset_value(&borrower_deposits)?;
        // liquidate_value includes the premium of the liquidator
        let liquidate_value = Self::get_asset_value(repayment_underlying)?.checked_mul(&market.liquidate_incentive)?;
        if collateral_value.lt(&liquidate_value)? {
            return Err(Error::<T>::InsufficientCollateral.into());
        }

        // Calculate the collateral amount to seize from the borrower
        let real_collateral_underlying_amount = liquidate_value.convert_to(collateral_asset_id)?;
        Self::liquidated_transfer(
            &liquidator,
            &borrower,
            &repayment_underlying,
            &real_collateral_underlying_amount,
            &market,
        )?;

        Ok(())
    }

    #[require_transactional]
    fn liquidated_transfer(
        liquidator: &T::AccountId,
        borrower: &T::AccountId,
        repayment: &Amount<T>,
        collateral_underlying: &Amount<T>,
        market: &Market<BalanceOf<T>>,
    ) -> DispatchResult {
        let liquidation_asset_id = repayment.currency();
        let collateral_asset_id = collateral_underlying.currency();

        log::error!(
            target: "loans::liquidated_transfer",
            "liquidator: {:?}, borrower: {:?}, liquidation_asset_id: {:?},
                collateral_asset_id: {:?}, repay_amount: {:?}, collateral_underlying.amount(): {:?}",
            liquidator,
            borrower,
            repayment.currency(),
            collateral_underlying.currency(),
            repayment.amount(),
            collateral_underlying.amount()
        );

        // update borrow index after accrue interest.
        Self::update_reward_borrow_index(liquidation_asset_id)?;
        Self::distribute_borrower_reward(liquidation_asset_id, liquidator)?;

        // 1.liquidator repays borrower's debt,
        // transfer from liquidator to module account
        repayment.transfer(liquidator, &Self::account_id())?;

        // 2.the system reduces borrower's debt
        let account_borrows_new =
            Self::current_borrow_balance(borrower, liquidation_asset_id)?.checked_sub(&repayment)?;
        let total_borrows_new = Self::total_borrows(liquidation_asset_id).checked_sub(&repayment)?;
        AccountBorrows::<T>::insert(
            liquidation_asset_id,
            borrower,
            BorrowSnapshot {
                principal: account_borrows_new.amount(),
                borrow_index: Self::borrow_index(liquidation_asset_id),
            },
        );
        TotalBorrows::<T>::insert(liquidation_asset_id, total_borrows_new.amount());

        // update supply index before modify supply balance.
        Self::update_reward_supply_index(collateral_asset_id)?;
        Self::distribute_supplier_reward(collateral_asset_id, liquidator)?;
        Self::distribute_supplier_reward(collateral_asset_id, borrower)?;
        Self::distribute_supplier_reward(collateral_asset_id, &Self::incentive_reward_account_id())?;

        // 3.the liquidator will receive voucher token from borrower
        let lend_token_id = Self::lend_token_id(collateral_asset_id)?;
        let amount_to_liquidate = collateral_underlying.to_lend_token()?;
        // Decrease the amount of collateral the borrower deposited
        AccountDeposits::<T>::try_mutate_exists(lend_token_id, borrower, |deposits| -> DispatchResult {
            let d = deposits
                .unwrap_or_default()
                .checked_sub(amount_to_liquidate.amount())
                .ok_or(ArithmeticError::Underflow)?;
            if d.is_zero() {
                // remove deposits storage if zero balance
                *deposits = None;
            } else {
                *deposits = Some(d);
            }
            Ok(())
        })?;
        // Unlock this balance to make it transferrable
        amount_to_liquidate.unlock_on(borrower)?;

        let incentive_reserved = amount_to_liquidate
            .checked_div(&market.liquidate_incentive)?
            .mul_ratio_floor(market.liquidate_incentive_reserved_factor);

        // increase liquidator's voucher_balance
        let liquidator_amount = amount_to_liquidate.checked_sub(&incentive_reserved)?;
        liquidator_amount.transfer(borrower, liquidator)?;

        // increase reserve's voucher_balance
        incentive_reserved.transfer(borrower, &Self::incentive_reward_account_id())?;

        Self::deposit_event(Event::<T>::LiquidatedBorrow {
            liquidator: liquidator.clone(),
            borrower: borrower.clone(),
            liquidation_currency_id: liquidation_asset_id,
            collateral_currency_id: collateral_asset_id,
            repay_amount: repayment.amount(),
            collateral_underlying_amount: collateral_underlying.amount(),
        });

        Ok(())
    }

    pub fn lock_if_account_deposited(account_id: &T::AccountId, lend_tokens: &Amount<T>) -> DispatchResult {
        // if the receiver already has their collateral deposited
        let deposit = Pallet::<T>::account_deposits(lend_tokens.currency(), account_id);
        if !deposit.is_zero() {
            // then any incoming `lend_tokens` must automatically be deposited as collateral
            // to enforce the "collateral toggle"
            Self::do_deposit_collateral(account_id, &lend_tokens)?;
        }
        Ok(())
    }

    // Ensures a given `asset_id` is an active market.
    fn ensure_active_market(asset_id: CurrencyId<T>) -> Result<Market<BalanceOf<T>>, DispatchError> {
        Self::active_markets()
            .find(|(id, _)| id == &asset_id)
            .map(|(_, market)| market)
            .ok_or_else(|| Error::<T>::MarketNotActivated.into())
    }

    /// Ensure supplying `amount` asset does not exceed the market's supply cap.
    fn ensure_under_supply_cap(asset: &Amount<T>) -> DispatchResult {
        let asset_id = asset.currency();

        let market = Self::market(asset_id)?;
        // Assets holded by market currently.
        let current_cash = Self::balance(asset_id, &Self::account_id());
        let total_cash = current_cash.checked_add(&asset)?;
        ensure!(
            total_cash.amount() <= market.supply_cap,
            Error::<T>::SupplyCapacityExceeded
        );

        Ok(())
    }

    /// Ensure borrowing `amount` asset does not exceed the market's borrow cap.
    fn ensure_under_borrow_cap(asset: &Amount<T>) -> DispatchResult {
        let asset_id = asset.currency();
        let market = Self::market(asset_id)?;
        let total_borrows = Self::total_borrows(asset_id);
        let new_total_borrows = total_borrows.checked_add(&asset)?;
        ensure!(
            new_total_borrows.amount() <= market.borrow_cap,
            Error::<T>::BorrowCapacityExceeded
        );

        Ok(())
    }

    /// Make sure there is enough cash available in the pool
    /// TODO: Compared to Compound's implementation, it seems like
    /// this function should not subtract the total reserves from the total cash,
    /// possibly to allow the treasury to deposit liquidity and enable users to exit
    /// their lend token positions in case of 100% utilization in the market.
    /// Decide if this function needs to be modified as such, or remove the `add_reserves`
    /// extrinsic since it currently does nothing useful.
    ///
    /// See the redeem check in Compound V2 (also the borrow check):
    /// - `getCashPrior() > redeemAmount`:
    /// https://github.com/compound-finance/compound-protocol/blob/a3214f67b73310d547e00fc578e8355911c9d376/contracts/CToken.sol#L518
    /// - but getCashPrior is the entire balance of the contract:
    /// https://github.com/compound-finance/compound-protocol/blob/a3214f67b73310d547e00fc578e8355911c9d376/contracts/CToken.sol#L1125
    fn ensure_enough_cash(amount: &Amount<T>) -> DispatchResult {
        let reducible_cash =
            Self::get_total_cash(amount.currency()).checked_sub(&Self::total_reserves(amount.currency()))?;
        if reducible_cash.lt(&amount)? {
            return Err(Error::<T>::InsufficientCash.into());
        }

        Ok(())
    }

    /// Ensures a given `lend_token_id` is unique in `Markets` and `UnderlyingAssetId`.
    fn ensure_lend_token(lend_token_id: CurrencyId<T>) -> DispatchResult {
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

    /// Ensures that `account` has sufficient liquidity to cover a withdrawal
    /// Returns `Err` If InsufficientLiquidity
    /// `account`: account that needs a liquidity check
    /// `reduce_amount`: amount to reduce the liquidity (collateral) of the `account` by
    fn ensure_liquidity(account: &T::AccountId, reduce_amount: Amount<T>) -> DispatchResult {
        if Self::get_account_liquidity(account)?.liquidity().ge(&reduce_amount)? {
            return Ok(());
        }
        Err(Error::<T>::InsufficientLiquidity.into())
    }

    /// Transferrable balance in the pallet account (`free - frozen`)
    fn get_total_cash(asset_id: CurrencyId<T>) -> Amount<T> {
        Amount::new(
            orml_tokens::Pallet::<T>::reducible_balance(asset_id, &Self::account_id(), true),
            asset_id,
        )
    }

    /// Get the total balance of `who`.
    /// Ignores any frozen balance of this account (`free + reserved`)
    fn balance(asset_id: CurrencyId<T>, who: &T::AccountId) -> Amount<T> {
        let balance = <orml_tokens::Pallet<T> as MultiCurrency<T::AccountId>>::total_balance(asset_id, who);
        Amount::new(balance, asset_id)
    }

    /// Total issuance of lending tokens (lend_tokens), given the underlying
    pub fn total_supply(asset_id: CurrencyId<T>) -> Result<Amount<T>, DispatchError> {
        let lend_token_id = Self::lend_token_id(asset_id)?;
        let issuance = orml_tokens::Pallet::<T>::total_issuance(lend_token_id);
        Ok(Amount::new(issuance, lend_token_id))
    }

    /// Free lending tokens (lend_tokens) of an account, given the underlying
    pub fn free_lend_tokens(asset_id: CurrencyId<T>, account_id: &T::AccountId) -> Result<Amount<T>, DispatchError> {
        let lend_token_id = Self::lend_token_id(asset_id)?;
        let amount = Amount::new(
            orml_tokens::Pallet::<T>::free_balance(lend_token_id, account_id),
            lend_token_id,
        );
        Ok(amount)
    }

    /// Reserved lending tokens (lend_tokens) of an account, given the underlying
    pub fn reserved_lend_tokens(
        asset_id: CurrencyId<T>,
        account_id: &T::AccountId,
    ) -> Result<Amount<T>, DispatchError> {
        let lend_token_id = Self::lend_token_id(asset_id)?;
        let amount = Amount::new(
            orml_tokens::Pallet::<T>::reserved_balance(lend_token_id, account_id),
            lend_token_id,
        );
        Ok(amount)
    }

    // Returns the value of the asset, in the reference currency.
    // Returns `Err` if oracle price not ready or arithmetic error.
    pub fn get_asset_value(asset: &Amount<T>) -> Result<Amount<T>, DispatchError> {
        asset.convert_to(T::ReferenceAssetId::get())
    }

    // Returns a stored Market.
    //
    // Returns `Err` if market does not exist.
    pub fn market(asset_id: CurrencyId<T>) -> Result<Market<BalanceOf<T>>, DispatchError> {
        Markets::<T>::try_get(asset_id).map_err(|_err| Error::<T>::MarketDoesNotExist.into())
    }

    // Mutates a stored Market.
    //
    // Returns `Err` if market does not exist.
    pub(crate) fn mutate_market<F>(asset_id: CurrencyId<T>, cb: F) -> Result<Market<BalanceOf<T>>, DispatchError>
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
    fn active_markets() -> impl Iterator<Item = (CurrencyId<T>, Market<BalanceOf<T>>)> {
        Markets::<T>::iter().filter(|(_, market)| market.state == MarketState::Active)
    }

    // Returns the lend_token_id of the related asset
    //
    // Returns `Err` if market does not exist.
    pub fn lend_token_id(asset_id: CurrencyId<T>) -> Result<CurrencyId<T>, DispatchError> {
        if let Ok(market) = Self::market(asset_id) {
            Ok(market.lend_token_id)
        } else {
            Err(Error::<T>::MarketDoesNotExist.into())
        }
    }

    // Returns the incentive reward account
    pub fn incentive_reward_account_id() -> T::AccountId {
        T::PalletId::get().into_sub_account_truncating(INCENTIVE_SUB_ACCOUNT)
    }
}

impl<T: Config> LoansTrait<CurrencyId<T>, AccountIdOf<T>, Amount<T>> for Pallet<T> {
    fn do_mint(supplier: &AccountIdOf<T>, amount: &Amount<T>) -> Result<(), DispatchError> {
        let asset_id = amount.currency();
        Self::ensure_active_market(asset_id)?;
        Self::ensure_under_supply_cap(&amount)?;

        Self::accrue_interest(asset_id)?;

        // update supply index before modify supply balance.
        Self::update_reward_supply_index(asset_id)?;
        Self::distribute_supplier_reward(asset_id, supplier)?;

        let voucher = amount.to_lend_token()?;
        ensure!(!voucher.is_zero(), Error::<T>::InvalidExchangeRate);

        amount.transfer(supplier, &Self::account_id())?;

        voucher.mint_to(supplier)?;

        Self::deposit_event(Event::<T>::Deposited {
            account_id: supplier.clone(),
            currency_id: asset_id,
            amount: amount.amount(),
        });
        Ok(())
    }

    fn do_borrow(borrower: &AccountIdOf<T>, borrow: &Amount<T>) -> Result<(), DispatchError> {
        let asset_id = borrow.currency();
        Self::ensure_active_market(asset_id)?;

        Self::accrue_interest(asset_id)?;
        Self::borrow_allowed(borrower, &borrow)?;

        // update borrow index after accrue interest.
        Self::update_reward_borrow_index(asset_id)?;
        Self::distribute_borrower_reward(asset_id, borrower)?;

        let account_borrows = Self::current_borrow_balance(borrower, asset_id)?;
        let account_borrows_new = account_borrows.checked_add(borrow)?;
        let total_borrows = Self::total_borrows(asset_id);
        let total_borrows_new = total_borrows.checked_add(&borrow)?;
        AccountBorrows::<T>::insert(
            asset_id,
            borrower,
            BorrowSnapshot {
                principal: account_borrows_new.amount(),
                borrow_index: Self::borrow_index(asset_id),
            },
        );
        TotalBorrows::<T>::insert(asset_id, total_borrows_new.amount());
        borrow.transfer(&Self::account_id(), borrower)?;

        Self::deposit_event(Event::<T>::Borrowed {
            account_id: borrower.clone(),
            currency_id: asset_id,
            amount: borrow.amount(),
        });
        Ok(())
    }

    fn do_deposit_collateral(supplier: &AccountIdOf<T>, lend_token_amount: &Amount<T>) -> Result<(), DispatchError> {
        // If the given asset_id is not a valid lend_token, fetching the underlying will fail
        let underlying_id = Self::underlying_id(lend_token_amount.currency())?;
        Self::ensure_active_market(underlying_id)?;

        // Will fail if supplier has insufficient free tokens
        lend_token_amount.lock_on(supplier)?;

        // Increase the amount of collateral deposited
        let deposit = Self::account_deposits(lend_token_amount.currency(), supplier);
        let new_deposit = deposit.checked_add(&lend_token_amount)?;
        AccountDeposits::<T>::insert(lend_token_amount.currency(), supplier, new_deposit.amount());

        Self::deposit_event(Event::<T>::DepositCollateral {
            account_id: supplier.clone(),
            currency_id: lend_token_amount.currency(),
            amount: lend_token_amount.amount(),
        });
        Ok(())
    }

    fn do_withdraw_collateral(supplier: &AccountIdOf<T>, voucher: &Amount<T>) -> Result<(), DispatchError> {
        // If the given asset_id is not a valid lend_token, fetching the underlying will fail
        let underlying_id = Self::underlying_id(voucher.currency())?;
        Self::ensure_active_market(underlying_id)?;

        let total_collateral_value = Self::total_collateral_value(supplier)?;
        let collateral_amount_value = Self::collateral_amount_value(&voucher)?;
        let total_borrowed_value = Self::total_borrowed_value(supplier)?;
        log::trace!(
            target: "loans::collateral_asset",
            "total_collateral_value: {:?}, collateral_asset_value: {:?}, total_borrowed_value: {:?}",
            total_collateral_value.amount(),
            collateral_amount_value.amount(),
            total_borrowed_value.amount(),
        );

        if total_collateral_value.lt(&total_borrowed_value.checked_add(&collateral_amount_value)?)? {
            return Err(Error::<T>::InsufficientLiquidity.into());
        }

        voucher.unlock_on(supplier)?;

        // Decrease the amount of collateral deposited
        AccountDeposits::<T>::try_mutate_exists(voucher.currency(), supplier, |deposits| -> DispatchResult {
            let d = deposits
                .unwrap_or_default()
                .checked_sub(voucher.amount())
                .ok_or(ArithmeticError::Underflow)?;
            if d.is_zero() {
                // remove deposits storage if zero balance
                *deposits = None;
            } else {
                *deposits = Some(d);
            }
            Ok(())
        })?;

        Self::deposit_event(Event::<T>::WithdrawCollateral {
            account_id: supplier.clone(),
            currency_id: voucher.currency(),
            amount: voucher.amount(),
        });
        Ok(())
    }

    fn do_repay_borrow(borrower: &AccountIdOf<T>, borrow: &Amount<T>) -> Result<(), DispatchError> {
        let asset_id = borrow.currency();
        Self::ensure_active_market(asset_id)?;
        Self::accrue_interest(asset_id)?;
        let account_borrows = Self::current_borrow_balance(borrower, asset_id)?;
        Self::do_repay_borrow_with_amount(borrower, asset_id, &account_borrows, &borrow)?;
        Self::deposit_event(Event::<T>::RepaidBorrow {
            account_id: borrower.clone(),
            currency_id: asset_id,
            amount: borrow.amount(),
        });
        Ok(())
    }

    fn do_redeem(supplier: &AccountIdOf<T>, underlying: &Amount<T>, voucher: &Amount<T>) -> Result<(), DispatchError> {
        let asset_id = underlying.currency();

        // if the receiver has collateral locked
        if !Pallet::<T>::account_deposits(voucher.currency(), &supplier).is_zero() {
            // Withdraw the `lend_tokens` from the borrow collateral, so they are redeemable.
            // This assumes that a user cannot have both `free` and `locked` lend tokens at
            // the same time (for the purposes of lending and borrowing).
            Self::do_withdraw_collateral(&supplier, &voucher)?;
        }

        Self::ensure_active_market(asset_id)?;
        Self::accrue_interest(asset_id)?;

        Self::redeem_allowed(supplier, &voucher)?;

        Self::update_reward_supply_index(asset_id)?;
        Self::distribute_supplier_reward(asset_id, supplier)?;

        // Need to first `lock_on` in order to `burn_from` because:
        // 1) only the `free` lend_tokens are redeemable
        // 2) `burn_from` can only be called on locked tokens.
        voucher.lock_on(supplier)?;
        voucher.burn_from(supplier)?;

        underlying
            .transfer(&Self::account_id(), supplier)
            .map_err(|_| Error::<T>::InsufficientCash)?;

        Self::deposit_event(Event::<T>::Redeemed {
            account_id: supplier.clone(),
            currency_id: asset_id,
            amount: underlying.amount(),
        });
        Ok(())
    }

    // NOTE: used in OracleApi, so don't use oracle calls here or it'll recurse forever
    fn recompute_underlying_amount(lend_tokens: &Amount<T>) -> Result<Amount<T>, DispatchError> {
        // This function could be called externally to this pallet, with interest
        // possibly not having accrued for a few blocks. This would result in using an
        // outdated exchange rate. Call `accrue_interest` to avoid this.
        let underlying_id = Self::underlying_id(lend_tokens.currency())?;
        Self::ensure_active_market(underlying_id)?;
        Self::accrue_interest(underlying_id)?;
        let exchange_rate = Self::exchange_rate_stored(underlying_id)?;
        Ok(lend_tokens.checked_mul(&exchange_rate)?.set_currency(underlying_id))
    }

    // Returns a stored asset_id
    //
    // Returns `Err` if asset_id does not exist, it also means that lend_token_id is invalid.
    fn underlying_id(lend_token_id: CurrencyId<T>) -> Result<CurrencyId<T>, DispatchError> {
        UnderlyingAssetId::<T>::try_get(lend_token_id).map_err(|_err| Error::<T>::InvalidLendTokenId.into())
    }

    // NOTE: used in OracleApi, so don't use oracle calls here or it'll recurse forever
    fn recompute_collateral_amount(underlying: &Amount<T>) -> Result<Amount<T>, DispatchError> {
        // This function could be called externally to this pallet, with interest
        // possibly not having accrued for a few blocks. This would result in using an
        // outdated exchange rate. Call `accrue_interest` to avoid this.
        Self::ensure_active_market(underlying.currency())?;
        Self::accrue_interest(underlying.currency())?;
        let exchange_rate = Self::exchange_rate_stored(underlying.currency())?;

        let lend_token_id = Self::lend_token_id(underlying.currency())?;

        Ok(underlying.checked_div(&exchange_rate)?.set_currency(lend_token_id))
    }
}

impl<T: Config> LoansMarketDataProvider<CurrencyId<T>, BalanceOf<T>> for Pallet<T> {
    fn get_market_info(asset_id: CurrencyId<T>) -> Result<MarketInfo, DispatchError> {
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

    fn get_market_status(asset_id: CurrencyId<T>) -> Result<MarketStatus<BalanceOf<T>>, DispatchError> {
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

    fn get_full_interest_rate(asset_id: CurrencyId<T>) -> Option<Rate> {
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

impl<T: Config> OnExchangeRateChange<CurrencyId<T>> for Pallet<T> {
    fn on_exchange_rate_change(currency_id: &CurrencyId<T>) {
        // todo: propagate error
        if let Ok(lend_token_id) = Pallet::<T>::lend_token_id(*currency_id) {
            T::OnExchangeRateChange::on_exchange_rate_change(&lend_token_id)
        }
    }
}
