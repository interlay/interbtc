//! # Currency Wrappers

#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]
#![feature(const_fn_trait_bound)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod amount;

use codec::{EncodeLike, FullCodec};
use frame_support::{
    dispatch::DispatchResult,
    traits::{Currency, ExistenceRequirement, Get, Imbalance, OnUnbalanced, WithdrawReasons},
    unsigned::TransactionValidityError,
};
use orml_traits::{MultiCurrency, MultiReservableCurrency};
use pallet_transaction_payment::OnChargeTransaction;
use primitives::TruncateFixedPointToInt;
use sp_runtime::{
    traits::{
        AtLeast32BitUnsigned, CheckedDiv, DispatchInfoOf, MaybeSerializeDeserialize, PostDispatchInfoOf, Saturating,
        Zero,
    },
    transaction_validity::InvalidTransaction,
    FixedPointNumber, FixedPointOperand,
};
use sp_std::{
    convert::{TryFrom, TryInto},
    fmt::Debug,
    marker::PhantomData,
};

pub use amount::Amount;
pub use pallet::*;

mod types;
use types::*;
pub use types::{CurrencyConversion, CurrencyId};

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
