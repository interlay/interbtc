//! # PolkaBTC Collateral Module
//! Based on the [specification](https://interlay.gitlab.io/polkabtc-spec/spec/collateral.html).

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

type BalanceOf<T> = <<T as Config>::DOT as Currency<<T as frame_system::Config>::AccountId>>::Balance;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;

    /// ## Configuration
    /// The pallet's configuration trait.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The DOT currency
        type DOT: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;

        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    }

    // The pallet's events
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    #[pallet::metadata(T::AccountId = "AccountId", BalanceOf<T> = "Balance")]
    pub enum Event<T: Config> {
        LockCollateral(T::AccountId, BalanceOf<T>),
        ReleaseCollateral(T::AccountId, BalanceOf<T>),
        SlashCollateral(T::AccountId, T::AccountId, BalanceOf<T>),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Account has insufficient balance
        InsufficientFunds,
        /// Account has insufficient collateral
        InsufficientCollateralAvailable,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

    /// Note that an account's free and reserved balances are handled
    /// through the Balances module.
    ///
    /// Total locked DOT collateral
    #[pallet::storage]
    pub type TotalCollateral<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // The pallet's dispatchable functions.
    #[pallet::call]
    impl<T: Config> Pallet<T> {}
}

// "Internal" functions, callable by code.
impl<T: Config> Pallet<T> {
    /// Total supply of DOT
    pub fn get_total_supply() -> BalanceOf<T> {
        T::DOT::total_issuance()
    }

    /// Total locked DOT collateral
    pub fn get_total_collateral() -> BalanceOf<T> {
        <TotalCollateral<T>>::get()
    }

    /// Increase the locked collateral
    pub fn increase_total_collateral(amount: BalanceOf<T>) {
        let new_collateral = Self::get_total_collateral() + amount;
        <TotalCollateral<T>>::put(new_collateral);
    }

    /// Decrease the locked collateral
    pub fn decrease_total_collateral(amount: BalanceOf<T>) {
        let new_collateral = Self::get_total_collateral() - amount;
        <TotalCollateral<T>>::put(new_collateral);
    }

    /// Balance of an account (wrapper)
    pub fn get_balance_from_account(account: &T::AccountId) -> BalanceOf<T> {
        T::DOT::free_balance(account)
    }

    /// Locked balance of account
    pub fn get_collateral_from_account(account: &T::AccountId) -> BalanceOf<T> {
        T::DOT::reserved_balance(account)
    }

    /// Transfer DOT collateral
    ///
    /// # Arguments
    ///
    /// * `source` - the account to send dot
    /// * `destination` - the account receiving dot
    /// * `amount` - amount of DOT
    pub fn transfer(source: T::AccountId, destination: T::AccountId, amount: BalanceOf<T>) -> DispatchResult {
        T::DOT::transfer(&source, &destination, amount, ExistenceRequirement::AllowDeath)
    }

    /// Lock DOT collateral
    ///
    /// # Arguments
    ///
    /// * `sender` - the account locking tokens
    /// * `amount` - to be locked amount of DOT
    pub fn lock_collateral(sender: &T::AccountId, amount: BalanceOf<T>) -> DispatchResult {
        T::DOT::reserve(sender, amount).map_err(|_| Error::<T>::InsufficientFunds)?;

        Self::increase_total_collateral(amount);

        Self::deposit_event(Event::LockCollateral(sender.clone(), amount));
        Ok(())
    }

    /// Release DOT collateral
    ///
    /// # Arguments
    ///
    /// * `sender` - the account releasing tokens
    /// * `amount` - the to be released amount of DOT
    pub fn release_collateral(sender: &T::AccountId, amount: BalanceOf<T>) -> DispatchResult {
        ensure!(
            T::DOT::reserved_balance(&sender) >= amount,
            Error::<T>::InsufficientCollateralAvailable
        );
        T::DOT::unreserve(sender, amount);

        Self::decrease_total_collateral(amount);

        Self::deposit_event(Event::ReleaseCollateral(sender.clone(), amount));

        Ok(())
    }

    /// Slash DOT collateral and assign to a receiver. Can only fail if
    /// the sender account has too low collateral. The balance on the
    /// receiver is not locked.
    ///
    /// # Arguments
    ///
    /// * `sender` - the account being slashed
    /// * `receiver` - the receiver of the amount
    /// * `amount` - the to be slashed amount
    pub fn slash_collateral(sender: T::AccountId, receiver: T::AccountId, amount: BalanceOf<T>) -> DispatchResult {
        ensure!(
            T::DOT::reserved_balance(&sender) >= amount,
            Error::<T>::InsufficientCollateralAvailable
        );
        Self::slash_collateral_saturated(sender, receiver, amount)?;
        Ok(())
    }

    /// Like slash_collateral, but with additional options to tweak the behavior
    ///
    /// # Arguments
    ///
    /// * `sender` - the account being slashed
    /// * `receiver` - the receiver of the amount
    /// * `amount` - the to be slashed amount
    /// * `saturated` - If false, this will fail if insufficient collateral is available.
    /// Otherwise, it will slash whatever is available
    /// * 'to_reserved` - if true, lock the received funds
    pub fn slash_collateral_saturated(
        sender: T::AccountId,
        receiver: T::AccountId,
        amount: BalanceOf<T>,
    ) -> Result<BalanceOf<T>, DispatchError> {
        // slash the sender's collateral
        let (slashed, remainder) = T::DOT::slash_reserved(&sender, amount);

        // add slashed amount to receiver and create account if it does not exists
        T::DOT::resolve_creating(&receiver, slashed);

        // subtraction should not be able to fail since remainder <= amount
        let slashed_amount = amount - remainder;

        Self::deposit_event(Event::SlashCollateral(sender, receiver.clone(), slashed_amount));

        // reserve the created amount for the receiver. This should not be able to fail, since the
        // call above will have created enough free balance to lock.
        T::DOT::reserve(&receiver, slashed_amount).map_err(|_| Error::<T>::InsufficientFunds)?;

        Ok(slashed_amount)
    }
}
