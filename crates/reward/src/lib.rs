//! # Reward Module
//! Based on the [Scalable Reward Distribution](https://solmaz.io/2019/02/24/scalable-reward-changing/) algorithm.

#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

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
    traits::{CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, MaybeSerializeDeserialize, Zero},
    ArithmeticError,
};
use sp_std::{convert::TryInto, fmt::Debug};

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
        type Event: From<Event<Self, I>> + IsType<<Self as frame_system::Config>::Event>;

        /// Signed fixed point type.
        type SignedFixedPoint: FixedPointNumber + TruncateFixedPointToInt + Encode + EncodeLike + Decode + TypeInfo;

        /// The reward identifier type.
        type RewardId: Parameter + Member + MaybeSerializeDeserialize + Debug + MaxEncodedLen;

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
            reward_id: T::RewardId,
            amount: T::SignedFixedPoint,
        },
        DistributeReward {
            currency_id: T::CurrencyId,
            amount: T::SignedFixedPoint,
        },
        WithdrawStake {
            reward_id: T::RewardId,
            amount: T::SignedFixedPoint,
        },
        WithdrawReward {
            reward_id: T::RewardId,
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
    impl<T: Config<I>, I: 'static> Hooks<T::BlockNumber> for Pallet<T, I> {}

    /// The total stake deposited to this reward pool.
    #[pallet::storage]
    #[pallet::getter(fn total_stake)]
    pub type TotalStake<T: Config<I>, I: 'static = ()> = StorageValue<_, SignedFixedPoint<T, I>, ValueQuery>;

    /// The total unclaimed rewards distributed to this reward pool.
    /// NOTE: this is currently only used for integration tests.
    #[pallet::storage]
    #[pallet::getter(fn total_rewards)]
    pub type TotalRewards<T: Config<I>, I: 'static = ()> =
        StorageMap<_, Blake2_128Concat, T::CurrencyId, SignedFixedPoint<T, I>, ValueQuery>;

    /// Used to compute the rewards for a participant's stake.
    #[pallet::storage]
    #[pallet::getter(fn reward_per_token)]
    pub type RewardPerToken<T: Config<I>, I: 'static = ()> =
        StorageMap<_, Blake2_128Concat, T::CurrencyId, SignedFixedPoint<T, I>, ValueQuery>;

    /// The stake of a participant in this reward pool.
    #[pallet::storage]
    #[pallet::getter(fn stake)]
    pub type Stake<T: Config<I>, I: 'static = ()> =
        StorageMap<_, Blake2_128Concat, T::RewardId, SignedFixedPoint<T, I>, ValueQuery>;

    /// Accounts for previous changes in stake size.
    #[pallet::storage]
    pub type RewardTally<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::CurrencyId,
        Blake2_128Concat,
        T::RewardId,
        SignedFixedPoint<T, I>,
        ValueQuery,
    >;

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T, I = ()>(_);

    // The pallet's dispatchable functions.
    #[pallet::call]
    impl<T: Config<I>, I: 'static> Pallet<T, I> {}
}

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
    pub fn get_total_rewards(
        currency_id: T::CurrencyId,
    ) -> Result<<T::SignedFixedPoint as FixedPointNumber>::Inner, DispatchError> {
        Ok(Self::total_rewards(currency_id)
            .truncate_to_inner()
            .ok_or(Error::<T, I>::TryIntoIntError)?)
    }

    pub fn deposit_stake(reward_id: &T::RewardId, amount: SignedFixedPoint<T, I>) -> Result<(), DispatchError> {
        checked_add_mut!(Stake<T, I>, reward_id, &amount);
        checked_add_mut!(TotalStake<T, I>, &amount);

        for currency_id in [T::GetNativeCurrencyId::get(), T::GetWrappedCurrencyId::get()] {
            <RewardTally<T, I>>::mutate(currency_id, reward_id, |reward_tally| {
                let reward_per_token = Self::reward_per_token(currency_id);
                let reward_per_token_mul_amount =
                    reward_per_token.checked_mul(&amount).ok_or(ArithmeticError::Overflow)?;
                *reward_tally = reward_tally
                    .checked_add(&reward_per_token_mul_amount)
                    .ok_or(ArithmeticError::Overflow)?;
                Ok::<_, DispatchError>(())
            })?;
        }

        Self::deposit_event(Event::<T, I>::DepositStake {
            reward_id: reward_id.clone(),
            amount,
        });

        Ok(())
    }

    pub fn distribute_reward(currency_id: T::CurrencyId, reward: SignedFixedPoint<T, I>) -> DispatchResult {
        let total_stake = Self::total_stake();
        ensure!(!total_stake.is_zero(), Error::<T, I>::ZeroTotalStake);

        let reward_div_total_stake = reward.checked_div(&total_stake).ok_or(ArithmeticError::Underflow)?;
        checked_add_mut!(RewardPerToken<T, I>, currency_id, &reward_div_total_stake);
        checked_add_mut!(TotalRewards<T, I>, currency_id, &reward);

        Self::deposit_event(Event::<T, I>::DistributeReward {
            currency_id,
            amount: reward,
        });
        Ok(())
    }

    pub fn compute_reward(
        currency_id: T::CurrencyId,
        account_id: &T::RewardId,
    ) -> Result<<SignedFixedPoint<T, I> as FixedPointNumber>::Inner, DispatchError> {
        let stake = Self::stake(account_id);
        let reward_per_token = Self::reward_per_token(currency_id);
        // FIXME: this can easily overflow with large numbers
        let stake_mul_reward_per_token = stake.checked_mul(&reward_per_token).ok_or(ArithmeticError::Overflow)?;
        let reward_tally = <RewardTally<T, I>>::get(currency_id, account_id);
        // TODO: this can probably be saturated
        let reward = stake_mul_reward_per_token
            .checked_sub(&reward_tally)
            .ok_or(ArithmeticError::Underflow)?
            .truncate_to_inner()
            .ok_or(Error::<T, I>::TryIntoIntError)?;
        Ok(reward)
    }

    pub fn withdraw_stake(reward_id: &T::RewardId, amount: SignedFixedPoint<T, I>) -> Result<(), DispatchError> {
        if amount > Self::stake(reward_id) {
            return Err(Error::<T, I>::InsufficientFunds.into());
        }

        checked_sub_mut!(Stake<T, I>, &reward_id, &amount);
        checked_sub_mut!(TotalStake<T, I>, &amount);

        for currency_id in [T::GetNativeCurrencyId::get(), T::GetWrappedCurrencyId::get()] {
            <RewardTally<T, I>>::mutate(currency_id, reward_id, |reward_tally| {
                let reward_per_token = Self::reward_per_token(currency_id);
                let reward_per_token_mul_amount =
                    reward_per_token.checked_mul(&amount).ok_or(ArithmeticError::Overflow)?;

                *reward_tally = reward_tally
                    .checked_sub(&reward_per_token_mul_amount)
                    .ok_or(ArithmeticError::Underflow)?;
                Ok::<_, DispatchError>(())
            })?;
        }

        Self::deposit_event(Event::<T, I>::WithdrawStake {
            reward_id: reward_id.clone(),
            amount,
        });
        Ok(())
    }

    pub fn withdraw_reward(
        reward_id: &T::RewardId,
        currency_id: T::CurrencyId,
    ) -> Result<<SignedFixedPoint<T, I> as FixedPointNumber>::Inner, DispatchError> {
        let reward = Self::compute_reward(currency_id, reward_id)?;
        let reward_as_fixed =
            SignedFixedPoint::<T, I>::checked_from_integer(reward).ok_or(Error::<T, I>::TryIntoIntError)?;
        checked_sub_mut!(TotalRewards<T, I>, currency_id, &reward_as_fixed);

        let stake = Self::stake(reward_id);
        let reward_per_token = Self::reward_per_token(currency_id);
        <RewardTally<T, I>>::insert(
            currency_id,
            reward_id,
            stake.checked_mul(&reward_per_token).ok_or(ArithmeticError::Overflow)?,
        );

        Self::deposit_event(Event::<T, I>::WithdrawReward {
            currency_id,
            reward_id: reward_id.clone(),
            amount: reward_as_fixed,
        });
        Ok(reward)
    }
}

pub trait Rewards<AccountId, Balance, CurrencyId> {
    /// Return the stake associated with the `account_id`.
    fn get_stake(account_id: &AccountId) -> Result<Balance, DispatchError>;

    /// Deposit an `amount` of stake to the `account_id`.
    fn deposit_stake(account_id: &AccountId, amount: Balance) -> DispatchResult;

    /// Distribute the `amount` to all participants OR error if zero total stake.
    fn distribute_reward(amount: Balance, currency_id: CurrencyId) -> DispatchResult;

    /// Compute the expected reward for the `account_id`.
    fn compute_reward(account_id: &AccountId, currency_id: CurrencyId) -> Result<Balance, DispatchError>;

    /// Withdraw an `amount` of stake from the `account_id`.
    fn withdraw_stake(account_id: &AccountId, amount: Balance) -> DispatchResult;

    /// Withdraw all rewards from the `account_id`.
    fn withdraw_reward(account_id: &AccountId, currency_id: CurrencyId) -> Result<Balance, DispatchError>;
}

impl<T, I, Balance> Rewards<T::RewardId, Balance, T::CurrencyId> for Pallet<T, I>
where
    T: Config<I>,
    I: 'static,
    Balance: BalanceToFixedPoint<SignedFixedPoint<T, I>>,
    <T::SignedFixedPoint as FixedPointNumber>::Inner: TryInto<Balance>,
{
    fn get_stake(reward_id: &T::RewardId) -> Result<Balance, DispatchError> {
        Pallet::<T, I>::stake(reward_id)
            .truncate_to_inner()
            .ok_or(Error::<T, I>::TryIntoIntError)?
            .try_into()
            .map_err(|_| Error::<T, I>::TryIntoIntError.into())
    }

    fn deposit_stake(reward_id: &T::RewardId, amount: Balance) -> DispatchResult {
        Pallet::<T, I>::deposit_stake(reward_id, amount.to_fixed().ok_or(Error::<T, I>::TryIntoIntError)?)
    }

    fn distribute_reward(amount: Balance, currency_id: T::CurrencyId) -> DispatchResult {
        Pallet::<T, I>::distribute_reward(currency_id, amount.to_fixed().ok_or(Error::<T, I>::TryIntoIntError)?)
    }

    fn compute_reward(reward_id: &T::RewardId, currency_id: T::CurrencyId) -> Result<Balance, DispatchError> {
        Pallet::<T, I>::compute_reward(currency_id, reward_id)?
            .try_into()
            .map_err(|_| Error::<T, I>::TryIntoIntError.into())
    }

    fn withdraw_stake(reward_id: &T::RewardId, amount: Balance) -> DispatchResult {
        Pallet::<T, I>::withdraw_stake(reward_id, amount.to_fixed().ok_or(Error::<T, I>::TryIntoIntError)?)
    }

    fn withdraw_reward(reward_id: &T::RewardId, currency_id: T::CurrencyId) -> Result<Balance, DispatchError> {
        Pallet::<T, I>::withdraw_reward(reward_id, currency_id)?
            .try_into()
            .map_err(|_| Error::<T, I>::TryIntoIntError.into())
    }
}

pub trait ModifyStake<AccountId, Balance> {
    /// Deposit stake for an account.
    fn deposit_stake(account_id: &AccountId, amount: Balance) -> DispatchResult;
    /// Withdraw all stake for an account.
    fn withdraw_stake(account_id: &AccountId) -> DispatchResult;
}

impl<T, I, Balance> ModifyStake<T::RewardId, Balance> for Pallet<T, I>
where
    T: Config<I>,
    I: 'static,
    Balance: BalanceToFixedPoint<SignedFixedPoint<T, I>>,
{
    fn deposit_stake(reward_id: &T::RewardId, amount: Balance) -> DispatchResult {
        Pallet::<T, I>::deposit_stake(reward_id, amount.to_fixed().ok_or(Error::<T, I>::TryIntoIntError)?)
    }

    fn withdraw_stake(reward_id: &T::RewardId) -> DispatchResult {
        Pallet::<T, I>::withdraw_stake(reward_id, Pallet::<T, I>::stake(reward_id))
    }
}

impl<AccountId, Balance> ModifyStake<AccountId, Balance> for () {
    fn deposit_stake(_: &AccountId, _: Balance) -> DispatchResult {
        Ok(())
    }
    fn withdraw_stake(_: &AccountId) -> DispatchResult {
        Ok(())
    }
}
