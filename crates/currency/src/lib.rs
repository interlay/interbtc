//! # Currency Wrappers

#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]
#![feature(const_fn_trait_bound)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use codec::{EncodeLike, FullCodec};
use frame_support::{
    dispatch::{DispatchError, DispatchResult},
    ensure,
    traits::{Currency, ExistenceRequirement, Get, Imbalance, OnUnbalanced, WithdrawReasons},
    unsigned::TransactionValidityError,
};
use orml_traits::{MultiCurrency, MultiReservableCurrency};
use pallet_transaction_payment::OnChargeTransaction;
use primitives::TruncateFixedPointToInt;
use sp_runtime::{
    traits::{
        AtLeast32BitUnsigned, CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, DispatchInfoOf,
        MaybeSerializeDeserialize, PostDispatchInfoOf, Saturating, UniqueSaturatedInto, Zero,
    },
    transaction_validity::InvalidTransaction,
    FixedPointNumber, FixedPointOperand,
};
use sp_std::{
    convert::{TryFrom, TryInto},
    fmt::Debug,
    marker::PhantomData,
};

pub use monetary::Amount;
pub use pallet::*;

mod types;
pub use types::CurrencyConversion;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;

    /// ## Configuration
    /// The pallet's configuration trait.
    #[pallet::config]
    pub trait Config: frame_system::Config + orml_tokens::Config<Balance = BalanceOf<Self>> {
        type UnsignedFixedPoint: FixedPointNumber<Inner = BalanceOf<Self>>
            + TruncateFixedPointToInt
            + Encode
            + EncodeLike
            + Decode
            + MaybeSerializeDeserialize;

        type SignedInner: Debug
            + CheckedDiv
            + TryFrom<BalanceOf<Self>>
            + TryInto<BalanceOf<Self>>
            + MaybeSerializeDeserialize;

        type SignedFixedPoint: FixedPointNumber<Inner = SignedInner<Self>>
            + TruncateFixedPointToInt
            + Encode
            + EncodeLike
            + Decode
            + MaybeSerializeDeserialize;

        type Balance: AtLeast32BitUnsigned
            + FixedPointOperand
            + MaybeSerializeDeserialize
            + FullCodec
            + Copy
            + Default
            + Debug;

        /// Wrapped currency: INTERBTC.
        #[pallet::constant]
        type GetWrappedCurrencyId: Get<CurrencyId<Self>>;

        type CurrencyConversion: types::CurrencyConversion<Amount<Self>, CurrencyId<Self>>;
    }

    #[pallet::error]
    pub enum Error<T> {
        ArithmeticOverflow,
        ArithmeticUnderflow,
        TryIntoIntError,
        InvalidCurrency,
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);
}

type CurrencyId<T> = <T as orml_tokens::Config>::CurrencyId;
type BalanceOf<T> = <T as Config>::Balance;
type SignedFixedPoint<T> = <T as Config>::SignedFixedPoint;
type UnsignedFixedPoint<T> = <T as Config>::UnsignedFixedPoint;
type SignedInner<T> = <T as Config>::SignedInner;

#[cfg_attr(feature = "testing-utils", mocktopus::macros::mockable)]
mod monetary {
    use super::*;

    #[cfg_attr(feature = "testing-utils", derive(Copy))]
    #[derive(Clone, PartialEq, Eq, Debug)]
    pub struct Amount<T: Config> {
        amount: BalanceOf<T>,
        currency_id: CurrencyId<T>,
    }

    // NOTE: all operations involving fixed point arguments operate on unsigned fixed point values,
    // unless the function name explicitly indicates it works on signed values
    impl<T: Config> Amount<T> {
        pub const fn new(amount: BalanceOf<T>, currency_id: CurrencyId<T>) -> Self {
            Self { amount, currency_id }
        }

        pub fn zero(currency_id: CurrencyId<T>) -> Self {
            Self::new(0u32.into(), currency_id)
        }

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

        pub fn amount(&self) -> BalanceOf<T> {
            self.amount
        }

        pub fn currency(&self) -> CurrencyId<T> {
            self.currency_id
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

        pub fn to_signed_fixed_point(&self) -> Result<SignedFixedPoint<T>, DispatchError> {
            let signed_inner =
                TryInto::<SignedInner<T>>::try_into(self.amount).map_err(|_| Error::<T>::TryIntoIntError)?;
            let signed_fixed_point = <T as pallet::Config>::SignedFixedPoint::checked_from_integer(signed_inner)
                .ok_or(Error::<T>::TryIntoIntError)?;
            Ok(signed_fixed_point)
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

        // lock, unlock, etc

        pub fn convert_to(&self, currency_id: CurrencyId<T>) -> Result<Self, DispatchError> {
            T::CurrencyConversion::convert(self, currency_id)
        }

        pub fn is_zero(&self) -> bool {
            self.amount.is_zero()
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
}

pub fn get_free_balance<T: Config>(currency_id: T::CurrencyId, account: &T::AccountId) -> Amount<T> {
    let amount = <orml_tokens::Pallet<T>>::free_balance(currency_id, account);
    Amount::new(amount, currency_id)
}

pub fn get_reserved_balance<T: Config>(currency_id: T::CurrencyId, account: &T::AccountId) -> Amount<T> {
    let amount = <orml_tokens::Pallet<T>>::reserved_balance(currency_id, account);
    Amount::new(amount, currency_id)
}

type NegativeImbalanceOf<T, GetCurrencyId> = <orml_tokens::CurrencyAdapter<T, GetCurrencyId> as Currency<
    <T as frame_system::Config>::AccountId,
>>::NegativeImbalance;

type PositiveImbalanceOf<T, GetCurrencyId> = <orml_tokens::CurrencyAdapter<T, GetCurrencyId> as Currency<
    <T as frame_system::Config>::AccountId,
>>::PositiveImbalance;

pub struct PaymentCurrencyAdapter<T, GetCurrencyId, OU>(PhantomData<(T, GetCurrencyId, OU)>);

// https://github.com/paritytech/substrate/blob/0bda86540d44b09da6f1ea6656f3f52d5447db81/frame/transaction-payment/src/payment.rs#L62
impl<T, GetCurrencyId, OU> OnChargeTransaction<T> for PaymentCurrencyAdapter<T, GetCurrencyId, OU>
where
    T: pallet_transaction_payment::Config + orml_tokens::Config,
    GetCurrencyId: Get<T::CurrencyId>,
    OU: OnUnbalanced<NegativeImbalanceOf<T, GetCurrencyId>>,
{
    type LiquidityInfo = Option<NegativeImbalanceOf<T, GetCurrencyId>>;
    type Balance = T::Balance;

    fn withdraw_fee(
        who: &T::AccountId,
        _call: &T::Call,
        _dispatch_info: &DispatchInfoOf<T::Call>,
        fee: Self::Balance,
        tip: Self::Balance,
    ) -> Result<Self::LiquidityInfo, TransactionValidityError> {
        if fee.is_zero() {
            return Ok(None);
        }

        let withdraw_reason = if tip.is_zero() {
            WithdrawReasons::TRANSACTION_PAYMENT
        } else {
            WithdrawReasons::TRANSACTION_PAYMENT | WithdrawReasons::TIP
        };

        match <orml_tokens::CurrencyAdapter<T, GetCurrencyId> as Currency<T::AccountId>>::withdraw(
            who,
            fee,
            withdraw_reason,
            ExistenceRequirement::KeepAlive,
        ) {
            Ok(imbalance) => Ok(Some(imbalance)),
            Err(_) => Err(InvalidTransaction::Payment.into()),
        }
    }

    fn correct_and_deposit_fee(
        who: &T::AccountId,
        _dispatch_info: &DispatchInfoOf<T::Call>,
        _post_info: &PostDispatchInfoOf<T::Call>,
        corrected_fee: Self::Balance,
        tip: Self::Balance,
        already_withdrawn: Self::LiquidityInfo,
    ) -> Result<(), TransactionValidityError> {
        if let Some(paid) = already_withdrawn {
            // Calculate how much refund we should return
            let refund_amount = paid.peek().saturating_sub(corrected_fee);
            // refund to the the account that paid the fees. If this fails, the
            // account might have dropped below the existential balance. In
            // that case we don't refund anything.
            let refund_imbalance =
                <orml_tokens::CurrencyAdapter<T, GetCurrencyId> as Currency<T::AccountId>>::deposit_into_existing(
                    who,
                    refund_amount,
                )
                .unwrap_or_else(|_| PositiveImbalanceOf::<T, GetCurrencyId>::zero());
            // merge the imbalance caused by paying the fees and refunding parts of it again.
            let adjusted_paid = paid
                .offset(refund_imbalance)
                .same()
                .map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Payment))?;
            // Call someone else to handle the imbalance (fee and tip separately)
            let (tip, fee) = adjusted_paid.split(tip);
            OU::on_unbalanceds(Some(fee).into_iter().chain(Some(tip)));
        }
        Ok(())
    }
}

pub trait OnSweep<AccountId, Balance> {
    fn on_sweep(who: &AccountId, amount: Balance) -> DispatchResult;
}

impl<AccountId, Balance> OnSweep<AccountId, Balance> for () {
    fn on_sweep(_: &AccountId, _: Balance) -> DispatchResult {
        Ok(())
    }
}

pub struct SweepFunds<T, GetAccountId>(PhantomData<(T, GetAccountId)>);

impl<T, GetAccountId> OnSweep<T::AccountId, Amount<T>> for SweepFunds<T, GetAccountId>
where
    T: Config,
    GetAccountId: Get<T::AccountId>,
{
    fn on_sweep(who: &T::AccountId, amount: Amount<T>) -> DispatchResult {
        // transfer the funds to treasury account
        amount.transfer(who, &GetAccountId::get())
    }
}
