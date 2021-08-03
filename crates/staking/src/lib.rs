//! # Staking Module
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
    traits::Get,
};
use sp_arithmetic::{FixedPointNumber, FixedPointOperand};
use sp_runtime::traits::{CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, MaybeSerializeDeserialize, One, Zero};
use sp_std::{cmp, marker::PhantomData};

pub(crate) type SignedFixedPoint<T> = <T as Config>::SignedFixedPoint;

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

        /// The `Inner` type of the `SignedFixedPoint`.
        type SignedInner: CheckedDiv + Ord + FixedPointOperand;

        /// Signed fixed point type.
        type SignedFixedPoint: FixedPointNumber<Inner = Self::SignedInner> + Encode + EncodeLike + Decode;

        /// The currency ID type.
        type CurrencyId: Parameter + Member + Copy + MaybeSerializeDeserialize + Ord;
    }

    // The pallet's events
    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    #[pallet::metadata(
        T::CurrencyId = "CurrencyId",
        T::AccountId = "AccountId",
        T::SignedFixedPoint = "SignedFixedPoint",
        T::Index = "Index"
    )]
    pub enum Event<T: Config> {
        DepositStake(T::CurrencyId, T::AccountId, T::AccountId, T::SignedFixedPoint),
        DistributeReward(T::CurrencyId, T::AccountId, T::SignedFixedPoint),
        WithdrawStake(T::CurrencyId, T::AccountId, T::AccountId, T::SignedFixedPoint),
        WithdrawReward(T::Index, T::CurrencyId, T::AccountId, T::AccountId, T::SignedFixedPoint),
        ForceRefund(T::CurrencyId, T::AccountId),
        IncreaseNonce(T::CurrencyId, T::AccountId, T::Index),
    }

    #[pallet::error]
    pub enum Error<T> {
        ArithmeticOverflow,
        ArithmeticUnderflow,
        TryIntoIntError,
        InsufficientFunds,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

    /// The total stake - this will increase on deposit and decrease on withdrawal.
    #[pallet::storage]
    #[pallet::getter(fn total_stake_at_index)]
    pub type TotalStake<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::CurrencyId,
        Blake2_128Concat,
        (T::Index, T::AccountId),
        SignedFixedPoint<T>,
        ValueQuery,
    >;

    /// The total stake - this will increase on deposit and decrease on withdrawal or slashing.
    #[pallet::storage]
    #[pallet::getter(fn total_current_stake_at_index)]
    pub type TotalCurrentStake<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::CurrencyId,
        Blake2_128Concat,
        (T::Index, T::AccountId),
        SignedFixedPoint<T>,
        ValueQuery,
    >;

    /// The total unclaimed rewards distributed to this reward pool.
    /// NOTE: this is currently only used for integration tests.
    #[pallet::storage]
    #[pallet::getter(fn total_rewards)]
    pub type TotalRewards<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::CurrencyId,
        Blake2_128Concat,
        (T::Index, T::AccountId),
        SignedFixedPoint<T>,
        ValueQuery,
    >;

    /// Used to compute the rewards for a participant's stake.
    #[pallet::storage]
    #[pallet::getter(fn reward_per_token)]
    pub type RewardPerToken<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::CurrencyId,
        Blake2_128Concat,
        (T::Index, T::AccountId),
        SignedFixedPoint<T>,
        ValueQuery,
    >;

    /// Used to compute the amount to slash from a participant's stake.
    #[pallet::storage]
    #[pallet::getter(fn slash_per_token)]
    pub type SlashPerToken<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::CurrencyId,
        Blake2_128Concat,
        (T::Index, T::AccountId),
        SignedFixedPoint<T>,
        ValueQuery,
    >;

    /// The stake of a participant in this reward pool.
    #[pallet::storage]
    pub type Stake<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::CurrencyId,
        Blake2_128Concat,
        (T::Index, T::AccountId, T::AccountId),
        SignedFixedPoint<T>,
        ValueQuery,
    >;

    /// Accounts for previous changes in stake size.
    #[pallet::storage]
    pub type RewardTally<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::CurrencyId,
        Blake2_128Concat,
        (T::Index, T::AccountId, T::AccountId),
        SignedFixedPoint<T>,
        ValueQuery,
    >;

    /// Accounts for previous changes in stake size.
    #[pallet::storage]
    pub type SlashTally<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::CurrencyId,
        Blake2_128Concat,
        (T::Index, T::AccountId, T::AccountId),
        SignedFixedPoint<T>,
        ValueQuery,
    >;

    /// The nonce of the current staking pool, used in force refunds.
    /// This is a strictly increasing value.
    #[pallet::storage]
    pub type Nonce<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, T::CurrencyId, Blake2_128Concat, T::AccountId, T::Index, ValueQuery>;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // The pallet's dispatchable functions.
    #[pallet::call]
    impl<T: Config> Pallet<T> {}
}

macro_rules! checked_add_mut {
    ($storage:ty, $currency:expr, $amount:expr) => {
        <$storage>::mutate($currency, |value| {
            *value = value.checked_add($amount).ok_or(Error::<T>::ArithmeticOverflow)?;
            Ok::<_, Error<T>>(*value)
        })?;
    };
    ($storage:ty, $currency:expr, $account:expr, $amount:expr) => {
        <$storage>::mutate($currency, $account, |value| {
            *value = value.checked_add($amount).ok_or(Error::<T>::ArithmeticOverflow)?;
            Ok::<_, Error<T>>(*value)
        })?;
    };
}

macro_rules! checked_sub_mut {
    ($storage:ty, $currency:expr, $amount:expr) => {
        <$storage>::mutate($currency, |value| {
            *value = value.checked_sub($amount).ok_or(Error::<T>::ArithmeticUnderflow)?;
            Ok::<_, Error<T>>(*value)
        })?;
    };
    ($storage:ty, $currency:expr, $account:expr, $amount:expr) => {
        <$storage>::mutate($currency, $account, |value| {
            *value = value.checked_sub($amount).ok_or(Error::<T>::ArithmeticUnderflow)?;
            Ok::<_, Error<T>>(*value)
        })?;
    };
}

// "Internal" functions, callable by code.
impl<T: Config> Pallet<T> {
    /// Get the stake associated with a vault / nominator.
    pub(crate) fn stake(
        nonce: T::Index,
        currency_id: T::CurrencyId,
        vault_id: &T::AccountId,
        nominator_id: &T::AccountId,
    ) -> SignedFixedPoint<T> {
        <Stake<T>>::get(currency_id, (nonce, vault_id, nominator_id))
    }

    pub fn total_current_stake(
        currency_id: T::CurrencyId,
        vault_id: &T::AccountId,
    ) -> Result<<SignedFixedPoint<T> as FixedPointNumber>::Inner, DispatchError> {
        let nonce = Self::nonce(currency_id, vault_id);
        Self::total_current_stake_at_index(currency_id, (nonce, vault_id))
            .into_inner()
            .checked_div(&SignedFixedPoint::<T>::accuracy())
            .ok_or(Error::<T>::ArithmeticUnderflow.into())
    }

    pub(crate) fn reward_tally(
        nonce: T::Index,
        currency_id: T::CurrencyId,
        vault_id: &T::AccountId,
        nominator_id: &T::AccountId,
    ) -> SignedFixedPoint<T> {
        <RewardTally<T>>::get(currency_id, (nonce, vault_id, nominator_id))
    }

    pub(crate) fn slash_tally(
        nonce: T::Index,
        currency_id: T::CurrencyId,
        vault_id: &T::AccountId,
        nominator_id: &T::AccountId,
    ) -> SignedFixedPoint<T> {
        <SlashTally<T>>::get(currency_id, (nonce, vault_id, nominator_id))
    }

    pub fn nonce(currency_id: T::CurrencyId, vault_id: &T::AccountId) -> T::Index {
        <Nonce<T>>::get(currency_id, vault_id)
    }

    /// Deposit an `amount` of stake to the `vault_id` for the `nominator_id`.
    pub fn deposit_stake(
        currency_id: T::CurrencyId,
        vault_id: &T::AccountId,
        nominator_id: &T::AccountId,
        amount: SignedFixedPoint<T>,
    ) -> Result<(), DispatchError> {
        let nonce = Self::nonce(currency_id, vault_id);
        checked_add_mut!(Stake<T>, currency_id, (nonce, vault_id, nominator_id), &amount);
        checked_add_mut!(TotalStake<T>, currency_id, (nonce, vault_id), &amount);
        checked_add_mut!(TotalCurrentStake<T>, currency_id, (nonce, vault_id), &amount);

        <RewardTally<T>>::mutate(currency_id, (nonce, vault_id, nominator_id), |reward_tally| {
            let reward_per_token = Self::reward_per_token(currency_id, (nonce, vault_id));
            let reward_per_token_mul_amount = reward_per_token
                .checked_mul(&amount)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            *reward_tally = reward_tally
                .checked_add(&reward_per_token_mul_amount)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            Ok::<_, Error<T>>(())
        })?;

        <SlashTally<T>>::mutate(currency_id, (nonce, vault_id, nominator_id), |slash_tally| {
            let slash_per_token = Self::slash_per_token(currency_id, (nonce, vault_id));
            let slash_per_token_mul_amount = slash_per_token
                .checked_mul(&amount)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            *slash_tally = slash_tally
                .checked_add(&slash_per_token_mul_amount)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            Ok::<_, Error<T>>(())
        })?;

        Self::deposit_event(Event::<T>::DepositStake(
            currency_id,
            vault_id.clone(),
            nominator_id.clone(),
            amount,
        ));
        Ok(())
    }

    /// Slash an `amount` of stake from the `vault_id`.
    pub fn slash_stake(
        currency_id: T::CurrencyId,
        vault_id: &T::AccountId,
        amount: SignedFixedPoint<T>,
    ) -> Result<(), DispatchError> {
        let nonce = Self::nonce(currency_id, vault_id);
        let total_stake = Self::total_stake_at_index(currency_id, (nonce, vault_id));
        if amount.is_zero() {
            return Ok(());
        } else if total_stake.is_zero() {
            return Err(Error::<T>::InsufficientFunds.into());
        }

        let amount_div_total_stake = amount
            .checked_div(&total_stake)
            .ok_or(Error::<T>::ArithmeticUnderflow)?;
        checked_add_mut!(
            SlashPerToken<T>,
            currency_id,
            (nonce, vault_id),
            &amount_div_total_stake
        );

        checked_sub_mut!(TotalCurrentStake<T>, currency_id, (nonce, vault_id), &amount);
        // A slash means reward per token is no longer representative of the rewards
        // since `amount * reward_per_token` will be lost from the system. As such,
        // replenish rewards by the amount of reward lost with this slash
        Self::increase_rewards(
            nonce,
            currency_id,
            vault_id,
            Self::reward_per_token(currency_id, (nonce, vault_id))
                .checked_mul(&amount)
                .ok_or(Error::<T>::ArithmeticOverflow)?,
        )?;
        Ok(())
    }

    fn compute_amount_to_slash(
        stake: SignedFixedPoint<T>,
        slash_per_token: SignedFixedPoint<T>,
        slash_tally: SignedFixedPoint<T>,
    ) -> Result<SignedFixedPoint<T>, DispatchError> {
        let stake_mul_slash_per_token = stake
            .checked_mul(&slash_per_token)
            .ok_or(Error::<T>::ArithmeticOverflow)?;

        let to_slash = stake_mul_slash_per_token
            .checked_sub(&slash_tally)
            .ok_or(Error::<T>::ArithmeticUnderflow)?;

        Ok(to_slash)
    }

    /// Delegates to `compute_stake_at_index` with the current nonce.
    pub fn compute_stake(
        currency_id: T::CurrencyId,
        vault_id: &T::AccountId,
        nominator_id: &T::AccountId,
    ) -> Result<<SignedFixedPoint<T> as FixedPointNumber>::Inner, DispatchError> {
        let nonce = Self::nonce(currency_id, vault_id);
        Self::compute_stake_at_index(nonce, currency_id, vault_id, nominator_id)
    }

    /// Compute the stake in `vault_id` owned by `nominator_id`.
    pub fn compute_stake_at_index(
        nonce: T::Index,
        currency_id: T::CurrencyId,
        vault_id: &T::AccountId,
        nominator_id: &T::AccountId,
    ) -> Result<<SignedFixedPoint<T> as FixedPointNumber>::Inner, DispatchError> {
        let stake = Self::stake(nonce, currency_id, vault_id, nominator_id);
        let slash_per_token = Self::slash_per_token(currency_id, (nonce, vault_id));
        let slash_tally = Self::slash_tally(nonce, currency_id, vault_id, nominator_id);
        let to_slash = Self::compute_amount_to_slash(stake, slash_per_token, slash_tally)?;

        let stake_sub_to_slash = stake
            .checked_sub(&to_slash)
            .ok_or(Error::<T>::ArithmeticUnderflow)?
            .into_inner()
            .checked_div(&SignedFixedPoint::<T>::accuracy())
            .ok_or(Error::<T>::ArithmeticUnderflow)?;
        Ok(cmp::max(Zero::zero(), stake_sub_to_slash))
    }

    fn increase_rewards(
        nonce: T::Index,
        currency_id: T::CurrencyId,
        vault_id: &T::AccountId,
        reward: SignedFixedPoint<T>,
    ) -> Result<SignedFixedPoint<T>, DispatchError> {
        let total_current_stake = Self::total_current_stake_at_index(currency_id, (nonce, vault_id));
        if total_current_stake.is_zero() {
            return Ok(Zero::zero());
        }

        let reward_div_total_current_stake = reward
            .checked_div(&total_current_stake)
            .ok_or(Error::<T>::ArithmeticUnderflow)?;
        checked_add_mut!(
            RewardPerToken<T>,
            currency_id,
            (nonce, vault_id),
            &reward_div_total_current_stake
        );
        Ok(reward)
    }

    /// Distribute the `reward` to all participants.
    pub fn distribute_reward(
        currency_id: T::CurrencyId,
        vault_id: &T::AccountId,
        reward: SignedFixedPoint<T>,
    ) -> Result<SignedFixedPoint<T>, DispatchError> {
        let nonce = Self::nonce(currency_id, vault_id);

        let reward = Self::increase_rewards(nonce, currency_id, vault_id, reward)?;
        if reward.is_zero() {
            return Ok(Zero::zero());
        }
        checked_add_mut!(TotalRewards<T>, currency_id, (nonce, vault_id), &reward);

        Self::deposit_event(Event::<T>::DistributeReward(currency_id, vault_id.clone(), reward));
        Ok(reward)
    }

    /// Delegates to `compute_reward_at_index` with the current nonce.
    pub fn compute_reward(
        currency_id: T::CurrencyId,
        vault_id: &T::AccountId,
        nominator_id: &T::AccountId,
    ) -> Result<<SignedFixedPoint<T> as FixedPointNumber>::Inner, DispatchError> {
        let nonce = Self::nonce(currency_id, vault_id);
        Self::compute_reward_at_index(nonce, currency_id, vault_id, nominator_id)
    }

    /// Compute the expected reward for `nominator_id` who is nominating `vault_id`.
    pub fn compute_reward_at_index(
        nonce: T::Index,
        currency_id: T::CurrencyId,
        vault_id: &T::AccountId,
        nominator_id: &T::AccountId,
    ) -> Result<<SignedFixedPoint<T> as FixedPointNumber>::Inner, DispatchError> {
        let stake = SignedFixedPoint::<T>::checked_from_integer(Self::compute_stake_at_index(
            nonce,
            currency_id,
            vault_id,
            nominator_id,
        )?)
        .ok_or(Error::<T>::TryIntoIntError)?;
        let reward_per_token = Self::reward_per_token(currency_id, (nonce, vault_id));
        // FIXME: this can easily overflow with large numbers
        let stake_mul_reward_per_token = stake
            .checked_mul(&reward_per_token)
            .ok_or(Error::<T>::ArithmeticOverflow)?;
        let reward_tally = Self::reward_tally(nonce, currency_id, vault_id, nominator_id);
        // TODO: this can probably be saturated
        let reward = stake_mul_reward_per_token
            .checked_sub(&reward_tally)
            .ok_or(Error::<T>::ArithmeticUnderflow)?
            .into_inner()
            .checked_div(&SignedFixedPoint::<T>::accuracy())
            .ok_or(Error::<T>::ArithmeticUnderflow)?;
        Ok(cmp::max(Zero::zero(), reward))
    }

    fn apply_slash(
        currency_id: T::CurrencyId,
        vault_id: &T::AccountId,
        nominator_id: &T::AccountId,
    ) -> Result<SignedFixedPoint<T>, DispatchError> {
        let nonce = Self::nonce(currency_id, vault_id);
        let stake = Self::stake(nonce, currency_id, vault_id, nominator_id);
        let slash_per_token = Self::slash_per_token(currency_id, (nonce, vault_id));
        let slash_tally = Self::slash_tally(nonce, currency_id, vault_id, nominator_id);
        let to_slash = Self::compute_amount_to_slash(stake, slash_per_token, slash_tally)?;

        checked_sub_mut!(TotalStake<T>, currency_id, (nonce, vault_id), &to_slash);

        let stake = checked_sub_mut!(Stake<T>, currency_id, (nonce, vault_id, nominator_id), &to_slash);
        <SlashTally<T>>::insert(
            currency_id,
            (nonce, vault_id, nominator_id),
            stake
                .checked_mul(&slash_per_token)
                .ok_or(Error::<T>::ArithmeticOverflow)?,
        );

        return Ok(stake);
    }

    /// Withdraw an `amount` of stake from the `vault_id` for the `nominator_id`.
    pub fn withdraw_stake(
        currency_id: T::CurrencyId,
        vault_id: &T::AccountId,
        nominator_id: &T::AccountId,
        amount: SignedFixedPoint<T>,
    ) -> Result<(), DispatchError> {
        let nonce = Self::nonce(currency_id, vault_id);
        let stake = Self::apply_slash(currency_id, vault_id, nominator_id)?;
        if amount > stake {
            return Err(Error::<T>::InsufficientFunds.into());
        }

        checked_sub_mut!(Stake<T>, currency_id, (nonce, vault_id, nominator_id), &amount);
        checked_sub_mut!(TotalStake<T>, currency_id, (nonce, vault_id), &amount);
        checked_sub_mut!(TotalCurrentStake<T>, currency_id, (nonce, vault_id), &amount);

        <RewardTally<T>>::mutate(currency_id, (nonce, vault_id, nominator_id), |reward_tally| {
            let reward_per_token = Self::reward_per_token(currency_id, (nonce, vault_id));
            let reward_per_token_mul_amount = reward_per_token
                .checked_mul(&amount)
                .ok_or(Error::<T>::ArithmeticOverflow)?;

            *reward_tally = reward_tally
                .checked_sub(&reward_per_token_mul_amount)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;
            Ok::<_, Error<T>>(())
        })?;

        <SlashTally<T>>::mutate(currency_id, (nonce, vault_id, nominator_id), |slash_tally| {
            let slash_per_token = Self::slash_per_token(currency_id, (nonce, vault_id));
            let slash_per_token_mul_amount = slash_per_token
                .checked_mul(&amount)
                .ok_or(Error::<T>::ArithmeticOverflow)?;

            *slash_tally = slash_tally
                .checked_sub(&slash_per_token_mul_amount)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;
            Ok::<_, Error<T>>(())
        })?;

        Self::deposit_event(Event::<T>::WithdrawStake(
            currency_id,
            vault_id.clone(),
            nominator_id.clone(),
            amount,
        ));
        Ok(())
    }

    /// Delegates to `withdraw_reward_at_index` with the current nonce.
    pub fn withdraw_reward(
        currency_id: T::CurrencyId,
        vault_id: &T::AccountId,
        nominator_id: &T::AccountId,
    ) -> Result<<SignedFixedPoint<T> as FixedPointNumber>::Inner, DispatchError> {
        let nonce = Self::nonce(currency_id, vault_id);
        Self::withdraw_reward_at_index(nonce, currency_id, vault_id, nominator_id)
    }

    /// Withdraw all rewards earned by `vault_id` for the `nominator_id`.
    pub fn withdraw_reward_at_index(
        nonce: T::Index,
        currency_id: T::CurrencyId,
        vault_id: &T::AccountId,
        nominator_id: &T::AccountId,
    ) -> Result<<SignedFixedPoint<T> as FixedPointNumber>::Inner, DispatchError> {
        let reward = Self::compute_reward_at_index(nonce, currency_id, vault_id, nominator_id)?;
        let reward_as_fixed = SignedFixedPoint::<T>::checked_from_integer(reward).ok_or(Error::<T>::TryIntoIntError)?;
        checked_sub_mut!(TotalRewards<T>, currency_id, (nonce, vault_id), &reward_as_fixed);

        let stake = Self::stake(nonce, currency_id, vault_id, nominator_id);
        let reward_per_token = Self::reward_per_token(currency_id, (nonce, vault_id));
        <RewardTally<T>>::insert(
            currency_id,
            (nonce, vault_id, nominator_id),
            stake
                .checked_mul(&reward_per_token)
                .ok_or(Error::<T>::ArithmeticOverflow)?,
        );
        // TODO: transfer from `staking_pool(nonce)` to `nominator_id`
        Self::deposit_event(Event::<T>::WithdrawReward(
            nonce,
            currency_id,
            vault_id.clone(),
            nominator_id.clone(),
            reward_as_fixed,
        ));
        Ok(reward)
    }

    /// Force refund the entire nomination to `vault_id` by depositing it as reward.
    pub fn force_refund(currency_id: T::CurrencyId, vault_id: &T::AccountId) -> Result<(), DispatchError> {
        let nonce = Self::nonce(currency_id, vault_id);
        let total_current_stake = Self::total_current_stake_at_index(currency_id, (nonce, vault_id));
        // TODO: transfer `total_current_stake` from vault_id to `staking_pool(nonce)`
        Self::distribute_reward(currency_id, vault_id, total_current_stake)?;
        let reward = Self::withdraw_reward_at_index(nonce, currency_id, vault_id, vault_id)?;
        let reward_as_fixed = SignedFixedPoint::<T>::checked_from_integer(reward).ok_or(Error::<T>::TryIntoIntError)?;
        Self::increment_nonce(currency_id, vault_id)?;
        // Only re-deposit vault stake after increasing the nonce, so that it goes
        // in the new nonce's "pool".
        Self::deposit_stake(currency_id, vault_id, vault_id, reward_as_fixed)?;
        Self::deposit_event(Event::<T>::ForceRefund(currency_id, vault_id.clone()));
        Ok(())
    }

    pub fn increment_nonce(currency_id: T::CurrencyId, vault_id: &T::AccountId) -> DispatchResult {
        <Nonce<T>>::mutate(currency_id, vault_id, |nonce| {
            *nonce = nonce
                .checked_add(&T::Index::one())
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            Ok::<_, Error<T>>(())
        })?;
        Self::deposit_event(Event::<T>::IncreaseNonce(
            currency_id,
            vault_id.clone(),
            Self::nonce(currency_id, vault_id),
        ));
        Ok(())
    }
}

pub trait Staking<AccountId> {
    /// Signed fixed point type.
    type SignedFixedPoint: FixedPointNumber;

    /// Deposit an `amount` of stake to the `vault_id` for the `nominator_id`.
    fn deposit_stake(
        vault_id: &AccountId,
        nominator_id: &AccountId,
        amount: Self::SignedFixedPoint,
    ) -> Result<(), DispatchError>;

    /// Slash an `amount` of stake from the `vault_id`.
    fn slash_stake(vault_id: &AccountId, amount: Self::SignedFixedPoint) -> Result<(), DispatchError>;

    /// Compute the stake in `vault_id` owned by `nominator_id`.
    fn compute_stake(
        vault_id: &AccountId,
        nominator_id: &AccountId,
    ) -> Result<<Self::SignedFixedPoint as FixedPointNumber>::Inner, DispatchError>;

    /// Distribute the `reward` to all participants.
    fn distribute_reward(
        vault_id: &AccountId,
        reward: Self::SignedFixedPoint,
    ) -> Result<Self::SignedFixedPoint, DispatchError>;

    /// Compute the expected reward for `nominator_id` who is nominating `vault_id`.
    fn compute_reward(
        vault_id: &AccountId,
        nominator_id: &AccountId,
    ) -> Result<<Self::SignedFixedPoint as FixedPointNumber>::Inner, DispatchError>;

    /// Withdraw an `amount` of stake from the `vault_id` for the `nominator_id`.
    fn withdraw_stake(
        vault_id: &AccountId,
        nominator_id: &AccountId,
        amount: Self::SignedFixedPoint,
    ) -> Result<(), DispatchError>;

    /// Withdraw all rewards earned by `vault_id` for the `nominator_id`.
    fn withdraw_reward(
        vault_id: &AccountId,
        nominator_id: &AccountId,
    ) -> Result<<Self::SignedFixedPoint as FixedPointNumber>::Inner, DispatchError>;
}

pub struct StakingCurrencyAdapter<T, GetCurrencyId>(PhantomData<(T, GetCurrencyId)>);

impl<T, GetCurrencyId> Staking<T::AccountId> for StakingCurrencyAdapter<T, GetCurrencyId>
where
    T: Config,
    GetCurrencyId: Get<T::CurrencyId>,
{
    type SignedFixedPoint = SignedFixedPoint<T>;

    fn deposit_stake(
        vault_id: &T::AccountId,
        nominator_id: &T::AccountId,
        amount: Self::SignedFixedPoint,
    ) -> Result<(), DispatchError> {
        Pallet::<T>::deposit_stake(GetCurrencyId::get(), vault_id, nominator_id, amount)
    }

    fn slash_stake(vault_id: &T::AccountId, amount: Self::SignedFixedPoint) -> Result<(), DispatchError> {
        Pallet::<T>::slash_stake(GetCurrencyId::get(), vault_id, amount)
    }

    fn compute_stake(
        vault_id: &T::AccountId,
        nominator_id: &T::AccountId,
    ) -> Result<<Self::SignedFixedPoint as FixedPointNumber>::Inner, DispatchError> {
        Pallet::<T>::compute_stake(GetCurrencyId::get(), vault_id, nominator_id)
    }

    fn distribute_reward(
        vault_id: &T::AccountId,
        reward: Self::SignedFixedPoint,
    ) -> Result<Self::SignedFixedPoint, DispatchError> {
        Pallet::<T>::distribute_reward(GetCurrencyId::get(), vault_id, reward)
    }

    fn compute_reward(
        vault_id: &T::AccountId,
        nominator_id: &T::AccountId,
    ) -> Result<<Self::SignedFixedPoint as FixedPointNumber>::Inner, DispatchError> {
        Pallet::<T>::compute_reward(GetCurrencyId::get(), vault_id, nominator_id)
    }

    fn withdraw_stake(
        vault_id: &T::AccountId,
        nominator_id: &T::AccountId,
        amount: Self::SignedFixedPoint,
    ) -> Result<(), DispatchError> {
        Pallet::<T>::withdraw_stake(GetCurrencyId::get(), vault_id, nominator_id, amount)
    }

    fn withdraw_reward(
        vault_id: &T::AccountId,
        nominator_id: &T::AccountId,
    ) -> Result<<Self::SignedFixedPoint as FixedPointNumber>::Inner, DispatchError> {
        Pallet::<T>::withdraw_reward(GetCurrencyId::get(), vault_id, nominator_id)
    }
}
