//! # Escrow Pallet
//!
//! - [`Config`]
//! - [`Call`]
//!
//! ## Overview
//!
//! The escrow pallet allows accounts to lock the native currency and receive vote-escrowed tokens.
//! This voting power linearly decreases per block and tends toward zero as the height approaches
//! the max lockup period.
//!
//! This implementation is based in part on Curve's implementation, but explicitly follows
//! the specification at <https://spec.interlay.io/spec/escrow.html>.

#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

mod default_weights;
pub use default_weights::WeightInfo;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{
    ensure,
    traits::{
        BalanceStatus, Currency, ExistenceRequirement, Get, Imbalance, LockIdentifier, LockableCurrency,
        ReservableCurrency, SignedImbalance, WithdrawReasons,
    },
    transactional,
};
use reward::RewardsApi;
use scale_info::TypeInfo;
use sp_runtime::{
    traits::{AtLeast32BitUnsigned, CheckedSub, Convert, Saturating, Zero},
    DispatchError, DispatchResult,
};

const LOCK_ID: LockIdentifier = *b"escrowed";

type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
type PositiveImbalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::PositiveImbalance;
type NegativeImbalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::NegativeImbalance;

#[derive(Default, Encode, Decode, Debug, Clone, TypeInfo, MaxEncodedLen)]
pub struct Point<Balance, BlockNumber> {
    bias: Balance,
    slope: Balance,
    ts: BlockNumber,
}

impl<Balance: AtLeast32BitUnsigned + Default + Copy, BlockNumber: AtLeast32BitUnsigned + Copy>
    Point<Balance, BlockNumber>
{
    fn new<BlockNumberToBalance: Convert<BlockNumber, Balance>>(
        amount: Balance,
        start_height: BlockNumber,
        end_height: BlockNumber,
        max_period: BlockNumber,
    ) -> Self {
        let max_period = BlockNumberToBalance::convert(max_period);
        let height_diff = BlockNumberToBalance::convert(end_height.saturating_sub(start_height));

        let slope = amount.checked_div(&max_period).unwrap_or_default();
        let bias = slope.saturating_mul(height_diff);

        Self {
            bias,
            slope,
            ts: start_height,
        }
    }

    // w ^
    // 1 +
    //   | *
    //   |   *
    //   |     *
    //   |       *
    // 0 +---------+--> t
    //
    // Calculates the balance at some point in the future,
    // linearly **decreasing** since the start height.
    fn balance_at<BlockNumberToBalance: Convert<BlockNumber, Balance>>(&self, height: BlockNumber) -> Balance {
        let height_diff = BlockNumberToBalance::convert(height.saturating_sub(self.ts));
        self.bias.saturating_sub(self.slope.saturating_mul(height_diff))
    }

    // w ^
    // 1 +
    //   |       *
    //   |     *
    //   |   *
    //   | *
    // 0 +-------+----> t
    //
    // Calculates the balance at some point in the future,
    // linearly **increasing** since the start height.
    fn reverse_balance_at<BlockNumberToBalance: Convert<BlockNumber, Balance>>(
        &self,
        end: BlockNumber,
        now: BlockNumber,
    ) -> Balance {
        // NOTE: we could store the end height in `Point`, but this code is only
        // temporary whilst we rollout governance voting via restricted accounts
        let height_diff = BlockNumberToBalance::convert(end.saturating_sub(now));
        self.bias.saturating_sub(self.slope.saturating_mul(height_diff))
    }
}

#[derive(Default, Encode, Decode, Clone, TypeInfo, MaxEncodedLen)]
pub struct LockedBalance<Balance, BlockNumber> {
    pub amount: Balance,
    end: BlockNumber,
}

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{dispatch::DispatchResult, pallet_prelude::*};
    use frame_system::pallet_prelude::*;

    /// ## Configuration
    /// The pallet's configuration trait.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Convert the block number into a balance.
        type BlockNumberToBalance: Convert<Self::BlockNumber, BalanceOf<Self>>;

        /// The currency trait.
        type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

        /// All future times are rounded by this.
        #[pallet::constant]
        type Span: Get<Self::BlockNumber>;

        /// The maximum time for locks.
        #[pallet::constant]
        type MaxPeriod: Get<Self::BlockNumber>;

        /// Escrow reward pool.
        type EscrowRewards: reward::RewardsApi<(), Self::AccountId, BalanceOf<Self>>;

        /// Weight information for the extrinsics in this module.
        type WeightInfo: WeightInfo;
    }

    // The pallet's events
    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        Deposit {
            who: T::AccountId,
            amount: BalanceOf<T>,
            unlock_height: T::BlockNumber,
        },
        Withdraw {
            who: T::AccountId,
            amount: BalanceOf<T>,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Input amount must be non-zero.
        InputAmountZero,
        /// Lock already exists.
        LockFound,
        /// Lock does not exist.
        LockNotFound,
        /// Unlock height is not in the future.
        UnlockHeightNotInTheFuture,
        /// Unlock height is greater than max period.
        UnlockHeightTooFarInTheFuture,
        /// Lock amount must be non-zero.
        LockAmountZero,
        /// Unlock height should be greater than lock.
        UnlockHeightMustIncrease,
        /// Previous lock has not expired.
        LockNotExpired,
        /// Previous lock has expired.
        LockHasExpired,
        /// Lock amount is too large.
        LockAmountTooLow,
        /// Insufficient account balance.
        InsufficientFunds,
        /// Not supported.
        NotSupported,
        /// Incorrect Percent
        IncorrectPercent,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

    #[pallet::storage]
    #[pallet::getter(fn reserved_balance)]
    pub type Reserved<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, BalanceOf<T>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn locked_balance)]
    pub type Locked<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, LockedBalance<BalanceOf<T>, T::BlockNumber>, ValueQuery>;

    #[pallet::storage]
    pub type Epoch<T: Config> = StorageValue<_, T::Index, ValueQuery>;

    #[pallet::storage]
    pub type PointHistory<T: Config> =
        StorageMap<_, Identity, T::Index, Point<BalanceOf<T>, T::BlockNumber>, ValueQuery>;

    #[pallet::storage]
    pub type UserPointHistory<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Identity,
        T::Index,
        Point<BalanceOf<T>, T::BlockNumber>,
        ValueQuery,
    >;

    #[pallet::storage]
    pub type UserPointEpoch<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, T::Index, ValueQuery>;

    #[pallet::storage]
    pub type SlopeChanges<T: Config> = StorageMap<_, Blake2_128Concat, T::BlockNumber, BalanceOf<T>, ValueQuery>;

    // Accounts that are limited in how much they can mint.
    #[pallet::storage]
    pub type Limits<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, (T::BlockNumber, T::BlockNumber)>;

    // Accounts that are prohibited from locking tokens for voting.
    #[pallet::storage]
    pub type Blocks<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, bool, ValueQuery>;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // The pallet's dispatchable functions.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(<T as Config>::WeightInfo::create_lock())]
        #[transactional]
        pub fn create_lock(
            origin: OriginFor<T>,
            #[pallet::compact] amount: BalanceOf<T>,
            unlock_height: T::BlockNumber,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let now = Self::current_height();

            // lock time is rounded down to weeks
            let unlock_height = Self::round_height(unlock_height);

            // value MUST be non-zero
            ensure!(!amount.is_zero(), Error::<T>::InputAmountZero);

            // user MUST withdraw first
            ensure!(Self::locked_balance(&who).amount.is_zero(), Error::<T>::LockFound);

            // unlock MUST be in the future
            ensure!(unlock_height > now, Error::<T>::UnlockHeightNotInTheFuture);

            // height MUST NOT be greater than max
            let max_period = T::MaxPeriod::get();
            let end_height = now.saturating_add(max_period);
            ensure!(unlock_height <= end_height, Error::<T>::UnlockHeightTooFarInTheFuture);

            Self::deposit_for(&who, amount, unlock_height)
        }

        #[pallet::call_index(1)]
        #[pallet::weight(<T as Config>::WeightInfo::increase_amount())]
        #[transactional]
        pub fn increase_amount(origin: OriginFor<T>, #[pallet::compact] amount: BalanceOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let locked_balance = Self::locked_balance(&who);
            let now = Self::current_height();

            // value MUST be non-zero
            ensure!(!amount.is_zero(), Error::<T>::InputAmountZero);

            // lock MUST exist first
            ensure!(!locked_balance.amount.is_zero(), Error::<T>::LockNotFound);

            // lock MUST NOT be expired
            ensure!(locked_balance.end > now, Error::<T>::LockHasExpired);

            Self::deposit_for(&who, amount, Zero::zero()).into()
        }

        #[pallet::call_index(2)]
        #[pallet::weight(<T as Config>::WeightInfo::increase_unlock_height())]
        #[transactional]
        pub fn increase_unlock_height(origin: OriginFor<T>, unlock_height: T::BlockNumber) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let locked_balance = Self::locked_balance(&who);
            let now = Self::current_height();

            // lock time is rounded down to weeks
            let unlock_height = Self::round_height(unlock_height);

            // lock MUST NOT be expired
            ensure!(locked_balance.end > now, Error::<T>::LockHasExpired);

            // lock amount MUST be non-zero
            ensure!(!locked_balance.amount.is_zero(), Error::<T>::LockAmountZero);

            // lock duration MUST increase
            ensure!(unlock_height > locked_balance.end, Error::<T>::UnlockHeightMustIncrease);

            // height MUST NOT be greater than max
            let max_period = T::MaxPeriod::get();
            let end_height = now.saturating_add(max_period);
            ensure!(unlock_height <= end_height, Error::<T>::UnlockHeightTooFarInTheFuture);

            Self::deposit_for(&who, Zero::zero(), unlock_height)
        }

        #[pallet::call_index(3)]
        #[pallet::weight(<T as Config>::WeightInfo::withdraw())]
        #[transactional]
        pub fn withdraw(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Self::remove_lock(&who)
        }

        #[pallet::call_index(4)]
        #[pallet::weight(0)]
        #[transactional]
        pub fn set_account_limit(
            origin: OriginFor<T>,
            who: T::AccountId,
            start: T::BlockNumber,
            end: T::BlockNumber,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            <Limits<T>>::insert(&who, (start, end));
            Ok(().into())
        }

        #[pallet::call_index(5)]
        #[pallet::weight(0)]
        #[transactional]
        pub fn set_account_block(origin: OriginFor<T>, who: T::AccountId) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            <Blocks<T>>::insert(&who, true);
            Ok(().into())
        }

        /// Update the stake amount for a user.
        ///
        /// # Arguments
        ///
        /// * `origin` - Sender of the transaction.
        /// * `target_user` - The account ID of the user whose stake amount needs to be updated.
        #[pallet::call_index(6)]
        #[pallet::weight(<T as Config>::WeightInfo::update_user_stake())]
        #[transactional]
        pub fn update_user_stake(origin: OriginFor<T>, target_user: T::AccountId) -> DispatchResult {
            ensure_signed(origin)?;
            // call `deposit_for` for re calculation of stake amount
            Self::deposit_for(&target_user, Zero::zero(), Zero::zero())
        }
    }
}

type DefaultPoint<T> = Point<BalanceOf<T>, <T as frame_system::Config>::BlockNumber>;
type DefaultLockedBalance<T> = LockedBalance<BalanceOf<T>, <T as frame_system::Config>::BlockNumber>;

// "Internal" functions, callable by code.
impl<T: Config> Pallet<T> {
    fn current_height() -> T::BlockNumber {
        frame_system::Pallet::<T>::block_number()
    }

    fn round_height(height: T::BlockNumber) -> T::BlockNumber {
        let span = T::Span::get();
        (height / span) * span
    }

    // As in the Curve contract, we record global and per-user data, this
    // may necessitate multiple writes if the global points are outdated.
    // We do not interpret the zero-address as a global checkpoint.
    fn checkpoint(who: &T::AccountId, old_locked: DefaultLockedBalance<T>, new_locked: DefaultLockedBalance<T>) {
        let now = Self::current_height();
        let max_period = T::MaxPeriod::get();

        let u_old = if old_locked.end > now && old_locked.amount > Zero::zero() {
            Point::new::<T::BlockNumberToBalance>(old_locked.amount, now, old_locked.end, max_period)
        } else {
            Default::default()
        };

        let u_new = if new_locked.end > now && new_locked.amount > Zero::zero() {
            Point::new::<T::BlockNumberToBalance>(new_locked.amount, now, new_locked.end, max_period)
        } else {
            Default::default()
        };

        let mut old_dslope = <SlopeChanges<T>>::get(old_locked.end);
        let mut new_dslope = if !new_locked.end.is_zero() {
            if new_locked.end == old_locked.end {
                old_dslope
            } else {
                <SlopeChanges<T>>::get(new_locked.end)
            }
        } else {
            Zero::zero()
        };

        let mut epoch = <Epoch<T>>::get();
        let mut last_point = <PointHistory<T>>::get(epoch);
        let mut last_checkpoint = last_point.ts;

        let mut t_i = Self::round_height(last_checkpoint);
        while t_i < now {
            t_i.saturating_accrue(T::Span::get());
            let d_slope = if t_i > now {
                t_i = now;
                Zero::zero()
            } else {
                <SlopeChanges<T>>::get(t_i)
            };
            let height_diff = T::BlockNumberToBalance::convert(t_i.saturating_sub(last_checkpoint));
            last_point.bias.saturating_reduce(last_point.slope * height_diff);
            last_point.slope.saturating_accrue(d_slope);
            last_checkpoint = t_i;
            last_point.ts = t_i;
            epoch.saturating_inc();

            if t_i == now {
                break;
            }

            <PointHistory<T>>::insert(epoch, last_point.clone());
        }

        <Epoch<T>>::put(epoch);

        last_point
            .slope
            .saturating_accrue(u_new.slope.saturating_sub(u_old.slope));
        last_point.bias.saturating_accrue(u_new.bias.saturating_sub(u_old.bias));
        <PointHistory<T>>::insert(epoch, last_point);

        // schedule the slope change
        if old_locked.end > now {
            old_dslope.saturating_accrue(u_old.slope);
            if new_locked.end == old_locked.end {
                // new deposit
                old_dslope = old_dslope.saturating_sub(u_new.slope);
            }
            <SlopeChanges<T>>::insert(old_locked.end, old_dslope);
        }

        if new_locked.end > now && new_locked.end > old_locked.end {
            new_dslope = new_dslope.saturating_sub(u_new.slope);
            <SlopeChanges<T>>::insert(new_locked.end, new_dslope);
        }

        // finally update user history
        let user_epoch = <UserPointEpoch<T>>::mutate(who, |i| {
            i.saturating_inc();
            *i
        });
        <UserPointHistory<T>>::insert(who, user_epoch, u_new);
    }

    fn get_free_balance(who: &T::AccountId) -> BalanceOf<T> {
        let free_balance = T::Currency::free_balance(who);
        // prevent blocked accounts from minting
        if <Blocks<T>>::get(who) {
            Zero::zero()
        }
        // limit total deposit of restricted accounts
        else if let Some((start, end)) = <Limits<T>>::get(who) {
            // TODO: remove these restrictions in the future when the token distribution is complete
            let current_height = Self::current_height();
            let point = Point::new::<T::BlockNumberToBalance>(free_balance, start, end, end.saturating_sub(start));
            point.reverse_balance_at::<T::BlockNumberToBalance>(end, current_height)
        } else {
            free_balance
        }
    }

    fn deposit_for(who: &T::AccountId, amount: BalanceOf<T>, unlock_height: T::BlockNumber) -> DispatchResult {
        let old_locked = Self::locked_balance(who);
        let mut new_locked = old_locked.clone();
        new_locked.amount.saturating_accrue(amount);
        if unlock_height > Zero::zero() {
            new_locked.end = unlock_height;
        }

        // total amount can't be less than the max period to prevent rounding errors
        ensure!(
            new_locked.amount >= T::BlockNumberToBalance::convert(T::MaxPeriod::get()),
            Error::<T>::LockAmountTooLow,
        );

        ensure!(
            Self::get_free_balance(who) >= new_locked.amount,
            Error::<T>::InsufficientFunds,
        );
        T::Currency::set_lock(LOCK_ID, &who, new_locked.amount, WithdrawReasons::all());
        <Locked<T>>::insert(who, new_locked.clone());

        Self::checkpoint(who, old_locked, new_locked);

        // withdraw all stake and re-deposit escrow balance
        T::EscrowRewards::withdraw_all_stake(&(), who)?;
        T::EscrowRewards::deposit_stake(&(), who, Self::balance_at(who, None))?;

        Self::deposit_event(Event::<T>::Deposit {
            who: who.clone(),
            amount,
            unlock_height,
        });

        Ok(())
    }

    /// RPC helper
    pub fn round_height_and_deposit_for(
        who: &T::AccountId,
        amount: BalanceOf<T>,
        unlock_height: T::BlockNumber,
    ) -> DispatchResult {
        Self::deposit_for(who, amount, Self::round_height(unlock_height))
    }

    fn remove_lock(who: &T::AccountId) -> DispatchResult {
        let old_locked = <Locked<T>>::take(who);
        let amount = old_locked.amount;
        let current_height = Self::current_height();

        // lock MUST have expired
        ensure!(current_height >= old_locked.end, Error::<T>::LockNotExpired);

        // withdraw all stake
        T::EscrowRewards::withdraw_all_stake(&(), who)?;

        Self::checkpoint(who, old_locked, Default::default());

        T::Currency::remove_lock(LOCK_ID, &who);
        let _ = <UserPointHistory<T>>::clear_prefix(who, u32::max_value(), None);

        Self::deposit_event(Event::<T>::Withdraw {
            who: who.clone(),
            amount,
        });

        Ok(())
    }

    pub fn balance_at(who: &T::AccountId, height: Option<T::BlockNumber>) -> BalanceOf<T> {
        let height = height.unwrap_or(Self::current_height());
        let last_point = <UserPointHistory<T>>::get(who, <UserPointEpoch<T>>::get(who));
        last_point.balance_at::<T::BlockNumberToBalance>(height)
    }

    pub fn supply_at(point: DefaultPoint<T>, height: T::BlockNumber) -> BalanceOf<T> {
        let mut last_point = point;

        let mut t_i = Self::round_height(last_point.ts);
        while t_i < height {
            t_i.saturating_accrue(T::Span::get());

            let d_slope = if t_i > height {
                t_i = height;
                Zero::zero()
            } else {
                <SlopeChanges<T>>::get(t_i)
            };

            let height_diff = T::BlockNumberToBalance::convert(t_i.saturating_sub(last_point.ts));
            last_point.bias.saturating_reduce(last_point.slope * height_diff);

            if t_i == height {
                break;
            }

            last_point.slope.saturating_accrue(d_slope);
            last_point.ts = t_i;
        }

        last_point.bias
    }

    pub fn total_supply(height: Option<T::BlockNumber>) -> BalanceOf<T> {
        let height = height.unwrap_or(Self::current_height());
        let last_point = <PointHistory<T>>::get(<Epoch<T>>::get());
        Self::supply_at(last_point, height)
    }
}

impl<T: Config> Currency<T::AccountId> for Pallet<T> {
    type Balance = BalanceOf<T>;
    type PositiveImbalance = PositiveImbalanceOf<T>;
    type NegativeImbalance = NegativeImbalanceOf<T>;

    fn total_balance(who: &T::AccountId) -> Self::Balance {
        Pallet::<T>::balance_at(who, None)
    }

    // NOT SUPPORTED
    fn can_slash(_who: &T::AccountId, _value: Self::Balance) -> bool {
        false
    }

    fn total_issuance() -> Self::Balance {
        Pallet::<T>::total_supply(None)
    }

    fn minimum_balance() -> Self::Balance {
        T::Currency::minimum_balance()
    }

    // NOT SUPPORTED
    fn burn(_amount: Self::Balance) -> Self::PositiveImbalance {
        Imbalance::zero()
    }

    // NOT SUPPORTED
    fn issue(_amount: Self::Balance) -> Self::NegativeImbalance {
        Imbalance::zero()
    }

    fn free_balance(who: &T::AccountId) -> Self::Balance {
        Pallet::<T>::balance_at(who, None).saturating_sub(Pallet::<T>::reserved_balance(who))
    }

    // NOT SUPPORTED
    fn ensure_can_withdraw(
        _who: &T::AccountId,
        _amount: Self::Balance,
        _reasons: WithdrawReasons,
        _new_balance: Self::Balance,
    ) -> DispatchResult {
        Err(Error::<T>::NotSupported.into())
    }

    // NOT SUPPORTED
    fn transfer(
        _source: &T::AccountId,
        _dest: &T::AccountId,
        _value: Self::Balance,
        _existence_requirement: ExistenceRequirement,
    ) -> DispatchResult {
        Err(Error::<T>::NotSupported.into())
    }

    // NOT SUPPORTED
    fn slash(_who: &T::AccountId, _value: Self::Balance) -> (Self::NegativeImbalance, Self::Balance) {
        (Imbalance::zero(), Zero::zero())
    }

    // NOT SUPPORTED
    fn deposit_into_existing(
        _who: &T::AccountId,
        _value: Self::Balance,
    ) -> sp_std::result::Result<Self::PositiveImbalance, DispatchError> {
        Err(Error::<T>::NotSupported.into())
    }

    // NOT SUPPORTED
    fn deposit_creating(_who: &T::AccountId, _value: Self::Balance) -> Self::PositiveImbalance {
        Imbalance::zero()
    }

    // NOT SUPPORTED
    fn withdraw(
        _who: &T::AccountId,
        _value: Self::Balance,
        _reasons: WithdrawReasons,
        _liveness: ExistenceRequirement,
    ) -> sp_std::result::Result<Self::NegativeImbalance, DispatchError> {
        Err(Error::<T>::NotSupported.into())
    }

    fn make_free_balance_be(
        who: &T::AccountId,
        balance: Self::Balance,
    ) -> SignedImbalance<Self::Balance, Self::PositiveImbalance> {
        let now = Self::current_height();
        let max_period = T::MaxPeriod::get();
        let end_height = now.saturating_add(max_period);
        <UserPointHistory<T>>::insert(
            who,
            <UserPointEpoch<T>>::get(who),
            Point::new::<T::BlockNumberToBalance>(balance, now, end_height, max_period),
        );
        SignedImbalance::zero()
    }
}

impl<T: Config> ReservableCurrency<T::AccountId> for Pallet<T> {
    fn can_reserve(who: &T::AccountId, value: Self::Balance) -> bool {
        Pallet::<T>::free_balance(who).checked_sub(&value).is_some()
    }

    // NOT SUPPORTED
    fn slash_reserved(_who: &T::AccountId, _value: Self::Balance) -> (Self::NegativeImbalance, Self::Balance) {
        (Imbalance::zero(), Zero::zero())
    }

    fn reserved_balance(who: &T::AccountId) -> Self::Balance {
        Pallet::<T>::reserved_balance(who)
    }

    fn reserve(who: &T::AccountId, value: Self::Balance) -> DispatchResult {
        if !Pallet::<T>::can_reserve(who, value) {
            return Err(Error::<T>::InsufficientFunds.into());
        }
        <Reserved<T>>::mutate(who, |previous| previous.saturating_accrue(value));
        Ok(())
    }

    fn unreserve(who: &T::AccountId, value: Self::Balance) -> Self::Balance {
        <Reserved<T>>::mutate(who, |previous| {
            if value > *previous {
                let remainder = value.saturating_sub(*previous);
                *previous = Zero::zero();
                remainder
            } else {
                previous.saturating_reduce(value);
                Zero::zero()
            }
        })
    }

    // NOT SUPPORTED
    fn repatriate_reserved(
        _slashed: &T::AccountId,
        _beneficiary: &T::AccountId,
        _value: Self::Balance,
        _status: BalanceStatus,
    ) -> sp_std::result::Result<Self::Balance, DispatchError> {
        Err(Error::<T>::NotSupported.into())
    }
}
