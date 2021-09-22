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
use frame_support::{dispatch::DispatchError, ensure, traits::Get};
use primitives::{TruncateFixedPointToInt, VaultId};
use sp_arithmetic::FixedPointNumber;
use sp_runtime::traits::{CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, MaybeSerializeDeserialize, Zero};
use sp_std::{marker::PhantomData, vec::Vec};

pub(crate) type SignedFixedPoint<T> = <T as Config>::SignedFixedPoint;

pub use pallet::*;

pub type DefaultVaultId<T> = VaultId<<T as frame_system::Config>::AccountId, <T as Config>::CurrencyId>;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;

    /// ## Configuration
    /// The pallet's configuration trait.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Signed fixed point type.
        type SignedFixedPoint: FixedPointNumber + TruncateFixedPointToInt + Encode + EncodeLike + Decode;

        /// The currency ID type.
        type CurrencyId: Parameter + Member + Copy + MaybeSerializeDeserialize + Ord;

        /// Supported reward currencies.
        #[pallet::constant]
        type GetRewardCurrencyIds: Get<Vec<Self::CurrencyId>>;
    }

    // The pallet's events
    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    #[pallet::metadata(
        T::CurrencyId = "CurrencyId",
        DefaultVaultId<T> = "VaultId",
        T::SignedFixedPoint = "SignedFixedPoint"
    )]
    pub enum Event<T: Config> {
        DepositStake(DefaultVaultId<T>, T::SignedFixedPoint),
        DistributeReward(T::CurrencyId, T::SignedFixedPoint),
        WithdrawStake(DefaultVaultId<T>, T::SignedFixedPoint),
        WithdrawReward(T::CurrencyId, DefaultVaultId<T>, T::SignedFixedPoint),
    }

    #[pallet::error]
    pub enum Error<T> {
        ArithmeticOverflow,
        ArithmeticUnderflow,
        TryIntoIntError,
        InsufficientFunds,
        InvalidCurrencyId,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

    /// The total stake deposited to this reward pool.
    #[pallet::storage]
    #[pallet::getter(fn total_stake)]
    pub type TotalStake<T: Config> = StorageValue<_, SignedFixedPoint<T>, ValueQuery>;

    /// The total unclaimed rewards distributed to this reward pool.
    /// NOTE: this is currently only used for integration tests.
    #[pallet::storage]
    #[pallet::getter(fn total_rewards)]
    pub type TotalRewards<T: Config> = StorageMap<_, Blake2_128Concat, T::CurrencyId, SignedFixedPoint<T>, ValueQuery>;

    /// Used to compute the rewards for a participant's stake.
    #[pallet::storage]
    #[pallet::getter(fn reward_per_token)]
    pub type RewardPerToken<T: Config> =
        StorageMap<_, Blake2_128Concat, T::CurrencyId, SignedFixedPoint<T>, ValueQuery>;

    /// The stake of a participant in this reward pool.
    #[pallet::storage]
    #[pallet::getter(fn stake)]
    pub type Stake<T: Config> = StorageMap<_, Blake2_128Concat, DefaultVaultId<T>, SignedFixedPoint<T>, ValueQuery>;

    /// Accounts for previous changes in stake size.
    #[pallet::storage]
    pub type RewardTally<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        DefaultVaultId<T>,
        Blake2_128Concat,
        T::CurrencyId,
        SignedFixedPoint<T>,
        ValueQuery,
    >;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // The pallet's dispatchable functions.
    #[pallet::call]
    impl<T: Config> Pallet<T> {}
}

macro_rules! checked_add_mut {
    ($storage:ty, $amount:expr) => {
        <$storage>::mutate(|value| {
            *value = value.checked_add($amount).ok_or(Error::<T>::ArithmeticOverflow)?;
            Ok::<_, Error<T>>(())
        })?;
    };
    ($storage:ty, $currency:expr, $amount:expr) => {
        <$storage>::mutate($currency, |value| {
            *value = value.checked_add($amount).ok_or(Error::<T>::ArithmeticOverflow)?;
            Ok::<_, Error<T>>(())
        })?;
    };
    ($storage:ty, $currency:expr, $account:expr, $amount:expr) => {
        <$storage>::mutate($currency, $account, |value| {
            *value = value.checked_add($amount).ok_or(Error::<T>::ArithmeticOverflow)?;
            Ok::<_, Error<T>>(())
        })?;
    };
}

macro_rules! checked_sub_mut {
    ($storage:ty, $amount:expr) => {
        <$storage>::mutate(|value| {
            *value = value.checked_sub($amount).ok_or(Error::<T>::ArithmeticUnderflow)?;
            Ok::<_, Error<T>>(())
        })?;
    };
    ($storage:ty, $currency:expr, $amount:expr) => {
        <$storage>::mutate($currency, |value| {
            *value = value.checked_sub($amount).ok_or(Error::<T>::ArithmeticUnderflow)?;
            Ok::<_, Error<T>>(())
        })?;
    };
    ($storage:ty, $currency:expr, $account:expr, $amount:expr) => {
        <$storage>::mutate($currency, $account, |value| {
            *value = value.checked_sub($amount).ok_or(Error::<T>::ArithmeticUnderflow)?;
            Ok::<_, Error<T>>(())
        })?;
    };
}

// "Internal" functions, callable by code.
impl<T: Config> Pallet<T> {
    pub fn get_total_rewards(
        currency_id: T::CurrencyId,
    ) -> Result<<T::SignedFixedPoint as FixedPointNumber>::Inner, DispatchError> {
        ensure!(
            T::GetRewardCurrencyIds::get().contains(&currency_id),
            Error::<T>::InvalidCurrencyId,
        );

        Ok(Self::total_rewards(currency_id)
            .truncate_to_inner()
            .ok_or(Error::<T>::TryIntoIntError)?)
    }

    pub fn deposit_stake(account_id: &DefaultVaultId<T>, amount: SignedFixedPoint<T>) -> Result<(), DispatchError> {
        checked_add_mut!(Stake<T>, account_id, &amount);
        checked_add_mut!(TotalStake<T>, &amount);

        for currency_id in T::GetRewardCurrencyIds::get() {
            <RewardTally<T>>::mutate(account_id, currency_id, |reward_tally| {
                let reward_per_token = Self::reward_per_token(currency_id);
                let reward_per_token_mul_amount = reward_per_token
                    .checked_mul(&amount)
                    .ok_or(Error::<T>::ArithmeticOverflow)?;
                *reward_tally = reward_tally
                    .checked_add(&reward_per_token_mul_amount)
                    .ok_or(Error::<T>::ArithmeticOverflow)?;
                Ok::<_, Error<T>>(())
            })?;
        }

        Self::deposit_event(Event::<T>::DepositStake(account_id.clone(), amount));
        Ok(())
    }

    pub fn distribute_reward(
        currency_id: T::CurrencyId,
        reward: SignedFixedPoint<T>,
    ) -> Result<SignedFixedPoint<T>, DispatchError> {
        ensure!(
            T::GetRewardCurrencyIds::get().contains(&currency_id),
            Error::<T>::InvalidCurrencyId,
        );

        let total_stake = Self::total_stake();
        if total_stake.is_zero() {
            return Ok(reward);
        }

        let reward_div_total_stake = reward
            .checked_div(&total_stake)
            .ok_or(Error::<T>::ArithmeticUnderflow)?;
        checked_add_mut!(RewardPerToken<T>, currency_id, &reward_div_total_stake);
        checked_add_mut!(TotalRewards<T>, currency_id, &reward);

        Self::deposit_event(Event::<T>::DistributeReward(currency_id, reward));
        Ok(Zero::zero())
    }

    pub fn compute_reward(
        currency_id: T::CurrencyId,
        account_id: &DefaultVaultId<T>,
    ) -> Result<<SignedFixedPoint<T> as FixedPointNumber>::Inner, DispatchError> {
        ensure!(
            T::GetRewardCurrencyIds::get().contains(&currency_id),
            Error::<T>::InvalidCurrencyId,
        );

        let stake = Self::stake(account_id);
        let reward_per_token = Self::reward_per_token(currency_id);
        // FIXME: this can easily overflow with large numbers
        let stake_mul_reward_per_token = stake
            .checked_mul(&reward_per_token)
            .ok_or(Error::<T>::ArithmeticOverflow)?;
        let reward_tally = <RewardTally<T>>::get(account_id, currency_id);
        // TODO: this can probably be saturated
        let reward = stake_mul_reward_per_token
            .checked_sub(&reward_tally)
            .ok_or(Error::<T>::ArithmeticUnderflow)?
            .truncate_to_inner()
            .ok_or(Error::<T>::TryIntoIntError)?;
        Ok(reward)
    }

    pub fn withdraw_stake(account_id: &DefaultVaultId<T>, amount: SignedFixedPoint<T>) -> Result<(), DispatchError> {
        if amount > Self::stake(account_id) {
            return Err(Error::<T>::InsufficientFunds.into());
        }

        checked_sub_mut!(Stake<T>, &account_id, &amount);
        checked_sub_mut!(TotalStake<T>, &amount);

        for currency_id in T::GetRewardCurrencyIds::get() {
            <RewardTally<T>>::mutate(account_id, currency_id, |reward_tally| {
                let reward_per_token = Self::reward_per_token(currency_id);
                let reward_per_token_mul_amount = reward_per_token
                    .checked_mul(&amount)
                    .ok_or(Error::<T>::ArithmeticOverflow)?;

                *reward_tally = reward_tally
                    .checked_sub(&reward_per_token_mul_amount)
                    .ok_or(Error::<T>::ArithmeticUnderflow)?;
                Ok::<_, Error<T>>(())
            })?;
        }

        Self::deposit_event(Event::<T>::WithdrawStake(account_id.clone(), amount));
        Ok(())
    }

    pub fn withdraw_reward(
        currency_id: T::CurrencyId,
        account_id: &DefaultVaultId<T>,
    ) -> Result<<SignedFixedPoint<T> as FixedPointNumber>::Inner, DispatchError> {
        let reward = Self::compute_reward(currency_id, account_id)?;
        let reward_as_fixed = SignedFixedPoint::<T>::checked_from_integer(reward).ok_or(Error::<T>::TryIntoIntError)?;
        checked_sub_mut!(TotalRewards<T>, currency_id, &reward_as_fixed);

        let stake = Self::stake(account_id);
        let reward_per_token = Self::reward_per_token(currency_id);
        <RewardTally<T>>::insert(
            account_id,
            currency_id,
            stake
                .checked_mul(&reward_per_token)
                .ok_or(Error::<T>::ArithmeticOverflow)?,
        );

        Self::deposit_event(Event::<T>::WithdrawReward(
            currency_id,
            account_id.clone(),
            reward_as_fixed,
        ));
        Ok(reward)
    }
}

pub trait Rewards<VaultId> {
    /// Signed fixed point type.
    type SignedFixedPoint: FixedPointNumber;

    /// Return the stake associated with the `account_id`.
    fn get_stake(account_id: &VaultId) -> Self::SignedFixedPoint;

    /// Deposit an `amount` of stake to the `account_id`.
    fn deposit_stake(account_id: &VaultId, amount: Self::SignedFixedPoint) -> Result<(), DispatchError>;

    /// Distribute the `reward` to all participants OR return the leftover.
    fn distribute_reward(reward: Self::SignedFixedPoint) -> Result<Self::SignedFixedPoint, DispatchError>;

    /// Compute the expected reward for the `account_id`.
    fn compute_reward(
        account_id: &VaultId,
    ) -> Result<<Self::SignedFixedPoint as FixedPointNumber>::Inner, DispatchError>;

    /// Withdraw an `amount` of stake from the `account_id`.
    fn withdraw_stake(account_id: &VaultId, amount: Self::SignedFixedPoint) -> Result<(), DispatchError>;

    /// Withdraw all rewards from the `account_id`.
    fn withdraw_reward(
        account_id: &VaultId,
    ) -> Result<<Self::SignedFixedPoint as FixedPointNumber>::Inner, DispatchError>;
}

pub struct RewardsCurrencyAdapter<T, GetCurrencyId>(PhantomData<(T, GetCurrencyId)>);

impl<T, GetCurrencyId> Rewards<DefaultVaultId<T>> for RewardsCurrencyAdapter<T, GetCurrencyId>
where
    T: Config,
    GetCurrencyId: Get<T::CurrencyId>,
{
    type SignedFixedPoint = SignedFixedPoint<T>;

    fn get_stake(account_id: &DefaultVaultId<T>) -> Self::SignedFixedPoint {
        Pallet::<T>::stake(account_id)
    }

    fn deposit_stake(account_id: &DefaultVaultId<T>, amount: Self::SignedFixedPoint) -> Result<(), DispatchError> {
        Pallet::<T>::deposit_stake(account_id, amount)
    }

    fn distribute_reward(reward: Self::SignedFixedPoint) -> Result<Self::SignedFixedPoint, DispatchError> {
        Pallet::<T>::distribute_reward(GetCurrencyId::get(), reward)
    }

    fn compute_reward(
        account_id: &DefaultVaultId<T>,
    ) -> Result<<Self::SignedFixedPoint as FixedPointNumber>::Inner, DispatchError> {
        Pallet::<T>::compute_reward(GetCurrencyId::get(), account_id)
    }

    fn withdraw_stake(account_id: &DefaultVaultId<T>, amount: Self::SignedFixedPoint) -> Result<(), DispatchError> {
        Pallet::<T>::withdraw_stake(account_id, amount)
    }

    fn withdraw_reward(
        account_id: &DefaultVaultId<T>,
    ) -> Result<<Self::SignedFixedPoint as FixedPointNumber>::Inner, DispatchError> {
        Pallet::<T>::withdraw_reward(GetCurrencyId::get(), account_id)
    }
}
