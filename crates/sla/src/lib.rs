//! # PolkaBTC SLA Module
//! Based on the [specification](https://interlay.gitlab.io/polkabtc-spec/spec/sla.html).

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

use crate::types::{Inner, RelayerEvent, VaultEvent};
use codec::{Decode, Encode, EncodeLike};
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, dispatch::DispatchError, traits::Currency, transactional,
    weights::Weight,
};
use frame_system::ensure_root;
use sp_arithmetic::{traits::*, FixedPointNumber};
use sp_std::{convert::TryInto, vec::Vec};

pub(crate) type DOT<T> = <<T as collateral::Config>::DOT as Currency<<T as frame_system::Config>::AccountId>>::Balance;
pub(crate) type PolkaBTC<T> =
    <<T as treasury::Config>::PolkaBTC as Currency<<T as frame_system::Config>::AccountId>>::Balance;

pub(crate) type SignedFixedPoint<T> = <T as Config>::SignedFixedPoint;

/// The pallet's configuration trait.
pub trait Config: frame_system::Config + collateral::Config + treasury::Config + vault_registry::Config {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

    type SignedFixedPoint: FixedPointNumber + Encode + EncodeLike + Decode;
}

// The pallet's storage items.
decl_storage! {
    trait Store for Module<T: Config> as Sla {
        /// Mapping from accounts of vaults/relayers to their sla score
        VaultSla get(fn vault_sla): map hasher(blake2_128_concat) T::AccountId => T::SignedFixedPoint;
        RelayerSla get(fn relayer_sla): map hasher(blake2_128_concat) T::AccountId => T::SignedFixedPoint;

        // TODO: deduplicate this with the storage in the staked_relayers pallet
        RelayerStake get(fn relayer_stake): map hasher(blake2_128_concat) T::AccountId => T::SignedFixedPoint;

        // number of issues executed by all vaults together; used for calculating the average issue size,
        // which is used in SLA updates
        TotalIssueCount: u32;
        // sum of all issue amounts
        LifetimeIssued: u128;

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
        VaultRefunded get(fn vault_refunded) config(): T::SignedFixedPoint;
        RelayerBlockSubmission get(fn relayer_block_submission) config(): T::SignedFixedPoint;
        RelayerDuplicateBlockSubmission get(fn relayer_duplicate_block_submission) config(): T::SignedFixedPoint;
        RelayerCorrectNoDataVoteOrReport get(fn relayer_correct_no_data_vote_or_report) config(): T::SignedFixedPoint;
        RelayerCorrectInvalidVoteOrReport get(fn relayer_correct_invalid_vote_or_report) config(): T::SignedFixedPoint;
        RelayerCorrectTheftReport get(fn relayer_correct_theft_report) config(): T::SignedFixedPoint;
        RelayerFalseNoDataVoteOrReport get(fn relayer_false_no_data_vote_or_report) config(): T::SignedFixedPoint;
        RelayerFalseInvalidVoteOrReport get(fn relayer_false_invalid_vote_or_report) config(): T::SignedFixedPoint;
        RelayerIgnoredVote get(fn relayer_ignored_vote) config(): T::SignedFixedPoint;
    }
}

// The pallet's events.
decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Config>::AccountId,
        SignedFixedPoint = SignedFixedPoint<T>,
    {
        // [vault_id, bounded_new_sla, delta_sla]
        UpdateVaultSLA(AccountId, SignedFixedPoint, SignedFixedPoint),
        // [relayer_id, new_sla, delta_sla]
        UpdateRelayerSLA(AccountId, SignedFixedPoint, SignedFixedPoint),
    }
);

// The pallet's dispatchable functions.
decl_module! {
    /// The module declaration.
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        // Initialize errors
        type Error = Error<T>;

        // Initialize events
        fn deposit_event() = default;

        fn on_runtime_upgrade() -> Weight {
            Self::_on_runtime_upgrade();
            0
        }

        /// Set the sla delta for the given relayer event.
        ///
        /// # Arguments
        ///
        /// * `origin` - the dispatch origin of this call (must be _Root_)
        /// * `event` - relayer event to update
        /// * `value` - sla delta
        ///
        /// # Weight: `O(1)`
        #[weight = 0]
        #[transactional]
        pub fn set_relayer_sla(origin, event: RelayerEvent, value: T::SignedFixedPoint) {
            ensure_root(origin)?;
            Self::_set_relayer_sla(event, value);
        }
    }
}

// "Internal" functions, callable by code.
#[cfg_attr(test, mockable)]
impl<T: Config> Module<T> {
    // Public functions exposed to other pallets

    fn _on_runtime_upgrade() {
        if !LifetimeIssued::exists() {
            let amount = ext::vault_registry::get_total_issued_tokens::<T>(false).unwrap();
            let amount = Self::polkabtc_to_u128(amount).unwrap();
            LifetimeIssued::set(amount);
        }
    }

    /// Update the SLA score of the vault on given the event.
    ///
    /// # Arguments
    ///
    /// * `vault_id` - account id of the vault
    /// * `event` - the event that has happened
    pub fn event_update_vault_sla(
        vault_id: &T::AccountId,
        event: VaultEvent<PolkaBTC<T>>,
    ) -> Result<(), DispatchError> {
        let current_sla = <VaultSla<T>>::get(vault_id);
        let delta_sla = match event {
            VaultEvent::RedeemFailure => <VaultRedeemFailure<T>>::get(),
            VaultEvent::SubmittedIssueProof => <VaultSubmittedIssueProof<T>>::get(),
            VaultEvent::Refunded => <VaultRefunded<T>>::get(),
            VaultEvent::ExecutedIssue(amount) => Self::_executed_issue_sla_change(amount)?,
        };

        let new_sla = current_sla
            .checked_add(&delta_sla)
            .ok_or(Error::<T>::ArithmeticOverflow)?;
        let max_sla = <VaultTargetSla<T>>::get(); // todo: check that this is indeed the max

        let bounded_new_sla = Self::_limit(T::SignedFixedPoint::zero(), new_sla, max_sla);

        <VaultSla<T>>::insert(vault_id, bounded_new_sla);
        Self::deposit_event(<Event<T>>::UpdateVaultSLA(vault_id.clone(), bounded_new_sla, delta_sla));

        Ok(())
    }

    /// Update the SLA score of the relayer on the given event.
    ///
    /// # Arguments
    ///
    /// * `relayer_id` - account id of the relayer
    /// * `event` - the event that has happened
    pub fn event_update_relayer_sla(relayer_id: &T::AccountId, event: RelayerEvent) -> Result<(), DispatchError> {
        let current_sla = <RelayerSla<T>>::get(relayer_id);
        let delta_sla = Self::_get_relayer_sla(event);

        let max = <RelayerTargetSla<T>>::get(); // todo: check that this is indeed the max
        let min = T::SignedFixedPoint::zero();

        let potential_new_sla = current_sla
            .checked_add(&delta_sla)
            .ok_or(Error::<T>::ArithmeticOverflow)?;

        let new_sla = Self::_limit(min, potential_new_sla, max);

        if current_sla != new_sla {
            let stake = Self::get_relayer_stake(relayer_id);
            let total_relayer_score = <TotalRelayerScore<T>>::get();

            // todo: check if we can get problems with rounding errors
            let calculate_new_total_relayer_score = || {
                let actual_delta_sla = new_sla.checked_sub(&current_sla)?;
                // convert stake to fixed point
                let delta_score = actual_delta_sla.checked_mul(&stake)?;
                let new_total_relayer_score = total_relayer_score.checked_add(&delta_score)?;
                Some(new_total_relayer_score)
            };
            let new_total = calculate_new_total_relayer_score().ok_or(Error::<T>::InvalidTotalRelayerScore)?;

            <TotalRelayerScore<T>>::set(new_total);
            <RelayerSla<T>>::insert(relayer_id, new_sla);
            Self::deposit_event(<Event<T>>::UpdateRelayerSLA(relayer_id.clone(), new_sla, delta_sla));
        }

        Ok(())
    }

    /// Calculates the rewards for all vaults.
    /// The reward for each vault is the sum of:
    /// total_reward_for_issued * (Vault issued PolkaBTC / total issued PolkaBTC), and
    /// total_reward_for_locked * (Vault locked DOT / total locked DOT)
    pub fn get_vault_rewards(
        total_reward_for_issued_in_polka_btc: PolkaBTC<T>,
        total_reward_for_locked_in_polka_btc: PolkaBTC<T>,
        total_reward_for_issued_in_dot: DOT<T>,
        total_reward_for_locked_in_dot: DOT<T>,
    ) -> Result<Vec<(T::AccountId, PolkaBTC<T>, DOT<T>)>, DispatchError> {
        let total_issued = Self::polkabtc_to_u128(ext::vault_registry::get_total_issued_tokens::<T>(false)?)?;
        let total_locked = Self::dot_to_u128(ext::vault_registry::get_total_backing_collateral::<T>(false)?)?;

        let total_reward_for_issued_in_polka_btc = Self::polkabtc_to_u128(total_reward_for_issued_in_polka_btc)?;
        let total_reward_for_locked_in_polka_btc = Self::polkabtc_to_u128(total_reward_for_locked_in_polka_btc)?;

        let total_reward_for_issued_in_dot = Self::dot_to_u128(total_reward_for_issued_in_dot)?;
        let total_reward_for_locked_in_dot = Self::dot_to_u128(total_reward_for_locked_in_dot)?;

        let calculate_reward = |account_id: T::AccountId| {
            // each vault gets total_reward * (issued_amount / total_issued).
            let vault = ext::vault_registry::get_vault_from_id::<T>(&account_id)?;
            if vault.is_liquidated() {
                return Ok(None);
            }
            let issued_amount = vault.issued_tokens;

            let issued_amount = Self::polkabtc_to_u128(issued_amount)?;
            let issued_reward_in_polka_btc = issued_amount
                .checked_mul(total_reward_for_issued_in_polka_btc)
                .ok_or(Error::<T>::ArithmeticOverflow)?
                .checked_div(total_issued)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;

            let issued_reward_in_dot = issued_amount
                .checked_mul(total_reward_for_issued_in_dot)
                .ok_or(Error::<T>::ArithmeticOverflow)?
                .checked_div(total_issued)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;

            let locked_amount = ext::vault_registry::get_backing_collateral::<T>(&account_id)?;
            let locked_amount = Self::dot_to_u128(locked_amount)?;
            let locked_reward_in_polka_btc = locked_amount
                .checked_mul(total_reward_for_locked_in_polka_btc)
                .ok_or(Error::<T>::ArithmeticOverflow)?
                .checked_div(total_locked)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;

            let locked_reward_in_dot = locked_amount
                .checked_mul(total_reward_for_locked_in_dot)
                .ok_or(Error::<T>::ArithmeticOverflow)?
                .checked_div(total_locked)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;

            Result::<_, DispatchError>::Ok(Some((
                account_id,
                Self::u128_to_polkabtc(
                    issued_reward_in_polka_btc
                        .checked_add(locked_reward_in_polka_btc)
                        .ok_or(Error::<T>::ArithmeticOverflow)?,
                )?,
                Self::u128_to_dot(
                    issued_reward_in_dot
                        .checked_add(locked_reward_in_dot)
                        .ok_or(Error::<T>::ArithmeticOverflow)?,
                )?,
            )))
        };

        <VaultSla<T>>::iter()
            .filter_map(|(account_id, _)| calculate_reward(account_id).transpose())
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
        total_reward_polka_btc: PolkaBTC<T>,
        total_reward_dot: DOT<T>,
    ) -> Result<Vec<(T::AccountId, PolkaBTC<T>, DOT<T>)>, DispatchError> {
        <RelayerSla<T>>::iter()
            .map(|(account_id, _)| {
                Ok((
                    account_id.clone(),
                    Self::u128_to_polkabtc(Self::_calculate_relayer_reward(
                        &account_id,
                        Self::polkabtc_to_u128(total_reward_polka_btc)?,
                    )?)?,
                    Self::u128_to_dot(Self::_calculate_relayer_reward(
                        &account_id,
                        Self::dot_to_u128(total_reward_dot)?,
                    )?)?,
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
    /// * `reimburse` - if true, this function returns 110-130%. If false, it returns 10-30%
    pub fn calculate_slashed_amount(
        vault_id: &T::AccountId,
        stake: DOT<T>,
        reimburse: bool,
    ) -> Result<DOT<T>, DispatchError> {
        let current_sla = <VaultSla<T>>::get(vault_id);

        let liquidation_threshold = ext::vault_registry::get_liquidation_collateral_threshold::<T>();
        let liquidation_threshold = Self::fixed_point_unsigned_to_signed(liquidation_threshold)?;
        let premium_redeem_threshold = ext::vault_registry::get_premium_redeem_threshold::<T>();
        let premium_redeem_threshold = Self::fixed_point_unsigned_to_signed(premium_redeem_threshold)?;

        let total =
            Self::_calculate_slashed_amount(current_sla, stake, liquidation_threshold, premium_redeem_threshold)?;

        if reimburse {
            Ok(total)
        } else {
            // vault is already losing the btc, so subtract the equivalent value of the lost btc
            Ok(total.checked_sub(&stake).ok_or(Error::<T>::ArithmeticUnderflow)?)
        }
    }

    /// Explicitly set the vault's SLA score, used in tests.
    pub fn set_vault_sla(vault_id: &T::AccountId, sla: SignedFixedPoint<T>) {
        <VaultSla<T>>::insert(vault_id, sla);
    }

    /// initializes the relayer's stake. Not that this module assumes that once set, the stake
    /// remains unchanged forever
    pub fn initialize_relayer_stake(relayer_id: &T::AccountId, stake: DOT<T>) -> Result<(), DispatchError> {
        let stake = Self::dot_to_u128(stake)?;
        let stake = T::SignedFixedPoint::checked_from_rational(stake, 1u128).ok_or(Error::<T>::TryIntoIntError)?;
        <RelayerStake<T>>::insert(relayer_id, stake);

        Ok(())
    }

    // Private functions internal to this pallet

    /// Calculate the amount that is slashed when the the vault fails to execute; See the
    /// documentation of calculate_slashed_amount; this function differs only in that it has
    /// the thesholds are parameters.
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
        let slashed_raw = calculate_slashed_collateral().ok_or(Error::<T>::InvalidSlashedAmount)?;
        Self::u128_to_dot(slashed_raw)
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
    fn _executed_issue_sla_change(amount: PolkaBTC<T>) -> Result<T::SignedFixedPoint, DispatchError> {
        let amount_raw = Self::polkabtc_to_u128(amount)?;

        // update the number of issues performed
        let count = TotalIssueCount::mutate(|x| {
            *x = x.saturating_add(1);
            *x as u128
        });
        let total = LifetimeIssued::mutate(|x| {
            *x = x.saturating_add(amount_raw);
            *x
        });

        // calculate the average on raw input rather than in fixed_point - we don't want to fail
        // if the result can not be losslessly represented. By using raw division we will be off
        // but at most one Planck, which is acceptable
        let average_raw = total.checked_div(count).ok_or(Error::<T>::ArithmeticOverflow)?;

        let average = T::SignedFixedPoint::checked_from_rational(average_raw, 1).ok_or(Error::<T>::TryIntoIntError)?;

        let max_sla_change = <VaultExecutedIssueMaxSlaChange<T>>::get();

        // increase = (amount / average) * max_sla_change
        let amount = Self::polkabtc_to_fixed_point(amount)?;
        let potential_sla_increase = amount
            .checked_div(&average)
            .ok_or(Error::<T>::ArithmeticUnderflow)?
            .checked_mul(&max_sla_change)
            .ok_or(Error::<T>::ArithmeticOverflow)?;

        let ret = Self::_limit(T::SignedFixedPoint::zero(), potential_sla_increase, max_sla_change);
        Ok(ret)
    }

    /// returns `value` if it is between `min` and `max`; otherwise it returns the bound
    fn _limit(min: T::SignedFixedPoint, value: T::SignedFixedPoint, max: T::SignedFixedPoint) -> T::SignedFixedPoint {
        if value < min {
            min
        } else if value > max {
            max
        } else {
            value
        }
    }

    /// Gets the SLA change corresponding to the given event from storage
    fn _get_relayer_sla(event: RelayerEvent) -> T::SignedFixedPoint {
        match event {
            RelayerEvent::BlockSubmission => <RelayerBlockSubmission<T>>::get(),
            RelayerEvent::DuplicateBlockSubmission => <RelayerDuplicateBlockSubmission<T>>::get(),
            RelayerEvent::CorrectNoDataVoteOrReport => <RelayerCorrectNoDataVoteOrReport<T>>::get(),
            RelayerEvent::CorrectInvalidVoteOrReport => <RelayerCorrectInvalidVoteOrReport<T>>::get(),
            RelayerEvent::CorrectTheftReport => <RelayerCorrectTheftReport<T>>::get(),
            RelayerEvent::FalseNoDataVoteOrReport => <RelayerFalseNoDataVoteOrReport<T>>::get(),
            RelayerEvent::FalseInvalidVoteOrReport => <RelayerFalseInvalidVoteOrReport<T>>::get(),
            RelayerEvent::IgnoredVote => <RelayerIgnoredVote<T>>::get(),
        }
    }

    /// Updates the SLA change corresponding to the given event in storage
    fn _set_relayer_sla(event: RelayerEvent, value: T::SignedFixedPoint) {
        match event {
            RelayerEvent::BlockSubmission => <RelayerBlockSubmission<T>>::set(value),
            RelayerEvent::DuplicateBlockSubmission => <RelayerDuplicateBlockSubmission<T>>::set(value),
            RelayerEvent::CorrectNoDataVoteOrReport => <RelayerCorrectNoDataVoteOrReport<T>>::set(value),
            RelayerEvent::CorrectInvalidVoteOrReport => <RelayerCorrectInvalidVoteOrReport<T>>::set(value),
            RelayerEvent::CorrectTheftReport => <RelayerCorrectTheftReport<T>>::set(value),
            RelayerEvent::FalseNoDataVoteOrReport => <RelayerFalseNoDataVoteOrReport<T>>::set(value),
            RelayerEvent::FalseInvalidVoteOrReport => <RelayerFalseInvalidVoteOrReport<T>>::set(value),
            RelayerEvent::IgnoredVote => <RelayerIgnoredVote<T>>::set(value),
        }
    }

    /// Calculate the reward of a given relayer, given the total reward for the whole relayer pool
    fn _calculate_relayer_reward(relayer_id: &T::AccountId, total_reward: u128) -> Result<u128, DispatchError> {
        let stake = Self::get_relayer_stake(&relayer_id);
        let sla = <RelayerSla<T>>::get(&relayer_id);
        let total_relayer_score = <TotalRelayerScore<T>>::get();

        if total_relayer_score.is_zero() {
            return Ok(0);
        }

        let calculate_reward = || {
            let score = stake.checked_mul(&sla)?;
            let share = score.checked_div(&total_relayer_score)?;
            let reward = share.checked_mul_int(total_reward)?;
            Some(reward)
        };
        Ok(calculate_reward().ok_or(Error::<T>::InvalidRelayerReward)?)
    }

    fn get_relayer_stake(relayer_id: &T::AccountId) -> SignedFixedPoint<T> {
        <RelayerStake<T>>::get(relayer_id)
    }

    /// Convert a given threshold from the vault registry to a signed fixed point type
    fn fixed_point_unsigned_to_signed<U: FixedPointNumber>(value: U) -> Result<SignedFixedPoint<T>, DispatchError> {
        let raw: i128 = value
            .into_inner()
            .unique_saturated_into()
            .try_into()
            .map_err(|_| Error::<T>::TryIntoIntError)?;

        let ret = T::SignedFixedPoint::checked_from_rational(raw, U::accuracy()).ok_or(Error::<T>::TryIntoIntError)?;
        Ok(ret)
    }

    fn dot_to_u128(x: DOT<T>) -> Result<u128, DispatchError> {
        TryInto::<u128>::try_into(x).map_err(|_| Error::<T>::TryIntoIntError.into())
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

    fn polkabtc_to_fixed_point(x: PolkaBTC<T>) -> Result<T::SignedFixedPoint, DispatchError> {
        let y = TryInto::<u128>::try_into(x).map_err(|_| Error::<T>::TryIntoIntError)?;
        let inner = TryInto::<Inner<T>>::try_into(y).map_err(|_| Error::<T>::TryIntoIntError)?;
        Ok(T::SignedFixedPoint::checked_from_integer(inner).ok_or(Error::<T>::TryIntoIntError)?)
    }
}

decl_error! {
    pub enum Error for Module<T: Config> {
        TryIntoIntError,
        ArithmeticOverflow,
        ArithmeticUnderflow,
        InvalidTotalRelayerScore,
        InvalidSlashedAmount,
        InvalidRelayerReward,
    }
}
