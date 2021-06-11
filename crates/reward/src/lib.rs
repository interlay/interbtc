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
use frame_support::{dispatch::DispatchError, traits::Get};
use sp_arithmetic::FixedPointNumber;
use sp_runtime::traits::{CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, MaybeSerializeDeserialize, Zero};
use sp_std::{fmt::Debug, marker::PhantomData, vec::Vec};

pub(crate) type SignedFixedPoint<T, I = ()> = <T as Config<I>>::SignedFixedPoint;

pub use pallet::*;

pub type Vault = pallet::Instance1;
pub type Relayer = pallet::Instance2;

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
        type SignedFixedPoint: FixedPointNumber + Encode + EncodeLike + Decode;

        /// The currency ID type.
        type CurrencyId: Parameter + Member + Copy + MaybeSerializeDeserialize + Ord;
    }

    #[derive(Encode, Decode, Clone, Copy, PartialEq, Debug)]
    pub enum RewardPool<AccountId> {
        Global,
        Local(AccountId),
    }

    // The pallet's events
    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    #[pallet::metadata(T::CurrencyId = "CurrencyId", RewardPool<T::AccountId> = "RewardPool", T::AccountId = "AccountId", T::SignedFixedPoint = "SignedFixedPoint")]
    pub enum Event<T: Config<I>, I: 'static = ()> {
        DepositStake(
            T::CurrencyId,
            RewardPool<T::AccountId>,
            T::AccountId,
            T::SignedFixedPoint,
        ),
        WithdrawStake(
            T::CurrencyId,
            RewardPool<T::AccountId>,
            T::AccountId,
            T::SignedFixedPoint,
        ),
        WithdrawReward(
            T::CurrencyId,
            RewardPool<T::AccountId>,
            T::AccountId,
            T::SignedFixedPoint,
        ),
    }

    #[pallet::error]
    pub enum Error<T, I = ()> {
        ArithmeticOverflow,
        ArithmeticUnderflow,
        TryIntoIntError,
        InsufficientFunds,
    }

    #[pallet::hooks]
    impl<T: Config<I>, I: 'static> Hooks<T::BlockNumber> for Pallet<T, I> {}

    /// The total stake deposited to this reward pool.
    #[pallet::storage]
    #[pallet::getter(fn total_stake)]
    pub type TotalStake<T: Config<I>, I: 'static = ()> =
        StorageMap<_, Blake2_128Concat, (T::CurrencyId, RewardPool<T::AccountId>), SignedFixedPoint<T, I>, ValueQuery>;

    /// The total unclaimed rewards distributed to this reward pool.
    /// NOTE: this is currently only used for integration tests.
    #[pallet::storage]
    #[pallet::getter(fn total_rewards)]
    pub type TotalRewards<T: Config<I>, I: 'static = ()> =
        StorageMap<_, Blake2_128Concat, (T::CurrencyId, RewardPool<T::AccountId>), SignedFixedPoint<T, I>, ValueQuery>;

    /// Used to compute the rewards for a participant's stake.
    #[pallet::storage]
    #[pallet::getter(fn reward_per_token)]
    pub type RewardPerToken<T: Config<I>, I: 'static = ()> =
        StorageMap<_, Blake2_128Concat, (T::CurrencyId, RewardPool<T::AccountId>), SignedFixedPoint<T, I>, ValueQuery>;

    /// The stake of a participant in this reward pool.
    #[pallet::storage]
    #[pallet::getter(fn stake)]
    pub type Stake<T: Config<I>, I: 'static = ()> = StorageMap<
        _,
        Blake2_128Concat,
        (T::CurrencyId, RewardPool<T::AccountId>, T::AccountId),
        SignedFixedPoint<T, I>,
        ValueQuery,
    >;

    /// Accounts for previous changes in stake size.
    #[pallet::storage]
    pub type RewardTally<T: Config<I>, I: 'static = ()> = StorageMap<
        _,
        Blake2_128Concat,
        (T::CurrencyId, RewardPool<T::AccountId>, T::AccountId),
        SignedFixedPoint<T, I>,
        ValueQuery,
    >;

    #[pallet::pallet]
    pub struct Pallet<T, I = ()>(_);

    // The pallet's dispatchable functions.
    #[pallet::call]
    impl<T: Config<I>, I: 'static> Pallet<T, I> {}
}

macro_rules! checked_add_mut {
    ($storage:ty, $amount:expr) => {
        <$storage>::mutate(|value| {
            *value = value.checked_add($amount).ok_or(Error::<T, I>::ArithmeticOverflow)?;
            Ok::<_, Error<T, I>>(())
        })?;
    };
    ($storage:ty, $account:expr, $amount:expr) => {
        <$storage>::mutate($account, |value| {
            *value = value.checked_add($amount).ok_or(Error::<T, I>::ArithmeticOverflow)?;
            Ok::<_, Error<T, I>>(())
        })?;
    };
}

macro_rules! checked_sub_mut {
    ($storage:ty, $amount:expr) => {
        <$storage>::mutate(|value| {
            *value = value.checked_sub($amount).ok_or(Error::<T, I>::ArithmeticUnderflow)?;
            Ok::<_, Error<T, I>>(())
        })?;
    };
    ($storage:ty, $account:expr, $amount:expr) => {
        <$storage>::mutate($account, |value| {
            *value = value.checked_sub($amount).ok_or(Error::<T, I>::ArithmeticUnderflow)?;
            Ok::<_, Error<T, I>>(())
        })?;
    };
}

// "Internal" functions, callable by code.
impl<T: Config<I>, I: 'static> Pallet<T, I> {
    pub fn participants() -> Vec<(
        (T::CurrencyId, RewardPool<T::AccountId>, T::AccountId),
        T::SignedFixedPoint,
    )> {
        <Stake<T, I>>::iter().collect()
    }

    pub fn get_total_rewards(
        currency_id: T::CurrencyId,
        reward_pool: RewardPool<T::AccountId>,
    ) -> Result<<T::SignedFixedPoint as FixedPointNumber>::Inner, DispatchError> {
        Ok(Self::total_rewards((currency_id, reward_pool))
            .into_inner()
            .checked_div(&SignedFixedPoint::<T, I>::accuracy())
            .ok_or(Error::<T, I>::ArithmeticUnderflow)?)
    }

    fn get_stake(
        currency_id: T::CurrencyId,
        reward_pool: RewardPool<T::AccountId>,
        account_id: &T::AccountId,
    ) -> SignedFixedPoint<T, I> {
        Self::stake((currency_id, reward_pool, account_id))
    }

    fn deposit_stake(
        currency_id: T::CurrencyId,
        reward_pool: RewardPool<T::AccountId>,
        account_id: &T::AccountId,
        amount: SignedFixedPoint<T, I>,
    ) -> Result<(), DispatchError> {
        checked_add_mut!(Stake<T, I>, (currency_id, &reward_pool, account_id), &amount);
        checked_add_mut!(TotalStake<T, I>, (currency_id, &reward_pool), &amount);

        <RewardTally<T, I>>::mutate((currency_id, &reward_pool, account_id), |reward_tally| {
            let reward_per_token = Self::reward_per_token((currency_id, &reward_pool));
            let reward_per_token_mul_amount = reward_per_token
                .checked_mul(&amount)
                .ok_or(Error::<T, I>::ArithmeticOverflow)?;
            *reward_tally = reward_tally
                .checked_add(&reward_per_token_mul_amount)
                .ok_or(Error::<T, I>::ArithmeticOverflow)?;
            Ok::<_, Error<T, I>>(())
        })?;

        Self::deposit_event(Event::<T, I>::DepositStake(
            currency_id,
            reward_pool,
            account_id.clone(),
            amount,
        ));
        Ok(())
    }

    fn distribute(
        currency_id: T::CurrencyId,
        reward_pool: RewardPool<T::AccountId>,
        reward: SignedFixedPoint<T, I>,
    ) -> Result<SignedFixedPoint<T, I>, DispatchError> {
        let total_stake = Self::total_stake((currency_id, &reward_pool));
        if total_stake.is_zero() {
            return Ok(SignedFixedPoint::<T, I>::zero());
        }

        let reward_div_total_stake = reward
            .checked_div(&total_stake)
            .ok_or(Error::<T, I>::ArithmeticUnderflow)?;
        checked_add_mut!(RewardPerToken<T, I>, (currency_id, &reward_pool), &reward_div_total_stake);
        checked_add_mut!(TotalRewards<T, I>, (currency_id, &reward_pool), &reward);
        Ok(reward)
    }

    fn compute_reward(
        currency_id: T::CurrencyId,
        reward_pool: &RewardPool<T::AccountId>,
        account_id: &T::AccountId,
    ) -> Result<<SignedFixedPoint<T, I> as FixedPointNumber>::Inner, DispatchError> {
        let stake = <Stake<T, I>>::get((currency_id, reward_pool, account_id));
        let reward_per_token = Self::reward_per_token((currency_id, reward_pool));
        // FIXME: this can easily overflow with large numbers
        let stake_mul_reward_per_token = stake
            .checked_mul(&reward_per_token)
            .ok_or(Error::<T, I>::ArithmeticOverflow)?;
        let reward_tally = <RewardTally<T, I>>::get((currency_id, reward_pool, account_id));
        // TODO: this can probably be saturated
        let reward = stake_mul_reward_per_token
            .checked_sub(&reward_tally)
            .ok_or(Error::<T, I>::ArithmeticUnderflow)?
            .into_inner()
            .checked_div(&SignedFixedPoint::<T, I>::accuracy())
            .ok_or(Error::<T, I>::ArithmeticUnderflow)?;
        Ok(reward)
    }

    fn withdraw_stake(
        currency_id: T::CurrencyId,
        reward_pool: RewardPool<T::AccountId>,
        account_id: &T::AccountId,
        amount: SignedFixedPoint<T, I>,
    ) -> Result<(), DispatchError> {
        if amount > <Stake<T, I>>::get((currency_id, &reward_pool, account_id)) {
            return Err(Error::<T, I>::InsufficientFunds.into());
        }

        checked_sub_mut!(Stake<T, I>, (currency_id, &reward_pool, &account_id), &amount);
        checked_sub_mut!(TotalStake<T, I>, (currency_id, &reward_pool), &amount);

        <RewardTally<T, I>>::mutate((currency_id, &reward_pool, account_id), |reward_tally| {
            let reward_per_token = Self::reward_per_token((currency_id, &reward_pool));
            let reward_per_token_mul_amount = reward_per_token
                .checked_mul(&amount)
                .ok_or(Error::<T, I>::ArithmeticOverflow)?;

            *reward_tally = reward_tally
                .checked_sub(&reward_per_token_mul_amount)
                .ok_or(Error::<T, I>::ArithmeticUnderflow)?;
            Ok::<_, Error<T, I>>(())
        })?;

        Self::deposit_event(Event::<T, I>::WithdrawStake(
            currency_id,
            reward_pool,
            account_id.clone(),
            amount,
        ));
        Ok(())
    }

    fn withdraw_reward(
        currency_id: T::CurrencyId,
        reward_pool: RewardPool<T::AccountId>,
        account_id: &T::AccountId,
    ) -> Result<<SignedFixedPoint<T, I> as FixedPointNumber>::Inner, DispatchError> {
        let reward = Self::compute_reward(currency_id, &reward_pool, account_id)?;
        let reward_as_fixed =
            SignedFixedPoint::<T, I>::checked_from_integer(reward).ok_or(Error::<T, I>::TryIntoIntError)?;
        checked_sub_mut!(TotalRewards<T, I>, (currency_id, &reward_pool), &reward_as_fixed);

        let stake = <Stake<T, I>>::get((currency_id, &reward_pool, account_id));
        let reward_per_token = Self::reward_per_token((currency_id, &reward_pool));
        <RewardTally<T, I>>::insert(
            (currency_id, &reward_pool, account_id),
            stake
                .checked_mul(&reward_per_token)
                .ok_or(Error::<T, I>::ArithmeticOverflow)?,
        );

        Self::deposit_event(Event::<T, I>::WithdrawReward(
            currency_id,
            reward_pool,
            account_id.clone(),
            reward_as_fixed,
        ));
        Ok(reward)
    }
}

pub trait Rewards<AccountId> {
    /// Signed fixed point type.
    type SignedFixedPoint: FixedPointNumber;

    /// Return the stake associated with the `account_id`.
    fn get_stake(reward_pool: RewardPool<AccountId>, account_id: &AccountId) -> Self::SignedFixedPoint;

    /// Deposit an `amount` of stake to the `account_id`.
    fn deposit_stake(
        reward_pool: RewardPool<AccountId>,
        account_id: &AccountId,
        amount: Self::SignedFixedPoint,
    ) -> Result<(), DispatchError>;

    /// Distribute the `reward` to all participants.
    fn distribute(
        reward_pool: RewardPool<AccountId>,
        reward: Self::SignedFixedPoint,
    ) -> Result<Self::SignedFixedPoint, DispatchError>;

    /// Compute the expected reward for the `account_id`.
    fn compute_reward(
        reward_pool: &RewardPool<AccountId>,
        account_id: &AccountId,
    ) -> Result<<Self::SignedFixedPoint as FixedPointNumber>::Inner, DispatchError>;

    /// Withdraw an `amount` of stake from the `account_id`.
    fn withdraw_stake(
        reward_pool: RewardPool<AccountId>,
        account_id: &AccountId,
        amount: Self::SignedFixedPoint,
    ) -> Result<(), DispatchError>;

    /// Withdraw all rewards from the `account_id`.
    fn withdraw_reward(
        reward_pool: RewardPool<AccountId>,
        account_id: &AccountId,
    ) -> Result<<Self::SignedFixedPoint as FixedPointNumber>::Inner, DispatchError>;
}

pub struct RewardsCurrencyAdapter<T, I, GetCurrencyId>(PhantomData<(T, I, GetCurrencyId)>);

impl<T, I, GetCurrencyId> Rewards<T::AccountId> for RewardsCurrencyAdapter<T, I, GetCurrencyId>
where
    T: Config<I>,
    I: 'static,
    GetCurrencyId: Get<T::CurrencyId>,
{
    type SignedFixedPoint = SignedFixedPoint<T, I>;

    fn get_stake(reward_pool: RewardPool<T::AccountId>, account_id: &T::AccountId) -> Self::SignedFixedPoint {
        Pallet::<T, I>::get_stake(GetCurrencyId::get(), reward_pool, account_id)
    }

    fn deposit_stake(
        reward_pool: RewardPool<T::AccountId>,
        account_id: &T::AccountId,
        amount: Self::SignedFixedPoint,
    ) -> Result<(), DispatchError> {
        Pallet::<T, I>::deposit_stake(GetCurrencyId::get(), reward_pool, account_id, amount)
    }

    fn distribute(
        reward_pool: RewardPool<T::AccountId>,
        reward: Self::SignedFixedPoint,
    ) -> Result<Self::SignedFixedPoint, DispatchError> {
        Pallet::<T, I>::distribute(GetCurrencyId::get(), reward_pool, reward)
    }

    fn compute_reward(
        reward_pool: &RewardPool<T::AccountId>,
        account_id: &T::AccountId,
    ) -> Result<<Self::SignedFixedPoint as FixedPointNumber>::Inner, DispatchError> {
        Pallet::<T, I>::compute_reward(GetCurrencyId::get(), reward_pool, account_id)
    }

    fn withdraw_stake(
        reward_pool: RewardPool<T::AccountId>,
        account_id: &T::AccountId,
        amount: Self::SignedFixedPoint,
    ) -> Result<(), DispatchError> {
        Pallet::<T, I>::withdraw_stake(GetCurrencyId::get(), reward_pool, account_id, amount)
    }

    fn withdraw_reward(
        reward_pool: RewardPool<T::AccountId>,
        account_id: &T::AccountId,
    ) -> Result<<Self::SignedFixedPoint as FixedPointNumber>::Inner, DispatchError> {
        Pallet::<T, I>::withdraw_reward(GetCurrencyId::get(), reward_pool, account_id)
    }
}
