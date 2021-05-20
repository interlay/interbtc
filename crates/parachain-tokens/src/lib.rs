#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

mod types;

use codec::FullCodec;
use cumulus_primitives_core::ParaId;
use frame_support::{
    dispatch::{DispatchError, DispatchResultWithPostInfo, Weight},
    traits::Get,
    transactional,
};
use sp_runtime::traits::{AtLeast32BitUnsigned, Convert};
use sp_std::{convert::TryInto, fmt::Debug, prelude::*, vec::Vec};
pub use types::{CurrencyAdapter, MultiCurrency, NativeAsset};
use xcm::v0::{Error as XcmError, ExecuteXcm, Junction::*, MultiAsset, MultiLocation, NetworkId, Order, Outcome, Xcm};

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

        /// Convert the AccountId type to bytes.
        type AccountId32Convert: Convert<Self::AccountId, [u8; 32]>;

        /// This chain's parachain ID.
        type ParaId: Get<ParaId>;

        /// The XCM message executor.
        type XcmExecutor: ExecuteXcm<Self::Call>;

        /// The currencies to manage.
        type MultiCurrency: Into<Vec<u8>> + FullCodec + Clone + Debug + Default + PartialEq;

        /// The core balance type.
        type Balance: AtLeast32BitUnsigned + FullCodec + Copy + MaybeSerializeDeserialize + Debug + Default;
    }

    // The pallet's events
    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    #[pallet::metadata(
        T::AccountId = "AccountId",
        T::MultiCurrency = "MultiCurrency",
        T::Balance = "Balance"
    )]
    pub enum Event<T: Config> {
        /// Transferred currency to parachain.
        /// [origin, para_id, recipient, network, currency, amount]
        Transfer(
            T::AccountId,
            ParaId,
            T::AccountId,
            NetworkId,
            T::MultiCurrency,
            T::Balance,
        ),
    }

    #[pallet::error]
    pub enum Error<T> {
        XcmExecutionFailed,
        TryIntoIntError,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // The pallet's dispatchable functions.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Transfer to parachain.
        #[pallet::weight(1000)]
        #[transactional]
        pub fn transfer_to_parachain(
            origin: OriginFor<T>,
            para_id: ParaId,
            recipient: T::AccountId,
            network: NetworkId,
            currency: T::MultiCurrency,
            #[pallet::compact] amount: T::Balance,
            max_weight: Weight,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            if para_id == T::ParaId::get() {
                return Ok(().into());
            }

            let raw_amount = Self::tokens_to_u128(amount)?;
            Self::execute_xcm(
                AccountId32 {
                    network: network.clone(),
                    id: T::AccountId32Convert::convert(who.clone()),
                }
                .into(),
                Self::_transfer_to_parachain(
                    para_id,
                    recipient.clone(),
                    network.clone(),
                    currency.clone(),
                    raw_amount,
                ),
                max_weight,
            )
            .map_err(|_| Error::<T>::XcmExecutionFailed)?;

            Self::deposit_event(Event::<T>::Transfer(who, para_id, recipient, network, currency, amount));

            Ok(().into())
        }
    }
}

// "Internal" functions, callable by code.
impl<T: Config> Pallet<T> {
    fn execute_xcm(origin: MultiLocation, message: Xcm<T::Call>, weight_limit: Weight) -> Result<(), XcmError> {
        match T::XcmExecutor::execute_xcm(origin, message, weight_limit) {
            Outcome::Complete(_) => Ok(()),
            Outcome::Incomplete(_, err) => Err(err),
            Outcome::Error(err) => Err(err),
        }
    }

    fn _transfer_to_parachain(
        para_id: ParaId,
        recipient: T::AccountId,
        network: NetworkId,
        currency: T::MultiCurrency,
        amount: u128,
    ) -> Xcm<T::Call> {
        Xcm::WithdrawAsset {
            assets: vec![MultiAsset::ConcreteFungible {
                id: GeneralKey(currency.into()).into(),
                amount,
            }],
            effects: vec![Order::DepositReserveAsset {
                assets: vec![MultiAsset::All],
                dest: (Parent, Parachain(para_id.into())).into(),
                effects: vec![Order::DepositAsset {
                    assets: vec![MultiAsset::All],
                    dest: AccountId32 {
                        network,
                        id: T::AccountId32Convert::convert(recipient),
                    }
                    .into(),
                }],
            }],
        }
    }

    fn tokens_to_u128<R: TryInto<u128>>(x: R) -> Result<u128, DispatchError> {
        TryInto::<u128>::try_into(x).map_err(|_| Error::<T>::TryIntoIntError.into())
    }
}
