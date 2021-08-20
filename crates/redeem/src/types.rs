pub use primitives::redeem::{RedeemRequest, RedeemRequestStatus};
use sp_runtime::DispatchError;

use crate::{ext, Config};
use codec::{Decode, Encode};
use currency::Amount;
use frame_support::traits::Get;

/// Storage version.
#[derive(Encode, Decode, Eq, PartialEq)]
pub enum Version {
    /// Initial version.
    V0,
    /// BtcAddress type with script format.
    V1,
    /// RedeemRequestStatus, removed amount_dot and amount_polka_btc
    V2,
    /// ActiveBlockNumber, btc_height, transfer_fee_btc
    V3,
}

pub(crate) type BalanceOf<T> = <T as vault_registry::Config>::Balance;

pub(crate) type Collateral<T> = BalanceOf<T>;

pub(crate) type Wrapped<T> = BalanceOf<T>;

pub trait RedeemRequestExt<T: Config> {
    fn amount_btc(&self) -> Amount<T>;
    fn fee(&self) -> Amount<T>;
    fn premium(&self) -> Result<Amount<T>, DispatchError>;
    fn transfer_fee_btc(&self) -> Amount<T>;
}
impl<T: Config> RedeemRequestExt<T> for RedeemRequest<T::AccountId, T::BlockNumber, BalanceOf<T>> {
    fn amount_btc(&self) -> Amount<T> {
        Amount::new(self.amount_btc, T::GetWrappedCurrencyId::get())
    }
    fn fee(&self) -> Amount<T> {
        Amount::new(self.fee, T::GetWrappedCurrencyId::get())
    }
    fn premium(&self) -> Result<Amount<T>, DispatchError> {
        let currency_id = ext::vault_registry::get_collateral_currency::<T>(&self.vault)?;
        Ok(Amount::new(self.premium, currency_id))
    }
    fn transfer_fee_btc(&self) -> Amount<T> {
        Amount::new(self.transfer_fee_btc, T::GetWrappedCurrencyId::get())
    }
}
