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
use primitives::{BalanceToFixedPoint, TruncateFixedPointToInt, VaultCurrencyPair, VaultId};
use scale_info::TypeInfo;
use sp_arithmetic::{FixedPointNumber, FixedPointOperand};
use sp_runtime::{
    traits::{CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, MaybeSerializeDeserialize, One, Zero},
    ArithmeticError,
};
use sp_std::{cmp, convert::TryInto};

pub(crate) type SignedFixedPoint<T> = <T as Config>::SignedFixedPoint;

pub use pallet::*;

pub type DefaultVaultId<T> = VaultId<<T as frame_system::Config>::AccountId, <T as Config>::CurrencyId>;
pub type DefaultVaultCurrencyPair<T> = VaultCurrencyPair<<T as Config>::CurrencyId>;
pub type NominatorId<T> = <T as frame_system::Config>::AccountId;

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
        type SignedFixedPoint: FixedPointNumber<Inner = Self::SignedInner>
            + TruncateFixedPointToInt
            + Encode
            + EncodeLike
            + Decode
            + TypeInfo;

        #[pallet::constant]
        type GetNativeCurrencyId: Get<Self::CurrencyId>;

        /// The currency ID type.
        type CurrencyId: Parameter + Member + Copy + MaybeSerializeDeserialize + Ord;
    }

    // The pallet's events
    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        DepositStake {
            vault_id: DefaultVaultId<T>,
            nominator_id: T::AccountId,
            amount: T::SignedFixedPoint,
        },
        DistributeReward {
            currency_id: T::CurrencyId,
            vault_id: DefaultVaultId<T>,
            amount: T::SignedFixedPoint,
        },
        WithdrawStake {
            vault_id: DefaultVaultId<T>,
            nominator_id: T::AccountId,
            amount: T::SignedFixedPoint,
        },
        WithdrawReward {
            nonce: T::Index,
            currency_id: T::CurrencyId,
            vault_id: DefaultVaultId<T>,
            nominator_id: T::AccountId,
            amount: T::SignedFixedPoint,
        },
        ForceRefund {
            vault_id: DefaultVaultId<T>,
        },
        IncreaseNonce {
            vault_id: DefaultVaultId<T>,
            new_nonce: T::Index,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Unable to convert value.
        TryIntoIntError,
        /// Balance not sufficient to withdraw stake.
        InsufficientFunds,
        /// Cannot slash zero total stake.
        SlashZeroTotalStake,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

    /// The total stake - this will increase on deposit and decrease on withdrawal.
    #[pallet::storage]
    #[pallet::getter(fn total_stake_at_index)]
    pub type TotalStake<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::Index,
        Blake2_128Concat,
        DefaultVaultId<T>,
        SignedFixedPoint<T>,
        ValueQuery,
    >;

    /// The total stake - this will increase on deposit and decrease on withdrawal or slashing.
    #[pallet::storage]
    #[pallet::getter(fn total_current_stake_at_index)]
    pub type TotalCurrentStake<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::Index,
        Blake2_128Concat,
        DefaultVaultId<T>,
        SignedFixedPoint<T>,
        ValueQuery,
    >;

    /// The total unclaimed rewards distributed to this reward pool.
    /// NOTE: this is currently only used for integration tests.
    // TODO: conditionally compile this
    #[pallet::storage]
    #[pallet::getter(fn total_rewards)]
    pub type TotalRewards<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::CurrencyId,
        Blake2_128Concat,
        (T::Index, DefaultVaultId<T>),
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
        (T::Index, DefaultVaultId<T>),
        SignedFixedPoint<T>,
        ValueQuery,
    >;

    /// Used to compute the amount to slash from a participant's stake.
    #[pallet::storage]
    pub type SlashPerToken<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::Index,
        Blake2_128Concat,
        DefaultVaultId<T>,
        SignedFixedPoint<T>,
        ValueQuery,
    >;

    /// The stake of a participant in this reward pool.
    #[pallet::storage]
    pub type Stake<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::Index,
        Blake2_128Concat,
        (DefaultVaultId<T>, T::AccountId),
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
        (T::Index, DefaultVaultId<T>, T::AccountId),
        SignedFixedPoint<T>,
        ValueQuery,
    >;

    /// Accounts for previous changes in stake size.
    #[pallet::storage]
    pub type SlashTally<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::Index,
        Blake2_128Concat,
        (DefaultVaultId<T>, T::AccountId),
        SignedFixedPoint<T>,
        ValueQuery,
    >;

    /// The nonce of the current staking pool, used in force refunds.
    /// This is a strictly increasing value.
    #[pallet::storage]
    pub type Nonce<T: Config> = StorageMap<_, Blake2_128Concat, DefaultVaultId<T>, T::Index, ValueQuery>;

    #[pallet::pallet]
    #[pallet::without_storage_info] // no MaxEncodedLen for <T as frame_system::Config>::Index
    pub struct Pallet<T>(_);

    // The pallet's dispatchable functions.
    #[pallet::call]
    impl<T: Config> Pallet<T> {}
}

macro_rules! checked_add_mut {
    ($storage:ty, $currency:expr, $amount:expr) => {
        <$storage>::mutate($currency, |value| {
            *value = value.checked_add($amount).ok_or(ArithmeticError::Overflow)?;
            Ok::<_, DispatchError>(*value)
        })?
    };
    ($storage:ty, $currency:expr, $account:expr, $amount:expr) => {
        <$storage>::mutate($currency, $account, |value| {
            *value = value.checked_add($amount).ok_or(ArithmeticError::Overflow)?;
            Ok::<_, DispatchError>(*value)
        })?
    };
}

macro_rules! checked_sub_mut {
    ($storage:ty, $currency:expr, $amount:expr) => {
        <$storage>::mutate($currency, |value| {
            *value = value.checked_sub($amount).ok_or(ArithmeticError::Underflow)?;
            Ok::<_, DispatchError>(*value)
        })?
    };
    ($storage:ty, $currency:expr, $account:expr, $amount:expr) => {
        <$storage>::mutate($currency, $account, |value| {
            *value = value.checked_sub($amount).ok_or(ArithmeticError::Underflow)?;
            Ok::<_, DispatchError>(*value)
        })?
    };
}

// "Internal" functions, callable by code.
impl<T: Config> Pallet<T> {
    /// Get the stake associated with a vault / nominator.
    pub fn stake(vault_id: &DefaultVaultId<T>, nominator_id: &T::AccountId) -> SignedFixedPoint<T> {
        let nonce = Self::nonce(vault_id);
        Self::stake_at_index(nonce, vault_id, nominator_id)
    }

    fn stake_at_index(
        nonce: T::Index,
        vault_id: &DefaultVaultId<T>,
        nominator_id: &T::AccountId,
    ) -> SignedFixedPoint<T> {
        <Stake<T>>::get(nonce, (vault_id, nominator_id))
    }

    /// Get the total stake *after* slashing.
    pub fn total_current_stake(
        vault_id: &DefaultVaultId<T>,
    ) -> Result<<SignedFixedPoint<T> as FixedPointNumber>::Inner, DispatchError> {
        let nonce = Self::nonce(vault_id);
        let total = Self::total_current_stake_at_index(nonce, vault_id);
        total.truncate_to_inner().ok_or(Error::<T>::TryIntoIntError.into())
    }

    fn reward_tally(
        nonce: T::Index,
        currency_id: T::CurrencyId,
        vault_id: &DefaultVaultId<T>,
        nominator_id: &T::AccountId,
    ) -> SignedFixedPoint<T> {
        <RewardTally<T>>::get(currency_id, (nonce, vault_id, nominator_id))
    }

    /// Get the nominator's `slash_tally` for the staking pool.
    pub fn slash_tally(vault_id: &DefaultVaultId<T>, nominator_id: &T::AccountId) -> SignedFixedPoint<T> {
        let nonce = Self::nonce(vault_id);
        Self::slash_tally_at_index(nonce, vault_id, nominator_id)
    }

    fn slash_tally_at_index(
        nonce: T::Index,
        vault_id: &DefaultVaultId<T>,
        nominator_id: &T::AccountId,
    ) -> SignedFixedPoint<T> {
        <SlashTally<T>>::get(nonce, (vault_id, nominator_id))
    }

    /// Get the newest nonce for the staking pool.
    pub fn nonce(vault_id: &DefaultVaultId<T>) -> T::Index {
        <Nonce<T>>::get(vault_id)
    }

    /// Get the vault's `slash_per_token` for the staking pool.
    pub fn slash_per_token(vault_id: &DefaultVaultId<T>) -> SignedFixedPoint<T> {
        let nonce = Self::nonce(vault_id);
        Self::slash_per_token_at_index(nonce, vault_id)
    }

    fn slash_per_token_at_index(nonce: T::Index, vault_id: &DefaultVaultId<T>) -> SignedFixedPoint<T> {
        <SlashPerToken<T>>::get(nonce, vault_id)
    }

    /// Deposit an `amount` of stake to the `vault_id` for the `nominator_id`.
    pub fn deposit_stake(
        vault_id: &DefaultVaultId<T>,
        nominator_id: &T::AccountId,
        amount: SignedFixedPoint<T>,
    ) -> DispatchResult {
        let nonce = Self::nonce(vault_id);
        Self::apply_slash(vault_id, nominator_id)?;

        checked_add_mut!(Stake<T>, nonce, (vault_id, nominator_id), &amount);
        checked_add_mut!(TotalStake<T>, nonce, vault_id, &amount);
        checked_add_mut!(TotalCurrentStake<T>, nonce, vault_id, &amount);

        <SlashTally<T>>::mutate(nonce, (vault_id, nominator_id), |slash_tally| {
            let slash_per_token = Self::slash_per_token_at_index(nonce, vault_id);
            let slash_per_token_mul_amount = slash_per_token.checked_mul(&amount).ok_or(ArithmeticError::Overflow)?;
            *slash_tally = slash_tally
                .checked_add(&slash_per_token_mul_amount)
                .ok_or(ArithmeticError::Overflow)?;
            Ok::<_, DispatchError>(())
        })?;

        for currency_id in [vault_id.wrapped_currency(), T::GetNativeCurrencyId::get()] {
            <RewardTally<T>>::mutate(currency_id, (nonce, vault_id, nominator_id), |reward_tally| {
                let reward_per_token = Self::reward_per_token(currency_id, (nonce, vault_id));
                let reward_per_token_mul_amount =
                    reward_per_token.checked_mul(&amount).ok_or(ArithmeticError::Overflow)?;
                *reward_tally = reward_tally
                    .checked_add(&reward_per_token_mul_amount)
                    .ok_or(ArithmeticError::Overflow)?;
                Ok::<_, DispatchError>(())
            })?;
        }

        Self::deposit_event(Event::<T>::DepositStake {
            vault_id: vault_id.clone(),
            nominator_id: nominator_id.clone(),
            amount,
        });
        Ok(())
    }

    /// Slash an `amount` of stake from the `vault_id`.
    pub fn slash_stake(
        currency_id: T::CurrencyId,
        vault_id: &DefaultVaultId<T>,
        amount: SignedFixedPoint<T>,
    ) -> DispatchResult {
        let nonce = Self::nonce(vault_id);
        let total_stake = Self::total_stake_at_index(nonce, vault_id);
        if amount.is_zero() {
            return Ok(());
        } else if total_stake.is_zero() {
            return Err(Error::<T>::SlashZeroTotalStake.into());
        }

        let amount_div_total_stake = amount.checked_div(&total_stake).ok_or(ArithmeticError::Underflow)?;
        checked_add_mut!(SlashPerToken<T>, nonce, vault_id, &amount_div_total_stake);

        checked_sub_mut!(TotalCurrentStake<T>, nonce, vault_id, &amount);
        // A slash means reward per token is no longer representative of the rewards
        // since `amount * reward_per_token` will be lost from the system. As such,
        // replenish rewards by the amount of reward lost with this slash
        Self::increase_rewards(
            nonce,
            currency_id,
            vault_id,
            Self::reward_per_token(currency_id, (nonce, vault_id))
                .checked_mul(&amount)
                .ok_or(ArithmeticError::Overflow)?,
        )?;
        Ok(())
    }

    fn compute_amount_to_slash(
        stake: SignedFixedPoint<T>,
        slash_per_token: SignedFixedPoint<T>,
        slash_tally: SignedFixedPoint<T>,
    ) -> Result<SignedFixedPoint<T>, DispatchError> {
        let stake_mul_slash_per_token = stake.checked_mul(&slash_per_token).ok_or(ArithmeticError::Overflow)?;

        let to_slash = stake_mul_slash_per_token
            .checked_sub(&slash_tally)
            .ok_or(ArithmeticError::Underflow)?;

        Ok(to_slash)
    }

    /// Delegates to `compute_stake_at_index` with the current nonce.
    pub fn compute_stake(
        vault_id: &DefaultVaultId<T>,
        nominator_id: &T::AccountId,
    ) -> Result<<SignedFixedPoint<T> as FixedPointNumber>::Inner, DispatchError> {
        let nonce = Self::nonce(vault_id);
        Self::compute_stake_at_index(nonce, vault_id, nominator_id)
    }

    /// Compute the stake in `vault_id` owned by `nominator_id`.
    pub fn compute_stake_at_index(
        nonce: T::Index,
        vault_id: &DefaultVaultId<T>,
        nominator_id: &T::AccountId,
    ) -> Result<<SignedFixedPoint<T> as FixedPointNumber>::Inner, DispatchError> {
        let stake = Self::stake_at_index(nonce, vault_id, nominator_id);
        let slash_per_token = Self::slash_per_token_at_index(nonce, vault_id);
        let slash_tally = Self::slash_tally_at_index(nonce, vault_id, nominator_id);
        let to_slash = Self::compute_amount_to_slash(stake, slash_per_token, slash_tally)?;

        let stake_sub_to_slash = stake
            .checked_sub(&to_slash)
            .ok_or(ArithmeticError::Underflow)?
            .truncate_to_inner()
            .ok_or(Error::<T>::TryIntoIntError)?;
        Ok(cmp::max(Zero::zero(), stake_sub_to_slash))
    }

    fn increase_rewards(
        nonce: T::Index,
        currency_id: T::CurrencyId,
        vault_id: &DefaultVaultId<T>,
        reward: SignedFixedPoint<T>,
    ) -> Result<SignedFixedPoint<T>, DispatchError> {
        let total_current_stake = Self::total_current_stake_at_index(nonce, vault_id);
        if total_current_stake.is_zero() {
            return Ok(Zero::zero());
        }

        let reward_div_total_current_stake = reward
            .checked_div(&total_current_stake)
            .ok_or(ArithmeticError::Underflow)?;
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
        vault_id: &DefaultVaultId<T>,
        reward: SignedFixedPoint<T>,
    ) -> Result<SignedFixedPoint<T>, DispatchError> {
        let nonce = Self::nonce(vault_id);

        let reward = Self::increase_rewards(nonce, currency_id, vault_id, reward)?;
        if reward.is_zero() {
            return Ok(Zero::zero());
        }
        checked_add_mut!(TotalRewards<T>, currency_id, (nonce, vault_id), &reward);

        Self::deposit_event(Event::<T>::DistributeReward {
            currency_id,
            vault_id: vault_id.clone(),
            amount: reward,
        });
        Ok(reward)
    }

    /// Delegates to `compute_reward_at_index` with the current nonce.
    pub fn compute_reward(
        currency_id: T::CurrencyId,
        vault_id: &DefaultVaultId<T>,
        nominator_id: &T::AccountId,
    ) -> Result<<SignedFixedPoint<T> as FixedPointNumber>::Inner, DispatchError> {
        let nonce = Self::nonce(vault_id);
        Self::compute_reward_at_index(nonce, currency_id, vault_id, nominator_id)
    }

    /// Compute the expected reward for `nominator_id` who is nominating `vault_id`.
    pub fn compute_reward_at_index(
        nonce: T::Index,
        currency_id: T::CurrencyId,
        vault_id: &DefaultVaultId<T>,
        nominator_id: &T::AccountId,
    ) -> Result<<SignedFixedPoint<T> as FixedPointNumber>::Inner, DispatchError> {
        let stake =
            SignedFixedPoint::<T>::checked_from_integer(Self::compute_stake_at_index(nonce, vault_id, nominator_id)?)
                .ok_or(Error::<T>::TryIntoIntError)?;
        let reward_per_token = Self::reward_per_token(currency_id, (nonce, vault_id));
        // FIXME: this can easily overflow with large numbers
        let stake_mul_reward_per_token = stake.checked_mul(&reward_per_token).ok_or(ArithmeticError::Overflow)?;
        let reward_tally = Self::reward_tally(nonce, currency_id, vault_id, nominator_id);
        // TODO: this can probably be saturated
        let reward = stake_mul_reward_per_token
            .checked_sub(&reward_tally)
            .ok_or(ArithmeticError::Underflow)?
            .truncate_to_inner()
            .ok_or(Error::<T>::TryIntoIntError)?;
        Ok(cmp::max(Zero::zero(), reward))
    }

    fn apply_slash(
        vault_id: &DefaultVaultId<T>,
        nominator_id: &T::AccountId,
    ) -> Result<SignedFixedPoint<T>, DispatchError> {
        let nonce = Self::nonce(vault_id);
        let stake = Self::stake_at_index(nonce, vault_id, nominator_id);
        let slash_per_token = Self::slash_per_token_at_index(nonce, vault_id);
        let slash_tally = Self::slash_tally_at_index(nonce, vault_id, nominator_id);
        let to_slash = Self::compute_amount_to_slash(stake, slash_per_token, slash_tally)?;

        checked_sub_mut!(TotalStake<T>, nonce, vault_id, &to_slash);

        let stake = checked_sub_mut!(Stake<T>, nonce, (vault_id, nominator_id), &to_slash);
        <SlashTally<T>>::insert(
            nonce,
            (vault_id, nominator_id),
            stake.checked_mul(&slash_per_token).ok_or(ArithmeticError::Overflow)?,
        );

        return Ok(stake);
    }

    /// Withdraw an `amount` of stake from the `vault_id` for the `nominator_id`.
    pub fn withdraw_stake(
        vault_id: &DefaultVaultId<T>,
        nominator_id: &T::AccountId,
        amount: SignedFixedPoint<T>,
        index: Option<T::Index>,
    ) -> DispatchResult {
        let nonce = index.unwrap_or(Self::nonce(vault_id));
        let stake = Self::apply_slash(vault_id, nominator_id)?;

        if amount.is_zero() {
            return Ok(());
        } else if amount > stake {
            return Err(Error::<T>::InsufficientFunds.into());
        }

        checked_sub_mut!(Stake<T>, nonce, (vault_id, nominator_id), &amount);
        checked_sub_mut!(TotalStake<T>, nonce, vault_id, &amount);
        checked_sub_mut!(TotalCurrentStake<T>, nonce, vault_id, &amount);

        <SlashTally<T>>::mutate(nonce, (vault_id, nominator_id), |slash_tally| {
            let slash_per_token = Self::slash_per_token_at_index(nonce, vault_id);
            let slash_per_token_mul_amount = slash_per_token.checked_mul(&amount).ok_or(ArithmeticError::Overflow)?;

            *slash_tally = slash_tally
                .checked_sub(&slash_per_token_mul_amount)
                .ok_or(ArithmeticError::Underflow)?;
            Ok::<_, DispatchError>(())
        })?;

        for currency_id in [vault_id.wrapped_currency(), T::GetNativeCurrencyId::get()] {
            <RewardTally<T>>::mutate(currency_id, (nonce, vault_id, nominator_id), |reward_tally| {
                let reward_per_token = Self::reward_per_token(currency_id, (nonce, vault_id));
                let reward_per_token_mul_amount =
                    reward_per_token.checked_mul(&amount).ok_or(ArithmeticError::Overflow)?;

                *reward_tally = reward_tally
                    .checked_sub(&reward_per_token_mul_amount)
                    .ok_or(ArithmeticError::Underflow)?;
                Ok::<_, DispatchError>(())
            })?;
        }

        Self::deposit_event(Event::<T>::WithdrawStake {
            vault_id: vault_id.clone(),
            nominator_id: nominator_id.clone(),
            amount,
        });
        Ok(())
    }

    /// Delegates to `withdraw_reward_at_index` with the current nonce.
    pub fn withdraw_reward(
        currency_id: T::CurrencyId,
        vault_id: &DefaultVaultId<T>,
        nominator_id: &T::AccountId,
    ) -> Result<<SignedFixedPoint<T> as FixedPointNumber>::Inner, DispatchError> {
        let nonce = Self::nonce(vault_id);
        Self::withdraw_reward_at_index(nonce, currency_id, vault_id, nominator_id)
    }

    /// Withdraw all rewards earned by `vault_id` for the `nominator_id`.
    pub fn withdraw_reward_at_index(
        nonce: T::Index,
        currency_id: T::CurrencyId,
        vault_id: &DefaultVaultId<T>,
        nominator_id: &T::AccountId,
    ) -> Result<<SignedFixedPoint<T> as FixedPointNumber>::Inner, DispatchError> {
        let reward = Self::compute_reward_at_index(nonce, currency_id, vault_id, nominator_id)?;
        let reward_as_fixed = SignedFixedPoint::<T>::checked_from_integer(reward).ok_or(Error::<T>::TryIntoIntError)?;
        checked_sub_mut!(TotalRewards<T>, currency_id, (nonce, vault_id), &reward_as_fixed);

        let stake = Self::stake_at_index(nonce, vault_id, nominator_id);
        let reward_per_token = Self::reward_per_token(currency_id, (nonce, vault_id));
        <RewardTally<T>>::insert(
            currency_id,
            (nonce, vault_id, nominator_id),
            stake.checked_mul(&reward_per_token).ok_or(ArithmeticError::Overflow)?,
        );
        Self::deposit_event(Event::<T>::WithdrawReward {
            nonce,
            currency_id,
            vault_id: vault_id.clone(),
            nominator_id: nominator_id.clone(),
            amount: reward_as_fixed,
        });
        Ok(reward)
    }

    /// Force refund the entire nomination to `vault_id` by depositing it as reward. It
    /// returns the amount of collateral that is refunded
    pub fn force_refund(
        vault_id: &DefaultVaultId<T>,
    ) -> Result<<SignedFixedPoint<T> as FixedPointNumber>::Inner, DispatchError> {
        let nonce = Self::nonce(vault_id);
        let total_current_stake = Self::total_current_stake_at_index(nonce, vault_id);

        // only withdraw the vault's stake from the current pool
        // nominators must withdraw manually using the nonce
        let stake = SignedFixedPoint::<T>::checked_from_integer(Self::compute_stake_at_index(
            nonce,
            vault_id,
            &vault_id.account_id,
        )?)
        .ok_or(Error::<T>::TryIntoIntError)?;
        Self::withdraw_stake(vault_id, &vault_id.account_id, stake, Some(nonce))?;
        Self::increment_nonce(vault_id)?;

        // only deposit vault stake after increasing the nonce
        Self::deposit_stake(vault_id, &vault_id.account_id, stake)?;
        Self::deposit_event(Event::<T>::ForceRefund {
            vault_id: vault_id.clone(),
        });

        let refunded_collateral = total_current_stake
            .checked_sub(&stake)
            .ok_or(ArithmeticError::Underflow)?
            .truncate_to_inner()
            .ok_or(Error::<T>::TryIntoIntError)?;

        Ok(refunded_collateral)
    }

    pub fn increment_nonce(vault_id: &DefaultVaultId<T>) -> DispatchResult {
        <Nonce<T>>::mutate(vault_id, |nonce| {
            *nonce = nonce.checked_add(&T::Index::one()).ok_or(ArithmeticError::Overflow)?;
            Ok::<_, DispatchError>(())
        })?;
        Self::deposit_event(Event::<T>::IncreaseNonce {
            vault_id: vault_id.clone(),
            new_nonce: Self::nonce(vault_id),
        });
        Ok(())
    }
}

pub trait Staking<VaultId, NominatorId, Index, Balance, CurrencyId> {
    /// Get the newest nonce for the staking pool.
    fn nonce(vault_id: &VaultId) -> Index;

    /// Deposit an `amount` of stake to the `vault_id` for the `nominator_id`.
    fn deposit_stake(vault_id: &VaultId, nominator_id: &NominatorId, amount: Balance) -> Result<(), DispatchError>;

    /// Slash an `amount` of stake from the `vault_id`.
    fn slash_stake(vault_id: &VaultId, amount: Balance, currency_id: CurrencyId) -> Result<(), DispatchError>;

    /// Compute the stake in `vault_id` owned by `nominator_id`.
    fn compute_stake(vault_id: &VaultId, nominator_id: &NominatorId) -> Result<Balance, DispatchError>;

    /// Compute the total stake in `vault_id` **after** slashing.
    fn total_stake(vault_id: &VaultId) -> Result<Balance, DispatchError>;

    /// Distribute the `reward` to all participants.
    fn distribute_reward(
        vault_id: &VaultId,
        reward: Balance,
        currency_id: CurrencyId,
    ) -> Result<Balance, DispatchError>;

    /// Compute the expected reward for `nominator_id` who is nominating `vault_id`.
    fn compute_reward(
        vault_id: &VaultId,
        nominator_id: &NominatorId,
        currency_id: CurrencyId,
    ) -> Result<Balance, DispatchError>;

    /// Withdraw an `amount` of stake from the `vault_id` for the `nominator_id`.
    fn withdraw_stake(
        vault_id: &VaultId,
        nominator_id: &NominatorId,
        amount: Balance,
        index: Option<Index>,
    ) -> DispatchResult;

    /// Withdraw all rewards earned by `vault_id` for the `nominator_id`.
    fn withdraw_reward(
        vault_id: &VaultId,
        nominator_id: &NominatorId,
        index: Option<Index>,
        currency_id: CurrencyId,
    ) -> Result<Balance, DispatchError>;

    /// Force refund the entire nomination to `vault_id`.
    fn force_refund(vault_id: &VaultId) -> Result<Balance, DispatchError>;
}

impl<T, Balance> Staking<DefaultVaultId<T>, T::AccountId, T::Index, Balance, T::CurrencyId> for Pallet<T>
where
    T: Config,

    Balance: BalanceToFixedPoint<SignedFixedPoint<T>>,
    <T::SignedFixedPoint as FixedPointNumber>::Inner: TryInto<Balance>,
{
    fn nonce(vault_id: &DefaultVaultId<T>) -> T::Index {
        Pallet::<T>::nonce(vault_id)
    }

    fn deposit_stake(vault_id: &DefaultVaultId<T>, nominator_id: &T::AccountId, amount: Balance) -> DispatchResult {
        Pallet::<T>::deposit_stake(
            vault_id,
            nominator_id,
            amount.to_fixed().ok_or(Error::<T>::TryIntoIntError)?,
        )
    }

    fn slash_stake(vault_id: &DefaultVaultId<T>, amount: Balance, currency_id: T::CurrencyId) -> DispatchResult {
        Pallet::<T>::slash_stake(
            currency_id,
            vault_id,
            amount.to_fixed().ok_or(Error::<T>::TryIntoIntError)?,
        )
    }

    fn compute_stake(vault_id: &DefaultVaultId<T>, nominator_id: &T::AccountId) -> Result<Balance, DispatchError> {
        Pallet::<T>::compute_stake(vault_id, nominator_id)?
            .try_into()
            .map_err(|_| Error::<T>::TryIntoIntError.into())
    }

    fn total_stake(vault_id: &DefaultVaultId<T>) -> Result<Balance, DispatchError> {
        Pallet::<T>::total_current_stake(vault_id)?
            .try_into()
            .map_err(|_| Error::<T>::TryIntoIntError.into())
    }

    fn distribute_reward(
        vault_id: &DefaultVaultId<T>,
        amount: Balance,
        currency_id: T::CurrencyId,
    ) -> Result<Balance, DispatchError> {
        Pallet::<T>::distribute_reward(
            currency_id,
            vault_id,
            amount.to_fixed().ok_or(Error::<T>::TryIntoIntError)?,
        )?
        .truncate_to_inner()
        .ok_or(Error::<T>::TryIntoIntError)?
        .try_into()
        .map_err(|_| Error::<T>::TryIntoIntError.into())
    }

    fn compute_reward(
        vault_id: &DefaultVaultId<T>,
        nominator_id: &T::AccountId,
        currency_id: T::CurrencyId,
    ) -> Result<Balance, DispatchError> {
        Pallet::<T>::compute_reward(currency_id, vault_id, nominator_id)?
            .try_into()
            .map_err(|_| Error::<T>::TryIntoIntError.into())
    }

    fn withdraw_stake(
        vault_id: &DefaultVaultId<T>,
        nominator_id: &T::AccountId,
        amount: Balance,
        index: Option<T::Index>,
    ) -> DispatchResult {
        Pallet::<T>::withdraw_stake(
            vault_id,
            nominator_id,
            amount.to_fixed().ok_or(Error::<T>::TryIntoIntError)?,
            index,
        )
    }

    fn withdraw_reward(
        vault_id: &DefaultVaultId<T>,
        nominator_id: &T::AccountId,
        index: Option<T::Index>,
        currency_id: T::CurrencyId,
    ) -> Result<Balance, DispatchError> {
        let nonce = index.unwrap_or(Pallet::<T>::nonce(vault_id));
        Pallet::<T>::withdraw_reward_at_index(nonce, currency_id, vault_id, nominator_id)?
            .try_into()
            .map_err(|_| Error::<T>::TryIntoIntError.into())
    }

    fn force_refund(vault_id: &DefaultVaultId<T>) -> Result<Balance, DispatchError> {
        Pallet::<T>::force_refund(vault_id)?
            .try_into()
            .map_err(|_| Error::<T>::TryIntoIntError.into())
    }
}
