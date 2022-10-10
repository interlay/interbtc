//! # ClientsInfo Module
//! Stores information about clients that comprise the network, such as vaults and oracles.

// #![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_runtime::traits::Zero;

mod default_weights;

pub use default_weights::WeightInfo;

use frame_support::{dispatch::DispatchResult, pallet_prelude::*, traits::Get, transactional};
use sp_runtime::{
    traits::{Convert, Saturating},
    SaturatedConversion,
};
use sp_std::{fmt::Debug, vec::Vec};
use xcm::{
    latest::{Error as XcmError, Instruction::*, MultiAsset, MultiLocation},
    v2::Fungibility,
};
use xcm_executor::{
    traits::{ShouldExecute, TransactAsset},
    Assets,
};

#[cfg(test)]
mod mock;

#[derive(Encode, Decode, Eq, PartialEq, Clone, Default, TypeInfo, Debug)]
pub struct ClientRelease<Hash> {
    /// URI to the client release binary.
    pub uri: Vec<u8>,
    /// The SHA256 checksum of the client binary.
    pub checksum: Hash,
}

pub use pallet::*;

pub trait LocationCategorizer {
    fn is_local_account(location: MultiLocation) -> bool;
}

#[frame_support::pallet]
pub mod pallet {
    use crate::*;

    use codec::FullCodec;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::{AtLeast32BitUnsigned, Saturating};

    #[pallet::pallet]
    #[pallet::generate_store(trait Store)]
    #[pallet::without_storage_info] // ClientRelease struct contains vec which doesn't implement MaxEncodedLen
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type Event: From<Event<Self>>
            + Into<<Self as frame_system::Config>::Event>
            + IsType<<Self as frame_system::Config>::Event>;

        /// Weight information for the extrinsics in this module.
        type WeightInfo: WeightInfo;

        type Transactor: TransactAsset;

        type CurrencyId: Parameter + Member + Copy + MaybeSerializeDeserialize + Ord;

        type CurrencyIdConvert: Convert<MultiAsset, Option<Self::CurrencyId>>;

        type Balance: AtLeast32BitUnsigned
            + Member
            + FullCodec
            + Copy
            + Saturating
            + Default
            + Debug
            + TypeInfo
            + MaxEncodedLen;

        type LocationCategorizer: LocationCategorizer;

        /// The interval at which the budget is reset.
        #[pallet::constant]
        type Period: Get<Self::BlockNumber>;
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {}

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {}

    #[pallet::error]
    pub enum Error<T> {}

    /// The total balance sent to other chains since the last checkpoint.
    #[pallet::storage]
    pub(super) type TotalOutbound<T: Config> = StorageMap<_, Blake2_128Concat, T::CurrencyId, T::Balance, ValueQuery>;

    /// The total balance received other chains since the last checkpoint.
    #[pallet::storage]
    pub(super) type TotalInbound<T: Config> = StorageMap<_, Blake2_128Concat, T::CurrencyId, T::Balance, ValueQuery>;

    /// The total balance allowed to be sent to other chains per interval.
    #[pallet::storage]
    pub(super) type OutboundLimit<T: Config> = StorageMap<_, Blake2_128Concat, T::CurrencyId, T::Balance, ValueQuery>;

    /// The total balance allowed to be received from other chains per interval.
    // todo: does having 2 separate limits make sense? Maybe 1 is enough
    #[pallet::storage]
    pub(super) type InboundLimit<T: Config> = StorageMap<_, Blake2_128Concat, T::CurrencyId, T::Balance, ValueQuery>;

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
        fn on_initialize(n: T::BlockNumber) -> Weight {
            let _ = Self::begin_block(n);
            0
        }
    }
}

#[cfg_attr(test, mockable)]
impl<T: Config> Pallet<T> {
    fn begin_block(height: T::BlockNumber) -> DispatchResult {
        if (height % T::Period::get()).is_zero() {
            let _ = <TotalInbound<T>>::clear(u32::max_value(), None);
            let _ = <TotalOutbound<T>>::clear(u32::max_value(), None);
        }
        Ok(())
    }

    fn extract_amount(asset: &MultiAsset) -> Result<(T::Balance, T::CurrencyId), XcmError> {
        let amount = match asset {
            &MultiAsset {
                fun: Fungibility::Fungible(x),
                ..
            } => x.saturated_into(),
            _ => return Err(XcmError::FailedToTransactAsset("FailedToMatchFungible")),
        };
        let currency_id = T::CurrencyIdConvert::convert(asset.clone())
            .ok_or(XcmError::FailedToTransactAsset("CurrencyIdConversionFailed"))?;
        Ok((amount, currency_id))
    }

    fn on_deposit(asset: &MultiAsset, location: &MultiLocation) -> Result<(), XcmError> {
        let (amount, currency_id) = Self::extract_amount(asset)?;
        if T::LocationCategorizer::is_local_account(location.clone()) {
            <TotalInbound<T>>::mutate(currency_id, |x| {
                x.saturating_accrue(amount);
                Ok(())
            })
        } else {
            <TotalOutbound<T>>::mutate(currency_id, |x| {
                x.saturating_accrue(amount);
                Ok(())
            })
        }
    }
}
impl<T: Config> TransactAsset for Pallet<T> {
    fn deposit_asset(asset: &MultiAsset, location: &MultiLocation) -> Result<(), XcmError> {
        Self::on_deposit(asset, location)?;
        T::Transactor::deposit_asset(asset, location)
    }

    fn withdraw_asset(asset: &MultiAsset, location: &MultiLocation) -> Result<Assets, XcmError> {
        T::Transactor::withdraw_asset(asset, location)
    }

    fn transfer_asset(asset: &MultiAsset, from: &MultiLocation, to: &MultiLocation) -> Result<Assets, XcmError> {
        Self::on_deposit(asset, to)?;
        T::Transactor::transfer_asset(asset, from, to)
    }
}

pub struct And<T: ShouldExecute, U: ShouldExecute>(PhantomData<(T, U)>);

impl<T: ShouldExecute, U: ShouldExecute> ShouldExecute for And<T, U> {
    fn should_execute<Call>(
        origin: &MultiLocation,
        message: &mut xcm::v2::Xcm<Call>,
        max_weight: frame_support::weights::Weight,
        weight_credit: &mut frame_support::weights::Weight,
    ) -> Result<(), ()> {
        T::should_execute(origin, message, max_weight, weight_credit)?;
        U::should_execute(origin, message, max_weight, weight_credit)?;
        // only if both returned ok, we return ok
        Ok(())
    }
}

impl<T: Config> ShouldExecute for Pallet<T> {
    fn should_execute<Call>(
        origin: &MultiLocation,
        message: &mut xcm::v2::Xcm<Call>,
        _max_weight: frame_support::weights::Weight,
        _weight_credit: &mut frame_support::weights::Weight,
    ) -> Result<(), ()> {
        let first_instruction = match message.0.iter().next() {
            Some(x) => x,
            None => return Ok(()), // not hitting our limit filter
        };

        let is_outbound = T::LocationCategorizer::is_local_account(origin.clone());

        if is_outbound {
            // xtokens executes the following on outbound transfers:
            // transfer_to_reserve: [WithdrawAsset, InitiateReserveWithdraw]
            // transfer_self_reserve_asset: TransferReserveAsset
            // transfer_to_non_reserve: [WithdrawAsset, InitiateReserveWithdraw]
            match first_instruction {
                WithdrawAsset(assets) | TransferReserveAsset { assets, .. } => {
                    for asset in assets.inner() {
                        let (amount, currency_id) = Self::extract_amount(asset).map_err(|_| ())?;
                        let limit = <OutboundLimit<T>>::get(currency_id);
                        let total_outbound = <TotalOutbound<T>>::get(currency_id);
                        if total_outbound.saturating_add(amount) > limit {
                            return Err(()); // disallow!
                        }
                    }
                }
                _ => {}
            }
        } else {
            match first_instruction {
                ReceiveTeleportedAsset(assets) | WithdrawAsset(assets) | ClaimAsset { assets, .. } => {
                    for asset in assets.inner() {
                        let (amount, currency_id) = Self::extract_amount(asset).map_err(|_| ())?;
                        let limit = <InboundLimit<T>>::get(currency_id);
                        let total_inbound = <TotalInbound<T>>::get(currency_id);
                        if total_inbound.saturating_add(amount) > limit {
                            return Err(()); // disallow!
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}
