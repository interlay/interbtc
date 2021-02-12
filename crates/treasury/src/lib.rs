#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(test)]
extern crate mocktopus;

use frame_support::traits::{Currency, ExistenceRequirement, ReservableCurrency};
/// # PolkaBTC Treasury implementation
/// The Treasury module according to the specification at
/// https://interlay.gitlab.io/polkabtc-spec/spec/treasury.html
// Substrate
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, dispatch::DispatchResult, ensure,
};
use sp_runtime::ModuleId;

type BalanceOf<T> =
    <<T as Config>::PolkaBTC as Currency<<T as frame_system::Config>::AccountId>>::Balance;

/// The treasury's module id, used for deriving its sovereign account ID.
const _MODULE_ID: ModuleId = ModuleId(*b"ily/trsy");

/// The pallet's configuration trait.
/// Instantiation of this pallet requires the existence of a module that
/// implements Currency and ReservableCurrency. The Balances module can be used
/// for this. The Balances module then gives functions for total supply, balances
/// of accounts, and any function defined by the Currency and ReservableCurrency
/// traits.
pub trait Config: frame_system::Config {
    /// The PolkaBTC currency
    type PolkaBTC: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;

    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;
}

// This pallet's storage items.
decl_storage! {
    trait Store for Module<T: Config> as Treasury {
        /// ## Storage
        /// Note that account's balances and locked balances are handled
        /// through the Balances module.
        ///
        /// Total locked PolkaDOT
        TotalLocked: BalanceOf<T>;
    }
}

// The pallet's events
decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Config>::AccountId,
        Balance = BalanceOf<T>,
    {
        Mint(AccountId, Balance),
        Lock(AccountId, Balance),
        Burn(AccountId, Balance),
    }
);

// The pallet's dispatchable functions.
decl_module! {
    /// The module declaration.
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        // Initializing events
        // this is needed only if you are using events in your pallet
        fn deposit_event() = default;
    }
}

impl<T: Config> Module<T> {
    /// Total supply of PolkaBTC
    pub fn get_total_supply() -> BalanceOf<T> {
        T::PolkaBTC::total_issuance()
    }

    /// Balance of an account (wrapper)
    pub fn get_balance_from_account(account: T::AccountId) -> BalanceOf<T> {
        T::PolkaBTC::free_balance(&account)
    }

    /// Locked balance of an account (wrapper)
    pub fn get_locked_balance_from_account(account: T::AccountId) -> BalanceOf<T> {
        T::PolkaBTC::reserved_balance(&account)
    }

    /// Increase the supply of locked PolkaBTC
    pub fn increase_total_locked(amount: BalanceOf<T>) {
        let new_locked = <TotalLocked<T>>::get() + amount;
        <TotalLocked<T>>::put(new_locked);
    }

    /// Decrease the supply of locked PolkaBTC
    pub fn decrease_total_locked(amount: BalanceOf<T>) {
        let new_locked = <TotalLocked<T>>::get() - amount;
        <TotalLocked<T>>::put(new_locked);
    }

    /// Mint new tokens
    ///
    /// # Arguments
    ///
    /// * `requester` - PolkaBTC user requesting new tokens
    /// * `amount` - to be issued amount of PolkaBTC
    pub fn mint(requester: T::AccountId, amount: BalanceOf<T>) {
        // adds the amount to the total balance of tokens
        let minted_tokens = T::PolkaBTC::issue(amount);
        // adds the added amount to the requester's balance
        T::PolkaBTC::resolve_creating(&requester, minted_tokens);

        Self::deposit_event(RawEvent::Mint(requester, amount));
    }

    /// Lock PolkaBTC tokens to burn them. Note: this removes them from the
    /// free balance of PolkaBTC and adds them to the locked supply of PolkaBTC.
    ///
    /// # Arguments
    ///
    /// * `redeemer` - the account redeeming tokens
    /// * `amount` - to be locked amount of PolkaBTC
    pub fn lock(redeemer: T::AccountId, amount: BalanceOf<T>) -> DispatchResult {
        T::PolkaBTC::reserve(&redeemer, amount).map_err(|_| Error::<T>::InsufficientFunds)?;

        // update total locked balance
        Self::increase_total_locked(amount);

        Self::deposit_event(RawEvent::Lock(redeemer, amount));
        Ok(())
    }

    /// Burn previously locked PolkaBTC tokens
    ///
    /// # Arguments
    ///
    /// * `redeemer` - the account redeeming tokens
    /// * `amount` - the to be burned amount of PolkaBTC
    pub fn burn(redeemer: T::AccountId, amount: BalanceOf<T>) -> DispatchResult {
        ensure!(
            T::PolkaBTC::reserved_balance(&redeemer) >= amount,
            Error::<T>::InsufficientLockedFunds
        );

        // burn the tokens from the locked balance
        Self::decrease_total_locked(amount);

        // burn the tokens for the redeemer
        // remainder should always be 0 and is checked above
        let (_burned_tokens, _remainder) = T::PolkaBTC::slash_reserved(&redeemer, amount);

        Self::deposit_event(RawEvent::Burn(redeemer, amount));

        Ok(())
    }

    /// Transfer PolkaBTC tokens, may kill the source account if the balance
    /// falls below the `ExistentialDeposit` const
    ///
    /// # Arguments
    ///
    /// * `source` - the account transferring tokens
    /// * `destination` - the account receiving tokens
    /// * `amount` - amount of PolkaBTC
    pub fn transfer(
        source: T::AccountId,
        destination: T::AccountId,
        amount: BalanceOf<T>,
    ) -> DispatchResult {
        T::PolkaBTC::transfer(
            &source,
            &destination,
            amount,
            ExistenceRequirement::AllowDeath,
        )
    }

    /// Transfer locked PolkaBTC to the free balance of another account
    ///
    /// # Arguments
    ///
    /// * `source` - the account with locked tokens
    /// * `destination` - the account receiving tokens
    /// * `amount` - amount of PolkaBTC
    pub fn unlock_and_transfer(
        source: T::AccountId,
        destination: T::AccountId,
        amount: BalanceOf<T>,
    ) -> DispatchResult {
        // repatriate_reserved but create account
        T::PolkaBTC::slash_reserved(&source, amount);
        T::PolkaBTC::deposit_creating(&destination, amount);

        // unlock the tokens from the locked balance
        Self::decrease_total_locked(amount);

        Ok(())
    }
}

decl_error! {
    pub enum Error for Module<T: Config> {
        InsufficientFunds,
        InsufficientLockedFunds,
    }
}
