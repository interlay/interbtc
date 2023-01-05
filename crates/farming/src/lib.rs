//! # Farming Module
//! Distributes rewards to LPs.

#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

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
    traits::{AccountIdConversion, AtLeast32Bit, Saturating},
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
    /// Minimum stake before rewards
    pub minimum_stake: Balance,
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

    /// Returns true if the schedule is valid
    pub fn is_ready(&self, now: BlockNumber, total_stake: Balance) -> bool {
        now.ge(&self.start_height) && (now % self.period).is_zero() && total_stake.ge(&self.minimum_stake)
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
        /// The farming pallet id, used for deriving pool accounts.
        #[pallet::constant]
        type FarmingPalletId: Get<PalletId>;

        /// The treasury pallet id, used for deriving its sovereign account ID.
        #[pallet::constant]
        type TreasuryPalletId: Get<PalletId>;

        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        type MultiCurrency: MultiReservableCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;

        type LpRewards: RewardsApi<
            CurrencyIdOf<Self>, // pool id is the lp token
            AccountIdOf<Self>,
            BalanceOf<Self>,
            CurrencyId = CurrencyIdOf<Self>,
        >;

        /// The maximum reward schedules
        type MaxRewardSchedules: Get<u32>;
    }

    // The pallet's events
    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        RewardScheduleAdded {
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
        InsufficientFunds,
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
        #[pallet::weight(0)]
        #[transactional]
        pub fn create_reward_schedule(
            origin: OriginFor<T>,
            pool_id: CurrencyIdOf<T>,
            currency_id: CurrencyIdOf<T>,
            reward_schedule: RewardScheduleOf<T>,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let treasury_account_id = Self::treasury_account_id();
            // TODO: do we want to generate a new pool id?
            let pool_account_id = Self::pool_account_id(&pool_id);

            T::MultiCurrency::transfer(
                currency_id,
                &treasury_account_id,
                &pool_account_id,
                reward_schedule.total().ok_or(ArithmeticError::Overflow)?,
            )?;

            RewardSchedules::<T>::insert(pool_id, currency_id, reward_schedule.clone());

            Self::deposit_event(Event::RewardScheduleAdded {
                pool_id,
                reward_schedule,
            });
            Ok(().into())
        }

        #[pallet::weight(0)]
        #[transactional]
        pub fn remove_reward_schedule(
            origin: OriginFor<T>,
            pool_id: CurrencyIdOf<T>,
            currency_id: CurrencyIdOf<T>,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;

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

        #[pallet::weight(0)]
        pub fn deposit(origin: OriginFor<T>, pool_id: CurrencyIdOf<T>, amount: BalanceOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // reserve lp tokens to prevent spending
            T::MultiCurrency::reserve(pool_id.clone(), &who, amount)?;

            // deposit lp tokens as stake
            T::LpRewards::deposit_stake(&pool_id, &who, amount)
        }

        #[pallet::weight(0)]
        pub fn withdraw(origin: OriginFor<T>, pool_id: CurrencyIdOf<T>, amount: BalanceOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // unreserve lp tokens to allow spending
            let _remaining = T::MultiCurrency::unreserve(pool_id.clone(), &who, amount);
            // TODO: check remaining is non-zeo

            // withdraw lp tokens from stake
            T::LpRewards::withdraw_stake(&pool_id, &who, amount)
        }

        #[pallet::weight(0)]
        pub fn claim(origin: OriginFor<T>, pool_id: CurrencyIdOf<T>, currency_id: CurrencyIdOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let pool_account_id = Self::pool_account_id(&pool_id);

            // get reward from staking pool
            let reward = T::LpRewards::withdraw_reward(&pool_id, &who, currency_id)?;
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
        schedules
            .into_iter()
            .filter(|(pool_id, _, schedule)| {
                schedule.is_ready(height, T::LpRewards::get_total_stake(&pool_id).unwrap_or_default())
            })
            .for_each(|(pool_id, currency_id, mut reward_schedule)| {
                if let Some(amount) = reward_schedule.take() {
                    if let Ok(_) = T::LpRewards::distribute_reward(&pool_id, currency_id, amount) {
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
                }
            });
        Ok(())
    }
}
