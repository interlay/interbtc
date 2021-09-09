use crate::{
    pallet::{self, Config, Error},
    types::*,
};

use frame_support::{
    dispatch::{DispatchError, DispatchResult},
    ensure,
};
use orml_traits::{MultiCurrency, MultiReservableCurrency};
use primitives::TruncateFixedPointToInt;
use sp_runtime::{
    traits::{CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, UniqueSaturatedInto, Zero},
    FixedPointNumber,
};
use sp_std::{convert::TryInto, fmt::Debug};

#[cfg_attr(feature = "testing-utils", derive(Copy))]
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Amount<T: Config> {
    amount: BalanceOf<T>,
    currency_id: CurrencyId<T>,
}

#[cfg_attr(feature = "testing-utils", mocktopus::macros::mockable)]
impl<T: Config> Amount<T> {
    pub const fn new(amount: BalanceOf<T>, currency_id: CurrencyId<T>) -> Self {
        Self { amount, currency_id }
    }

    pub fn amount(&self) -> BalanceOf<T> {
        self.amount
    }

    pub fn currency(&self) -> CurrencyId<T> {
        self.currency_id
    }
}

#[cfg_attr(feature = "testing-utils", mocktopus::macros::mockable)]
mod conversions {
    use super::*;

    impl<T: Config> Amount<T> {
        pub fn from_signed_fixed_point(
            amount: SignedFixedPoint<T>,
            currency_id: CurrencyId<T>,
        ) -> Result<Self, DispatchError> {
            let amount = amount
                .truncate_to_inner()
                .ok_or(Error::<T>::TryIntoIntError)?
                .try_into()
                .map_err(|_| Error::<T>::TryIntoIntError)?;
            Ok(Self::new(amount, currency_id))
        }

        pub fn to_signed_fixed_point(&self) -> Result<SignedFixedPoint<T>, DispatchError> {
            let signed_inner =
                TryInto::<SignedInner<T>>::try_into(self.amount).map_err(|_| Error::<T>::TryIntoIntError)?;
            let signed_fixed_point = <T as pallet::Config>::SignedFixedPoint::checked_from_integer(signed_inner)
                .ok_or(Error::<T>::TryIntoIntError)?;
            Ok(signed_fixed_point)
        }

        pub fn convert_to(&self, currency_id: CurrencyId<T>) -> Result<Self, DispatchError> {
            T::CurrencyConversion::convert(self, currency_id)
        }
    }
}

#[cfg_attr(feature = "testing-utils", mocktopus::macros::mockable)]
mod math {
    use super::*;

    impl<T: Config> Amount<T> {
        pub fn zero(currency_id: CurrencyId<T>) -> Self {
            Self::new(0u32.into(), currency_id)
        }

        pub fn is_zero(&self) -> bool {
            self.amount.is_zero()
        }

        fn checked_fn<F>(&self, other: &Self, f: F, err: Error<T>) -> Result<Self, DispatchError>
        where
            F: Fn(&BalanceOf<T>, &BalanceOf<T>) -> Option<BalanceOf<T>>,
        {
            if self.currency_id != other.currency_id {
                return Err(Error::<T>::InvalidCurrency.into());
            }
            let amount = f(&self.amount, &other.amount).ok_or(err)?;

            Ok(Self {
                amount,
                currency_id: self.currency_id,
            })
        }

        pub fn checked_add(&self, other: &Self) -> Result<Self, DispatchError> {
            self.checked_fn(
                other,
                <BalanceOf<T> as CheckedAdd>::checked_add,
                Error::<T>::ArithmeticOverflow,
            )
        }

        pub fn checked_sub(&self, other: &Self) -> Result<Self, DispatchError> {
            self.checked_fn(
                other,
                <BalanceOf<T> as CheckedSub>::checked_sub,
                Error::<T>::ArithmeticUnderflow,
            )
        }

        pub fn saturating_sub(&self, other: &Self) -> Result<Self, DispatchError> {
            ensure!(self.currency_id == other.currency_id, Error::<T>::InvalidCurrency);
            self.checked_sub(other)
                .or_else(|_| Ok(Self::new(0u32.into(), self.currency_id)))
        }

        pub fn checked_fixed_point_mul(&self, scalar: &UnsignedFixedPoint<T>) -> Result<Self, DispatchError> {
            let amount = scalar
                .checked_mul_int(self.amount)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;
            Ok(Self {
                amount,
                currency_id: self.currency_id,
            })
        }

        pub fn checked_fixed_point_mul_rounded_up(
            &self,
            scalar: &UnsignedFixedPoint<T>,
        ) -> Result<Self, DispatchError> {
            let self_fixed_point =
                UnsignedFixedPoint::<T>::checked_from_integer(self.amount).ok_or(Error::<T>::TryIntoIntError)?;

            // do the multiplication
            let product = self_fixed_point
                .checked_mul(&scalar)
                .ok_or(Error::<T>::ArithmeticOverflow)?;

            // convert to inner
            let product_inner = UniqueSaturatedInto::<u128>::unique_saturated_into(product.into_inner());

            // convert to u128 by dividing by a rounded up division by accuracy
            let accuracy = UniqueSaturatedInto::<u128>::unique_saturated_into(UnsignedFixedPoint::<T>::accuracy());
            let amount = product_inner
                .checked_add(accuracy)
                .ok_or(Error::<T>::ArithmeticOverflow)?
                .checked_sub(1)
                .ok_or(Error::<T>::ArithmeticUnderflow)?
                .checked_div(accuracy)
                .ok_or(Error::<T>::ArithmeticUnderflow)?
                .try_into()
                .map_err(|_| Error::<T>::TryIntoIntError)?;

            Ok(Self {
                amount,
                currency_id: self.currency_id,
            })
        }

        pub fn checked_div(&self, scalar: &UnsignedFixedPoint<T>) -> Result<Self, DispatchError> {
            let amount = UnsignedFixedPoint::<T>::checked_from_integer(self.amount)
                .ok_or(Error::<T>::TryIntoIntError)?
                .checked_div(&scalar)
                .ok_or(Error::<T>::ArithmeticOverflow)?
                .truncate_to_inner()
                .ok_or(Error::<T>::TryIntoIntError)?;
            Ok(Self {
                amount,
                currency_id: self.currency_id,
            })
        }

        pub fn ratio(&self, other: &Self) -> Result<UnsignedFixedPoint<T>, DispatchError> {
            ensure!(self.currency_id == other.currency_id, Error::<T>::InvalidCurrency);
            let ratio = UnsignedFixedPoint::<T>::checked_from_rational(self.amount, other.amount)
                .ok_or(Error::<T>::TryIntoIntError)?;
            Ok(ratio)
        }

        pub fn min(&self, other: &Self) -> Result<Self, DispatchError> {
            ensure!(self.currency_id == other.currency_id, Error::<T>::InvalidCurrency);
            Ok(if self.le(other)? { self.clone() } else { other.clone() })
        }

        pub fn lt(&self, other: &Self) -> Result<bool, DispatchError> {
            ensure!(self.currency_id == other.currency_id, Error::<T>::InvalidCurrency);
            Ok(self.amount < other.amount)
        }

        pub fn le(&self, other: &Self) -> Result<bool, DispatchError> {
            ensure!(self.currency_id == other.currency_id, Error::<T>::InvalidCurrency);
            Ok(self.amount <= other.amount)
        }

        pub fn eq(&self, other: &Self) -> Result<bool, DispatchError> {
            ensure!(self.currency_id == other.currency_id, Error::<T>::InvalidCurrency);
            Ok(self.amount == other.amount)
        }

        pub fn ge(&self, other: &Self) -> Result<bool, DispatchError> {
            ensure!(self.currency_id == other.currency_id, Error::<T>::InvalidCurrency);
            Ok(self.amount >= other.amount)
        }

        pub fn gt(&self, other: &Self) -> Result<bool, DispatchError> {
            ensure!(self.currency_id == other.currency_id, Error::<T>::InvalidCurrency);
            Ok(self.amount > other.amount)
        }

        pub fn rounded_mul(&self, fraction: UnsignedFixedPoint<T>) -> Result<Self, DispatchError> {
            // we add 0.5 before we do the final integer division to round the result we return.
            // note that unwrapping is safe because we use a constant
            let rounding_addition = UnsignedFixedPoint::<T>::checked_from_rational(1, 2).unwrap();

            let amount = UnsignedFixedPoint::<T>::checked_from_integer(self.amount)
                .ok_or(Error::<T>::ArithmeticOverflow)?
                .checked_mul(&fraction)
                .ok_or(Error::<T>::ArithmeticOverflow)?
                .checked_add(&rounding_addition)
                .ok_or(Error::<T>::ArithmeticOverflow)?
                .truncate_to_inner()
                .ok_or(Error::<T>::TryIntoIntError)?;

            Ok(Self {
                amount,
                currency_id: self.currency_id,
            })
        }
    }
}

#[cfg_attr(feature = "testing-utils", mocktopus::macros::mockable)]
mod actions {
    use super::*;

    impl<T: Config> Amount<T> {
        pub fn transfer(&self, source: &T::AccountId, destination: &T::AccountId) -> Result<(), DispatchError> {
            <orml_tokens::Pallet<T> as MultiCurrency<T::AccountId>>::transfer(
                self.currency_id,
                source,
                destination,
                self.amount,
            )
        }

        pub fn lock_on(&self, account_id: &T::AccountId) -> Result<(), DispatchError> {
            <orml_tokens::Pallet<T>>::reserve(self.currency_id, account_id, self.amount)
        }

        pub fn unlock_on(&self, account_id: &T::AccountId) -> Result<(), DispatchError> {
            ensure!(
                <orml_tokens::Pallet<T>>::unreserve(self.currency_id, account_id, self.amount).is_zero(),
                orml_tokens::Error::<T>::BalanceTooLow
            );
            Ok(())
        }

        pub fn burn_from(&self, account_id: &T::AccountId) -> DispatchResult {
            ensure!(
                <orml_tokens::Pallet<T>>::slash_reserved(self.currency_id, account_id, self.amount).is_zero(),
                orml_tokens::Error::<T>::BalanceTooLow
            );
            Ok(())
        }

        pub fn mint_to(&self, account_id: &T::AccountId) -> DispatchResult {
            <orml_tokens::Pallet<T>>::deposit(self.currency_id, account_id, self.amount)
        }
    }
}

#[cfg(feature = "testing-utils")]
mod testing_utils {
    use super::*;
    use sp_std::{
        cmp::{Ordering, PartialOrd},
        ops::{Add, AddAssign, Div, Mul, Sub, SubAssign},
    };

    impl<T: Config> Amount<T> {
        pub fn with_amount<F: FnOnce(BalanceOf<T>) -> BalanceOf<T>>(&self, f: F) -> Self {
            Self {
                amount: f(self.amount),
                currency_id: self.currency_id,
            }
        }
    }
    impl<T: Config> AddAssign for Amount<T> {
        fn add_assign(&mut self, other: Self) {
            *self = self.clone() + other;
        }
    }

    impl<T: Config> SubAssign for Amount<T> {
        fn sub_assign(&mut self, other: Self) {
            *self = self.clone() - other;
        }
    }

    impl<T: Config> Add<Self> for Amount<T> {
        type Output = Self;

        fn add(self, other: Self) -> Self {
            if self.currency_id != other.currency_id {
                panic!("Adding two different currencies")
            }
            Self {
                amount: self.amount + other.amount,
                currency_id: self.currency_id,
            }
        }
    }

    impl<T: Config> Sub for Amount<T> {
        type Output = Self;

        fn sub(self, other: Self) -> Self {
            if self.currency_id != other.currency_id {
                panic!("Subtracting two different currencies")
            }
            Self {
                amount: self.amount - other.amount,
                currency_id: self.currency_id,
            }
        }
    }

    impl<T: Config> Mul<BalanceOf<T>> for Amount<T> {
        type Output = Self;

        fn mul(self, other: BalanceOf<T>) -> Self {
            Self {
                amount: self.amount * other,
                currency_id: self.currency_id,
            }
        }
    }

    impl<T: Config> Div<BalanceOf<T>> for Amount<T> {
        type Output = Self;

        fn div(self, other: BalanceOf<T>) -> Self {
            Self {
                amount: self.amount / other,
                currency_id: self.currency_id,
            }
        }
    }

    impl<T: Config> PartialOrd for Amount<T> {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            if self.currency_id != other.currency_id {
                None
            } else {
                Some(self.amount.cmp(&other.amount))
            }
        }
    }
}
