use crate::Config;
use codec::{Decode, Encode, MaxEncodedLen};
use currency::Amount;
use frame_support::traits::Get;
pub use primitives::replace::{ReplaceRequest, ReplaceRequestStatus};
use primitives::VaultId;
use scale_info::TypeInfo;
use sp_core::H160;
use sp_runtime::DispatchError;
use vault_registry::types::CurrencyId;

/// Storage version.
#[derive(Encode, Decode, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
pub enum Version {
    /// Initial version.
    V0,
    /// BtcAddress type with script format.
    V1,
    /// Status, make all fields non-optional, remove open_time
    V2,
    /// active block number, btc_height
    V3,
}

pub(crate) type BalanceOf<T> = <T as vault_registry::Config>::Balance;

pub(crate) type DefaultVaultId<T> = VaultId<<T as frame_system::Config>::AccountId, CurrencyId<T>>;

#[derive(Encode, Decode, Default, Clone, PartialEq)]
pub(crate) struct ReplaceRequestV0<AccountId, BlockNumber, Balance> {
    pub old_vault: AccountId,
    pub open_time: BlockNumber,
    pub amount: Balance,
    pub griefing_collateral: Balance,
    pub new_vault: Option<AccountId>,
    pub collateral: Balance,
    pub accept_time: Option<BlockNumber>,
    pub btc_address: H160,
    pub completed: bool,
}

pub type DefaultReplaceRequest<T> = ReplaceRequest<
    <T as frame_system::Config>::AccountId,
    <T as frame_system::Config>::BlockNumber,
    BalanceOf<T>,
    CurrencyId<T>,
>;

pub trait ReplaceRequestExt<T: Config> {
    fn amount(&self) -> Amount<T>;
    fn griefing_collateral(&self) -> Amount<T>;
    fn collateral(&self) -> Result<Amount<T>, DispatchError>;
}

impl<T: Config> ReplaceRequestExt<T> for DefaultReplaceRequest<T> {
    fn amount(&self) -> Amount<T> {
        Amount::new(self.amount, self.old_vault.wrapped_currency())
    }
    fn griefing_collateral(&self) -> Amount<T> {
        Amount::new(self.griefing_collateral, T::GetGriefingCollateralCurrencyId::get())
    }
    fn collateral(&self) -> Result<Amount<T>, DispatchError> {
        Ok(Amount::new(self.collateral, self.new_vault.collateral_currency()))
    }
}
