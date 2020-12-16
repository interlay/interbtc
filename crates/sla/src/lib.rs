//! # PolkaBTC SLA Pallet

#![deny(warnings)]
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

pub(crate) type SignedFixedPoint<T> = <T as Trait>::SignedFixedPoint;

/// The pallet's configuration trait.
pub trait Trait: frame_system::Trait + collateral::Trait + treasury::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    type SignedFixedPoint: FixedPointNumber + Encode + EncodeLike + Decode;
}

// The pallet's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as Sla {
        /// Mapping from accounts of vaults/relayers to their sla score
        VaultSla get(fn vault_sla): map hasher(blake2_128_concat) T::AccountId => T::SignedFixedPoint;
        RelayerSla get(fn relayer_sla): map hasher(blake2_128_concat) T::AccountId => T::SignedFixedPoint;

        // total amount issued by each vault, which is used in calculating SLA update, and in reward calculation
        VaultTotalIssuedAmount get(fn vault_total_issued_amount): map hasher(blake2_128_concat) T::AccountId => PolkaBTC<T>;

        // total amount issued by all vaults together; used for calculating the average issue size,
        // which is used in SLA updates
        TotalIssuedAmount: PolkaBTC<T>;
        TotalIssueCount: u32;

        // sum of all relayer scores, used in relayer reward calculation
        TotalRelayerScore: T::SignedFixedPoint;

        // target (max) SLA scores
        VaultTargetSla get(fn vault_target_sla) config(): T::SignedFixedPoint;
        RelayerTargetSla get(fn relayer_target_sla) config(): T::SignedFixedPoint;

        // vault & relayer SLA score rewards/punishments for the actions defined in
        // https://interlay.gitlab.io/polkabtc-econ/spec/sla/actions.html#actions
        // Positive and negative values indicate rewards and punishments, respectively
        VaultRedeemFailure get(fn vault_redeem_failure_sla_change) config(): T::SignedFixedPoint;
        VaultExecutedIssueMaxSlaChange get(fn vault_executed_issue_max_sla_change) config(): T::SignedFixedPoint;
        VaultSubmittedIssueProof get(fn vault_submitted_issue_proof) config(): T::SignedFixedPoint;
        RelayerBlockSubmission get(fn relayer_block_submission) config(): T::SignedFixedPoint;
        RelayerCorrectNoDataVoteOrReport get(fn relayer_correct_no_data_vote_or_report) config(): T::SignedFixedPoint;
        RelayerCorrectInvalidVoteOrReport get(fn relayer_correct_invalid_vote_or_report) config(): T::SignedFixedPoint;
        RelayerCorrectLiquidationReport get(fn relayer_correct_liquidation_report) config(): T::SignedFixedPoint;
        RelayerCorrectTheftReport get(fn relayer_correct_theft_report) config(): T::SignedFixedPoint;
        RelayerCorrectOracleOfflineReport get(fn relayer_correct_oracle_offline_report) config(): T::SignedFixedPoint;
        RelayerFalseNoDataVoteOrReport get(fn relayer_false_no_data_vote_or_report) config(): T::SignedFixedPoint;
        RelayerFalseInvalidVoteOrReport get(fn relayer_false_invalid_vote_or_report) config(): T::SignedFixedPoint;
        RelayerIgnoredVote get(fn relayer_ignored_vote) config(): T::SignedFixedPoint;
    }
}

// The pallet's events.
decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Trait>::AccountId,
        SignedFixedPoint = SignedFixedPoint<T>,
    {
        UpdateVaultSLA(AccountId, SignedFixedPoint),
        UpdateRelayerSLA(AccountId, SignedFixedPoint),
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
    // Public functions exposed to other pallets

    /// Update the SLA score of the vault on given the event.
    ///
    /// # Arguments
    ///
    /// * `vault_id` - account id of the vault
    /// * `event` - the event that has happened
    pub fn event_update_vault_sla(
        vault_id: T::AccountId,
        event: VaultEvent<PolkaBTC<T>>,
    ) -> Result<(), DispatchError> {
        let current_sla = <VaultSla<T>>::get(vault_id.clone());
        let delta_sla = match event {
            VaultEvent::RedeemFailure => <VaultRedeemFailure<T>>::get(),
            VaultEvent::SubmittedIssueProof => <VaultSubmittedIssueProof<T>>::get(),
            VaultEvent::ExecutedIssue(amount) => {
                Self::_executed_issue_sla_change(amount, &vault_id)?
            }
        };

        let new_sla = current_sla
            .checked_add(&delta_sla)
            .ok_or(Error::<T>::MathError)?;
        let max_sla = <VaultTargetSla<T>>::get(); // todo: check that this is indeed the max

        let bounded_new_sla = Self::_limit(T::SignedFixedPoint::zero(), new_sla, max_sla);

        <VaultSla<T>>::insert(vault_id.clone(), bounded_new_sla);
        Self::deposit_event(<Event<T>>::UpdateVaultSLA(vault_id, bounded_new_sla));

        Ok(())
    }

    /// Update the SLA score of the relayer on the given event.
    ///
    /// # Arguments
    ///
    /// * `relayer_id` - account id of the relayer
    /// * `event` - the event that has happened
    pub fn event_update_relayer_sla(
        relayer_id: T::AccountId,
        event: RelayerEvent,
    ) -> Result<(), DispatchError> {
        let current_sla = <RelayerSla<T>>::get(relayer_id.clone());
        let delta_sla = Self::_relayer_sla_change(event);

        let max = <RelayerTargetSla<T>>::get(); // todo: check that this is indeed the max
        let min = T::SignedFixedPoint::zero();

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

    /// Calculates the rewards for all vaults.
    /// The reward for each vault is the sum of:
    /// total_reward_for_issued * (Vault issued PolkaBTC / total issued PolkaBTC), and
    /// total_reward_for_locked * (Vault locked DOT / total locked DOT)
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

    /// Calculate the rewards for all staked relayers.
    /// We distribute rewards to Staked Relayers, based on a scoring system which takes into account
    /// their SLA and locked stake.
    /// score(relayer) = relayer.sla * relayer.stake
    /// reward(relayer) = totalReward * (relayer.score / totalRelayerScore)
    /// where totalReward is the amount of fees currently distributed and totalRelayerScore is the sum
    /// of the scores of all active Staked Relayers.
    ///
    /// # Arguments
    ///
    /// * `total_reward` - the total reward for the entire pool
    pub fn get_relayer_rewards(
        total_reward: PolkaBTC<T>,
    ) -> Result<Vec<(T::AccountId, PolkaBTC<T>)>, DispatchError> {
        <RelayerSla<T>>::iter()
            .map(|(account_id, _)| {
                Ok((
                    account_id.clone(),
                    Self::_calculate_relayer_reward(account_id.clone(), total_reward)?,
                ))
            })
            .collect()
    }

    /// Calculate the amount that is slashed when the the vault fails to execute.
    /// We reduce the amount of slashed collateral based on a Vaults SLA. The minimum amount
    /// slashed is given by the LiquidationThreshold, the maximum amount slashed by the
    /// PremiumRedeemThreshold. The actual slashed amount of collateral is a linear function
    /// parameterized by the two thresholds:
    /// MinSlashed = LiquidationThreshold (currently 110%)
    /// MaxSlashed =  PremiumRedeemThreshold (currently 130%)
    /// RealSlashed = PremiumRedeemThreshold - (PremiumRedeemThreshold - LiquidationThreshold) * (SLA / SLATarget)
    ///
    /// # Arguments
    ///
    /// * `vault_id` - account of the vault in question
    /// * `stake` - the amount of collateral placed for the redeem/replace
    /// * `liquidation_threshold` - liquidation threshold, scaled by `granularity`
    /// * `premium_redeem_threshold` - premium redeem threshold, scaled by `granularity`
    /// * `granularity` - scale factor of the thresholds, e.g. a threshold of 10^granularity would indicate 100%
    pub fn calculate_slashed_amount(
        vault_id: T::AccountId,
        stake: DOT<T>,
        liquidation_threshold: u128,
        premium_redeem_threshold: u128,
        granularity: u32,
    ) -> Result<DOT<T>, DispatchError> {
        let current_sla = <VaultSla<T>>::get(vault_id);

        let liquidation_threshold =
            Self::_threshold_to_fixed_point(liquidation_threshold, granularity)?;
        let premium_redeem_threshold =
            Self::_threshold_to_fixed_point(premium_redeem_threshold, granularity)?;

        Self::_calculate_slashed_amount(
            current_sla,
            stake,
            liquidation_threshold,
            premium_redeem_threshold,
        )
    }

    /// Explicitly set the vault's SLA score, used in tests.
    pub fn set_vault_sla(vault_id: T::AccountId, sla: SignedFixedPoint<T>) {
        <VaultSla<T>>::insert(vault_id.clone(), sla);
    }

    // Private functions internal to this pallet

    /// Calculate the amount that is slashed when the the vault fails to execute; See the
    /// documentation of calculate_slashed_amount; this function differs only in the types
    /// of the thresholds.
    fn _calculate_slashed_amount(
        current_sla: SignedFixedPoint<T>,
        stake: DOT<T>,
        liquidation_threshold: SignedFixedPoint<T>,
        premium_redeem_threshold: SignedFixedPoint<T>,
    ) -> Result<DOT<T>, DispatchError> {
        let range = premium_redeem_threshold - liquidation_threshold;
        let max_sla = <VaultTargetSla<T>>::get();
        let stake = Self::dot_to_u128(stake)?;

        // basic formula we use is:
        // result = stake * (premium_redeem_threshold - (current_sla / max_sla) * range);
        // however, we multiply by (max_sla / max_sla) to eliminate one division operator:
        // result = stake * ((premium_redeem_threshold * max_sla - current_sla * range) / max_sla)
        let calculate_slashed_collateral = || {
            // let numerator = premium_redeem_threshold * max_sla - current_sla * range;
            let numerator = T::SignedFixedPoint::checked_sub(
                &premium_redeem_threshold.checked_mul(&max_sla)?,
                &current_sla.checked_mul(&range)?,
            )?;

            let stake_scaling_factor = numerator.checked_div(&max_sla)?;

            stake_scaling_factor.checked_mul_int(stake)
        };
        let slashed_raw = calculate_slashed_collateral().ok_or(Error::<T>::MathError)?;
        Ok(Self::u128_to_dot(slashed_raw)?)
    }

    /// Calculates the potential sla change for when an issue has been completed on the given vault.
    /// The value will be clipped between 0 and VaultExecutedIssueMaxSlaChange, but it does not take
    /// into consideration vault's current SLA. That is, it can return a value > 0 when its sla is
    /// already at the maximum.
    ///
    /// # Arguments
    ///
    /// * `amount` - the amount of polkabtc that was issued
    /// * `vault_id` - account of the vault
    fn _executed_issue_sla_change(
        amount: PolkaBTC<T>,
        vault_id: &T::AccountId,
    ) -> Result<T::SignedFixedPoint, DispatchError> {
        // update account total
        let account_total = <VaultTotalIssuedAmount<T>>::get(vault_id.clone());
        let new_account_total = amount
            .checked_add(&account_total)
            .ok_or(Error::<T>::MathError)?;
        <VaultTotalIssuedAmount<T>>::insert(vault_id.clone(), new_account_total);

        // read average
        let mut total = <TotalIssuedAmount<T>>::get();
        let mut count = <TotalIssueCount>::get();
        // update average
        total = total.checked_add(&amount).ok_or(Error::<T>::MathError)?;
        count = count.checked_add(1).ok_or(Error::<T>::MathError)?;
        // write back
        <TotalIssuedAmount<T>>::set(total);
        <TotalIssueCount>::set(count);

        let average = total / count.into();

        let max_sla_change = <VaultExecutedIssueMaxSlaChange<T>>::get();

        // increase = (amount / average) * max_sla_change
        let total_raw = Self::polkabtc_to_u128(total)?;
        let average_raw = Self::polkabtc_to_u128(average)?;

        let fraction = T::SignedFixedPoint::checked_from_rational(total_raw, average_raw)
            .ok_or(Error::<T>::TryIntoIntError)?;
        let potential_sla_increase = fraction
            .checked_mul(&max_sla_change)
            .ok_or(Error::<T>::MathError)?;

        let ret = Self::_limit(
            T::SignedFixedPoint::zero(),
            potential_sla_increase,
            max_sla_change,
        );
        Ok(ret)
    }

    /// returns `value` if it is between `min` and `max`; otherwise it returns the bound
    fn _limit(
        min: T::SignedFixedPoint,
        value: T::SignedFixedPoint,
        max: T::SignedFixedPoint,
    ) -> T::SignedFixedPoint {
        if value < min {
            min
        } else if value > max {
            max
        } else {
            value
        }
    }

    /// Gets the SLA change corresponding to the given event from storage
    fn _relayer_sla_change(event: RelayerEvent) -> T::SignedFixedPoint {
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

    /// Calculate the reward of a given relayer, given the total reward for the whole relayer pool
    fn _calculate_relayer_reward(
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

    /// Gets the staked collateral of the given relayer as a fixed point type
    fn _get_relayer_stake_as_fixed_point(
        relayer_id: T::AccountId,
    ) -> Result<T::SignedFixedPoint, DispatchError> {
        let stake = ext::collateral::get_collateral_from_account::<T>(relayer_id.clone());
        let stake = Self::dot_to_u128(stake)?;
        let stake = T::SignedFixedPoint::checked_from_rational(stake, 1u128)
            .ok_or(Error::<T>::TryIntoIntError)?;
        Ok(stake)
    }

    /// Convert a given threshold from the vault registry to a fixed point type
    fn _threshold_to_fixed_point(
        value: u128,
        granularity: u32,
    ) -> Result<SignedFixedPoint<T>, DispatchError> {
        // TODO: use SignedFixedPoint type in vault_registry
        let scaling_factor = 10u128.pow(granularity);
        let ret = T::SignedFixedPoint::checked_from_rational(value, scaling_factor)
            .ok_or(Error::<T>::TryIntoIntError);
        Ok(ret?)
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
