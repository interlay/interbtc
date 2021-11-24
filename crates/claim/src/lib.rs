//! # Claim Module
//! Distributes block rewards to participants.

// #![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use frame_support::{
    traits::{Currency, Get},
    transactional,
};
use frame_system::{ensure_root, ensure_signed};
use orml_vesting::VestingSchedule;
use sp_runtime::{
    traits::{AtLeast32Bit, Convert, Saturating, StaticLookup, Zero},
    DispatchResult,
};

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    type BalanceOf<T> =
        <<T as orml_vesting::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    /// ## Configuration
    /// The pallet's configuration trait.
    #[pallet::config]
    pub trait Config: frame_system::Config + orml_vesting::Config {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Convert the block number into a balance.
        type BlockNumberToBalance: Convert<Self::BlockNumber, BalanceOf<Self>>;

        #[pallet::constant]
        type StartHeight: Get<Self::BlockNumber>;

        #[pallet::constant]
        type EndHeight: Get<Self::BlockNumber>;
    }

    // The pallet's events
    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        Inflation { total_inflation: BalanceOf<T> },
    }

    #[pallet::error]
    pub enum Error<T> {
        InvalidVestingSchedule,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

    #[pallet::storage]
    #[pallet::getter(fn locked_balance)]
    pub type Balances<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, BalanceOf<T>, ValueQuery>;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub balances: Vec<(T::AccountId, BalanceOf<T>)>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                balances: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            for (account_id, balance) in &self.balances {
                Balances::<T>::insert(account_id, balance);
            }
        }
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // The pallet's dispatchable functions.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(0)]
        #[transactional]
        pub fn insert_balance_claims(
            origin: OriginFor<T>,
            balances: Vec<(T::AccountId, BalanceOf<T>)>,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            for (account_id, balance) in &balances {
                Balances::<T>::insert(account_id, balance);
            }
            Ok(().into())
        }

        #[pallet::weight(0)]
        #[transactional]
        pub fn claim_balance(
            origin: OriginFor<T>,
            who: <T::Lookup as StaticLookup>::Source,
        ) -> DispatchResultWithPostInfo {
            // TODO: figure out how to coerce lookup type
            let _ = ensure_signed(origin)?;
            Self::try_claim_balance(who)?;
            Ok(().into())
        }
    }
}

// "Internal" functions, callable by code.
impl<T: Config> Pallet<T> {
    fn try_claim_balance(who: <T::Lookup as StaticLookup>::Source) -> DispatchResult {
        // TODO: verify claim
        // TODO: transfer free balance
        orml_vesting::Pallet::<T>::update_vesting_schedules(
            frame_system::RawOrigin::Root.into(),
            who.clone(),
            vec![compute_vesting_schedule::<_, _, T::BlockNumberToBalance>(
                T::StartHeight::get(),
                T::EndHeight::get(),
                Zero::zero(), // TODO: use correct period
                <Balances<T>>::take(T::Lookup::lookup(who.clone())?),
            )
            .ok_or(Error::<T>::InvalidVestingSchedule)?],
        )?;

        Ok(())
    }
}

pub fn compute_vesting_schedule<BlockNumber, Balance, BlockNumberToBalance>(
    start: BlockNumber,
    end: BlockNumber,
    period: BlockNumber,
    balance: Balance,
) -> Option<VestingSchedule<BlockNumber, Balance>>
where
    BlockNumber: AtLeast32Bit + Copy,
    Balance: AtLeast32Bit + Copy,
    BlockNumberToBalance: Convert<BlockNumber, Balance>,
{
    let height_diff = end.saturating_sub(start);
    let period_count = height_diff / period;

    Some(VestingSchedule {
        start,
        period,
        period_count: period_count.try_into().ok()?,
        per_period: balance / BlockNumberToBalance::convert(period_count),
    })
}
