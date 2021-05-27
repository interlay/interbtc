//! # Currency Module
//! Based on the [Collateral specification](https://interlay.gitlab.io/polkabtc-spec/spec/collateral.html).
//! Based on the [Treasury specification](https://interlay.gitlab.io/polkabtc-spec/spec/treasury.html).

#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use frame_support::{
    dispatch::{DispatchError, DispatchResult},
    ensure,
    traits::{Currency, ExistenceRequirement, ReservableCurrency},
};
use sp_runtime::traits::{CheckedAdd, CheckedSub};
use sp_std::vec::Vec;

pub type BalanceOf<T, I = ()> =
    <<T as Config<I>>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

pub use pallet::*;

pub type Collateral = pallet::Instance1;
pub type Wrapped = pallet::Instance2;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;

    /// ## Configuration
    /// The pallet's configuration trait.
    #[pallet::config]
    pub trait Config<I: 'static = ()>: frame_system::Config {
        /// The overarching event type.
        type Event: From<Event<Self, I>> + IsType<<Self as frame_system::Config>::Event>;

        /// The currency to manage.
        type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;

        /// The user-friendly name of the managed currency.
        #[pallet::constant]
        type Name: Get<Vec<u8>>;

        /// The identifier of the currency - e.g. ticker symbol.
        #[pallet::constant]
        type Symbol: Get<Vec<u8>>;

        /// The number of decimals used to represent one unit.
        #[pallet::constant]
        type Decimals: Get<u8>;
    }

    // The pallet's events
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    #[pallet::metadata(T::AccountId = "AccountId", BalanceOf<T, I> = "Balance")]
    pub enum Event<T: Config<I>, I: 'static = ()> {
        Mint(T::AccountId, BalanceOf<T, I>),
        Lock(T::AccountId, BalanceOf<T, I>),
        Unlock(T::AccountId, BalanceOf<T, I>),
        Burn(T::AccountId, BalanceOf<T, I>),
        Release(T::AccountId, BalanceOf<T, I>),
        Slash(T::AccountId, T::AccountId, BalanceOf<T, I>),
    }

    #[pallet::error]
    pub enum Error<T, I = ()> {
        /// Account has insufficient free balance
        InsufficientFreeBalance,
        /// Account has insufficient reserved balance
        InsufficientReservedBalance,
        /// Arithmetic overflow
        ArithmeticOverflow,
        /// Arithmetic underflow
        ArithmeticUnderflow,
    }

    #[pallet::hooks]
    impl<T: Config<I>, I: 'static> Hooks<T::BlockNumber> for Pallet<T, I> {}

    /// Note that an account's free and reserved balances are handled
    /// through the Balances module.
    ///
    /// Total locked balance
    #[pallet::storage]
    pub type TotalLocked<T: Config<I>, I: 'static = ()> = StorageValue<_, BalanceOf<T, I>, ValueQuery>;

    #[pallet::pallet]
    pub struct Pallet<T, I = ()>(_);

    // The pallet's dispatchable functions.
    #[pallet::call]
    impl<T: Config<I>, I: 'static> Pallet<T, I> {}
}

// "Internal" functions, callable by code.
impl<T: Config<I>, I: 'static> Pallet<T, I> {
    /// Total supply
    pub fn get_total_supply() -> BalanceOf<T, I> {
        T::Currency::total_issuance()
    }

    /// Total locked
    pub fn get_total_locked() -> BalanceOf<T, I> {
        <TotalLocked<T, I>>::get()
    }

    /// Balance of an account (wrapper)
    pub fn get_free_balance(account: &T::AccountId) -> BalanceOf<T, I> {
        T::Currency::free_balance(account)
    }

    /// Locked balance of an account (wrapper)
    pub fn get_reserved_balance(account: &T::AccountId) -> BalanceOf<T, I> {
        T::Currency::reserved_balance(account)
    }

    /// Increase the total supply of locked
    pub fn increase_total_locked(amount: BalanceOf<T, I>) -> DispatchResult {
        let new_locked = Self::get_total_locked()
            .checked_add(&amount)
            .ok_or(Error::<T, I>::ArithmeticOverflow)?;
        <TotalLocked<T, I>>::put(new_locked);
        Ok(())
    }

    /// Decrease the total supply of locked
    pub fn decrease_total_locked(amount: BalanceOf<T, I>) -> DispatchResult {
        let new_locked = Self::get_total_locked()
            .checked_sub(&amount)
            .ok_or(Error::<T, I>::ArithmeticUnderflow)?;
        <TotalLocked<T, I>>::put(new_locked);
        Ok(())
    }

    /// Mint an `amount` to the `account`.
    ///
    /// # Arguments
    ///
    /// * `account` - recipient account
    /// * `amount` - amount to credit
    pub fn mint(account: T::AccountId, amount: BalanceOf<T, I>) {
        // adds the amount to the total balance of tokens
        let minted = T::Currency::issue(amount);
        // adds the minted amount to the account's balance
        T::Currency::resolve_creating(&account, minted);

        Self::deposit_event(Event::Mint(account, amount));
    }

    /// Lock an `amount` of currency. Note: this removes it from the
    /// free balance and adds it to the locked supply.
    ///
    /// # Arguments
    ///
    /// * `account` - the account to operate on
    /// * `amount` - the amount to lock
    pub fn lock(account: &T::AccountId, amount: BalanceOf<T, I>) -> DispatchResult {
        T::Currency::reserve(account, amount).map_err(|_| Error::<T, I>::InsufficientFreeBalance)?;

        // update total locked balance
        Self::increase_total_locked(amount)?;

        Self::deposit_event(Event::Lock(account.clone(), amount));
        Ok(())
    }

    /// Unlock an `amount` of currency. Note: this removes it from the
    /// locked supply and adds it to the free balance.
    ///
    /// # Arguments
    ///
    /// * `account` - the account to operate on
    /// * `amount` - the amount to unlock
    pub fn unlock(account: T::AccountId, amount: BalanceOf<T, I>) -> DispatchResult {
        ensure!(
            T::Currency::unreserve(&account, amount) == 0u32.into(),
            Error::<T, I>::InsufficientReservedBalance
        );

        // update total locked balance
        Self::decrease_total_locked(amount)?;

        Self::deposit_event(Event::Unlock(account, amount));
        Ok(())
    }

    /// Burn an `amount` of previously locked currency.
    ///
    /// # Arguments
    ///
    /// * `account` - the account to operate on
    /// * `amount` - the amount to burn
    pub fn burn(account: &T::AccountId, amount: BalanceOf<T, I>) -> DispatchResult {
        ensure!(
            T::Currency::reserved_balance(account) >= amount,
            Error::<T, I>::InsufficientReservedBalance
        );

        // burn the tokens from the locked balance
        Self::decrease_total_locked(amount)?;

        // burn the tokens for the account
        // remainder should always be 0 and is checked above
        let (_burned_tokens, _remainder) = T::Currency::slash_reserved(&account, amount);

        Self::deposit_event(Event::Burn(account.clone(), amount));

        Ok(())
    }

    /// Release an `amount` of previously locked currency.
    ///
    /// # Arguments
    ///
    /// * `account` - the account to operate on
    /// * `amount` - the amount to burn
    pub fn release(account: &T::AccountId, amount: BalanceOf<T, I>) -> DispatchResult {
        ensure!(
            T::Currency::reserved_balance(&account) >= amount,
            Error::<T, I>::InsufficientReservedBalance
        );
        T::Currency::unreserve(account, amount);

        Self::decrease_total_locked(amount)?;

        Self::deposit_event(Event::Release(account.clone(), amount));

        Ok(())
    }

    /// Slash the currency and assign it to a receiver. Can only fail if
    /// the sender account's balance is too low. The balance on the
    /// receiver is not locked.
    ///
    /// # Arguments
    ///
    /// * `sender` - the account being slashed
    /// * `receiver` - the receiver of the amount
    /// * `amount` - the to be slashed amount
    pub fn slash(sender: T::AccountId, receiver: T::AccountId, amount: BalanceOf<T, I>) -> DispatchResult {
        ensure!(
            T::Currency::reserved_balance(&sender) >= amount,
            Error::<T, I>::InsufficientReservedBalance
        );
        Self::slash_saturated(sender, receiver, amount)?;
        Ok(())
    }

    /// Like slash, but with additional options to tweak the behavior
    ///
    /// # Arguments
    ///
    /// * `sender` - the account being slashed
    /// * `receiver` - the receiver of the amount
    /// * `amount` - the to be slashed amount
    pub fn slash_saturated(
        sender: T::AccountId,
        receiver: T::AccountId,
        amount: BalanceOf<T, I>,
    ) -> Result<BalanceOf<T, I>, DispatchError> {
        // slash the sender's currency
        let (slashed, remainder) = T::Currency::slash_reserved(&sender, amount);

        // add slashed amount to receiver and create account if it does not exists
        T::Currency::resolve_creating(&receiver, slashed);

        // subtraction should not be able to fail since remainder <= amount
        let slashed_amount = amount - remainder;

        Self::deposit_event(Event::Slash(sender, receiver.clone(), slashed_amount));

        // reserve the created amount for the receiver. This should not be able to fail, since the
        // call above will have created enough free balance to lock.
        T::Currency::reserve(&receiver, slashed_amount).map_err(|_| Error::<T, I>::InsufficientFreeBalance)?;

        Ok(slashed_amount)
    }

    /// Transfer an `amount` to the `destination`, may kill the `source` account if the balance
    /// falls below the `ExistentialDeposit` const.
    ///
    /// # Arguments
    ///
    /// * `source` - the account transferring tokens
    /// * `destination` - the account receiving tokens
    /// * `amount` - amount to transfer
    pub fn transfer(source: &T::AccountId, destination: &T::AccountId, amount: BalanceOf<T, I>) -> DispatchResult {
        T::Currency::transfer(source, destination, amount, ExistenceRequirement::AllowDeath)
    }

    /// Transfer locked to the free balance of another account
    ///
    /// # Arguments
    ///
    /// * `source` - the account with locked tokens
    /// * `destination` - the account receiving tokens
    /// * `amount` - the amount to transfer
    pub fn unlock_and_transfer(
        source: &T::AccountId,
        destination: &T::AccountId,
        amount: BalanceOf<T, I>,
    ) -> DispatchResult {
        // repatriate_reserved but create account
        T::Currency::slash_reserved(&source, amount);
        T::Currency::deposit_creating(&destination, amount);

        // unlock the tokens from the locked balance
        Self::decrease_total_locked(amount)?;

        Ok(())
    }

    /// Transfer free to the locked balance of another account
    ///
    /// # Arguments
    ///
    /// * `source` - the account with free tokens
    /// * `destination` - the account receiving locked tokens
    /// * `amount` - the amount to transfer
    pub fn transfer_and_lock(
        source: &T::AccountId,
        destination: &T::AccountId,
        amount: BalanceOf<T, I>,
    ) -> DispatchResult {
        Self::transfer(&source, &destination, amount)?;
        Self::lock(&destination, amount)?;
        Ok(())
    }
}
