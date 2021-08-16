pub use primitives::replace::{ReplaceRequest, ReplaceRequestStatus};

use codec::{Decode, Encode};
use sp_core::H160;

/// Storage version.
#[derive(Encode, Decode, Eq, PartialEq)]
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

pub(crate) type Collateral<T> = BalanceOf<T>;

pub(crate) type Wrapped<T> = BalanceOf<T>;

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
