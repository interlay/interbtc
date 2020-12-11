//! # PolkaBTC Fee Pallet

#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

mod ext;

#[cfg(test)]
extern crate mocktopus;

#[cfg(test)]
use mocktopus::macros::mockable;

use codec::{Decode, Encode, EncodeLike};
use frame_support::{decl_error, decl_event, decl_module, decl_storage};
use frame_support::{
    dispatch::{DispatchError, DispatchResult},
    traits::Currency,
    weights::Weight,
};
use frame_system::ensure_signed;
use sp_arithmetic::traits::*;
use sp_arithmetic::FixedPointNumber;
use sp_std::convert::TryInto;

pub(crate) type DOT<T> =
    <<T as collateral::Trait>::DOT as Currency<<T as frame_system::Trait>::AccountId>>::Balance;

pub(crate) type PolkaBTC<T> =
    <<T as treasury::Trait>::PolkaBTC as Currency<<T as frame_system::Trait>::AccountId>>::Balance;

// TODO: concrete type is the same, circumvent this conversion
pub(crate) type Inner<T> = <<T as Trait>::FixedPoint as FixedPointNumber>::Inner;

/// The pallet's configuration trait.
pub trait Trait: frame_system::Trait + collateral::Trait + treasury::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    type FixedPoint: FixedPointNumber + Encode + EncodeLike + Decode;
}

// The pallet's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as Fee {
        /// # Issue

        /// Fee share that users need to pay to issue PolkaBTC.
        IssueFee get(fn issue_fee) config(): T::FixedPoint;

        /// Default griefing collateral (in DOT) as a percentage of the locked
        /// collateral of a Vault a user has to lock to issue PolkaBTC.
        IssueGriefingCollateral get(fn issue_griefing_collateral) config(): T::FixedPoint;

        /// # Redeem

        /// Fee share that users need to pay to redeem PolkaBTC.
        RedeemFee get(fn redeem_fee) config(): T::FixedPoint;

        /// # Vault Registry

        /// If users execute a redeem with a Vault flagged for premium redeem,
        /// they can earn a DOT premium, slashed from the Vault's collateral.
        PremiumRedeemFee get(fn premium_redeem_fee) config(): T::FixedPoint;

        /// Fee paid to Vaults to auction / force-replace undercollateralized Vaults.
        /// This is slashed from the replaced Vault's collateral.
        AuctionRedeemFee get(fn auction_redeem_fee) config(): T::FixedPoint;

        /// Fee that a Vault has to pay if it fails to execute redeem or replace requests
        /// (for redeem, on top of the slashed BTC-in-DOT value of the request). The fee is
        /// paid in DOT based on the PolkaBTC amount at the current exchange rate.
        PunishmentFee get(fn punishment_fee) config(): T::FixedPoint;

        /// # Replace

        /// Default griefing collateral (in DOT) as a percentage of the to-be-locked DOT collateral
        /// of the new Vault. This collateral will be slashed and allocated to the replacing Vault
        /// if the to-be-replaced Vault does not transfer BTC on time.
        ReplaceGriefingCollateral get(fn replace_griefing_collateral) config(): T::FixedPoint;

        /// AccountId of the fee pool.
        AccountId get(fn account_id) config(): T::AccountId;

        EpochPeriod get(fn epoch_period) config(): T::BlockNumber;

        EpochRewards get(fn epoch_rewards): PolkaBTC<T>;

        TotalRewards: map hasher(blake2_128_concat) T::AccountId => PolkaBTC<T>;
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

        fn on_initialize(n: T::BlockNumber) -> Weight {
            if let Err(e) = Self::begin_block(n) {
                sp_runtime::print(e);
            }
            0
        }

        #[weight = 0]
        fn withdraw(origin, amount: PolkaBTC<T>) -> DispatchResult
        {
            let signer = ensure_signed(origin)?;
            let amount = <TotalRewards<T>>::get(signer.clone());
            ext::treasury::transfer::<T>(Self::account_id(), signer, amount)?;
            Ok(())
        }

    }
}

// "Internal" functions, callable by code.
#[cfg_attr(test, mockable)]
impl<T: Trait> Module<T> {
    fn begin_block(height: T::BlockNumber) -> DispatchResult {
        // only calculate rewards per epoch
        if height % <EpochPeriod<T>>::get() == 0.into() {
            // TODO: calculate rewards from SLA pallet

            // clear total rewards for current epoch
            <EpochRewards<T>>::kill();
        }

        Ok(())
    }

    fn btc_to_inner(x: PolkaBTC<T>) -> Result<Inner<T>, DispatchError> {
        // TODO: concrete type is the same, circumvent this conversion
        let y = TryInto::<u128>::try_into(x).map_err(|_| Error::<T>::TryIntoIntError)?;
        TryInto::<Inner<T>>::try_into(y).map_err(|_| Error::<T>::TryIntoIntError.into())
    }

    fn inner_to_btc(x: Inner<T>) -> Result<PolkaBTC<T>, DispatchError> {
        // TODO: add try_into for `FixedPointOperand` upstream
        let y = UniqueSaturatedInto::<u128>::unique_saturated_into(x);
        TryInto::<PolkaBTC<T>>::try_into(y).map_err(|_| Error::<T>::TryIntoIntError.into())
    }

    fn dot_to_inner(x: DOT<T>) -> Result<Inner<T>, DispatchError> {
        // TODO: concrete type is the same, circumvent this conversion
        let y = TryInto::<u128>::try_into(x).map_err(|_| Error::<T>::TryIntoIntError)?;
        TryInto::<Inner<T>>::try_into(y).map_err(|_| Error::<T>::TryIntoIntError.into())
    }

    fn inner_to_dot(x: Inner<T>) -> Result<DOT<T>, DispatchError> {
        // TODO: add try_into for `FixedPointOperand` upstream
        let y = UniqueSaturatedInto::<u128>::unique_saturated_into(x);
        TryInto::<DOT<T>>::try_into(y).map_err(|_| Error::<T>::TryIntoIntError.into())
    }

    fn calculate_for(
        amount: Inner<T>,
        percentage: T::FixedPoint,
    ) -> Result<Inner<T>, DispatchError> {
        T::FixedPoint::checked_from_integer(amount)
            .ok_or(Error::<T>::ArithmeticOverflow)?
            .checked_mul(&percentage)
            .ok_or(Error::<T>::ArithmeticOverflow)?
            .into_inner()
            .checked_div(&T::FixedPoint::accuracy())
            .ok_or(Error::<T>::ArithmeticUnderflow.into())
    }

    fn btc_for(
        amount: PolkaBTC<T>,
        percentage: T::FixedPoint,
    ) -> Result<PolkaBTC<T>, DispatchError> {
        Self::inner_to_btc(Self::calculate_for(
            Self::btc_to_inner(amount)?,
            percentage,
        )?)
    }

    fn dot_for(amount: DOT<T>, percentage: T::FixedPoint) -> Result<DOT<T>, DispatchError> {
        Self::inner_to_dot(Self::calculate_for(
            Self::dot_to_inner(amount)?,
            percentage,
        )?)
    }

    pub fn get_issue_fee(amount: PolkaBTC<T>) -> Result<PolkaBTC<T>, DispatchError> {
        Self::btc_for(amount, <IssueFee<T>>::get())
    }

    pub fn get_issue_griefing_collateral(amount: DOT<T>) -> Result<DOT<T>, DispatchError> {
        Self::dot_for(amount, <IssueGriefingCollateral<T>>::get())
    }

    pub fn get_redeem_fee(amount: PolkaBTC<T>) -> Result<PolkaBTC<T>, DispatchError> {
        Self::btc_for(amount, <RedeemFee<T>>::get())
    }

    pub fn get_premium_redeem_fee(amount: DOT<T>) -> Result<DOT<T>, DispatchError> {
        Self::dot_for(amount, <PremiumRedeemFee<T>>::get())
    }

    pub fn get_auction_redeem_fee(amount: DOT<T>) -> Result<DOT<T>, DispatchError> {
        Self::dot_for(amount, <AuctionRedeemFee<T>>::get())
    }

    pub fn get_punishment_fee(amount: DOT<T>) -> Result<DOT<T>, DispatchError> {
        Self::dot_for(amount, <PunishmentFee<T>>::get())
    }

    pub fn get_replace_griefing_collateral(amount: DOT<T>) -> Result<DOT<T>, DispatchError> {
        Self::dot_for(amount, <ReplaceGriefingCollateral<T>>::get())
    }

    pub fn increase_rewards_for_epoch(amount: PolkaBTC<T>) {
        <EpochRewards<T>>::set(<EpochRewards<T>>::get() + amount);
    }
}

decl_error! {
    pub enum Error for Module<T: Trait> {
        /// Unable to convert value
        TryIntoIntError,
        ArithmeticOverflow,
        ArithmeticUnderflow,
    }
}
