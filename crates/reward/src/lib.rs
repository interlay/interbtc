//! # Reward Module
//! Based on the [Scalable Reward Distribution](https://solmaz.io/2019/02/24/scalable-reward-changing/) algorithm.

#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

pub mod migration;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use codec::{Decode, Encode, EncodeLike};
use frame_support::{
    dispatch::{DispatchError, DispatchResult},
    ensure,
    traits::Get,
};
use primitives::{BalanceToFixedPoint, TruncateFixedPointToInt};
use scale_info::TypeInfo;
use sp_arithmetic::FixedPointNumber;
use sp_runtime::{
    traits::{CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, MaybeSerializeDeserialize, Saturating, Zero},
    ArithmeticError,
};
use sp_std::{cmp::PartialOrd, collections::btree_set::BTreeSet, convert::TryInto, fmt::Debug};

pub(crate) type SignedFixedPoint<T, I = ()> = <T as Config<I>>::SignedFixedPoint;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;

    /// ## Configuration
    /// The pallet's configuration trait.
    #[pallet::config]
    pub trait Config<I: 'static = ()>: frame_system::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self, I>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Signed fixed point type.
        type SignedFixedPoint: FixedPointNumber + TruncateFixedPointToInt + Encode + EncodeLike + Decode + TypeInfo;

        /// The pool identifier type.
        type PoolId: Parameter + Member + MaybeSerializeDeserialize + Debug + MaxEncodedLen;

        /// The stake identifier type.
        type StakeId: Parameter + Member + MaybeSerializeDeserialize + Debug + MaxEncodedLen;

        /// The currency ID type.
        type CurrencyId: Parameter + Member + Copy + MaybeSerializeDeserialize + Ord + MaxEncodedLen;

        #[pallet::constant]
        type GetNativeCurrencyId: Get<Self::CurrencyId>;

        #[pallet::constant]
        type GetWrappedCurrencyId: Get<Self::CurrencyId>;
    }

    // The pallet's events
    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config<I>, I: 'static = ()> {
        DepositStake {
            pool_id: T::PoolId,
            stake_id: T::StakeId,
            amount: T::SignedFixedPoint,
        },
        DistributeReward {
            currency_id: T::CurrencyId,
            amount: T::SignedFixedPoint,
        },
        WithdrawStake {
            pool_id: T::PoolId,
            stake_id: T::StakeId,
            amount: T::SignedFixedPoint,
        },
        WithdrawReward {
            pool_id: T::PoolId,
            stake_id: T::StakeId,
            currency_id: T::CurrencyId,
            amount: T::SignedFixedPoint,
        },
    }

    #[pallet::error]
    pub enum Error<T, I = ()> {
        /// Unable to convert value.
        TryIntoIntError,
        /// Balance not sufficient to withdraw stake.
        InsufficientFunds,
        /// Cannot distribute rewards without stake.
        ZeroTotalStake,
    }

    #[pallet::hooks]
    impl<T: Config<I>, I: 'static> Hooks<T::BlockNumber> for Pallet<T, I> {
        fn on_runtime_upgrade() -> Weight {
            RewardCurrencies::<T, I>::mutate(|reward_currencies| {
                reward_currencies.insert(T::GetNativeCurrencyId::get());
                reward_currencies.insert(T::GetWrappedCurrencyId::get());
            });
            Weight::zero()
        }
    }

    /// The total stake deposited to this reward pool.
    #[pallet::storage]
    #[pallet::getter(fn total_stake)]
    pub type TotalStake<T: Config<I>, I: 'static = ()> =
        StorageMap<_, Blake2_128Concat, T::PoolId, SignedFixedPoint<T, I>, ValueQuery>;

    /// The total unclaimed rewards distributed to this reward pool.
    /// NOTE: this is currently only used for integration tests.
    #[pallet::storage]
    #[pallet::getter(fn total_rewards)]
    pub type TotalRewards<T: Config<I>, I: 'static = ()> =
        StorageMap<_, Blake2_128Concat, T::CurrencyId, SignedFixedPoint<T, I>, ValueQuery>;

    /// Used to compute the rewards for a participant's stake.
    #[pallet::storage]
    #[pallet::getter(fn reward_per_token)]
    pub type RewardPerToken<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::CurrencyId,
        Blake2_128Concat,
        T::PoolId,
        SignedFixedPoint<T, I>,
        ValueQuery,
    >;

    /// The stake of a participant in this reward pool.
    #[pallet::storage]
    pub type Stake<T: Config<I>, I: 'static = ()> =
        StorageMap<_, Blake2_128Concat, (T::PoolId, T::StakeId), SignedFixedPoint<T, I>, ValueQuery>;

    /// Accounts for previous changes in stake size.
    #[pallet::storage]
    pub type RewardTally<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::CurrencyId,
        Blake2_128Concat,
        (T::PoolId, T::StakeId),
        SignedFixedPoint<T, I>,
        ValueQuery,
    >;

    /// Track the currencies used for rewards.
    #[pallet::storage]
    pub type RewardCurrencies<T: Config<I>, I: 'static = ()> = StorageValue<_, BTreeSet<T::CurrencyId>, ValueQuery>;

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T, I = ()>(_);

    // The pallet's dispatchable functions.
    #[pallet::call]
    impl<T: Config<I>, I: 'static> Pallet<T, I> {}
}

#[macro_export]
macro_rules! checked_add_mut {
    ($storage:ty, $amount:expr) => {
        <$storage>::mutate(|value| {
            *value = value.checked_add($amount).ok_or(ArithmeticError::Overflow)?;
            Ok::<_, DispatchError>(())
        })?;
    };
    ($storage:ty, $currency:expr, $amount:expr) => {
        <$storage>::mutate($currency, |value| {
            *value = value.checked_add($amount).ok_or(ArithmeticError::Overflow)?;
            Ok::<_, DispatchError>(())
        })?;
    };
    ($storage:ty, $currency:expr, $account:expr, $amount:expr) => {
        <$storage>::mutate($currency, $account, |value| {
            *value = value.checked_add($amount).ok_or(ArithmeticError::Overflow)?;
            Ok::<_, DispatchError>(())
        })?;
    };
}

macro_rules! checked_sub_mut {
    ($storage:ty, $amount:expr) => {
        <$storage>::mutate(|value| {
            *value = value.checked_sub($amount).ok_or(ArithmeticError::Underflow)?;
            Ok::<_, DispatchError>(())
        })?;
    };
    ($storage:ty, $currency:expr, $amount:expr) => {
        <$storage>::mutate($currency, |value| {
            *value = value.checked_sub($amount).ok_or(ArithmeticError::Underflow)?;
            Ok::<_, DispatchError>(())
        })?;
    };
    ($storage:ty, $currency:expr, $account:expr, $amount:expr) => {
        <$storage>::mutate($currency, $account, |value| {
            *value = value.checked_sub($amount).ok_or(ArithmeticError::Underflow)?;
            Ok::<_, DispatchError>(())
        })?;
    };
}

// "Internal" functions, callable by code.
impl<T: Config<I>, I: 'static> Pallet<T, I> {
    // TODO: remove this after the migration, I have added
    // this because it's not clear whether the capacity migration
    // will run before or after the pallet hook and we need to make
    // sure these are included for any deposits / withdrawals
    fn add_default_currencies() {
        let reward_currencies = RewardCurrencies::<T, I>::get();
        if reward_currencies.contains(&T::GetNativeCurrencyId::get())
            && reward_currencies.contains(&T::GetWrappedCurrencyId::get())
        {
            return;
        }
        RewardCurrencies::<T, I>::mutate(|reward_currencies| {
            reward_currencies.insert(T::GetNativeCurrencyId::get());
            reward_currencies.insert(T::GetWrappedCurrencyId::get());
        });
    }

    pub fn stake(pool_id: &T::PoolId, stake_id: &T::StakeId) -> SignedFixedPoint<T, I> {
        Stake::<T, I>::get((pool_id, stake_id))
    }

    pub fn get_total_rewards(
        currency_id: T::CurrencyId,
    ) -> Result<<T::SignedFixedPoint as FixedPointNumber>::Inner, DispatchError> {
        Ok(Self::total_rewards(currency_id)
            .truncate_to_inner()
            .ok_or(Error::<T, I>::TryIntoIntError)?)
    }

    pub fn deposit_stake(
        pool_id: &T::PoolId,
        stake_id: &T::StakeId,
        amount: SignedFixedPoint<T, I>,
    ) -> Result<(), DispatchError> {
        checked_add_mut!(Stake<T, I>, (pool_id, stake_id), &amount);
        checked_add_mut!(TotalStake<T, I>, pool_id, &amount);

        Self::add_default_currencies();
        for currency_id in RewardCurrencies::<T, I>::get() {
            <RewardTally<T, I>>::mutate(currency_id, (pool_id, stake_id), |reward_tally| {
                let reward_per_token = Self::reward_per_token(currency_id, pool_id);
                let reward_per_token_mul_amount =
                    reward_per_token.checked_mul(&amount).ok_or(ArithmeticError::Overflow)?;
                *reward_tally = reward_tally
                    .checked_add(&reward_per_token_mul_amount)
                    .ok_or(ArithmeticError::Overflow)?;
                Ok::<_, DispatchError>(())
            })?;
        }

        Self::deposit_event(Event::<T, I>::DepositStake {
            pool_id: pool_id.clone(),
            stake_id: stake_id.clone(),
            amount,
        });

        Ok(())
    }

    pub fn distribute_reward(
        pool_id: &T::PoolId,
        currency_id: T::CurrencyId,
        reward: SignedFixedPoint<T, I>,
    ) -> DispatchResult {
        if reward.is_zero() {
            return Ok(());
        }
        let total_stake = Self::total_stake(pool_id);
        ensure!(!total_stake.is_zero(), Error::<T, I>::ZeroTotalStake);

        // track currency for future deposits / withdrawals
        RewardCurrencies::<T, I>::mutate(|reward_currencies| {
            reward_currencies.insert(currency_id);
        });

        let reward_div_total_stake = reward.checked_div(&total_stake).ok_or(ArithmeticError::Underflow)?;
        checked_add_mut!(RewardPerToken<T, I>, currency_id, pool_id, &reward_div_total_stake);
        checked_add_mut!(TotalRewards<T, I>, currency_id, &reward);

        Self::deposit_event(Event::<T, I>::DistributeReward {
            currency_id,
            amount: reward,
        });
        Ok(())
    }

    pub fn compute_reward(
        pool_id: &T::PoolId,
        stake_id: &T::StakeId,
        currency_id: T::CurrencyId,
    ) -> Result<<SignedFixedPoint<T, I> as FixedPointNumber>::Inner, DispatchError> {
        let stake = Self::stake(pool_id, stake_id);
        let reward_per_token = Self::reward_per_token(currency_id, pool_id);
        // FIXME: this can easily overflow with large numbers
        let stake_mul_reward_per_token = stake.checked_mul(&reward_per_token).ok_or(ArithmeticError::Overflow)?;
        let reward_tally = <RewardTally<T, I>>::get(currency_id, (pool_id, stake_id));
        // TODO: this can probably be saturated
        let reward = stake_mul_reward_per_token
            .checked_sub(&reward_tally)
            .ok_or(ArithmeticError::Underflow)?
            .truncate_to_inner()
            .ok_or(Error::<T, I>::TryIntoIntError)?;
        Ok(reward)
    }

    pub fn withdraw_stake(
        pool_id: &T::PoolId,
        stake_id: &T::StakeId,
        amount: SignedFixedPoint<T, I>,
    ) -> Result<(), DispatchError> {
        if amount > Self::stake(pool_id, stake_id) {
            return Err(Error::<T, I>::InsufficientFunds.into());
        }

        checked_sub_mut!(Stake<T, I>, (pool_id, stake_id), &amount);
        checked_sub_mut!(TotalStake<T, I>, pool_id, &amount);

        Self::add_default_currencies();
        for currency_id in RewardCurrencies::<T, I>::get() {
            <RewardTally<T, I>>::mutate(currency_id, (pool_id, stake_id), |reward_tally| {
                let reward_per_token = Self::reward_per_token(currency_id, pool_id);
                let reward_per_token_mul_amount =
                    reward_per_token.checked_mul(&amount).ok_or(ArithmeticError::Overflow)?;

                *reward_tally = reward_tally
                    .checked_sub(&reward_per_token_mul_amount)
                    .ok_or(ArithmeticError::Underflow)?;
                Ok::<_, DispatchError>(())
            })?;
        }

        Self::deposit_event(Event::<T, I>::WithdrawStake {
            pool_id: pool_id.clone(),
            stake_id: stake_id.clone(),
            amount,
        });
        Ok(())
    }

    pub fn withdraw_reward(
        pool_id: &T::PoolId,
        stake_id: &T::StakeId,
        currency_id: T::CurrencyId,
    ) -> Result<<SignedFixedPoint<T, I> as FixedPointNumber>::Inner, DispatchError> {
        let reward = Self::compute_reward(pool_id, stake_id, currency_id)?;
        let reward_as_fixed =
            SignedFixedPoint::<T, I>::checked_from_integer(reward).ok_or(Error::<T, I>::TryIntoIntError)?;
        checked_sub_mut!(TotalRewards<T, I>, currency_id, &reward_as_fixed);

        let stake = Self::stake(pool_id, stake_id);
        let reward_per_token = Self::reward_per_token(currency_id, pool_id);
        <RewardTally<T, I>>::insert(
            currency_id,
            (pool_id, stake_id),
            stake.checked_mul(&reward_per_token).ok_or(ArithmeticError::Overflow)?,
        );

        Self::deposit_event(Event::<T, I>::WithdrawReward {
            currency_id,
            pool_id: pool_id.clone(),
            stake_id: stake_id.clone(),
            amount: reward_as_fixed,
        });
        Ok(reward)
    }
}

pub trait RewardsApi<PoolId, StakeId, Balance>
where
    Balance: Saturating + PartialOrd,
{
    type CurrencyId;

    /// Distribute the `amount` to all participants OR error if zero total stake.
    fn distribute_reward(pool_id: &PoolId, currency_id: Self::CurrencyId, amount: Balance) -> DispatchResult;

    /// Compute the expected reward for the `stake_id`.
    fn compute_reward(
        pool_id: &PoolId,
        stake_id: &StakeId,
        currency_id: Self::CurrencyId,
    ) -> Result<Balance, DispatchError>;

    /// Withdraw all rewards from the `stake_id`.
    fn withdraw_reward(
        pool_id: &PoolId,
        stake_id: &StakeId,
        currency_id: Self::CurrencyId,
    ) -> Result<Balance, DispatchError>;

    /// Deposit stake for an account.
    fn deposit_stake(pool_id: &PoolId, stake_id: &StakeId, amount: Balance) -> DispatchResult;

    /// Withdraw stake for an account.
    fn withdraw_stake(pool_id: &PoolId, stake_id: &StakeId, amount: Balance) -> DispatchResult;

    /// Withdraw all stake for an account.
    fn withdraw_all_stake(pool_id: &PoolId, stake_id: &StakeId) -> DispatchResult {
        Self::withdraw_stake(pool_id, stake_id, Self::get_stake(pool_id, stake_id)?)
    }

    /// Return the stake associated with the `pool_id`.
    fn get_total_stake(pool_id: &PoolId) -> Result<Balance, DispatchError>;

    /// Return the stake associated with the `stake_id`.
    fn get_stake(pool_id: &PoolId, stake_id: &StakeId) -> Result<Balance, DispatchError>;

    /// Set the stake to `amount` for `stake_id` regardless of its current stake.
    fn set_stake(pool_id: &PoolId, stake_id: &StakeId, amount: Balance) -> DispatchResult {
        let current_stake = Self::get_stake(pool_id, stake_id)?;
        if current_stake < amount {
            let additional_stake = amount.saturating_sub(current_stake);
            Self::deposit_stake(pool_id, stake_id, additional_stake)
        } else if current_stake > amount {
            let surplus_stake = current_stake.saturating_sub(amount);
            Self::withdraw_stake(pool_id, stake_id, surplus_stake)
        } else {
            Ok(())
        }
    }
}

impl<T, I, Balance> RewardsApi<T::PoolId, T::StakeId, Balance> for Pallet<T, I>
where
    T: Config<I>,
    I: 'static,
    Balance: BalanceToFixedPoint<SignedFixedPoint<T, I>> + Saturating + PartialOrd,
    <T::SignedFixedPoint as FixedPointNumber>::Inner: TryInto<Balance>,
{
    type CurrencyId = T::CurrencyId;

    fn distribute_reward(pool_id: &T::PoolId, currency_id: T::CurrencyId, amount: Balance) -> DispatchResult {
        Pallet::<T, I>::distribute_reward(
            pool_id,
            currency_id,
            amount.to_fixed().ok_or(Error::<T, I>::TryIntoIntError)?,
        )
    }

    fn compute_reward(
        pool_id: &T::PoolId,
        stake_id: &T::StakeId,
        currency_id: T::CurrencyId,
    ) -> Result<Balance, DispatchError> {
        Pallet::<T, I>::compute_reward(pool_id, stake_id, currency_id)?
            .try_into()
            .map_err(|_| Error::<T, I>::TryIntoIntError.into())
    }

    fn withdraw_reward(
        pool_id: &T::PoolId,
        stake_id: &T::StakeId,
        currency_id: T::CurrencyId,
    ) -> Result<Balance, DispatchError> {
        Pallet::<T, I>::withdraw_reward(pool_id, stake_id, currency_id)?
            .try_into()
            .map_err(|_| Error::<T, I>::TryIntoIntError.into())
    }

    fn get_total_stake(pool_id: &T::PoolId) -> Result<Balance, DispatchError> {
        Pallet::<T, I>::total_stake(pool_id)
            .truncate_to_inner()
            .ok_or(Error::<T, I>::TryIntoIntError)?
            .try_into()
            .map_err(|_| Error::<T, I>::TryIntoIntError.into())
    }

    fn get_stake(pool_id: &T::PoolId, stake_id: &T::StakeId) -> Result<Balance, DispatchError> {
        Pallet::<T, I>::stake(pool_id, stake_id)
            .truncate_to_inner()
            .ok_or(Error::<T, I>::TryIntoIntError)?
            .try_into()
            .map_err(|_| Error::<T, I>::TryIntoIntError.into())
    }

    fn deposit_stake(pool_id: &T::PoolId, stake_id: &T::StakeId, amount: Balance) -> DispatchResult {
        Pallet::<T, I>::deposit_stake(
            pool_id,
            stake_id,
            amount.to_fixed().ok_or(Error::<T, I>::TryIntoIntError)?,
        )
    }

    fn withdraw_stake(pool_id: &T::PoolId, stake_id: &T::StakeId, amount: Balance) -> DispatchResult {
        Pallet::<T, I>::withdraw_stake(
            pool_id,
            stake_id,
            amount.to_fixed().ok_or(Error::<T, I>::TryIntoIntError)?,
        )
    }
}

impl<PoolId, StakeId, Balance> RewardsApi<PoolId, StakeId, Balance> for ()
where
    Balance: Saturating + PartialOrd + Default,
{
    type CurrencyId = ();

    fn distribute_reward(_: &PoolId, _: Self::CurrencyId, _: Balance) -> DispatchResult {
        Ok(())
    }

    fn compute_reward(_: &PoolId, _: &StakeId, _: Self::CurrencyId) -> Result<Balance, DispatchError> {
        Ok(Default::default())
    }

    fn withdraw_reward(_: &PoolId, _: &StakeId, _: Self::CurrencyId) -> Result<Balance, DispatchError> {
        Ok(Default::default())
    }

    fn get_total_stake(_: &PoolId) -> Result<Balance, DispatchError> {
        Ok(Default::default())
    }

    fn get_stake(_: &PoolId, _: &StakeId) -> Result<Balance, DispatchError> {
        Ok(Default::default())
    }

    fn deposit_stake(_: &PoolId, _: &StakeId, _: Balance) -> DispatchResult {
        Ok(())
    }

    fn withdraw_stake(_: &PoolId, _: &StakeId, _: Balance) -> DispatchResult {
        Ok(())
    }
}
