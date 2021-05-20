//! # Slashing
//!
//! This file contains the basis for nomination against vaults. In particular, any
//! operation that would deduct collateral from a vault must consider its nominators.
//!
//! ## Overview
//!
//! The algorithm is based on the following work:
//! https://solmaz.io/2019/02/24/scalable-reward-changing/
//!
//! Whenever a vault is slashed we update the so-called `slash_per_token` variable
//! which allows us to compute the actual collateral proportional to a participant.
//! For example, if Alice and Bob nominate some backing collateral to Charlie who is
//! subsequently slashed (maybe due to a failed redeem) then we want to slash all
//! participants without expensive iteration. In this case we only need to check
//! on withdraw that Alice or Bob has enough remaining collateral after subtracting
//! an amount `to_slash`.

use crate::{Backing, Config, Error, RichVault, SignedFixedPoint};
use sp_runtime::{
    traits::{CheckedAdd, CheckedDiv, CheckedSub, UniqueSaturatedInto},
    FixedPointNumber, FixedPointOperand,
};
use sp_std::convert::{TryFrom, TryInto};

#[derive(Debug, PartialEq)]
pub enum SlashingError {
    TryIntoIntError,
    ArithmeticUnderflow,
    ArithmeticOverflow,
    InsufficientFunds,
}

fn inner_to_backing<Backing: TryFrom<u128>, Inner: FixedPointOperand>(x: Inner) -> Result<Backing, SlashingError> {
    let y = UniqueSaturatedInto::<u128>::unique_saturated_into(x);
    TryInto::<Backing>::try_into(y).map_err(|_| SlashingError::TryIntoIntError)
}

fn backing_to_inner<Backing: TryInto<u128>, Inner: FixedPointOperand>(x: Backing) -> Result<Inner, SlashingError> {
    let y = TryInto::<u128>::try_into(x).map_err(|_| SlashingError::TryIntoIntError)?;
    TryInto::<Inner>::try_into(y).map_err(|_| SlashingError::TryIntoIntError)
}

fn backing_to_fixed<Backing: TryInto<u128>, SignedFixedPoint: FixedPointNumber>(
    x: Backing,
) -> Result<SignedFixedPoint, SlashingError> {
    let signed_fixed_point =
        SignedFixedPoint::checked_from_integer(backing_to_inner::<Backing, SignedFixedPoint::Inner>(x)?)
            .ok_or(SlashingError::TryIntoIntError)?;
    Ok(signed_fixed_point)
}

pub trait Collateral<Backing, SignedFixedPoint, E: From<SlashingError>> {
    /// Get the amount to slash per unit.
    fn get_slash_per_token(&self) -> Result<SignedFixedPoint, E>;

    /// Get the collateral held by a vault, excluding nomination.
    fn get_collateral(&self) -> Backing;

    fn mut_collateral<F>(&mut self, func: F) -> Result<(), E>
    where
        F: Fn(&mut Backing) -> Result<(), E>;

    /// Get the total collateral held by a vault, including nomination.
    fn get_total_collateral(&self) -> Result<Backing, E>;

    fn mut_total_collateral<F>(&mut self, func: F) -> Result<(), E>
    where
        F: Fn(&mut Backing) -> Result<(), E>;

    /// Get the backing collateral for a vault - after slashing.
    fn get_backing_collateral(&self) -> Result<Backing, E>;

    fn mut_backing_collateral<F>(&mut self, func: F) -> Result<(), E>
    where
        F: Fn(&mut Backing) -> Result<(), E>;

    /// Get the participant's prior `slash_tally`.
    fn get_slash_tally(&self) -> SignedFixedPoint;

    fn mut_slash_tally<F>(&mut self, func: F) -> Result<(), E>
    where
        F: Fn(&mut SignedFixedPoint) -> Result<(), E>;
}

macro_rules! checked_add_mut {
    ($self:ident, $mut:ident, $amount:expr) => {
        $self.$mut(|value| {
            *value = value.checked_add($amount).ok_or(SlashingError::ArithmeticOverflow)?;
            Ok(())
        })?;
    };
}

macro_rules! checked_sub_mut {
    ($self:ident, $mut:ident, $amount:expr) => {
        $self.$mut(|value| {
            *value = value.checked_sub($amount).ok_or(SlashingError::ArithmeticUnderflow)?;
            Ok(())
        })?;
    };
}

pub(crate) trait Slashable<
    Backing: TryInto<u128> + TryFrom<u128> + CheckedSub,
    SignedFixedPoint: FixedPointNumber,
    E: From<SlashingError>,
>: Collateral<Backing, SignedFixedPoint, E>
{
    fn mut_slash_per_token<F>(&mut self, func: F) -> Result<(), E>
    where
        F: Fn(&mut SignedFixedPoint) -> Result<(), E>;

    /// Slash an amount of the total collateral.
    fn slash_collateral(&mut self, amount: Backing) -> Result<(), E> {
        checked_sub_mut!(self, mut_backing_collateral, &amount);

        let amount_as_fixed = backing_to_fixed::<Backing, SignedFixedPoint>(amount)?;
        let total_collateral = self.get_total_collateral()?;
        let total_collateral_as_fixed = backing_to_fixed::<Backing, SignedFixedPoint>(total_collateral)?;
        let amount_div_total_collateral = amount_as_fixed
            .checked_div(&total_collateral_as_fixed)
            .unwrap_or(SignedFixedPoint::zero());
        checked_add_mut!(self, mut_slash_per_token, &amount_div_total_collateral);

        Ok(())
    }
}

pub trait TryDepositCollateral<
    Backing: TryInto<u128> + CheckedAdd,
    SignedFixedPoint: FixedPointNumber,
    E: From<SlashingError>,
>: Collateral<Backing, SignedFixedPoint, E>
{
    /// Called by the vault or nominator to deposit collateral.
    fn try_deposit_collateral(&mut self, amount: Backing) -> Result<(), E> {
        checked_add_mut!(self, mut_collateral, &amount);
        checked_add_mut!(self, mut_total_collateral, &amount);
        checked_add_mut!(self, mut_backing_collateral, &amount);

        let amount_as_fixed = backing_to_fixed::<Backing, SignedFixedPoint>(amount)?;
        let slash_per_token = self.get_slash_per_token()?;
        let slash_per_token_mul_amount = slash_per_token
            .checked_mul(&amount_as_fixed)
            .ok_or(SlashingError::ArithmeticOverflow)?;
        checked_add_mut!(self, mut_slash_tally, &slash_per_token_mul_amount);

        Ok(())
    }
}

impl<
        T: Collateral<Backing, SignedFixedPoint, E>,
        Backing: TryInto<u128> + CheckedAdd,
        SignedFixedPoint: FixedPointNumber,
        E: From<SlashingError>,
    > TryDepositCollateral<Backing, SignedFixedPoint, E> for T
{
}

pub trait TryWithdrawCollateral<
    Backing: TryInto<u128> + TryFrom<u128> + CheckedSub + PartialOrd,
    SignedFixedPoint: FixedPointNumber,
    E: From<SlashingError>,
>: Collateral<Backing, SignedFixedPoint, E>
{
    /// Recompute the actual "stake" of a vault or nominator.
    fn compute_collateral(&self) -> Result<Backing, E> {
        let collateral = backing_to_fixed::<Backing, SignedFixedPoint>(self.get_collateral())?;
        let to_slash = collateral
            .checked_mul(&self.get_slash_per_token()?)
            .ok_or(SlashingError::ArithmeticOverflow)?
            .checked_sub(&self.get_slash_tally())
            .ok_or(SlashingError::ArithmeticUnderflow)?;
        let collateral = collateral
            .checked_sub(&to_slash)
            .ok_or(SlashingError::ArithmeticUnderflow)?;
        inner_to_backing::<Backing, SignedFixedPoint::Inner>(
            collateral
                .into_inner()
                .checked_div(&SignedFixedPoint::accuracy())
                .ok_or(SlashingError::ArithmeticUnderflow)?,
        )
        .map_err(Into::into)
    }

    /// Called by the vault or nominator to withdraw collateral.
    fn try_withdraw_collateral(&mut self, amount: Backing) -> Result<(), E> {
        let actual_collateral = self.compute_collateral()?;
        if amount > actual_collateral {
            return Err(SlashingError::InsufficientFunds.into());
        }

        checked_sub_mut!(self, mut_collateral, &amount);
        checked_sub_mut!(self, mut_total_collateral, &amount);
        checked_sub_mut!(self, mut_backing_collateral, &amount);

        let amount_as_fixed = backing_to_fixed::<Backing, SignedFixedPoint>(amount)?;
        let slash_per_token_mul_amount = self
            .get_slash_per_token()?
            .checked_mul(&amount_as_fixed)
            .ok_or(SlashingError::ArithmeticOverflow)?;
        checked_sub_mut!(self, mut_slash_tally, &slash_per_token_mul_amount);

        Ok(())
    }
}

impl<
        T: Collateral<Backing, SignedFixedPoint, E>,
        Backing: TryInto<u128> + TryFrom<u128> + CheckedSub + PartialOrd,
        SignedFixedPoint: FixedPointNumber,
        E: From<SlashingError>,
    > TryWithdrawCollateral<Backing, SignedFixedPoint, E> for T
{
}

impl<T: Config> Slashable<Backing<T>, SignedFixedPoint<T>, Error<T>> for RichVault<T> {
    fn mut_slash_per_token<F>(&mut self, func: F) -> Result<(), Error<T>>
    where
        F: Fn(&mut SignedFixedPoint<T>) -> Result<(), Error<T>>,
    {
        func(&mut self.data.slash_per_token)?;
        <crate::Vaults<T>>::insert(&self.data.id, self.data.clone());
        Ok(())
    }
}

impl<T: Config> Collateral<Backing<T>, SignedFixedPoint<T>, Error<T>> for RichVault<T> {
    fn get_slash_per_token(&self) -> Result<SignedFixedPoint<T>, Error<T>> {
        Ok(self.data.slash_per_token)
    }

    fn get_collateral(&self) -> Backing<T> {
        self.data.collateral
    }

    fn mut_collateral<F>(&mut self, func: F) -> Result<(), Error<T>>
    where
        F: Fn(&mut Backing<T>) -> Result<(), Error<T>>,
    {
        func(&mut self.data.collateral)?;
        <crate::Vaults<T>>::insert(&self.data.id, self.data.clone());
        Ok(())
    }

    fn get_total_collateral(&self) -> Result<Backing<T>, Error<T>> {
        Ok(self.data.total_collateral)
    }

    fn mut_total_collateral<F>(&mut self, func: F) -> Result<(), Error<T>>
    where
        F: Fn(&mut Backing<T>) -> Result<(), Error<T>>,
    {
        func(&mut self.data.total_collateral)?;
        <crate::Vaults<T>>::insert(&self.data.id, self.data.clone());
        Ok(())
    }

    fn get_backing_collateral(&self) -> Result<Backing<T>, Error<T>> {
        Ok(self.data.backing_collateral)
    }

    fn mut_backing_collateral<F>(&mut self, func: F) -> Result<(), Error<T>>
    where
        F: Fn(&mut Backing<T>) -> Result<(), Error<T>>,
    {
        func(&mut self.data.backing_collateral)?;
        <crate::Vaults<T>>::insert(&self.data.id, self.data.clone());
        Ok(())
    }

    fn get_slash_tally(&self) -> SignedFixedPoint<T> {
        self.data.slash_tally
    }

    fn mut_slash_tally<F>(&mut self, func: F) -> Result<(), Error<T>>
    where
        F: Fn(&mut SignedFixedPoint<T>) -> Result<(), Error<T>>,
    {
        func(&mut self.data.slash_tally)?;
        <crate::Vaults<T>>::insert(&self.data.id, self.data.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frame_support::{assert_err, assert_ok};
    use sp_arithmetic::FixedI128;

    #[derive(Default)]
    struct SimpleVault {
        collateral: u128,
        total_collateral: u128,
        backing_collateral: u128,
        slash_per_token: FixedI128,
        slash_tally: FixedI128,
    }

    impl Slashable<u128, FixedI128, SlashingError> for SimpleVault {
        fn mut_slash_per_token<F>(&mut self, func: F) -> Result<(), SlashingError>
        where
            F: Fn(&mut FixedI128) -> Result<(), SlashingError>,
        {
            func(&mut self.slash_per_token)
        }
    }

    impl Collateral<u128, FixedI128, SlashingError> for SimpleVault {
        fn get_slash_per_token(&self) -> Result<FixedI128, SlashingError> {
            Ok(self.slash_per_token)
        }

        fn get_collateral(&self) -> u128 {
            self.collateral
        }

        fn mut_collateral<F>(&mut self, func: F) -> Result<(), SlashingError>
        where
            F: Fn(&mut u128) -> Result<(), SlashingError>,
        {
            func(&mut self.collateral)
        }

        fn get_total_collateral(&self) -> Result<u128, SlashingError> {
            Ok(self.total_collateral)
        }

        fn mut_total_collateral<F>(&mut self, func: F) -> Result<(), SlashingError>
        where
            F: Fn(&mut u128) -> Result<(), SlashingError>,
        {
            func(&mut self.total_collateral)
        }

        fn get_backing_collateral(&self) -> Result<u128, SlashingError> {
            Ok(self.backing_collateral)
        }

        fn mut_backing_collateral<F>(&mut self, func: F) -> Result<(), SlashingError>
        where
            F: Fn(&mut u128) -> Result<(), SlashingError>,
        {
            func(&mut self.backing_collateral)
        }

        fn get_slash_tally(&self) -> FixedI128 {
            self.slash_tally
        }

        fn mut_slash_tally<F>(&mut self, func: F) -> Result<(), SlashingError>
        where
            F: Fn(&mut FixedI128) -> Result<(), SlashingError>,
        {
            func(&mut self.slash_tally)
        }
    }

    #[test]
    fn should_deposit_and_withdraw() {
        let mut vault = SimpleVault::default();
        assert_ok!(vault.try_deposit_collateral(100));
        assert_ok!(vault.compute_collateral(), 100);
        assert_ok!(vault.try_withdraw_collateral(100));
        assert_ok!(vault.compute_collateral(), 0);
        assert_err!(vault.try_withdraw_collateral(100), SlashingError::InsufficientFunds);
    }

    #[test]
    fn should_deposit_and_slash() {
        let mut vault = SimpleVault::default();
        assert_ok!(vault.try_deposit_collateral(250));
        assert_ok!(vault.try_deposit_collateral(250));
        assert_ok!(vault.compute_collateral(), 500);
        assert_ok!(vault.slash_collateral(34));
        assert_ok!(vault.compute_collateral(), 500 - 34);
        assert_ok!(vault.slash_collateral(66));
        assert_ok!(vault.compute_collateral(), 500 - 34 - 66);
    }

    #[test]
    fn should_deposit_and_slash_and_withdraw() {
        let mut vault = SimpleVault::default();
        assert_ok!(vault.try_deposit_collateral(12312));
        assert_ok!(vault.slash_collateral(142));
        assert_err!(vault.try_withdraw_collateral(12312), SlashingError::InsufficientFunds);
        assert_ok!(vault.try_withdraw_collateral(12312 - 142));
    }

    #[test]
    fn should_deposit_and_withdraw_and_slash() {
        let mut vault = SimpleVault::default();
        assert_ok!(vault.try_deposit_collateral(560));
        assert_ok!(vault.try_withdraw_collateral(100));
        assert_ok!(vault.compute_collateral(), 460);
        assert_ok!(vault.slash_collateral(100));
        assert_ok!(vault.compute_collateral(), 360);
    }

    #[test]
    fn should_deposit_after_slash() {
        let mut vault = SimpleVault::default();
        assert_ok!(vault.try_deposit_collateral(33333));
        assert_ok!(vault.slash_collateral(33));
        assert_ok!(vault.try_deposit_collateral(33333));
        assert_ok!(vault.compute_collateral(), 33333 - 33 + 33333);
    }

    #[test]
    fn should_withdraw_after_slash() {
        let mut vault = SimpleVault::default();
        assert_ok!(vault.try_deposit_collateral(6878));
        assert_ok!(vault.slash_collateral(233));
        assert_ok!(vault.try_withdraw_collateral(100));
        assert_ok!(vault.compute_collateral(), 6878 - 233 - 100);
        assert_ok!(vault.slash_collateral(13));
        assert_ok!(vault.try_withdraw_collateral(100));
        assert_ok!(vault.compute_collateral(), 6878 - 233 - 100 - 13 - 100);
    }

    #[test]
    fn should_slash_proportionally_to_total() {
        let mut vault = SimpleVault::default();
        assert_ok!(vault.try_deposit_collateral(100));
        vault.total_collateral += 100;
        vault.backing_collateral += 100;
        assert_ok!(vault.slash_collateral(20));
        assert_eq!(vault.backing_collateral, 180);
        assert_eq!(vault.collateral, 100);
        assert_ok!(vault.compute_collateral(), 90);
        assert_err!(vault.try_withdraw_collateral(100), SlashingError::InsufficientFunds);
        assert_ok!(vault.try_withdraw_collateral(80));
        assert_ok!(vault.compute_collateral(), 10);
    }

    #[test]
    fn should_compute_total_after_withdraw() {
        let mut vault = SimpleVault::default();
        assert_ok!(vault.try_deposit_collateral(100));
        vault.total_collateral += 100;
        vault.backing_collateral += 100;
        assert_ok!(vault.slash_collateral(20));
        assert_eq!(vault.backing_collateral, 180);
        assert_ok!(vault.compute_collateral(), 90);
        assert_ok!(vault.try_withdraw_collateral(90));
        assert_eq!(vault.backing_collateral, 90);
    }
}
