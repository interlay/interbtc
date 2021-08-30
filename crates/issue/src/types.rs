use codec::{Decode, Encode};
use currency::Amount;
use frame_support::traits::Get;
pub use primitives::issue::{IssueRequest, IssueRequestStatus};

use crate::Config;

/// Storage version.
#[derive(Encode, Decode, Eq, PartialEq)]
pub enum Version {
    /// Initial version.
    V0,
    /// BtcAddress type with script format.
    V1,
    /// IssueRequestStatus
    V2,
    /// ActiveBlockNumber, btc_height
    V3,
}

pub(crate) type BalanceOf<T> = <T as vault_registry::Config>::Balance;

pub(crate) type Collateral<T> = BalanceOf<T>;

pub(crate) type Wrapped<T> = BalanceOf<T>;

pub trait IssueRequestExt<T: Config> {
    fn amount(&self) -> Amount<T>;
    fn fee(&self) -> Amount<T>;
    fn griefing_collateral(&self) -> Amount<T>;
}

impl<T: Config> IssueRequestExt<T> for IssueRequest<T::AccountId, T::BlockNumber, BalanceOf<T>> {
    fn amount(&self) -> Amount<T> {
        Amount::new(self.amount, T::GetWrappedCurrencyId::get())
    }
    fn fee(&self) -> Amount<T> {
        Amount::new(self.fee, T::GetWrappedCurrencyId::get())
    }
    fn griefing_collateral(&self) -> Amount<T> {
        Amount::new(self.griefing_collateral, T::GetGriefingCollateralCurrencyId::get())
    }
}
