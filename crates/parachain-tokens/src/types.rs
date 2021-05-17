use codec::{Decode, Encode};
use frame_support::{
    runtime_print,
    traits::{Currency, ExistenceRequirement::AllowDeath, WithdrawReasons},
};
use sp_runtime::traits::{CheckedConversion, SaturatedConversion};
use sp_std::{
    convert::{TryFrom, TryInto},
    fmt::Debug,
    marker::PhantomData,
    prelude::*,
};
use xcm::v0::{Error as XcmError, Junction, MultiAsset, MultiLocation, Result as XcmResult};
use xcm_executor::traits::{LocationConversion, TransactAsset};

#[cfg(not(feature = "disable-native-filter"))]
pub use xcm_executor::traits::NativeAsset;

#[cfg(feature = "disable-native-filter")]
use xcm_executor::traits::FilterAssetLocation;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

pub(crate) type Backing<T> =
    <<T as currency::Config<currency::Backing>>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

pub(crate) type Issuing<T> =
    <<T as currency::Config<currency::Issuing>>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[cfg(feature = "disable-native-filter")]
pub struct NativeAsset;

#[cfg(feature = "disable-native-filter")]
impl FilterAssetLocation for NativeAsset {
    fn filter_asset_location(_asset: &MultiAsset, _origin: &MultiLocation) -> bool {
        true
    }
}

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, PartialOrd, Ord)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum CurrencyId {
    DOT = 0,
    PolkaBTC = 1,
}

impl TryFrom<Vec<u8>> for CurrencyId {
    type Error = ();
    fn try_from(v: Vec<u8>) -> Result<CurrencyId, ()> {
        match v.as_slice() {
            b"DOT" => Ok(CurrencyId::DOT),
            b"POLKABTC" => Ok(CurrencyId::PolkaBTC),
            _ => Err(()),
        }
    }
}

impl Into<Vec<u8>> for CurrencyId {
    fn into(self) -> Vec<u8> {
        match self {
            CurrencyId::DOT => b"DOT".to_vec(),
            CurrencyId::PolkaBTC => b"POLKABTC".to_vec(),
        }
    }
}

pub struct CurrencyAdapter<Backing, Issuing, AccountIdConverter, AccountId>(
    PhantomData<Backing>,
    PhantomData<Issuing>,
    PhantomData<AccountIdConverter>,
    PhantomData<AccountId>,
);

impl<
        Backing: frame_support::traits::Currency<AccountId>,
        Issuing: frame_support::traits::Currency<AccountId>,
        AccountIdConverter: LocationConversion<AccountId>,
        AccountId: Debug, // can't get away without it since Currency is generic over it.
    > TransactAsset for CurrencyAdapter<Backing, Issuing, AccountIdConverter, AccountId>
{
    fn deposit_asset(asset: &MultiAsset, location: &MultiLocation) -> XcmResult {
        runtime_print!("Deposit asset: {:?}, location: {:?}", asset, location);
        let who = AccountIdConverter::from_location(location).ok_or(XcmError::BadOrigin)?;
        let currency_id = currency_id_from_asset(asset).ok_or(XcmError::Unimplemented)?;
        let amount: u128 = amount_from_asset::<u128>(asset)
            .ok_or(XcmError::BadOrigin)?
            .saturated_into();
        match currency_id {
            CurrencyId::DOT => {
                let balance_amount = amount.try_into().map_err(|_| XcmError::FailedToDecode)?;
                let _imbalance = Backing::deposit_creating(&who, balance_amount);
            }
            CurrencyId::PolkaBTC => {
                let balance_amount = amount.try_into().map_err(|_| XcmError::FailedToDecode)?;
                let _imbalance = Issuing::deposit_creating(&who, balance_amount);
            }
        }
        Ok(())
    }

    fn withdraw_asset(asset: &MultiAsset, location: &MultiLocation) -> Result<MultiAsset, XcmError> {
        runtime_print!("Withdraw asset: {:?}, location: {:?}", asset, location);
        let who = AccountIdConverter::from_location(location).ok_or(XcmError::BadOrigin)?;
        let currency_id = currency_id_from_asset(asset).ok_or(XcmError::Unimplemented)?;
        let amount: u128 = amount_from_asset::<u128>(asset)
            .ok_or(XcmError::BadOrigin)?
            .saturated_into();
        match currency_id {
            CurrencyId::DOT => {
                let balance_amount = amount.try_into().map_err(|_| XcmError::FailedToDecode)?;
                Backing::withdraw(&who, balance_amount, WithdrawReasons::TRANSFER, AllowDeath)
                    .map_err(|_| XcmError::CannotReachDestination)?;
            }
            CurrencyId::PolkaBTC => {
                let balance_amount = amount.try_into().map_err(|_| XcmError::FailedToDecode)?;
                Issuing::withdraw(&who, balance_amount, WithdrawReasons::TRANSFER, AllowDeath)
                    .map_err(|_| XcmError::CannotReachDestination)?;
            }
        }
        Ok(asset.clone())
    }
}

fn currency_id_from_asset(asset: &MultiAsset) -> Option<CurrencyId> {
    if let MultiAsset::ConcreteFungible { id: location, .. } = asset {
        if location == &MultiLocation::X1(Junction::Parent) {
            return Some(CurrencyId::DOT);
        }
        if let Some(Junction::GeneralKey(key)) = location.last() {
            return CurrencyId::try_from(key.clone()).ok();
        }
    }
    None
}

fn amount_from_asset<B: TryFrom<u128>>(asset: &MultiAsset) -> Option<B> {
    if let MultiAsset::ConcreteFungible { id: _, amount } = asset {
        return CheckedConversion::checked_from(*amount);
    }
    None
}
