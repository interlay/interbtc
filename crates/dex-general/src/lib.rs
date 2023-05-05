// Copyright 2021-2022 Zenlink.
// Licensed under Apache 2.0.

//! # Standard AMM Pallet
//!
//! Based on the Uniswap V2 architecture.
//!
//! ## Overview
//!
//! This pallet provides functionality for:
//!
//! - Creating pools
//! - Bootstrapping pools
//! - Adding / removing liquidity
//! - Swapping currencies

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]
#![feature(array_windows)]

pub use pallet::*;

use codec::{Decode, Encode, FullCodec};
use frame_support::{
    inherent::Vec, pallet_prelude::*, storage::bounded_btree_map::BoundedBTreeMap, traits::Get, PalletId, RuntimeDebug,
};
use orml_traits::MultiCurrency;
use sp_core::U256;
use sp_runtime::traits::{AccountIdConversion, Hash, MaybeSerializeDeserialize, One, StaticLookup, Zero};
use sp_std::{collections::btree_map::BTreeMap, convert::TryInto, fmt::Debug, prelude::*, vec};

mod fee;
mod primitives;
mod rpc;
mod swap;
mod traits;

#[cfg(any(feature = "runtime-benchmarks", test))]
pub mod benchmarking;

mod default_weights;

pub use default_weights::WeightInfo;
pub use primitives::{
    AssetBalance, AssetInfo, BootstrapParameter, PairMetadata, PairStatus,
    PairStatus::{Bootstrap, Disable, Trading},
    DEFAULT_FEE_RATE, FEE_ADJUSTMENT,
};
pub use rpc::PairInfo;
pub use traits::{ExportDexGeneral, GenerateLpAssetId};

#[allow(type_alias_bounds)]
type AccountIdOf<T: Config> = <T as frame_system::Config>::AccountId;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::dispatch::DispatchResult;
    use frame_system::pallet_prelude::*;

    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// The trait control all currencies
        type MultiCurrency: MultiCurrency<AccountIdOf<Self>, CurrencyId = Self::AssetId, Balance = AssetBalance>;
        /// This pallet id.
        #[pallet::constant]
        type PalletId: Get<PalletId>;
        /// The asset type.
        type AssetId: FullCodec
            + Eq
            + PartialEq
            + Ord
            + PartialOrd
            + Copy
            + MaybeSerializeDeserialize
            + AssetInfo
            + Debug
            + scale_info::TypeInfo
            + MaxEncodedLen;
        /// Generate the AssetId for the pair.
        type LpGenerate: GenerateLpAssetId<Self::AssetId>;
        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
        /// The maximum number of swaps allowed in routes
        #[pallet::constant]
        type MaxSwaps: Get<u16>;

        /// The maximum number of rewards that can be stored
        #[pallet::constant]
        type MaxBootstrapRewards: Get<u32>;

        /// The maximum number of limits that can be stored
        #[pallet::constant]
        type MaxBootstrapLimits: Get<u32>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn k_last)]
    /// Refer: https://github.com/Uniswap/uniswap-v2-core/blob/master/contracts/UniswapV2Pair.sol#L88
    /// Last unliquidated protocol fee;
    pub type KLast<T: Config> = StorageMap<_, Twox64Concat, (T::AssetId, T::AssetId), U256, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn fee_meta)]
    pub(super) type FeeMeta<T: Config> = StorageValue<_, (Option<T::AccountId>, u8), ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn lp_pairs)]
    pub type LiquidityPairs<T: Config> =
        StorageMap<_, Blake2_128Concat, (T::AssetId, T::AssetId), Option<T::AssetId>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn pair_status)]
    /// (T::AssetId, T::AssetId) -> PairStatus
    pub type PairStatuses<T: Config> = StorageMap<
        _,
        Twox64Concat,
        (T::AssetId, T::AssetId),
        PairStatus<AssetBalance, T::BlockNumber, T::AccountId>,
        ValueQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn bootstrap_personal_supply)]
    pub type BootstrapPersonalSupply<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        ((T::AssetId, T::AssetId), T::AccountId),
        (AssetBalance, AssetBalance),
        ValueQuery,
    >;

    /// End status of bootstrap
    ///
    /// BootstrapEndStatus: map bootstrap pair => pairStatus
    #[pallet::storage]
    #[pallet::getter(fn bootstrap_end_status)]
    pub type BootstrapEndStatus<T: Config> = StorageMap<
        _,
        Twox64Concat,
        (T::AssetId, T::AssetId),
        PairStatus<AssetBalance, T::BlockNumber, T::AccountId>,
        ValueQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn get_bootstrap_rewards)]
    pub type BootstrapRewards<T: Config> = StorageMap<
        _,
        Twox64Concat,
        (T::AssetId, T::AssetId),
        BoundedBTreeMap<T::AssetId, AssetBalance, T::MaxBootstrapRewards>,
        ValueQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn get_bootstrap_limits)]
    pub type BootstrapLimits<T: Config> = StorageMap<
        _,
        Twox64Concat,
        (T::AssetId, T::AssetId),
        BoundedBTreeMap<T::AssetId, AssetBalance, T::MaxBootstrapLimits>,
        ValueQuery,
    >;

    #[pallet::genesis_config]
    /// Refer: https://github.com/Uniswap/uniswap-v2-core/blob/master/contracts/UniswapV2Pair.sol#L88
    pub struct GenesisConfig<T: Config> {
        /// The receiver of the protocol fee.
        pub fee_receiver: Option<T::AccountId>,
        /// The higher the fee point, the smaller the
        /// cut of the exchange fee taken from LPs.
        pub fee_point: u8,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                fee_receiver: None,
                fee_point: 5,
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            <FeeMeta<T>>::put((&self.fee_receiver, &self.fee_point));
        }
    }

    #[cfg(feature = "std")]
    impl<T: Config> GenesisConfig<T> {
        /// Direct implementation of `GenesisBuild::build_storage`.
        ///
        /// Kept in order not to break dependency.
        pub fn build_storage(&self) -> Result<sp_runtime::Storage, String> {
            <Self as GenesisBuild<T>>::build_storage(self)
        }

        /// Direct implementation of `GenesisBuild::assimilate_storage`.
        ///
        /// Kept in order not to break dependency.
        pub fn assimilate_storage(&self, storage: &mut sp_runtime::Storage) -> Result<(), String> {
            <Self as GenesisBuild<T>>::assimilate_storage(self, storage)
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Swap

        /// Create a trading pair. \[asset_0, asset_1\]
        PairCreated(T::AssetId, T::AssetId),
        /// Add liquidity. \[owner, asset_0, asset_1, add_balance_0, add_balance_1,
        /// mint_balance_lp\]
        LiquidityAdded(
            T::AccountId,
            T::AssetId,
            T::AssetId,
            AssetBalance,
            AssetBalance,
            AssetBalance,
        ),
        /// Remove liquidity. \[owner, recipient, asset_0, asset_1, rm_balance_0, rm_balance_1,
        /// burn_balance_lp\]
        LiquidityRemoved(
            T::AccountId,
            T::AccountId,
            T::AssetId,
            T::AssetId,
            AssetBalance,
            AssetBalance,
            AssetBalance,
        ),
        /// Transact in trading \[owner, recipient, swap_path, balances\]
        AssetSwap(T::AccountId, T::AccountId, Vec<T::AssetId>, Vec<AssetBalance>),

        /// Contribute to bootstrap pair. \[who, asset_0, asset_0_contribute, asset_1_contribute\]
        BootstrapContribute(T::AccountId, T::AssetId, AssetBalance, T::AssetId, AssetBalance),

        /// A bootstrap pair end. \[asset_0, asset_1, asset_0_amount, asset_1_amount,
        /// total_lp_supply]
        BootstrapEnd(T::AssetId, T::AssetId, AssetBalance, AssetBalance, AssetBalance),

        /// Create a bootstrap pair. \[bootstrap_pair_account, asset_0, asset_1,
        /// total_supply_0,total_supply_1, capacity_supply_0,capacity_supply_1, end\]
        BootstrapCreated(
            T::AccountId,
            T::AssetId,
            T::AssetId,
            AssetBalance,
            AssetBalance,
            AssetBalance,
            AssetBalance,
            T::BlockNumber,
        ),

        /// Claim a bootstrap pair. \[bootstrap_pair_account, claimer, receiver, asset_0, asset_1,
        /// asset_0_refund, asset_1_refund, lp_amount\]
        BootstrapClaim(
            T::AccountId,
            T::AccountId,
            T::AccountId,
            T::AssetId,
            T::AssetId,
            AssetBalance,
            AssetBalance,
            AssetBalance,
        ),

        /// Update a bootstrap pair. \[caller, asset_0, asset_1,
        /// total_supply_0,total_supply_1, capacity_supply_0,capacity_supply_1\]
        BootstrapUpdate(
            T::AccountId,
            T::AssetId,
            T::AssetId,
            AssetBalance,
            AssetBalance,
            AssetBalance,
            AssetBalance,
            T::BlockNumber,
        ),

        /// Refund from disable bootstrap pair. \[bootstrap_pair_account, caller, asset_0, asset_1,
        /// asset_0_refund, asset_1_refund\]
        BootstrapRefund(
            T::AccountId,
            T::AccountId,
            T::AssetId,
            T::AssetId,
            AssetBalance,
            AssetBalance,
        ),

        /// Bootstrap distribute some rewards to contributors.
        DistributeReward(T::AssetId, T::AssetId, T::AccountId, Vec<(T::AssetId, AssetBalance)>),

        /// Charge reward into a bootstrap.
        ChargeReward(T::AssetId, T::AssetId, T::AccountId, Vec<(T::AssetId, AssetBalance)>),

        /// Withdraw all reward from a bootstrap.
        WithdrawReward(T::AssetId, T::AssetId, T::AccountId),
    }
    #[pallet::error]
    pub enum Error<T> {
        /// Require the admin who can reset the admin and receiver of the protocol fee.
        RequireProtocolAdmin,
        /// Require the admin candidate who can become new admin after confirm.
        RequireProtocolAdminCandidate,
        /// Invalid fee_rate
        InvalidFeeRate,
        /// Unsupported AssetId.
        UnsupportedAssetType,
        /// Account balance must be greater than or equal to the transfer amount.
        InsufficientAssetBalance,
        /// Account native currency balance must be greater than ExistentialDeposit.
        NativeBalanceTooLow,
        /// Trading pair can't be created.
        DeniedCreatePair,
        /// Trading pair already exists.
        PairAlreadyExists,
        /// Trading pair does not exist.
        PairNotExists,
        /// Asset does not exist.
        AssetNotExists,
        /// Liquidity is not enough.
        InsufficientLiquidity,
        /// Trading pair does have enough.
        InsufficientPairReserve,
        /// Get target amount is less than exception.
        InsufficientTargetAmount,
        /// Sold amount is more than exception.
        ExcessiveSoldAmount,
        /// Can't find pair though trading path.
        InvalidPath,
        /// Incorrect amount range.
        IncorrectAssetAmountRange,
        /// Overflow.
        Overflow,
        /// Transaction block number is larger than the end block number.
        Deadline,
        /// Location given was invalid or unsupported.
        AccountIdBadLocation,
        /// XCM execution failed.
        ExecutionFailed,
        /// Transfer to self by XCM message.
        DeniedTransferToSelf,
        /// Not in registered parachains.
        TargetChainNotRegistered,
        /// Can't pass the K value check
        InvariantCheckFailed,
        /// Created pair can't create now
        PairCreateForbidden,
        /// Pair is not in bootstrap
        NotInBootstrap,
        /// Amount of contribution is invalid.
        InvalidContributionAmount,
        /// Amount of contribution is invalid.
        UnqualifiedBootstrap,
        /// Zero contribute in bootstrap
        ZeroContribute,
        /// Bootstrap deny refund
        DenyRefund,
        /// Bootstrap is disable
        DisableBootstrap,
        /// Not eligible to contribute
        NotQualifiedAccount,
        /// Reward of bootstrap is not set.
        NoRewardTokens,
        /// Charge bootstrap extrinsic args has error,
        ChargeRewardParamsError,
        /// Exist some reward in bootstrap,
        ExistRewardsInBootstrap,
        /// The number of rewards exceeds the storage limit
        TooManyRewards,
        /// The number of limits exceeds the storage limit
        TooManyLimits,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Set the new receiver of the protocol fee.
        ///
        /// # Arguments
        ///
        /// - `send_to`:
        /// (1) Some(receiver): it turn on the protocol fee and the new receiver account.
        /// (2) None: it turn off the protocol fee.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::set_fee_receiver())]
        #[frame_support::transactional]
        pub fn set_fee_receiver(
            origin: OriginFor<T>,
            send_to: Option<<T::Lookup as StaticLookup>::Source>,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let receiver = match send_to {
                Some(r) => {
                    let account = T::Lookup::lookup(r)?;
                    Some(account)
                }
                None => None,
            };

            FeeMeta::<T>::mutate(|fee_meta| fee_meta.0 = receiver);

            Ok(())
        }

        /// Set the protocol fee point.
        ///
        /// # Arguments
        ///
        /// - `fee_point`:
        /// An integer y which satisfies the equation `1/x-1=y`
        /// where x is the percentage of the exchange fee
        /// e.g. 1/(1/6)-1=5, 1/(1/2)-1=1
        /// See section 2.4 of the Uniswap v2 whitepaper
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::set_fee_point())]
        #[frame_support::transactional]
        pub fn set_fee_point(origin: OriginFor<T>, fee_point: u8) -> DispatchResult {
            ensure_root(origin)?;

            FeeMeta::<T>::mutate(|fee_meta| fee_meta.1 = fee_point);

            Ok(())
        }

        /// Set the exchange fee rate.
        ///
        /// # Arguments
        ///
        /// - `asset_0`: Asset which makes up the pair
        /// - `asset_1`: Asset which makes up the pair
        /// - `fee_rate`:
        /// Value denoting the trading fee taken from the amount paid in,
        /// multiplied by the fee adjustment to simplify calculations.
        /// e.g. 0.3% / 100 = 0.003
        ///      0.003 * 10000 = 30
        /// See section 3.2.1 of the Uniswap v2 whitepaper
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::set_fee_point())]
        #[frame_support::transactional]
        pub fn set_exchange_fee(
            origin: OriginFor<T>,
            asset_0: T::AssetId,
            asset_1: T::AssetId,
            fee_rate: u128,
        ) -> DispatchResult {
            ensure_root(origin)?;

            // can't be more than 100%, paths are only valid
            // if the amount is greater than one
            ensure!(fee_rate < FEE_ADJUSTMENT, Error::<T>::InvalidFeeRate);

            let pair = Self::sort_asset_id(asset_0, asset_1);
            PairStatuses::<T>::try_mutate(pair, |status| match status {
                Trading(pair) => {
                    *status = Trading(PairMetadata {
                        pair_account: pair.pair_account.clone(),
                        total_supply: pair.total_supply,
                        fee_rate,
                    });
                    Ok(())
                }
                Bootstrap(_) => Err(Error::<T>::PairAlreadyExists),
                Disable => Err(Error::<T>::PairNotExists),
            })?;

            Ok(())
        }

        /// Create pair by two assets.
        ///
        /// The order of assets does not effect the result.
        ///
        /// # Arguments
        ///
        /// - `asset_0`: Asset which make up Pair
        /// - `asset_1`: Asset which make up Pair
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::create_pair())]
        #[frame_support::transactional]
        pub fn create_pair(
            origin: OriginFor<T>,
            asset_0: T::AssetId,
            asset_1: T::AssetId,
            fee_rate: u128,
        ) -> DispatchResult {
            ensure_root(origin)?;
            ensure!(
                asset_0.is_support() && asset_1.is_support(),
                Error::<T>::UnsupportedAssetType
            );

            ensure!(asset_0 != asset_1, Error::<T>::DeniedCreatePair);
            ensure!(fee_rate < FEE_ADJUSTMENT, Error::<T>::InvalidFeeRate);

            let pair = Self::sort_asset_id(asset_0, asset_1);
            PairStatuses::<T>::try_mutate(pair, |status| match status {
                Trading(_) => Err(Error::<T>::PairAlreadyExists),
                Bootstrap(params) => {
                    if Self::bootstrap_disable(params) {
                        BootstrapEndStatus::<T>::insert(pair, Bootstrap((*params).clone()));

                        *status = Trading(PairMetadata {
                            pair_account: Self::pair_account_id(pair.0, pair.1),
                            total_supply: Zero::zero(),
                            fee_rate,
                        });
                        Ok(())
                    } else {
                        Err(Error::<T>::PairAlreadyExists)
                    }
                }
                Disable => {
                    *status = Trading(PairMetadata {
                        pair_account: Self::pair_account_id(pair.0, pair.1),
                        total_supply: Zero::zero(),
                        fee_rate,
                    });
                    Ok(())
                }
            })?;

            Self::mutate_lp_pairs(asset_0, asset_1)?;

            Self::deposit_event(Event::PairCreated(asset_0, asset_1));
            Ok(())
        }

        /// Provide liquidity to a pair.
        ///
        /// The order of assets does not effect the result.
        ///
        /// # Arguments
        ///
        /// - `asset_0`: Asset which make up pair
        /// - `asset_1`: Asset which make up pair
        /// - `amount_0_desired`: Maximum amount of asset_0 added to the pair
        /// - `amount_1_desired`: Maximum amount of asset_1 added to the pair
        /// - `amount_0_min`: Minimum amount of asset_0 added to the pair
        /// - `amount_1_min`: Minimum amount of asset_1 added to the pair
        /// - `deadline`: Height of the cutoff block of this transaction
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::add_liquidity())]
        #[frame_support::transactional]
        #[allow(clippy::too_many_arguments)]
        pub fn add_liquidity(
            origin: OriginFor<T>,
            asset_0: T::AssetId,
            asset_1: T::AssetId,
            #[pallet::compact] amount_0_desired: AssetBalance,
            #[pallet::compact] amount_1_desired: AssetBalance,
            #[pallet::compact] amount_0_min: AssetBalance,
            #[pallet::compact] amount_1_min: AssetBalance,
            #[pallet::compact] deadline: T::BlockNumber,
        ) -> DispatchResult {
            ensure!(
                asset_0.is_support() && asset_1.is_support(),
                Error::<T>::UnsupportedAssetType
            );
            let who = ensure_signed(origin)?;
            let now = frame_system::Pallet::<T>::block_number();
            ensure!(deadline > now, Error::<T>::Deadline);

            Self::inner_add_liquidity(
                &who,
                asset_0,
                asset_1,
                amount_0_desired,
                amount_1_desired,
                amount_0_min,
                amount_1_min,
            )
        }

        /// Extract liquidity.
        ///
        /// The order of assets does not effect the result.
        ///
        /// # Arguments
        ///
        /// - `asset_0`: Asset which make up pair
        /// - `asset_1`: Asset which make up pair
        /// - `amount_asset_0_min`: Minimum amount of asset_0 to exact
        /// - `amount_asset_1_min`: Minimum amount of asset_1 to exact
        /// - `recipient`: Account that accepts withdrawal of assets
        /// - `deadline`: Height of the cutoff block of this transaction
        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::remove_liquidity())]
        #[frame_support::transactional]
        #[allow(clippy::too_many_arguments)]
        pub fn remove_liquidity(
            origin: OriginFor<T>,
            asset_0: T::AssetId,
            asset_1: T::AssetId,
            #[pallet::compact] liquidity: AssetBalance,
            #[pallet::compact] amount_0_min: AssetBalance,
            #[pallet::compact] amount_1_min: AssetBalance,
            recipient: <T::Lookup as StaticLookup>::Source,
            #[pallet::compact] deadline: T::BlockNumber,
        ) -> DispatchResult {
            ensure!(
                asset_0.is_support() && asset_1.is_support(),
                Error::<T>::UnsupportedAssetType
            );
            let who = ensure_signed(origin)?;
            let recipient = T::Lookup::lookup(recipient)?;
            let now = frame_system::Pallet::<T>::block_number();
            ensure!(deadline > now, Error::<T>::Deadline);

            Self::inner_remove_liquidity(
                &who,
                asset_0,
                asset_1,
                liquidity,
                amount_0_min,
                amount_1_min,
                &recipient,
            )
        }

        /// Sell amount of asset by path.
        ///
        /// # Arguments
        ///
        /// - `amount_in`: Amount of the asset will be sold
        /// - `amount_out_min`: Minimum amount of target asset
        /// - `path`: path can convert to pairs.
        /// - `recipient`: Account that receive the target asset
        /// - `deadline`: Height of the cutoff block of this transaction
        #[pallet::call_index(6)]
        #[pallet::weight(T::WeightInfo::swap_exact_assets_for_assets(path.len() as u32))]
        #[frame_support::transactional]
        pub fn swap_exact_assets_for_assets(
            origin: OriginFor<T>,
            #[pallet::compact] amount_in: AssetBalance,
            #[pallet::compact] amount_out_min: AssetBalance,
            path: Vec<T::AssetId>,
            recipient: <T::Lookup as StaticLookup>::Source,
            #[pallet::compact] deadline: T::BlockNumber,
        ) -> DispatchResult {
            ensure!(path.iter().all(|id| id.is_support()), Error::<T>::UnsupportedAssetType);

            let who = ensure_signed(origin)?;
            let recipient = T::Lookup::lookup(recipient)?;
            let now = frame_system::Pallet::<T>::block_number();
            ensure!(deadline > now, Error::<T>::Deadline);

            Self::inner_swap_exact_assets_for_assets(&who, amount_in, amount_out_min, &path, &recipient)
        }

        /// Buy amount of asset by path.
        ///
        /// # Arguments
        ///
        /// - `amount_out`: Amount of the asset will be bought
        /// - `amount_in_max`: Maximum amount of sold asset
        /// - `path`: path can convert to pairs.
        /// - `recipient`: Account that receive the target asset
        /// - `deadline`: Height of the cutoff block of this transaction
        #[pallet::call_index(7)]
        #[pallet::weight(T::WeightInfo::swap_assets_for_exact_assets(path.len() as u32))]
        #[frame_support::transactional]
        pub fn swap_assets_for_exact_assets(
            origin: OriginFor<T>,
            #[pallet::compact] amount_out: AssetBalance,
            #[pallet::compact] amount_in_max: AssetBalance,
            path: Vec<T::AssetId>,
            recipient: <T::Lookup as StaticLookup>::Source,
            #[pallet::compact] deadline: T::BlockNumber,
        ) -> DispatchResult {
            ensure!(path.iter().all(|id| id.is_support()), Error::<T>::UnsupportedAssetType);

            let who = ensure_signed(origin)?;
            let recipient = T::Lookup::lookup(recipient)?;
            let now = frame_system::Pallet::<T>::block_number();
            ensure!(deadline > now, Error::<T>::Deadline);

            Self::inner_swap_assets_for_exact_assets(&who, amount_out, amount_in_max, &path, &recipient)
        }

        /// Create bootstrap pair
        ///
        /// The order of asset don't affect result.
        ///
        /// # Arguments
        ///
        /// - `asset_0`: Asset which make up bootstrap pair
        /// - `asset_1`: Asset which make up bootstrap pair
        /// - `target_supply_0`: Target amount of asset_0 total contribute
        /// - `target_supply_0`: Target amount of asset_1 total contribute
        /// - `capacity_supply_0`: The max amount of asset_0 total contribute
        /// - `capacity_supply_1`: The max amount of asset_1 total contribute
        /// - `end`: The earliest ending block.
        #[pallet::call_index(8)]
        #[pallet::weight(T::WeightInfo::bootstrap_create(rewards.len() as u32, limits.len() as u32))]
        #[frame_support::transactional]
        #[allow(clippy::too_many_arguments)]
        pub fn bootstrap_create(
            origin: OriginFor<T>,
            asset_0: T::AssetId,
            asset_1: T::AssetId,
            #[pallet::compact] target_supply_0: AssetBalance,
            #[pallet::compact] target_supply_1: AssetBalance,
            #[pallet::compact] capacity_supply_0: AssetBalance,
            #[pallet::compact] capacity_supply_1: AssetBalance,
            #[pallet::compact] end: T::BlockNumber,
            rewards: Vec<T::AssetId>,
            limits: Vec<(T::AssetId, AssetBalance)>,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let pair = Self::sort_asset_id(asset_0, asset_1);

            let (target_supply_0, target_supply_1, capacity_supply_0, capacity_supply_1) = if pair.0 == asset_0 {
                (target_supply_0, target_supply_1, capacity_supply_0, capacity_supply_1)
            } else {
                (target_supply_1, target_supply_0, capacity_supply_1, capacity_supply_0)
            };

            PairStatuses::<T>::try_mutate(pair, |status| match status {
                Trading(_) => Err(Error::<T>::PairAlreadyExists),
                Bootstrap(params) => {
                    if Self::bootstrap_disable(params) {
                        *status = Bootstrap(BootstrapParameter {
                            target_supply: (target_supply_0, target_supply_1),
                            capacity_supply: (capacity_supply_0, capacity_supply_1),
                            accumulated_supply: params.accumulated_supply,
                            end_block_number: end,
                            pair_account: Self::account_id(),
                        });

                        // must no reward before update.
                        let exist_rewards = BootstrapRewards::<T>::get(pair);
                        for (_, exist_reward) in exist_rewards {
                            if exist_reward != Zero::zero() {
                                return Err(Error::<T>::ExistRewardsInBootstrap);
                            }
                        }

                        BootstrapRewards::<T>::insert(
                            pair,
                            BoundedBTreeMap::<T::AssetId, AssetBalance, T::MaxBootstrapRewards>::try_from(
                                rewards
                                    .into_iter()
                                    .map(|asset_id| (asset_id, Zero::zero()))
                                    .collect::<BTreeMap<T::AssetId, AssetBalance>>(),
                            )
                            .map_err(|_| Error::<T>::TooManyRewards)?,
                        );

                        BootstrapLimits::<T>::insert(
                            pair,
                            BoundedBTreeMap::<T::AssetId, AssetBalance, T::MaxBootstrapLimits>::try_from(
                                limits.into_iter().collect::<BTreeMap<T::AssetId, AssetBalance>>(),
                            )
                            .map_err(|_| Error::<T>::TooManyLimits)?,
                        );

                        Ok(())
                    } else {
                        Err(Error::<T>::PairAlreadyExists)
                    }
                }
                Disable => {
                    *status = Bootstrap(BootstrapParameter {
                        target_supply: (target_supply_0, target_supply_1),
                        capacity_supply: (capacity_supply_0, capacity_supply_1),
                        accumulated_supply: (Zero::zero(), Zero::zero()),
                        end_block_number: end,
                        pair_account: Self::account_id(),
                    });

                    BootstrapRewards::<T>::insert(
                        pair,
                        BoundedBTreeMap::<T::AssetId, AssetBalance, T::MaxBootstrapRewards>::try_from(
                            rewards
                                .into_iter()
                                .map(|asset_id| (asset_id, Zero::zero()))
                                .collect::<BTreeMap<T::AssetId, AssetBalance>>(),
                        )
                        .map_err(|_| Error::<T>::TooManyRewards)?,
                    );

                    BootstrapLimits::<T>::insert(
                        pair,
                        BoundedBTreeMap::<T::AssetId, AssetBalance, T::MaxBootstrapLimits>::try_from(
                            limits.into_iter().collect::<BTreeMap<T::AssetId, AssetBalance>>(),
                        )
                        .map_err(|_| Error::<T>::TooManyLimits)?,
                    );

                    Ok(())
                }
            })?;

            Self::deposit_event(Event::BootstrapCreated(
                Self::account_id(),
                pair.0,
                pair.1,
                target_supply_0,
                target_supply_1,
                capacity_supply_1,
                capacity_supply_0,
                end,
            ));
            Ok(())
        }

        /// Contribute some asset to a bootstrap pair
        ///
        /// # Arguments
        ///
        /// - `asset_0`: Asset which make up bootstrap pair
        /// - `asset_1`: Asset which make up bootstrap pair
        /// - `amount_0_contribute`: The amount of asset_0 contribute to this bootstrap pair
        /// - `amount_1_contribute`: The amount of asset_1 contribute to this bootstrap pair
        /// - `deadline`: Height of the cutoff block of this transaction
        #[pallet::call_index(9)]
        #[pallet::weight(T::WeightInfo::bootstrap_contribute())]
        #[frame_support::transactional]
        pub fn bootstrap_contribute(
            who: OriginFor<T>,
            asset_0: T::AssetId,
            asset_1: T::AssetId,
            #[pallet::compact] amount_0_contribute: AssetBalance,
            #[pallet::compact] amount_1_contribute: AssetBalance,
            #[pallet::compact] deadline: T::BlockNumber,
        ) -> DispatchResult {
            let who = ensure_signed(who)?;

            ensure!(
                Self::bootstrap_check_limits(asset_0, asset_1, &who),
                Error::<T>::NotQualifiedAccount
            );

            let now = frame_system::Pallet::<T>::block_number();
            ensure!(deadline > now, Error::<T>::Deadline);

            Self::do_bootstrap_contribute(who, asset_0, asset_1, amount_0_contribute, amount_1_contribute)
        }

        /// Claim lp asset from a bootstrap pair
        ///
        /// # Arguments
        ///
        /// - `asset_0`: Asset which make up bootstrap pair
        /// - `asset_1`: Asset which make up bootstrap pair
        /// - `deadline`: Height of the cutoff block of this transaction
        #[pallet::call_index(10)]
        #[pallet::weight(T::WeightInfo::bootstrap_claim())]
        #[frame_support::transactional]
        pub fn bootstrap_claim(
            origin: OriginFor<T>,
            recipient: <T::Lookup as StaticLookup>::Source,
            asset_0: T::AssetId,
            asset_1: T::AssetId,
            #[pallet::compact] deadline: T::BlockNumber,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let recipient = T::Lookup::lookup(recipient)?;

            let now = frame_system::Pallet::<T>::block_number();
            ensure!(deadline > now, Error::<T>::Deadline);

            Self::do_bootstrap_claim(who, recipient, asset_0, asset_1)
        }

        /// End a bootstrap pair
        ///
        /// # Arguments
        ///
        /// - `asset_0`: Asset which make up bootstrap pair
        /// - `asset_1`: Asset which make up bootstrap pair
        #[pallet::call_index(11)]
        #[pallet::weight(T::WeightInfo::bootstrap_end())]
        #[frame_support::transactional]
        pub fn bootstrap_end(origin: OriginFor<T>, asset_0: T::AssetId, asset_1: T::AssetId) -> DispatchResult {
            ensure_signed(origin)?;
            Self::mutate_lp_pairs(asset_0, asset_1)?;

            Self::do_end_bootstrap(asset_0, asset_1)
        }

        /// update a bootstrap pair
        ///
        /// # Arguments
        ///
        /// - `asset_0`: Asset which make up bootstrap pair
        /// - `asset_1`: Asset which make up bootstrap pair
        /// - `min_contribution_0`: The new min amount of asset_0 contribute
        /// - `min_contribution_0`: The new min amount of asset_1 contribute
        /// - `target_supply_0`: The new target amount of asset_0 total contribute
        /// - `target_supply_0`: The new target amount of asset_1 total contribute
        /// - `capacity_supply_0`: The new max amount of asset_0 total contribute
        /// - `capacity_supply_1`: The new max amount of asset_1 total contribute
        /// - `end`: The earliest ending block.
        #[pallet::call_index(12)]
        #[pallet::weight(T::WeightInfo::bootstrap_update(rewards.len() as u32, limits.len() as u32))]
        #[frame_support::transactional]
        #[allow(clippy::too_many_arguments)]
        pub fn bootstrap_update(
            origin: OriginFor<T>,
            asset_0: T::AssetId,
            asset_1: T::AssetId,
            #[pallet::compact] target_supply_0: AssetBalance,
            #[pallet::compact] target_supply_1: AssetBalance,
            #[pallet::compact] capacity_supply_0: AssetBalance,
            #[pallet::compact] capacity_supply_1: AssetBalance,
            #[pallet::compact] end: T::BlockNumber,
            rewards: Vec<T::AssetId>,
            limits: Vec<(T::AssetId, AssetBalance)>,
        ) -> DispatchResult {
            ensure_root(origin)?;
            let pair = Self::sort_asset_id(asset_0, asset_1);

            let (target_supply_0, target_supply_1, capacity_supply_0, capacity_supply_1) = if pair.0 == asset_0 {
                (target_supply_0, target_supply_1, capacity_supply_0, capacity_supply_1)
            } else {
                (target_supply_1, target_supply_0, capacity_supply_1, capacity_supply_0)
            };

            let pair_account = Self::pair_account_id(asset_0, asset_1);
            PairStatuses::<T>::try_mutate(pair, |status| match status {
                Trading(_) => Err(Error::<T>::PairAlreadyExists),
                Bootstrap(params) => {
                    *status = Bootstrap(BootstrapParameter {
                        target_supply: (target_supply_0, target_supply_1),
                        capacity_supply: (capacity_supply_0, capacity_supply_1),
                        accumulated_supply: params.accumulated_supply,
                        end_block_number: end,
                        pair_account: Self::account_id(),
                    });

                    // must no reward before update.
                    let exist_rewards = BootstrapRewards::<T>::get(pair);
                    for (_, exist_reward) in exist_rewards {
                        if exist_reward != Zero::zero() {
                            return Err(Error::<T>::ExistRewardsInBootstrap);
                        }
                    }

                    BootstrapRewards::<T>::insert(
                        pair,
                        BoundedBTreeMap::<T::AssetId, AssetBalance, T::MaxBootstrapRewards>::try_from(
                            rewards
                                .into_iter()
                                .map(|asset_id| (asset_id, Zero::zero()))
                                .collect::<BTreeMap<T::AssetId, AssetBalance>>(),
                        )
                        .map_err(|_| Error::<T>::TooManyRewards)?,
                    );

                    BootstrapLimits::<T>::insert(
                        pair,
                        BoundedBTreeMap::<T::AssetId, AssetBalance, T::MaxBootstrapLimits>::try_from(
                            limits.into_iter().collect::<BTreeMap<T::AssetId, AssetBalance>>(),
                        )
                        .map_err(|_| Error::<T>::TooManyLimits)?,
                    );

                    Ok(())
                }
                Disable => Err(Error::<T>::NotInBootstrap),
            })?;

            Self::deposit_event(Event::BootstrapUpdate(
                pair_account,
                pair.0,
                pair.1,
                target_supply_0,
                target_supply_1,
                capacity_supply_0,
                capacity_supply_1,
                end,
            ));
            Ok(())
        }

        /// Contributor refund from disable bootstrap pair
        ///
        /// # Arguments
        ///
        /// - `asset_0`: Asset which make up bootstrap pair
        /// - `asset_1`: Asset which make up bootstrap pair
        #[pallet::call_index(13)]
        #[pallet::weight(T::WeightInfo::bootstrap_refund())]
        #[frame_support::transactional]
        pub fn bootstrap_refund(origin: OriginFor<T>, asset_0: T::AssetId, asset_1: T::AssetId) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Self::do_bootstrap_refund(who, asset_0, asset_1)
        }

        #[pallet::call_index(14)]
        #[pallet::weight(T::WeightInfo::bootstrap_charge_reward(charge_rewards.len() as u32))]
        #[frame_support::transactional]
        pub fn bootstrap_charge_reward(
            origin: OriginFor<T>,
            asset_0: T::AssetId,
            asset_1: T::AssetId,
            charge_rewards: Vec<(T::AssetId, AssetBalance)>,
        ) -> DispatchResult {
            let pair = Self::sort_asset_id(asset_0, asset_1);
            let who = ensure_signed(origin)?;

            BootstrapRewards::<T>::try_mutate(pair, |rewards| -> DispatchResult {
                ensure!(
                    rewards.len() == charge_rewards.len(),
                    Error::<T>::ChargeRewardParamsError
                );

                for (asset_id, amount) in &charge_rewards {
                    let already_charge_amount = rewards.get(asset_id).ok_or(Error::<T>::NoRewardTokens)?;

                    T::MultiCurrency::transfer(*asset_id, &who, &Self::account_id(), *amount)?;
                    let new_charge_amount = already_charge_amount.checked_add(*amount).ok_or(Error::<T>::Overflow)?;

                    rewards
                        .try_insert(*asset_id, new_charge_amount)
                        .map_err(|_| Error::<T>::TooManyRewards)?;
                }

                Self::deposit_event(Event::ChargeReward(pair.0, pair.1, who, charge_rewards));

                Ok(())
            })?;

            Ok(())
        }

        #[pallet::call_index(15)]
        #[pallet::weight(T::WeightInfo::bootstrap_withdraw_reward())]
        #[frame_support::transactional]
        pub fn bootstrap_withdraw_reward(
            origin: OriginFor<T>,
            asset_0: T::AssetId,
            asset_1: T::AssetId,
            recipient: <T::Lookup as StaticLookup>::Source,
        ) -> DispatchResult {
            ensure_root(origin)?;
            let pair = Self::sort_asset_id(asset_0, asset_1);
            let recipient = T::Lookup::lookup(recipient)?;

            BootstrapRewards::<T>::try_mutate(pair, |rewards| -> DispatchResult {
                for (asset_id, amount) in rewards {
                    T::MultiCurrency::transfer(*asset_id, &Self::account_id(), &recipient, *amount)?;

                    *amount = Zero::zero();
                }
                Ok(())
            })?;

            Self::deposit_event(Event::WithdrawReward(pair.0, pair.1, recipient));

            Ok(())
        }
    }
}
