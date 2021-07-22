//! # Currency Wrappers

#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

use codec::FullCodec;
use frame_support::{
    dispatch::{DispatchError, DispatchResult},
    ensure,
    traits::{Currency, ExistenceRequirement, Get, Imbalance, OnUnbalanced, WithdrawReasons},
    unsigned::TransactionValidityError,
};
use orml_traits::{MultiCurrency, MultiReservableCurrency};
use pallet_transaction_payment::OnChargeTransaction;
use sp_runtime::{
    traits::{AtLeast32BitUnsigned, DispatchInfoOf, MaybeSerializeDeserialize, PostDispatchInfoOf, Saturating, Zero},
    transaction_validity::InvalidTransaction,
};
use sp_std::{fmt::Debug, marker::PhantomData};

pub trait ParachainCurrency<AccountId> {
    /// The balance of an account.
    type Balance: AtLeast32BitUnsigned + FullCodec + Copy + MaybeSerializeDeserialize + Debug + Default;

    fn get_total_supply() -> Self::Balance;

    fn get_free_balance(account: &AccountId) -> Self::Balance;

    fn get_reserved_balance(account: &AccountId) -> Self::Balance;

    fn mint(account: &AccountId, amount: Self::Balance) -> DispatchResult;

    fn lock(account: &AccountId, amount: Self::Balance) -> DispatchResult;

    fn unlock(account: &AccountId, amount: Self::Balance) -> DispatchResult;

    fn burn(account: &AccountId, amount: Self::Balance) -> DispatchResult;

    fn slash(from: AccountId, to: AccountId, amount: Self::Balance) -> DispatchResult;

    fn slash_saturated(from: AccountId, to: AccountId, amount: Self::Balance) -> Result<Self::Balance, DispatchError>;

    fn transfer(from: &AccountId, to: &AccountId, amount: Self::Balance) -> DispatchResult;

    fn unlock_and_transfer(from: &AccountId, to: &AccountId, amount: Self::Balance) -> DispatchResult {
        Self::unlock(from, amount)?;
        Self::transfer(from, to, amount)
    }

    fn transfer_and_lock(from: &AccountId, to: &AccountId, amount: Self::Balance) -> DispatchResult {
        Self::transfer(from, to, amount)?;
        Self::lock(to, amount)
    }
}

impl<T, GetCurrencyId> ParachainCurrency<T::AccountId> for orml_tokens::CurrencyAdapter<T, GetCurrencyId>
where
    T: orml_tokens::Config,
    GetCurrencyId: Get<T::CurrencyId>,
{
    type Balance = T::Balance;

    fn get_total_supply() -> Self::Balance {
        <orml_tokens::Pallet<T>>::total_issuance(GetCurrencyId::get())
    }

    fn get_free_balance(account: &T::AccountId) -> Self::Balance {
        <orml_tokens::Pallet<T>>::free_balance(GetCurrencyId::get(), account)
    }

    fn get_reserved_balance(account: &T::AccountId) -> Self::Balance {
        <orml_tokens::Pallet<T>>::reserved_balance(GetCurrencyId::get(), account)
    }

    fn mint(account: &T::AccountId, amount: Self::Balance) -> DispatchResult {
        <orml_tokens::Pallet<T>>::deposit(GetCurrencyId::get(), account, amount)
    }

    fn lock(account: &T::AccountId, amount: Self::Balance) -> DispatchResult {
        <orml_tokens::Pallet<T>>::reserve(GetCurrencyId::get(), account, amount)
    }

    fn unlock(account: &T::AccountId, amount: Self::Balance) -> DispatchResult {
        ensure!(
            <orml_tokens::Pallet<T>>::unreserve(GetCurrencyId::get(), account, amount).is_zero(),
            orml_tokens::Error::<T>::BalanceTooLow
        );
        Ok(())
    }

    fn burn(account: &T::AccountId, amount: Self::Balance) -> DispatchResult {
        ensure!(
            <orml_tokens::Pallet<T>>::slash_reserved(GetCurrencyId::get(), account, amount).is_zero(),
            orml_tokens::Error::<T>::BalanceTooLow
        );
        Ok(())
    }

    fn slash(from: T::AccountId, to: T::AccountId, amount: Self::Balance) -> DispatchResult {
        ensure!(
            <orml_tokens::Pallet<T>>::reserved_balance(GetCurrencyId::get(), &from) >= amount,
            orml_tokens::Error::<T>::BalanceTooLow
        );
        Self::slash_saturated(from, to, amount)?;
        Ok(())
    }

    fn slash_saturated(
        from: T::AccountId,
        to: T::AccountId,
        amount: Self::Balance,
    ) -> Result<Self::Balance, DispatchError> {
        // slash the sender's currency
        let remainder = <orml_tokens::Pallet<T>>::slash_reserved(GetCurrencyId::get(), &from, amount);

        // subtraction should not be able to fail since remainder <= amount
        let slashed_amount = amount - remainder;

        // add slashed amount to receiver and create account if it does not exist
        <orml_tokens::Pallet<T>>::deposit(GetCurrencyId::get(), &to, slashed_amount)?;

        // reserve the created amount for the receiver. This should not be able to fail, since the
        // call above will have created enough free balance to lock.
        <orml_tokens::Pallet<T>>::reserve(GetCurrencyId::get(), &to, slashed_amount)?;

        Ok(slashed_amount)
    }

    fn transfer(from: &T::AccountId, to: &T::AccountId, amount: Self::Balance) -> DispatchResult {
        <orml_tokens::Pallet<T> as MultiCurrency<T::AccountId>>::transfer(GetCurrencyId::get(), from, to, amount)
    }
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

pub struct SweepFunds<T, GetAccountId, GetCurrencyId>(PhantomData<(T, GetAccountId, GetCurrencyId)>);

impl<T, GetAccountId, GetCurrencyId> OnSweep<T::AccountId, T::Balance> for SweepFunds<T, GetAccountId, GetCurrencyId>
where
    T: orml_tokens::Config,
    GetAccountId: Get<T::AccountId>,
    GetCurrencyId: Get<T::CurrencyId>,
{
    fn on_sweep(who: &T::AccountId, amount: T::Balance) -> DispatchResult {
        // transfer the funds to treasury account
        <orml_tokens::Pallet<T> as MultiCurrency<T::AccountId>>::transfer(
            GetCurrencyId::get(),
            who,
            &GetAccountId::get(),
            amount,
        )
    }
}
