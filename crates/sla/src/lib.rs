//! # PolkaBTC SLA Pallet

// #![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(test)]
extern crate mocktopus;

#[cfg(test)]
use mocktopus::macros::mockable;

mod ext;
pub mod types;

use crate::types::{RelayerEvent, VaultEvent};
use codec::{Decode, Encode, EncodeLike};
use frame_support::traits::Currency;
use frame_support::{decl_error, decl_event, decl_module, decl_storage, dispatch::DispatchError};
use sp_arithmetic::traits::*;
use sp_arithmetic::FixedPointNumber;
use sp_std::convert::TryInto;
use sp_std::vec::Vec;

pub(crate) type DOT<T> =
    <<T as collateral::Trait>::DOT as Currency<<T as frame_system::Trait>::AccountId>>::Balance;
pub(crate) type PolkaBTC<T> =
    <<T as treasury::Trait>::PolkaBTC as Currency<<T as frame_system::Trait>::AccountId>>::Balance;

pub(crate) type FixedPoint<T> = <T as Trait>::FixedPoint;

/// The pallet's configuration trait.
pub trait Trait:
    frame_system::Trait + collateral::Trait + vault_registry::Trait + treasury::Trait
{
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    type FixedPoint: FixedPointNumber + Encode + EncodeLike + Decode;
}

// The pallet's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as Sla {
        /// Mapping from accounts of vaults to their sla score
        VaultSla get(fn vault_sla): map hasher(blake2_128_concat) T::AccountId => T::FixedPoint;

        /// Mapping from accounts of vaults to their sla score
        RelayerSla get(fn relayer_sla): map hasher(blake2_128_concat) T::AccountId => T::FixedPoint;

        // total amount issued by each vault
        VaultTotalIssuedAmount get(fn vault_total_issued_amount): map hasher(blake2_128_concat) T::AccountId => PolkaBTC<T>;

        // total amount issued by all relayers together
        TotalIssuedAmount: PolkaBTC<T>;
        TotalIssueCount: u32;

        TotalRelayerScore: T::FixedPoint;

        VaultTargetSla get(fn vault_target_sla) config(): T::FixedPoint;
        VaultRedeemFailure get(fn vault_redeem_failure_sla_change) config(): T::FixedPoint;
        VaultExecutedIssueMaxSlaChange get(fn vault_executed_issue_max_sla_change) config(): T::FixedPoint;
        VaultSubmittedIssueProof get(fn vault_submitted_issue_proof) config(): T::FixedPoint;
        RelayerTargetSla get(fn relayer_target_sla) config(): T::FixedPoint;
        RelayerBlockSubmission get(fn relayer_block_submission) config(): T::FixedPoint;
        RelayerCorrectNoDataVoteOrReport get(fn relayer_correct_no_data_vote_or_report) config(): T::FixedPoint;
        RelayerCorrectInvalidVoteOrReport get(fn relayer_correct_invalid_vote_or_report) config(): T::FixedPoint;
        RelayerCorrectLiquidationReport get(fn relayer_correct_liquidation_report) config(): T::FixedPoint;
        RelayerCorrectTheftReport get(fn relayer_correct_theft_report) config(): T::FixedPoint;
        RelayerCorrectOracleOfflineReport get(fn relayer_correct_oracle_offline_report) config(): T::FixedPoint;
        RelayerFalseNoDataVoteOrReport get(fn relayer_false_no_data_vote_or_report) config(): T::FixedPoint;
        RelayerFalseInvalidVoteOrReport get(fn relayer_false_invalid_vote_or_report) config(): T::FixedPoint;
        RelayerIgnoredVote get(fn relayer_ignored_vote) config(): T::FixedPoint;
    }
}

// The pallet's events.
decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Trait>::AccountId,
        FixedPoint = FixedPoint<T>,
    {
        UpdateVaultSLA(AccountId, FixedPoint),
        UpdateRelayerSLA(AccountId, FixedPoint),
    }
);

// The pallet's dispatchable functions.
decl_module! {
    /// The module declaration.
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        // Initialize errors
        type Error = Error<T>;

        // Initialize events
        fn deposit_event() = default;
    }
}

// "Internal" functions, callable by code.
#[cfg_attr(test, mockable)]
impl<T: Trait> Module<T> {
    pub fn event_update_vault_sla(
        vault_id: T::AccountId,
        event: VaultEvent<PolkaBTC<T>>,
    ) -> Result<(), DispatchError> {
        let current_sla = <VaultSla<T>>::get(vault_id.clone());
        let delta_sla = match event {
            VaultEvent::RedeemFailure => <VaultRedeemFailure<T>>::get(),
            VaultEvent::SubmittedIssueProof => <VaultSubmittedIssueProof<T>>::get(),
            VaultEvent::ExecutedIssue(amount) => {
                // update account total
                let account_total = <VaultTotalIssuedAmount<T>>::get(vault_id.clone());
                <VaultTotalIssuedAmount<T>>::insert(vault_id.clone(), amount + account_total);

                // update total amount
                let total = <TotalIssuedAmount<T>>::mutate(|total| {
                    *total += amount;
                    *total
                });
                // update total count
                let count = <TotalIssueCount>::mutate(|count| {
                    *count += 1;
                    *count
                });

                let average = total / count.into();

                let max_sla_change = <VaultExecutedIssueMaxSlaChange<T>>::get();

                // increase = (amount / average) * max_sla_change
                let total_raw = Self::polkabtc_to_u128(total)?;
                let average_raw = Self::polkabtc_to_u128(average)?;

                let fraction = T::FixedPoint::checked_from_rational(total_raw, average_raw)
                    .ok_or(Error::<T>::TryIntoIntError)?;
                let potential_sla_increase = fraction
                    .checked_mul(&max_sla_change)
                    .ok_or(Error::<T>::MathError)?;

                Self::_limit(
                    T::FixedPoint::zero(),
                    potential_sla_increase,
                    max_sla_change,
                )
            }
        };

        let new_sla = current_sla
            .checked_add(&delta_sla)
            .ok_or(Error::<T>::MathError)?;
        let max_sla = <VaultTargetSla<T>>::get(); // todo: check that this is indeed the max

        let bounded_new_sla = Self::_limit(T::FixedPoint::zero(), new_sla, max_sla);

        <VaultSla<T>>::insert(vault_id.clone(), bounded_new_sla);
        Self::deposit_event(<Event<T>>::UpdateVaultSLA(vault_id, bounded_new_sla));

        Ok(())
    }

    // returns `value` if it is between `min` and `max`; otherwise it returns the bound
    fn _limit(min: T::FixedPoint, value: T::FixedPoint, max: T::FixedPoint) -> T::FixedPoint {
        if value < min {
            min
        } else if value > max {
            max
        } else {
            value
        }
    }

    fn _get_delta_sla(event: RelayerEvent) -> T::FixedPoint {
        match event {
            RelayerEvent::BlockSubmission => <RelayerBlockSubmission<T>>::get(),
            RelayerEvent::CorrectNoDataVoteOrReport => <RelayerCorrectNoDataVoteOrReport<T>>::get(),
            RelayerEvent::CorrectInvalidVoteOrReport => {
                <RelayerCorrectInvalidVoteOrReport<T>>::get()
            }
            RelayerEvent::CorrectLiquidationReport => <RelayerCorrectLiquidationReport<T>>::get(),
            RelayerEvent::CorrectTheftReport => <RelayerCorrectTheftReport<T>>::get(),
            RelayerEvent::CorrectOracleOfflineReport => {
                <RelayerCorrectOracleOfflineReport<T>>::get()
            }
            RelayerEvent::FalseNoDataVoteOrReport => <RelayerFalseNoDataVoteOrReport<T>>::get(),
            RelayerEvent::FalseInvalidVoteOrReport => <RelayerFalseInvalidVoteOrReport<T>>::get(),
            RelayerEvent::IgnoredVote => <RelayerIgnoredVote<T>>::get(),
        }
    }

    pub fn event_update_relayer_sla(
        relayer_id: T::AccountId,
        event: RelayerEvent,
    ) -> Result<(), DispatchError> {
        let current_sla = <RelayerSla<T>>::get(relayer_id.clone());
        let delta_sla = Self::_get_delta_sla(event);

        let max = <RelayerTargetSla<T>>::get(); // todo: check that this is indeed the max
        let min = T::FixedPoint::zero();

        let potential_new_sla = current_sla
            .checked_add(&delta_sla)
            .ok_or(Error::<T>::MathError)?;

        let new_sla = Self::_limit(min, potential_new_sla, max);

        if current_sla != new_sla {
            let stake = Self::_get_relayer_stake_as_fixed_point(relayer_id.clone())?;
            let total_relayer_score = <TotalRelayerScore<T>>::get();

            // todo: check if we can get problems with rounding errors
            let calculate_new_total_relayer_score = || {
                let actual_delta_sla = new_sla.checked_sub(&current_sla)?;
                // convert stake to fixed point
                let delta_score = actual_delta_sla.checked_mul(&stake)?;
                let new_total_relayer_score = total_relayer_score.checked_add(&delta_score)?;
                Some(new_total_relayer_score)
            };
            let new_total = calculate_new_total_relayer_score().ok_or(Error::<T>::MathError)?;

            <TotalRelayerScore<T>>::set(new_total);
            <RelayerSla<T>>::insert(relayer_id.clone(), new_sla);
            Self::deposit_event(<Event<T>>::UpdateRelayerSLA(relayer_id, new_sla));
        }

        Ok(())
    }

    fn _get_relayer_stake_as_fixed_point(
        relayer_id: T::AccountId,
    ) -> Result<T::FixedPoint, DispatchError> {
        let stake = ext::collateral::get_collateral_from_account::<T>(relayer_id.clone());
        let stake = Self::dot_to_u128(stake)?;
        let stake = T::FixedPoint::checked_from_rational(stake, 1u128)
            .ok_or(Error::<T>::TryIntoIntError)?;
        Ok(stake)
    }

    pub fn get_vault_rewards(
        total_reward_for_issued: PolkaBTC<T>,
        total_reward_for_locked: PolkaBTC<T>,
    ) -> Result<Vec<(T::AccountId, PolkaBTC<T>)>, DispatchError> {
        let total_issued = Self::polkabtc_to_u128(<TotalIssuedAmount<T>>::get())?;
        let total_locked = Self::dot_to_u128(ext::collateral::get_total_collateral::<T>())?;

        let total_reward_for_issued = Self::polkabtc_to_u128(total_reward_for_issued)?;
        let total_reward_for_locked = Self::polkabtc_to_u128(total_reward_for_locked)?;

        let calculate_reward = |account_id, issued_amount| {
            // each vault gets total_reward * (issued_amount / total_issued).
            let issued_amount = Self::polkabtc_to_u128(issued_amount)?;
            let issued_reward = issued_amount
                .checked_mul(total_reward_for_issued)
                .ok_or(Error::<T>::MathError)?
                .checked_div(total_issued)
                .ok_or(Error::<T>::MathError)?;

            let locked_amount = ext::collateral::get_collateral_from_account::<T>(account_id);
            let locked_amount = Self::dot_to_u128(locked_amount)?;
            let locked_reward = locked_amount
                .checked_mul(total_reward_for_locked)
                .ok_or(Error::<T>::MathError)?
                .checked_div(total_locked)
                .ok_or(Error::<T>::MathError)?;

            Result::<_, DispatchError>::Ok(Self::u128_to_polkabtc(issued_reward + locked_reward)?)
        };

        <VaultTotalIssuedAmount<T>>::iter()
            .map(|(account_id, relayer_issue_amount)| {
                Ok((
                    account_id.clone(),
                    calculate_reward(account_id.clone(), relayer_issue_amount)?,
                ))
            })
            .collect()
    }

    pub fn get_relayer_rewards(
        total_reward: PolkaBTC<T>,
    ) -> Result<Vec<(T::AccountId, PolkaBTC<T>)>, DispatchError> {
        <RelayerSla<T>>::iter()
            .map(|(account_id, _)| {
                Ok((
                    account_id.clone(),
                    Self::calculate_relayer_reward(account_id.clone(), total_reward)?,
                ))
            })
            .collect()
    }

    fn calculate_relayer_reward(
        relayer_id: T::AccountId,
        total_reward: PolkaBTC<T>,
    ) -> Result<PolkaBTC<T>, DispatchError> {
        let total_reward = Self::polkabtc_to_u128(total_reward)?;
        let stake = Self::_get_relayer_stake_as_fixed_point(relayer_id.clone())?;
        let sla = <RelayerSla<T>>::get(relayer_id);
        let total_relayer_score = <TotalRelayerScore<T>>::get();

        let calculate_reward = || {
            let score = stake.checked_mul(&sla)?;
            let share = score.checked_div(&total_relayer_score)?;
            let reward = share.checked_mul_int(total_reward)?;
            Some(reward)
        };
        let reward = calculate_reward().ok_or(Error::<T>::MathError)?;
        Self::u128_to_polkabtc(reward)
    }

    fn calculate_slashed_amount(
        vault_id: T::AccountId,
        stake: DOT<T>,
    ) -> Result<DOT<T>, DispatchError> {
        let current_sla = <VaultSla<T>>::get(vault_id);

        let liquidation_threshold = Self::get_liquidation_threshold()?;
        let premium_redeem_threshold = Self::get_premium_redeem_threshold()?;

        Self::_calculate_slashed_amount(
            current_sla,
            stake,
            liquidation_threshold,
            premium_redeem_threshold,
        )
    }

    /// fetch liquidation_threshold from vault registry and convert
    fn get_liquidation_threshold() -> Result<FixedPoint<T>, DispatchError> {
        let liquidation_threshold =
            ext::vault_registry::get_liquidation_collateral_threshold::<T>();
        Self::vault_registry_threshold_to_fixed_point(liquidation_threshold)
    }

    /// fetch premium_redeem_threshold from vault registry and convert
    fn get_premium_redeem_threshold() -> Result<FixedPoint<T>, DispatchError> {
        let premium_redeem_threshold = ext::vault_registry::get_premium_redeem_threshold::<T>();
        Self::vault_registry_threshold_to_fixed_point(premium_redeem_threshold)
    }

    /// Convert a threshold from set in the vault registry to a fixed point type
    fn vault_registry_threshold_to_fixed_point(
        value: u128,
    ) -> Result<FixedPoint<T>, DispatchError> {
        // TODO: use FixedPoint type in vault_registry
        let scaling_factor = 10u128.pow(vault_registry::GRANULARITY);
        let ret = T::FixedPoint::checked_from_rational(value, scaling_factor)
            .ok_or(Error::<T>::TryIntoIntError);
        Ok(ret?)
    }

    fn _calculate_slashed_amount(
        current_sla: FixedPoint<T>,
        stake: DOT<T>,
        liquidation_threshold: FixedPoint<T>,
        premium_redeem_threshold: FixedPoint<T>,
    ) -> Result<DOT<T>, DispatchError> {
        let range = premium_redeem_threshold - liquidation_threshold;
        // // todo: check that this is indeed the max
        let max_sla = <VaultTargetSla<T>>::get();
        let stake = Self::dot_to_u128(stake)?;

        // basic formula we use is:
        // result = stake * (premium_redeem_threshold - (current_sla / max_sla) * range);
        // however, we mutliply by (max_sla / max_sla) to eliminate one division operator:
        // result = stake * ((premium_redeem_threshold*max_sla - current_sla * range) / max_sla)
        // let stake_scaling_factor = premium_redeem_threshold * max_sla - current_sla * range
        let calculate_scaling_factor = || {
            // let numerator = premium_redeem_threshold*max_sla - current_sla*range;
            let numerator = T::FixedPoint::checked_sub(
                &premium_redeem_threshold.checked_mul(&max_sla)?,
                &current_sla.checked_mul(&range)?,
            )?;

            let stake_scaling_factor = numerator.checked_div(&max_sla)?;

            stake_scaling_factor.checked_mul_int(stake)
        };
        let slashed_raw = calculate_scaling_factor().ok_or(Error::<T>::MathError)?;
        Ok(Self::u128_to_dot(slashed_raw)?)
    }

    fn dot_to_u128(x: DOT<T>) -> Result<u128, DispatchError> {
        TryInto::<u128>::try_into(x).map_err(|_| Error::<T>::MathError.into())
    }

    fn u128_to_dot(x: u128) -> Result<DOT<T>, DispatchError> {
        TryInto::<DOT<T>>::try_into(x).map_err(|_| Error::<T>::TryIntoIntError.into())
    }

    fn polkabtc_to_u128(x: PolkaBTC<T>) -> Result<u128, DispatchError> {
        TryInto::<u128>::try_into(x).map_err(|_| Error::<T>::TryIntoIntError.into())
    }

    fn u128_to_polkabtc(x: u128) -> Result<PolkaBTC<T>, DispatchError> {
        TryInto::<PolkaBTC<T>>::try_into(x).map_err(|_| Error::<T>::TryIntoIntError.into())
    }
}

decl_error! {
    pub enum Error for Module<T: Trait> {
        MathError,
        TryIntoIntError
    }
}
