#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use frame_support::dispatch::DispatchResult;
use frame_support::traits::{Currency, ExistenceRequirement, ReservableCurrency};
/// The Collateral module according to the specification at
/// https://interlay.gitlab.io/polkabtc-spec/spec/collateral.html
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, ensure, sp_runtime::ModuleId,
};

type BalanceOf<T> =
    <<T as Config>::DOT as Currency<<T as frame_system::Config>::AccountId>>::Balance;

/// The collateral's module id, used for deriving its sovereign account ID.
const _MODULE_ID: ModuleId = ModuleId(*b"ily/cltl");

/// The pallet's configuration trait.
pub trait Config: frame_system::Config {
    /// The DOT currency
    type DOT: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;

    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;
}

// This pallet's storage items.
decl_storage! {
    trait Store for Module<T: Config> as Collateral {
        /// ## Storage
        /// Note that account's balances and locked balances are handled
        /// through the Balances module.
        ///
        /// Total locked DOT collateral
        TotalCollateral: BalanceOf<T>;
    }
}

// The pallet's events
decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Config>::AccountId,
        Balance = BalanceOf<T>,
    {
        LockCollateral(AccountId, Balance),
        ReleaseCollateral(AccountId, Balance),
        SlashCollateral(AccountId, AccountId, Balance),
    }
);

decl_module! {
    /// The module declaration.
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        // Initializing events
        fn deposit_event() = default;
    }
}

impl<T: Config> Module<T> {
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
    pub fn transfer(
        source: T::AccountId,
        destination: T::AccountId,
        amount: BalanceOf<T>,
    ) -> DispatchResult {
        T::DOT::transfer(
            &source,
            &destination,
            amount,
            ExistenceRequirement::AllowDeath,
        )
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

        Self::deposit_event(RawEvent::LockCollateral(sender.clone(), amount));
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

        Self::deposit_event(RawEvent::ReleaseCollateral(sender.clone(), amount));

        Ok(())
    }

    /// Slash DOT collateral and assign to a receiver. Can only fail if
    /// the sender account has too low collateral.
    ///
    /// # Arguments
    ///
    /// * `sender` - the account being slashed
    /// * `receiver` - the receiver of the amount
    /// * `amount` - the to be slashed amount
    pub fn slash_collateral(
        sender: T::AccountId,
        receiver: T::AccountId,
        amount: BalanceOf<T>,
    ) -> DispatchResult {
        ensure!(
            T::DOT::reserved_balance(&sender) >= amount,
            Error::<T>::InsufficientCollateralAvailable
        );

        // slash the sender's collateral
        let (slashed, _remainder) = T::DOT::slash_reserved(&sender, amount);

        // add slashed amount to receiver and create account if it does not exists
        T::DOT::resolve_creating(&receiver, slashed);

        // reserve the created amount for the receiver
        T::DOT::reserve(&receiver, amount).map_err(|_| Error::<T>::InsufficientFunds)?;

        Self::deposit_event(RawEvent::SlashCollateral(sender, receiver, amount));

        Ok(())
    }
}

decl_error! {
    pub enum Error for Module<T: Config> {
        /// Account has insufficient balance
        InsufficientFunds,
        /// Account has insufficient collateral
        InsufficientCollateralAvailable,
    }
}
