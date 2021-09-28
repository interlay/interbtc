//! # Escrow Module
//! Receive vote-escrowed tokens for locking the native currency.

// #![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use codec::{Decode, Encode};
use frame_support::{
    ensure,
    traits::{
        BalanceStatus, Currency, ExistenceRequirement, Get, LockIdentifier, LockableCurrency, ReservableCurrency,
        SignedImbalance, WithdrawReasons,
    },
};
use sp_runtime::{
    traits::{AtLeast32BitUnsigned, Convert, Zero},
    DispatchError, DispatchResult,
};
use sp_std::marker::PhantomData;

const LOCK_ID: LockIdentifier = *b"escrowed";

type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
type PositiveImbalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::PositiveImbalance;
type NegativeImbalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::NegativeImbalance;
type MaxLocksOf<T> = <<T as Config>::Currency as LockableCurrency<<T as frame_system::Config>::AccountId>>::MaxLocks;

#[derive(Default, Encode, Decode, Debug)]
pub struct Point<Balance, BlockNumber> {
    bias: Balance,
    slope: Balance,
    ts: BlockNumber,
}

impl<Balance: AtLeast32BitUnsigned + Copy, BlockNumber: AtLeast32BitUnsigned + Copy> Point<Balance, BlockNumber> {
    fn new<BlockNumberToBalance: Convert<BlockNumber, Balance>>(
        amount: Balance,
        start_height: BlockNumber,
        end_height: BlockNumber,
        max_period: BlockNumber,
    ) -> Self {
        let max_period = BlockNumberToBalance::convert(max_period);
        let height_diff = BlockNumberToBalance::convert(end_height - start_height);

        let slope = amount / max_period;
        let bias = slope * height_diff;

        Self {
            bias,
            slope,
            ts: start_height,
        }
    }

    fn balance_at<BlockNumberToBalance: Convert<BlockNumber, Balance>>(&self, height: BlockNumber) -> Balance {
        let height_diff = BlockNumberToBalance::convert(height - self.ts);
        self.bias - (self.slope * (height_diff))
    }
}

#[derive(Default, Encode, Decode)]
pub struct LockedBalance<Balance, BlockNumber> {
    amount: Balance,
    end: BlockNumber,
}

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    /// ## Configuration
    /// The pallet's configuration trait.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Convert the block number into a balance.
        type BlockNumberToBalance: Convert<Self::BlockNumber, BalanceOf<Self>>;

        /// The currency trait.
        type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>
            + ReservableCurrency<Self::AccountId>;

        /// The maximum time for locks.
        #[pallet::constant]
        type MaxPeriod: Get<Self::BlockNumber>;
    }

    // The pallet's events
    #[pallet::event]
    // #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    #[pallet::metadata(T::AccountId = "AccountId")]
    pub enum Event<T: Config> {}

    #[pallet::error]
    pub enum Error<T> {
        InvalidAmount,
        InvalidHeight,
        NotExpired,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

    #[pallet::storage]
    #[pallet::getter(fn locked_balance)]
    pub type LockedBalances<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, LockedBalance<BalanceOf<T>, T::BlockNumber>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn point)]
    pub type Points<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, Point<BalanceOf<T>, T::BlockNumber>, ValueQuery>;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // The pallet's dispatchable functions.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(0)]
        pub fn lock(
            origin: OriginFor<T>,
            #[pallet::compact] amount: BalanceOf<T>,
            height: T::BlockNumber,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            Self::create_lock(&who, amount, height)?;
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn unlock(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            Self::remove_lock(&who)?;
            Ok(().into())
        }
    }
}

// "Internal" functions, callable by code.
impl<T: Config> Pallet<T> {
    fn current_height() -> T::BlockNumber {
        frame_system::Pallet::<T>::block_number()
    }

    fn create_lock(who: &T::AccountId, amount: BalanceOf<T>, height: T::BlockNumber) -> DispatchResult {
        // value MUST be non-zero
        ensure!(!amount.is_zero(), Error::<T>::InvalidAmount);

        // user MUST withdraw first
        ensure!(Self::locked_balance(who).amount.is_zero(), Error::<T>::InvalidAmount);

        // unlock MUST be in the future
        let start_height = Self::current_height();
        ensure!(height > start_height, Error::<T>::InvalidHeight);

        // height MUST not be greater than max
        let max_period = T::MaxPeriod::get();
        let end_height = start_height + max_period;
        ensure!(height <= end_height, Error::<T>::InvalidHeight);

        // TODO: reserve?
        T::Currency::set_lock(LOCK_ID, &who, amount, WithdrawReasons::all());
        <LockedBalances<T>>::insert(who, LockedBalance { amount, end: height });
        <Points<T>>::insert(
            who,
            Point::new::<T::BlockNumberToBalance>(amount, start_height, end_height, max_period),
        );

        Ok(())
    }

    fn remove_lock(who: &T::AccountId) -> DispatchResult {
        let locked_balance = <LockedBalances<T>>::get(who);
        let current_height = Self::current_height();

        // lock MUST have expired
        ensure!(current_height >= locked_balance.end, Error::<T>::NotExpired);

        T::Currency::remove_lock(LOCK_ID, &who);
        <LockedBalances<T>>::remove(who);
        <Points<T>>::remove(who);

        Ok(())
    }

    pub fn balance_at(who: &T::AccountId, height: Option<T::BlockNumber>) -> BalanceOf<T> {
        let height = height.unwrap_or(Self::current_height());
        let last_point = <Points<T>>::get(who);
        last_point.balance_at::<T::BlockNumberToBalance>(height)
    }
}

pub struct CurrencyAdapter<T>(PhantomData<T>);

impl<T> Currency<T::AccountId> for CurrencyAdapter<T>
where
    T: Config,
{
    type Balance = BalanceOf<T>;
    type PositiveImbalance = PositiveImbalanceOf<T>;
    type NegativeImbalance = NegativeImbalanceOf<T>;

    fn total_balance(who: &T::AccountId) -> Self::Balance {
        Pallet::<T>::balance_at(who, None)
    }

    fn can_slash(who: &T::AccountId, value: Self::Balance) -> bool {
        T::Currency::can_slash(who, value)
    }

    fn total_issuance() -> Self::Balance {
        T::Currency::total_issuance()
    }

    fn minimum_balance() -> Self::Balance {
        T::Currency::minimum_balance()
    }

    fn burn(amount: Self::Balance) -> Self::PositiveImbalance {
        T::Currency::burn(amount)
    }

    fn issue(amount: Self::Balance) -> Self::NegativeImbalance {
        T::Currency::issue(amount)
    }

    fn free_balance(who: &T::AccountId) -> Self::Balance {
        T::Currency::free_balance(who)
    }

    fn ensure_can_withdraw(
        who: &T::AccountId,
        amount: Self::Balance,
        reasons: WithdrawReasons,
        new_balance: Self::Balance,
    ) -> DispatchResult {
        T::Currency::ensure_can_withdraw(who, amount, reasons, new_balance)
    }

    fn transfer(
        source: &T::AccountId,
        dest: &T::AccountId,
        value: Self::Balance,
        existence_requirement: ExistenceRequirement,
    ) -> DispatchResult {
        T::Currency::transfer(source, dest, value, existence_requirement)
    }

    fn slash(who: &T::AccountId, value: Self::Balance) -> (Self::NegativeImbalance, Self::Balance) {
        T::Currency::slash(who, value)
    }

    fn deposit_into_existing(
        who: &T::AccountId,
        value: Self::Balance,
    ) -> sp_std::result::Result<Self::PositiveImbalance, DispatchError> {
        T::Currency::deposit_into_existing(who, value)
    }

    fn deposit_creating(who: &T::AccountId, value: Self::Balance) -> Self::PositiveImbalance {
        T::Currency::deposit_creating(who, value)
    }

    fn withdraw(
        who: &T::AccountId,
        value: Self::Balance,
        reasons: WithdrawReasons,
        liveness: ExistenceRequirement,
    ) -> sp_std::result::Result<Self::NegativeImbalance, DispatchError> {
        T::Currency::withdraw(who, value, reasons, liveness)
    }

    fn make_free_balance_be(
        who: &T::AccountId,
        balance: Self::Balance,
    ) -> SignedImbalance<Self::Balance, Self::PositiveImbalance> {
        T::Currency::make_free_balance_be(who, balance)
    }
}

impl<T> ReservableCurrency<T::AccountId> for CurrencyAdapter<T>
where
    T: Config,
{
    fn can_reserve(who: &T::AccountId, value: Self::Balance) -> bool {
        T::Currency::can_reserve(who, value)
    }

    fn slash_reserved(who: &T::AccountId, value: Self::Balance) -> (Self::NegativeImbalance, Self::Balance) {
        T::Currency::slash_reserved(who, value)
    }

    fn reserved_balance(who: &T::AccountId) -> Self::Balance {
        T::Currency::reserved_balance(who)
    }

    fn reserve(who: &T::AccountId, value: Self::Balance) -> DispatchResult {
        T::Currency::reserve(who, value)
    }

    fn unreserve(who: &T::AccountId, value: Self::Balance) -> Self::Balance {
        T::Currency::unreserve(who, value)
    }

    fn repatriate_reserved(
        slashed: &T::AccountId,
        beneficiary: &T::AccountId,
        value: Self::Balance,
        status: BalanceStatus,
    ) -> sp_std::result::Result<Self::Balance, DispatchError> {
        T::Currency::repatriate_reserved(slashed, beneficiary, value, status)
    }
}

impl<T> LockableCurrency<T::AccountId> for CurrencyAdapter<T>
where
    T: Config,
{
    type Moment = T::BlockNumber;
    type MaxLocks = MaxLocksOf<T>;

    fn set_lock(id: LockIdentifier, who: &T::AccountId, amount: Self::Balance, reasons: WithdrawReasons) {
        T::Currency::set_lock(id, who, amount, reasons)
    }

    fn extend_lock(id: LockIdentifier, who: &T::AccountId, amount: Self::Balance, reasons: WithdrawReasons) {
        T::Currency::extend_lock(id, who, amount, reasons)
    }

    fn remove_lock(id: LockIdentifier, who: &T::AccountId) {
        T::Currency::remove_lock(id, who)
    }
}
