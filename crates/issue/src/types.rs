use codec::{Decode, Encode, MaxEncodedLen};
use currency::Amount;
use frame_system::pallet_prelude::BlockNumberFor;
pub use primitives::issue::{IssueRequest, IssueRequestStatus};
use primitives::VaultId;
use scale_info::TypeInfo;
use vault_registry::types::CurrencyId;

use crate::Config;

/// Storage version.
#[derive(Encode, Decode, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
pub enum Version {
    /// Initial version.
    V0,
    /// BtcAddress type with script format.
    V1,
    /// IssueRequestStatus
    V2,
    /// ActiveBlockNumber, btc_height
    V3,
    /// Removed refund
    V4,
}

pub(crate) type BalanceOf<T> = <T as currency::Config>::Balance;

pub(crate) type DefaultVaultId<T> = VaultId<<T as frame_system::Config>::AccountId, CurrencyId<T>>;

pub type DefaultIssueRequest<T> =
    IssueRequest<<T as frame_system::Config>::AccountId, BlockNumberFor<T>, BalanceOf<T>, CurrencyId<T>>;

pub trait IssueRequestExt<T: Config> {
    fn amount(&self) -> Amount<T>;
    fn fee(&self) -> Amount<T>;
    fn griefing_collateral(&self) -> Amount<T>;
}

impl<T: Config> IssueRequestExt<T> for DefaultIssueRequest<T> {
    fn amount(&self) -> Amount<T> {
        Amount::new(self.amount, self.vault.wrapped_currency())
    }
    fn fee(&self) -> Amount<T> {
        Amount::new(self.fee, self.vault.wrapped_currency())
    }
    fn griefing_collateral(&self) -> Amount<T> {
        Amount::new(self.griefing_collateral, self.griefing_currency)
    }
}
