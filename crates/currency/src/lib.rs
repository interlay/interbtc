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
use frame_support::{dispatch::DispatchResult, traits::Get};
use orml_traits::{MultiCurrency, MultiReservableCurrency};
use primitives::TruncateFixedPointToInt;
use scale_info::TypeInfo;
use sp_runtime::{
    traits::{AtLeast32BitUnsigned, CheckedDiv, MaybeSerializeDeserialize},
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
            + MaybeSerializeDeserialize
            + TypeInfo;

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

        /// Native currency e.g. INTR/KINT
        #[pallet::constant]
        type GetNativeCurrencyId: Get<CurrencyId<Self>>;

        /// Relay chain currency e.g. DOT/KSM
        #[pallet::constant]
        type GetRelayChainCurrencyId: Get<CurrencyId<Self>>;

        /// Wrapped currency e.g. IBTC/KBTC
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

pub mod getters {
    use super::*;

    pub fn get_relay_chain_currency_id<T: Config>() -> CurrencyId<T> {
        <T as Config>::GetRelayChainCurrencyId::get()
    }

    pub fn get_native_currency_id<T: Config>() -> CurrencyId<T> {
        <T as Config>::GetNativeCurrencyId::get()
    }

    pub fn get_wrapped_currency_id<T: Config>() -> CurrencyId<T> {
        <T as Config>::GetWrappedCurrencyId::get()
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
