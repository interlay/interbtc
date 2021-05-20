use frame_support::runtime_print;
use sp_runtime::traits::{CheckedConversion, Convert};
use sp_std::{convert::TryFrom, fmt::Debug, marker::PhantomData, prelude::*};
use xcm::v0::{Error as XcmError, MultiAsset, MultiLocation, Result as XcmResult};
use xcm_executor::{
    traits::{Convert as TryConvert, TransactAsset},
    Assets,
};

#[cfg(not(feature = "disable-native-filter"))]
pub use xcm_builder::NativeAsset;

#[cfg(feature = "disable-native-filter")]
use xcm_executor::traits::FilterAssetLocation;

#[cfg(feature = "disable-native-filter")]
pub struct NativeAsset;

#[cfg(feature = "disable-native-filter")]
impl FilterAssetLocation for NativeAsset {
    fn filter_asset_location(_asset: &MultiAsset, _origin: &MultiLocation) -> bool {
        true
    }
}

pub trait MultiCurrency<AccountId> {
    fn deposit(&self, account_id: &AccountId, amount: u128) -> XcmResult;
    fn withdraw(&self, account_id: &AccountId, amount: u128) -> XcmResult;
}

pub struct CurrencyAdapter<AccountId, AccountIdConvert, Currency, CurrencyConverter>(
    PhantomData<AccountId>,
    PhantomData<AccountIdConvert>,
    PhantomData<Currency>,
    PhantomData<CurrencyConverter>,
);

impl<
        AccountId: Clone + Debug, // can't get away without it since Currency is generic over it.
        AccountIdConvert: TryConvert<MultiLocation, AccountId>,
        Currency: Clone + MultiCurrency<AccountId>,
        CurrencyConvert: Convert<MultiAsset, Option<Currency>>,
    > TransactAsset for CurrencyAdapter<AccountId, AccountIdConvert, Currency, CurrencyConvert>
{
    fn deposit_asset(asset: &MultiAsset, location: &MultiLocation) -> XcmResult {
        runtime_print!("Deposit asset: {:?}, location: {:?}", asset, location);
        match (
            AccountIdConvert::convert_ref(location).ok(),
            CurrencyConvert::convert(asset.clone()),
            amount_from_asset::<u128>(asset),
        ) {
            (Some(account_id), Some(currency), Some(amount)) => currency.deposit(&account_id, amount),
            _ => Err(XcmError::BadOrigin),
        }
    }

    fn withdraw_asset(asset: &MultiAsset, location: &MultiLocation) -> Result<Assets, XcmError> {
        runtime_print!("Withdraw asset: {:?}, location: {:?}", asset, location);

        match (
            AccountIdConvert::convert_ref(location).ok(),
            CurrencyConvert::convert(asset.clone()),
            amount_from_asset::<u128>(asset),
        ) {
            (Some(account_id), Some(currency), Some(amount)) => currency.withdraw(&account_id, amount),
            _ => Err(XcmError::BadOrigin),
        }?;

        Ok(asset.clone().into())
    }
}

fn amount_from_asset<B: TryFrom<u128>>(asset: &MultiAsset) -> Option<B> {
    if let MultiAsset::ConcreteFungible { id: _, amount } = asset {
        return CheckedConversion::checked_from(*amount);
    }
    None
}
