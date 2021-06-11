//! # SLA Module
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

pub mod types;

use crate::types::{BalanceOf, Inner, RelayerEvent, SignedFixedPoint, VaultEvent};
use codec::{Decode, Encode, EncodeLike, FullCodec};
use frame_support::{dispatch::DispatchError, transactional};
use frame_system::ensure_root;
use reward::RewardPool;
use sp_runtime::{
    traits::{MaybeSerializeDeserialize, *},
    FixedPointNumber, FixedPointOperand,
};
use sp_std::{
    convert::{TryFrom, TryInto},
    fmt::Debug,
};

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    /// ## Configuration
    /// The pallet's configuration trait.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Signed fixed point type.
        type SignedFixedPoint: FixedPointNumber<Inner = Self::SignedInner>
            + Encode
            + EncodeLike
            + Decode
            + MaybeSerializeDeserialize;

        /// The `Inner` type of the `SignedFixedPoint`.
        type SignedInner: Debug
            + One
            + CheckedMul
            + CheckedDiv
            + FixedPointOperand
            + TryFrom<<Self as Config>::Balance>
            + TryInto<<Self as Config>::Balance>;

        /// The primitive balance type.
        type Balance: AtLeast32BitUnsigned
            + FixedPointOperand
            + MaybeSerializeDeserialize
            + FullCodec
            + Copy
            + Default
            + Debug;

        /// Vault reward pool for the collateral currency.
        type CollateralVaultRewards: reward::Rewards<Self::AccountId, SignedFixedPoint = SignedFixedPoint<Self>>;

        /// Vault reward pool for the wrapped currency.
        type WrappedVaultRewards: reward::Rewards<Self::AccountId, SignedFixedPoint = SignedFixedPoint<Self>>;

        /// Relayer reward pool for the collateral currency.
        type CollateralRelayerRewards: reward::Rewards<Self::AccountId, SignedFixedPoint = SignedFixedPoint<Self>>;

        /// Relayer reward pool for the wrapped currency.
        type WrappedRelayerRewards: reward::Rewards<Self::AccountId, SignedFixedPoint = SignedFixedPoint<Self>>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    #[pallet::metadata(T::AccountId = "AccountId", SignedFixedPoint<T> = "SignedFixedPoint")]
    pub enum Event<T: Config> {
        // [vault_id, bounded_new_sla, delta_sla]
        UpdateVaultSLA(T::AccountId, SignedFixedPoint<T>, SignedFixedPoint<T>),
        // [relayer_id, new_sla, delta_sla]
        UpdateRelayerSLA(T::AccountId, SignedFixedPoint<T>, SignedFixedPoint<T>),
    }

    #[pallet::error]
    pub enum Error<T> {
        TryIntoIntError,
        ArithmeticOverflow,
        ArithmeticUnderflow,
        InvalidSlashedAmount,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

    /// Mapping from Vaults to their SLA score.
    #[pallet::storage]
    #[pallet::getter(fn vault_sla)]
    pub(super) type VaultSla<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, SignedFixedPoint<T>, ValueQuery>;

    /// Mapping from Relayers to their SLA score.
    #[pallet::storage]
    #[pallet::getter(fn relayer_sla)]
    pub(super) type RelayerSla<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, SignedFixedPoint<T>, ValueQuery>;

    /// Total number of issues executed by all vaults; used for calculating the average issue size.
    #[pallet::storage]
    pub(super) type TotalIssueCount<T: Config> = StorageValue<_, u32, ValueQuery>;

    #[pallet::storage]
    pub(super) type LifetimeIssued<T: Config> = StorageValue<_, u128, ValueQuery>;

    #[pallet::storage]
    pub(super) type AverageDeposit<T: Config> = StorageValue<_, SignedFixedPoint<T>, ValueQuery>;

    #[pallet::storage]
    pub(super) type AverageDepositCount<T: Config> = StorageValue<_, SignedFixedPoint<T>, ValueQuery>;

    #[pallet::storage]
    pub(super) type AverageWithdraw<T: Config> = StorageValue<_, SignedFixedPoint<T>, ValueQuery>;

    #[pallet::storage]
    pub(super) type AverageWithdrawCount<T: Config> = StorageValue<_, SignedFixedPoint<T>, ValueQuery>;

    // Target (max) SLA score for Vaults.
    #[pallet::storage]
    #[pallet::getter(fn vault_target_sla)]
    pub(super) type VaultTargetSla<T: Config> = StorageValue<_, SignedFixedPoint<T>, ValueQuery>;

    // Target (max) SLA score for Relayers.
    #[pallet::storage]
    #[pallet::getter(fn relayer_target_sla)]
    pub(super) type RelayerTargetSla<T: Config> = StorageValue<_, SignedFixedPoint<T>, ValueQuery>;

    // vault & relayer SLA score rewards/punishments for the actions defined in
    // https://interlay.gitlab.io/polkabtc-econ/spec/sla/actions.html#actions
    // Positive and negative values indicate rewards and punishments, respectively

    #[pallet::storage]
    #[pallet::getter(fn vault_redeem_failure_sla_change)]
    pub(super) type VaultRedeemFailure<T: Config> = StorageValue<_, SignedFixedPoint<T>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn vault_execute_issue_max_sla_change)]
    pub(super) type VaultExecuteIssueMaxSlaChange<T: Config> = StorageValue<_, SignedFixedPoint<T>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn vault_deposit_max_sla_change)]
    pub(super) type VaultDepositMaxSlaChange<T: Config> = StorageValue<_, SignedFixedPoint<T>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn vault_withdraw_max_sla_change)]
    pub(super) type VaultWithdrawMaxSlaChange<T: Config> = StorageValue<_, SignedFixedPoint<T>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn vault_submit_issue_proof)]
    pub(super) type VaultSubmitIssueProof<T: Config> = StorageValue<_, SignedFixedPoint<T>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn vault_refund)]
    pub(super) type VaultRefund<T: Config> = StorageValue<_, SignedFixedPoint<T>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn relayer_store_block)]
    pub(super) type RelayerStoreBlock<T: Config> = StorageValue<_, SignedFixedPoint<T>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn relayer_theft_report)]
    pub(super) type RelayerTheftReport<T: Config> = StorageValue<_, SignedFixedPoint<T>, ValueQuery>;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub vault_target_sla: SignedFixedPoint<T>,
        pub relayer_target_sla: SignedFixedPoint<T>,
        pub vault_redeem_failure_sla_change: SignedFixedPoint<T>,
        pub vault_execute_issue_max_sla_change: SignedFixedPoint<T>,
        pub vault_deposit_max_sla_change: SignedFixedPoint<T>,
        pub vault_withdraw_max_sla_change: SignedFixedPoint<T>,
        pub vault_submit_issue_proof: SignedFixedPoint<T>,
        pub vault_refund: SignedFixedPoint<T>,
        pub relayer_store_block: SignedFixedPoint<T>,
        pub relayer_theft_report: SignedFixedPoint<T>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                vault_target_sla: Default::default(),
                relayer_target_sla: Default::default(),
                vault_redeem_failure_sla_change: Default::default(),
                vault_execute_issue_max_sla_change: Default::default(),
                vault_deposit_max_sla_change: Default::default(),
                vault_withdraw_max_sla_change: Default::default(),
                vault_submit_issue_proof: Default::default(),
                vault_refund: Default::default(),
                relayer_store_block: Default::default(),
                relayer_theft_report: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            VaultTargetSla::<T>::put(self.vault_target_sla);
            RelayerTargetSla::<T>::put(self.relayer_target_sla);
            VaultRedeemFailure::<T>::put(self.vault_redeem_failure_sla_change);
            VaultExecuteIssueMaxSlaChange::<T>::put(self.vault_execute_issue_max_sla_change);
            VaultDepositMaxSlaChange::<T>::put(self.vault_deposit_max_sla_change);
            VaultWithdrawMaxSlaChange::<T>::put(self.vault_withdraw_max_sla_change);
            VaultSubmitIssueProof::<T>::put(self.vault_submit_issue_proof);
            VaultRefund::<T>::put(self.vault_refund);
            RelayerStoreBlock::<T>::put(self.relayer_store_block);
            RelayerTheftReport::<T>::put(self.relayer_theft_report);
        }
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // The pallet's dispatchable functions.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Set the sla delta for the given relayer event.
        ///
        /// # Arguments
        ///
        /// * `origin` - the dispatch origin of this call (must be _Root_)
        /// * `event` - relayer event to update
        /// * `value` - sla delta
        ///
        /// # Weight: `O(1)`
        #[pallet::weight(0)]
        #[transactional]
        pub fn set_relayer_sla(
            origin: OriginFor<T>,
            event: RelayerEvent,
            value: SignedFixedPoint<T>,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            Self::_set_relayer_sla(event, value);
            Ok(().into())
        }
    }
}

// "Internal" functions, callable by code.
#[cfg_attr(test, mockable)]
impl<T: Config> Pallet<T> {
    // Public functions exposed to other pallets

    /// Update the SLA score of the vault on given the event.
    ///
    /// # Arguments
    ///
    /// * `vault_id` - account id of the vault
    /// * `event` - the event that has happened
    pub fn event_update_vault_sla(
        vault_id: &T::AccountId,
        event: VaultEvent<BalanceOf<T>>,
    ) -> Result<(), DispatchError> {
        let current_sla = <VaultSla<T>>::get(vault_id);
        let delta_sla = match event {
            VaultEvent::RedeemFailure => <VaultRedeemFailure<T>>::get(),
            VaultEvent::SubmitIssueProof => <VaultSubmitIssueProof<T>>::get(),
            VaultEvent::Refund => <VaultRefund<T>>::get(),
            VaultEvent::ExecuteIssue(amount) => Self::_execute_issue_sla_change(amount)?,
            VaultEvent::Deposit(amount) => Self::_deposit_sla_change(amount)?,
            VaultEvent::Withdraw(amount) => Self::_withdraw_sla_change(amount)?,
            VaultEvent::Liquidate => return Self::_liquidate_sla(vault_id),
        };

        let new_sla = current_sla
            .checked_add(&delta_sla)
            .ok_or(Error::<T>::ArithmeticOverflow)?;
        let max_sla = <VaultTargetSla<T>>::get(); // todo: check that this is indeed the max

        let bounded_new_sla = Self::_limit(SignedFixedPoint::<T>::zero(), new_sla, max_sla);

        Self::adjust_stake::<T::CollateralVaultRewards>(vault_id, delta_sla)?;
        Self::adjust_stake::<T::WrappedVaultRewards>(vault_id, delta_sla)?;

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

        let max = <RelayerTargetSla<T>>::get(); // TODO: check that this is indeed the max
        let min = SignedFixedPoint::<T>::zero();

        let potential_new_sla = current_sla
            .checked_add(&delta_sla)
            .ok_or(Error::<T>::ArithmeticOverflow)?;

        let new_sla = Self::_limit(min, potential_new_sla, max);

        Self::adjust_stake::<T::CollateralRelayerRewards>(relayer_id, delta_sla)?;
        Self::adjust_stake::<T::WrappedRelayerRewards>(relayer_id, delta_sla)?;

        <RelayerSla<T>>::insert(relayer_id, new_sla);
        Self::deposit_event(<Event<T>>::UpdateRelayerSLA(relayer_id.clone(), new_sla, delta_sla));

        Ok(())
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
    pub fn calculate_slashed_amount<UnsignedFixedPoint: FixedPointNumber>(
        vault_id: &T::AccountId,
        stake: BalanceOf<T>,
        reimburse: bool,
        liquidation_threshold: UnsignedFixedPoint,
        premium_redeem_threshold: UnsignedFixedPoint,
    ) -> Result<BalanceOf<T>, DispatchError> {
        let current_sla = <VaultSla<T>>::get(vault_id);

        let liquidation_threshold = Self::fixed_point_unsigned_to_signed(liquidation_threshold)?;
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

    // Private functions internal to this pallet

    fn adjust_stake<R: reward::Rewards<T::AccountId, SignedFixedPoint = SignedFixedPoint<T>>>(
        account_id: &T::AccountId,
        delta_sla: SignedFixedPoint<T>,
    ) -> Result<(), DispatchError> {
        if delta_sla.is_positive() {
            R::deposit_stake(RewardPool::Global, account_id, delta_sla)?;
        } else if delta_sla.is_negative() {
            let remaining_stake = R::get_stake(&RewardPool::Global, account_id).min(delta_sla.saturating_abs());
            if remaining_stake > SignedFixedPoint::<T>::zero() {
                R::withdraw_stake(RewardPool::Global, account_id, remaining_stake)?;
            }
        }
        Ok(())
    }

    fn liquidate_stake<R: reward::Rewards<T::AccountId, SignedFixedPoint = SignedFixedPoint<T>>>(
        account_id: &T::AccountId,
    ) -> Result<(), DispatchError> {
        let remaining_stake = R::get_stake(&RewardPool::Global, account_id);
        if remaining_stake > SignedFixedPoint::<T>::zero() {
            R::withdraw_stake(RewardPool::Global, account_id, remaining_stake)?;
        }
        Ok(())
    }

    /// Calculate the amount that is slashed when the the vault fails to execute; See the
    /// documentation of calculate_slashed_amount; this function differs only in that it has
    /// the thesholds are parameters.
    fn _calculate_slashed_amount(
        current_sla: SignedFixedPoint<T>,
        stake: BalanceOf<T>,
        liquidation_threshold: SignedFixedPoint<T>,
        premium_redeem_threshold: SignedFixedPoint<T>,
    ) -> Result<BalanceOf<T>, DispatchError> {
        let range = premium_redeem_threshold - liquidation_threshold;
        let max_sla = <VaultTargetSla<T>>::get();
        let stake = TryInto::<T::SignedInner>::try_into(stake).map_err(|_| Error::<T>::TryIntoIntError)?;

        // basic formula we use is:
        // result = stake * (premium_redeem_threshold - (current_sla / max_sla) * range);
        // however, we multiply by (max_sla / max_sla) to eliminate one division operator:
        // result = stake * ((premium_redeem_threshold * max_sla - current_sla * range) / max_sla)
        let calculate_slashed_collateral = || {
            // let numerator = premium_redeem_threshold * max_sla - current_sla * range;
            let numerator = SignedFixedPoint::<T>::checked_sub(
                &premium_redeem_threshold.checked_mul(&max_sla)?,
                &current_sla.checked_mul(&range)?,
            )?;

            let stake_scaling_factor = numerator.checked_div(&max_sla)?;

            stake_scaling_factor.checked_mul_int(stake)
        };
        let slashed = calculate_slashed_collateral().ok_or(Error::<T>::InvalidSlashedAmount)?;
        Ok(slashed.try_into().map_err(|_| Error::<T>::TryIntoIntError)?)
    }

    /// Calculates the potential sla change for when an issue has been completed on the given vault.
    /// The value will be clipped between 0 and VaultExecuteIssueMaxSlaChange, but it does not take
    /// into consideration vault's current SLA. That is, it can return a value > 0 when its sla is
    /// already at the maximum.
    ///
    /// # Arguments
    ///
    /// * `amount` - the amount of tokens that were issued
    fn _execute_issue_sla_change(amount: BalanceOf<T>) -> Result<SignedFixedPoint<T>, DispatchError> {
        let amount_raw = Self::wrapped_to_u128(amount)?;

        // update the number of issues performed
        let count = <TotalIssueCount<T>>::mutate(|x| {
            *x = x.saturating_add(1);
            *x as u128
        });
        let total = <LifetimeIssued<T>>::mutate(|x| {
            *x = x.saturating_add(amount_raw);
            *x
        });

        // calculate the average on raw input rather than in fixed_point - we don't want to fail
        // if the result can not be losslessly represented. By using raw division we will be off
        // but at most one Planck, which is acceptable
        let average_raw = total.checked_div(count).ok_or(Error::<T>::ArithmeticOverflow)?;

        let average =
            SignedFixedPoint::<T>::checked_from_rational(average_raw, 1).ok_or(Error::<T>::TryIntoIntError)?;

        let max_sla_change = <VaultExecuteIssueMaxSlaChange<T>>::get();

        // increase = (amount / average) * max_sla_change
        let amount = Self::currency_to_fixed_point(amount)?;
        let potential_sla_increase = amount
            .checked_div(&average)
            .ok_or(Error::<T>::ArithmeticUnderflow)?
            .checked_mul(&max_sla_change)
            .ok_or(Error::<T>::ArithmeticOverflow)?;

        Ok(Self::_limit(
            SignedFixedPoint::<T>::zero(),
            potential_sla_increase,
            max_sla_change,
        ))
    }

    /// Calculates the potential sla change for a vault depositing collateral. The value will be
    /// clipped between 0 and VaultDepositMaxSlaChange, but it does not take into consideration
    /// vault's current SLA. It can return a value > 0 when its sla is already at the maximum.
    ///
    /// # Arguments
    ///
    /// * `amount` - the amount of tokens that were locked
    pub(crate) fn _deposit_sla_change(amount: BalanceOf<T>) -> Result<SignedFixedPoint<T>, DispatchError> {
        let max_sla_change = <VaultDepositMaxSlaChange<T>>::get();
        let amount = Self::currency_to_fixed_point(amount)?;

        let count = <AverageDepositCount<T>>::mutate(|x| {
            *x = x.saturating_add(SignedFixedPoint::<T>::one());
            *x
        });

        // new_average = (old_average * (n-1) + new_value) / n
        let average = <AverageDeposit<T>>::mutate(|x| {
            *x = x
                .saturating_mul(count.saturating_sub(SignedFixedPoint::<T>::one()))
                .saturating_add(amount)
                .checked_div(&count)
                .unwrap_or(SignedFixedPoint::<T>::zero());
            *x
        });

        // increase = (amount / average) * max_sla_change
        let potential_sla_increase = amount
            .checked_div(&average)
            .unwrap_or(SignedFixedPoint::<T>::zero())
            .checked_mul(&max_sla_change)
            .ok_or(Error::<T>::ArithmeticOverflow)?;

        Ok(Self::_limit(
            SignedFixedPoint::<T>::zero(),
            potential_sla_increase,
            max_sla_change,
        ))
    }

    /// Calculates the potential sla change for a vault withdrawing collateral. The value will be
    /// clipped between 0 and VaultWithdrawMaxSlaChange, but it does not take into consideration
    /// vault's current SLA. It can return a value > 0 when its sla is already at the maximum.
    ///
    /// # Arguments
    ///
    /// * `amount` - the amount of tokens that were unlocked
    pub(crate) fn _withdraw_sla_change(amount: BalanceOf<T>) -> Result<SignedFixedPoint<T>, DispatchError> {
        let max_sla_change = <VaultWithdrawMaxSlaChange<T>>::get();
        let amount = Self::currency_to_fixed_point(amount)?;

        let count = <AverageWithdrawCount<T>>::mutate(|x| {
            *x = x.saturating_add(SignedFixedPoint::<T>::one());
            *x
        });

        // new_average = (old_average * (n-1) + new_value) / n
        let average = <AverageWithdraw<T>>::mutate(|x| {
            *x = x
                .saturating_mul(count.saturating_sub(SignedFixedPoint::<T>::one()))
                .saturating_add(amount)
                .checked_div(&count)
                .unwrap_or(SignedFixedPoint::<T>::zero());
            *x
        });

        // increase = (amount / average) * max_sla_change
        let potential_sla_decrease = amount
            .checked_div(&average)
            .unwrap_or(SignedFixedPoint::<T>::zero())
            .checked_mul(&max_sla_change)
            .ok_or(Error::<T>::ArithmeticOverflow)?;

        Ok(Self::_limit(
            max_sla_change,
            potential_sla_decrease,
            SignedFixedPoint::<T>::zero(),
        ))
    }

    fn _liquidate_sla(vault_id: &T::AccountId) -> Result<(), DispatchError> {
        Self::liquidate_stake::<T::CollateralVaultRewards>(vault_id)?;
        Self::liquidate_stake::<T::WrappedVaultRewards>(vault_id)?;

        let delta_sla = <VaultSla<T>>::get(vault_id)
            .checked_mul(&SignedFixedPoint::<T>::saturating_from_integer(-1))
            .unwrap_or(Zero::zero());
        let bounded_new_sla = SignedFixedPoint::<T>::zero();
        <VaultSla<T>>::insert(vault_id, bounded_new_sla);
        Self::deposit_event(<Event<T>>::UpdateVaultSLA(vault_id.clone(), bounded_new_sla, delta_sla));

        Ok(())
    }

    /// returns `value` if it is between `min` and `max`; otherwise it returns the bound
    fn _limit(min: SignedFixedPoint<T>, value: SignedFixedPoint<T>, max: SignedFixedPoint<T>) -> SignedFixedPoint<T> {
        if value < min {
            min
        } else if value > max {
            max
        } else {
            value
        }
    }

    /// Gets the SLA change corresponding to the given event from storage
    fn _get_relayer_sla(event: RelayerEvent) -> SignedFixedPoint<T> {
        match event {
            RelayerEvent::StoreBlock => <RelayerStoreBlock<T>>::get(),
            RelayerEvent::TheftReport => <RelayerTheftReport<T>>::get(),
        }
    }

    /// Updates the SLA change corresponding to the given event in storage
    fn _set_relayer_sla(event: RelayerEvent, value: SignedFixedPoint<T>) {
        match event {
            RelayerEvent::StoreBlock => <RelayerStoreBlock<T>>::set(value),
            RelayerEvent::TheftReport => <RelayerTheftReport<T>>::set(value),
        }
    }

    /// Convert a given threshold from the vault registry to a signed fixed point type
    fn fixed_point_unsigned_to_signed<UnsignedFixedPoint: FixedPointNumber>(
        value: UnsignedFixedPoint,
    ) -> Result<SignedFixedPoint<T>, DispatchError> {
        let raw: i128 = value
            .into_inner()
            .unique_saturated_into()
            .try_into()
            .map_err(|_| Error::<T>::TryIntoIntError)?;

        let ret = SignedFixedPoint::<T>::checked_from_rational(raw, UnsignedFixedPoint::accuracy())
            .ok_or(Error::<T>::TryIntoIntError)?;
        Ok(ret)
    }

    fn wrapped_to_u128(x: BalanceOf<T>) -> Result<u128, DispatchError> {
        TryInto::<u128>::try_into(x).map_err(|_| Error::<T>::TryIntoIntError.into())
    }

    fn currency_to_fixed_point<C: TryInto<u128>>(x: C) -> Result<T::SignedFixedPoint, DispatchError> {
        let y = TryInto::<u128>::try_into(x).map_err(|_| Error::<T>::TryIntoIntError)?;
        let inner = TryInto::<Inner<T>>::try_into(y).map_err(|_| Error::<T>::TryIntoIntError)?;
        Ok(SignedFixedPoint::<T>::checked_from_integer(inner).ok_or(Error::<T>::TryIntoIntError)?)
    }
}
