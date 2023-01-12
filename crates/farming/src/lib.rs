//! # Farming Module
//! Root can create reward schedules which payout incentives
//! on a per period basis. Users can stake LP tokens, such as
//! those generated from an AMM or lending protocol to receive
//! these rewards by claiming.

// #![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

mod default_weights;
pub use default_weights::WeightInfo;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{dispatch::DispatchResult, traits::Get, transactional, weights::Weight, PalletId, RuntimeDebug};
use orml_traits::{MultiCurrency, MultiReservableCurrency};
use primitives::CurrencyId;
use reward::RewardsApi;
use scale_info::TypeInfo;
use sp_runtime::{
    traits::{AccountIdConversion, AtLeast32Bit, Saturating, Zero},
    ArithmeticError,
};

pub use pallet::*;

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

pub type CurrencyIdOf<T> =
    <<T as Config>::MultiCurrency as MultiCurrency<<T as frame_system::Config>::AccountId>>::CurrencyId;

type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;

pub(crate) type RewardScheduleOf<T> = RewardSchedule<<T as frame_system::Config>::BlockNumber, BalanceOf<T>>;

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct RewardSchedule<BlockNumber, Balance: MaxEncodedLen> {
    /// Block height to start after
    pub start_height: BlockNumber,
    /// Number of blocks between distribution
    pub period: BlockNumber,
    /// Number of periods remaining
    pub period_count: u32,
    /// Amount of tokens to release
    #[codec(compact)]
    pub per_period: Balance,
}

impl<BlockNumber: AtLeast32Bit + Copy, Balance: AtLeast32Bit + MaxEncodedLen + Copy>
    RewardSchedule<BlockNumber, Balance>
{
    /// Returns total amount to distribute, `None` if calculation overflows
    pub fn total(&self) -> Option<Balance> {
        self.per_period.checked_mul(&self.period_count.into())
    }

    /// Returns true if the schedule is ready
    pub fn is_ready(&self, now: BlockNumber) -> bool {
        now.ge(&self.start_height) && (now % self.period).is_zero()
    }

    /// Take the next reward and decrement the period count
    pub fn take(&mut self) -> Option<Balance> {
        if self.period_count.gt(&0) {
            self.period_count.saturating_dec();
            Some(self.per_period)
        } else {
            None
        }
    }
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::{ensure_root, ensure_signed, pallet_prelude::*};

    /// ## Configuration
    /// The pallet's configuration trait.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// The farming pallet id, used for deriving pool accounts.
        #[pallet::constant]
        type FarmingPalletId: Get<PalletId>;

        /// The treasury pallet id, used for deriving its sovereign account ID.
        #[pallet::constant]
        type TreasuryPalletId: Get<PalletId>;

        /// Currency handler to transfer tokens
        type MultiCurrency: MultiReservableCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;

        /// Reward pools to track stake
        type RewardPools: RewardsApi<
            CurrencyIdOf<Self>, // pool id is the lp token
            AccountIdOf<Self>,
            BalanceOf<Self>,
            CurrencyId = CurrencyIdOf<Self>,
        >;

        /// Weight information for the extrinsics.
        type WeightInfo: WeightInfo;
    }

    // The pallet's events
    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        RewardScheduleUpdated {
            pool_id: CurrencyIdOf<T>,
            reward_schedule: RewardScheduleOf<T>,
        },
        RewardScheduleRemoved {
            pool_id: CurrencyIdOf<T>,
        },
        RewardClaimed {
            account_id: AccountIdOf<T>,
            pool_id: CurrencyIdOf<T>,
            amount: BalanceOf<T>,
        },
        RewardDistributed {
            pool_id: CurrencyIdOf<T>,
            currency_id: CurrencyIdOf<T>,
            amount: BalanceOf<T>,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        ScheduleNotFound,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
        fn on_initialize(n: T::BlockNumber) -> Weight {
            if let Err(e) = Self::begin_block(n) {
                sp_runtime::print(e);
            }
            Weight::from_ref_time(0 as u64)
        }
    }

    #[pallet::storage]
    #[pallet::getter(fn reward_schedules)]
    pub type RewardSchedules<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        CurrencyIdOf<T>, // lp token
        Blake2_128Concat,
        CurrencyIdOf<T>, // reward currency
        RewardScheduleOf<T>,
        OptionQuery,
    >;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // The pallet's dispatchable functions.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Create or overwrite the reward schedule, if a reward schedule
        /// already exists for the rewards currency it will first distribute
        /// any remaining tokens to the rewards pool
        #[pallet::weight(T::WeightInfo::update_reward_schedule())]
        #[transactional]
        pub fn update_reward_schedule(
            origin: OriginFor<T>,
            pool_id: CurrencyIdOf<T>,
            currency_id: CurrencyIdOf<T>,
            reward_schedule: RewardScheduleOf<T>,
        ) -> DispatchResult {
            ensure_root(origin)?;

            // fund the pool account from treasury
            let treasury_account_id = Self::treasury_account_id();
            let pool_account_id = Self::pool_account_id(&pool_id);
            T::MultiCurrency::transfer(
                currency_id,
                &treasury_account_id,
                &pool_account_id,
                reward_schedule.total().ok_or(ArithmeticError::Overflow)?,
            )?;

            // distribute remaining balance from existing schedule
            if let Ok(previous_schedule) = RewardSchedules::<T>::try_get(pool_id, currency_id) {
                let amount = previous_schedule.total().ok_or(ArithmeticError::Overflow)?;
                if let Ok(_) = T::RewardPools::distribute_reward(&pool_id, currency_id, amount) {
                    // NOTE: if this fails, the total will not reflect the pool balance
                    // maybe we should fail or send to treasury instead?
                    Self::deposit_event(Event::RewardDistributed {
                        pool_id,
                        currency_id,
                        amount,
                    });
                }
            }

            // overwrite new schedule
            RewardSchedules::<T>::insert(pool_id, currency_id, reward_schedule.clone());
            Self::deposit_event(Event::RewardScheduleUpdated {
                pool_id,
                reward_schedule,
            });
            Ok(().into())
        }

        /// Explicitly remove a reward schedule and transfer any remaining
        /// balance to the treasury
        #[pallet::weight(T::WeightInfo::remove_reward_schedule())]
        #[transactional]
        pub fn remove_reward_schedule(
            origin: OriginFor<T>,
            pool_id: CurrencyIdOf<T>,
            currency_id: CurrencyIdOf<T>,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;

            // transfer unspent rewards to treasury
            let treasury_account_id = Self::treasury_account_id();
            let pool_account_id = Self::pool_account_id(&pool_id);
            T::MultiCurrency::transfer(
                currency_id,
                &pool_account_id,
                &treasury_account_id,
                T::MultiCurrency::total_balance(currency_id, &pool_account_id),
            )?;

            RewardSchedules::<T>::remove(pool_id, currency_id);
            Self::deposit_event(Event::RewardScheduleRemoved { pool_id });

            Ok(().into())
        }

        /// Stake the pool tokens in the reward pool
        #[pallet::weight(T::WeightInfo::deposit())]
        #[transactional]
        pub fn deposit(origin: OriginFor<T>, pool_id: CurrencyIdOf<T>, amount: BalanceOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // prevent depositing without reward schedule
            ensure!(
                !RewardSchedules::<T>::iter_prefix_values(pool_id).count().is_zero(),
                Error::<T>::ScheduleNotFound
            );

            // reserve lp tokens to prevent spending
            T::MultiCurrency::reserve(pool_id.clone(), &who, amount)?;

            // deposit lp tokens as stake
            T::RewardPools::deposit_stake(&pool_id, &who, amount)
        }

        /// Unstake the pool tokens from the reward pool
        #[pallet::weight(T::WeightInfo::withdraw())]
        #[transactional]
        pub fn withdraw(origin: OriginFor<T>, pool_id: CurrencyIdOf<T>, amount: BalanceOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // unreserve lp tokens to allow spending
            let _remaining = T::MultiCurrency::unreserve(pool_id.clone(), &who, amount);
            // TODO: check remaining is non-zeo

            // withdraw lp tokens from stake
            T::RewardPools::withdraw_stake(&pool_id, &who, amount)
        }

        /// Withdraw any accrued rewards from the reward pool
        #[pallet::weight(T::WeightInfo::claim())]
        #[transactional]
        pub fn claim(origin: OriginFor<T>, pool_id: CurrencyIdOf<T>, currency_id: CurrencyIdOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let pool_account_id = Self::pool_account_id(&pool_id);

            // get reward from staking pool
            let reward = T::RewardPools::withdraw_reward(&pool_id, &who, currency_id)?;
            // transfer from pool to user
            T::MultiCurrency::transfer(currency_id, &pool_account_id, &who, reward)?;

            Self::deposit_event(Event::RewardClaimed {
                account_id: who,
                pool_id,
                amount: reward,
            });

            Ok(())
        }
    }
}

// "Internal" functions, callable by code.
impl<T: Config> Pallet<T> {
    pub fn pool_account_id(pool_id: &CurrencyIdOf<T>) -> T::AccountId {
        T::FarmingPalletId::get().into_sub_account_truncating(pool_id)
    }

    pub fn treasury_account_id() -> T::AccountId {
        T::TreasuryPalletId::get().into_account_truncating()
    }

    pub(crate) fn begin_block(height: T::BlockNumber) -> DispatchResult {
        // TODO: measure weights, can we bound this somehow?
        let schedules = RewardSchedules::<T>::iter().collect::<Vec<_>>();
        // collect first to avoid modifying in-place
        schedules
            .into_iter()
            .filter(|(_, _, schedule)| schedule.is_ready(height))
            .for_each(|(pool_id, currency_id, mut reward_schedule)| {
                if let Some(amount) = reward_schedule.take() {
                    if let Ok(_) = T::RewardPools::distribute_reward(&pool_id, currency_id, amount) {
                        // only update the schedule if we could distribute the reward
                        RewardSchedules::<T>::insert(pool_id, currency_id, reward_schedule);
                        Self::deposit_event(Event::RewardDistributed {
                            pool_id,
                            currency_id,
                            amount,
                        });
                    }
                } else {
                    // period count is zero
                    RewardSchedules::<T>::remove(pool_id, currency_id);
                    // TODO: sweep leftover rewards
                }
            });
        Ok(())
    }
}
