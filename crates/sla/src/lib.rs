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

use codec::{Decode, Encode, EncodeLike};
use frame_support::traits::Currency;
use frame_support::{decl_error, decl_event, decl_module, decl_storage, dispatch::DispatchError};
use sp_arithmetic::traits::*;
use sp_arithmetic::FixedPointNumber;
use sp_std::convert::TryInto;

pub(crate) type DOT<T> =
    <<T as collateral::Trait>::DOT as Currency<<T as frame_system::Trait>::AccountId>>::Balance;
pub(crate) type FixedPoint<T> = <T as Trait>::FixedPoint;

/// The pallet's configuration trait.
pub trait Trait: frame_system::Trait + collateral::Trait + vault_registry::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    type FixedPoint: FixedPointNumber + Encode + EncodeLike + Decode;
}

enum VaultEvent {
    RedeemFailure,
    ExecutedIssue(u32),
    SubmittedIssueProof,
}

enum RelayerEvent {
    BlockSubmission,
    CorrectNoDataVoteOrReport,
    CorrectInvalidVoteOrReport,
    CorrectLiquidationReport,
    CorrectTheftReport,
    CorrectOracleOfflineReport,
    FalseNoDataVoteOrReport,
    FalseInvalidVoteOrReport,
    IgnoredVote,
}

// The pallet's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as Sla {
        /// Mapping from accounts of vaults to their sla score
        VaultSla get(fn vault_sla): map hasher(blake2_128_concat) T::AccountId => T::FixedPoint;

        /// Mapping from accounts of vaults to their sla score
        RelayerSla get(fn relayer_sla): map hasher(blake2_128_concat) T::AccountId => T::FixedPoint;

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
    {
        SetAccount(AccountId),
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
    fn event_update_vault_sla(
        vault_id: T::AccountId,
        event: VaultEvent,
    ) -> Result<(), DispatchError> {
        let current_sla = <VaultSla<T>>::get(vault_id.clone());
        let delta_sla = match event {
            VaultEvent::RedeemFailure => <VaultRedeemFailure<T>>::get(),
            VaultEvent::SubmittedIssueProof => <VaultSubmittedIssueProof<T>>::get(),
            VaultEvent::ExecutedIssue(_) => <VaultExecutedIssueMaxSlaChange<T>>::get(), // todo: do calculation
        };

        let new_sla = current_sla
            .checked_add(&delta_sla)
            .ok_or(Error::<T>::MathError)?;
        let max_sla = <VaultTargetSla<T>>::get(); // todo: check that this is indeed the max

        let bounded_new_sla = Self::_limit(T::FixedPoint::zero(), new_sla, max_sla);

        <VaultSla<T>>::insert(vault_id, bounded_new_sla);

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
    fn event_update_relayer_sla(
        relayer_id: T::AccountId,
        event: RelayerEvent,
    ) -> Result<(), DispatchError> {
        let current_sla = <RelayerSla<T>>::get(relayer_id.clone());
        let delta_sla = match event {
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
        };

        let max = <RelayerTargetSla<T>>::get(); // todo: check that this is indeed the max
        let min = T::FixedPoint::zero(); // todo: check that this is indeed the max

        let potential_new_sla = current_sla
            .checked_add(&delta_sla)
            .ok_or(Error::<T>::MathError)?;

        let new_sla = Self::_limit(min, potential_new_sla, max);

        if current_sla != new_sla {
            <RelayerSla<T>>::insert(relayer_id, new_sla);
        }

        Ok(())
    }

    fn calculate_slashed_amount(
        vault_id: T::AccountId,
        stake: DOT<T>,
    ) -> Result<DOT<T>, DispatchError> {
        let current_sla = <VaultSla<T>>::get(vault_id);

        let liquidation_threshold = Self::_get_liquidation_threshold()?;
        let premium_redeem_threshold = Self::_get_premium_redeem_threshold()?;

        Self::_calculate_slashed_amount(
            current_sla,
            stake,
            liquidation_threshold,
            premium_redeem_threshold,
        )
    }

    /// fetch liquidation_threshold from vault registery and convert
    fn _get_liquidation_threshold() -> Result<FixedPoint<T>, DispatchError> {
        let liquidation_threshold =
            <vault_registry::Module<T>>::_get_liquidation_collateral_threshold();
        Self::_vault_registery_threshold_to_fixed_point(liquidation_threshold)
    }

    /// fetch premium_redeem_threshold from vault registery and convert
    fn _get_premium_redeem_threshold() -> Result<FixedPoint<T>, DispatchError> {
        let premium_redeem_threshold = <vault_registry::Module<T>>::_get_premium_redeem_threshold();
        Self::_vault_registery_threshold_to_fixed_point(premium_redeem_threshold)
    }

    /// Convert a threshold from set in the vault registry to a fixed point type
    fn _vault_registery_threshold_to_fixed_point(
        value: u128,
    ) -> Result<FixedPoint<T>, DispatchError> {
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
}

decl_error! {
    pub enum Error for Module<T: Trait> {
        MathError,
        TryIntoIntError
    }
}
