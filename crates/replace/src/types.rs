use codec::{Decode, Encode};
use frame_support::traits::Currency;
use sp_core::H160;

pub(crate) type DOT<T> =
    <<T as collateral::Trait>::DOT as Currency<<T as system::Trait>::AccountId>>::Balance;
pub(crate) type PolkaBTC<T> =
    <<T as treasury::Trait>::PolkaBTC as Currency<<T as system::Trait>::AccountId>>::Balance;

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Replace<AccountId, BlockNumber, PolkaBTC, DOT> {
    pub old_vault: AccountId,
    pub open_time: BlockNumber,
    pub amount: PolkaBTC,
    pub griefing_collateral: DOT,
    pub new_vault: Option<AccountId>,
    pub collateral: DOT,
    pub accept_time: Option<BlockNumber>,
    pub btc_address: H160,
}

impl<AccountId, BlockNumber, PolkaBTC, DOT> Replace<AccountId, BlockNumber, PolkaBTC, DOT> {
    pub fn add_new_vault(
        &mut self,
        new_vault_id: AccountId,
        accept_time: BlockNumber,
        collateral: DOT,
        btc_address: H160,
    ) {
        self.new_vault = Some(new_vault_id);
        self.accept_time = Some(accept_time);
        self.collateral = collateral;
        self.btc_address = btc_address;
    }

    pub fn has_new_owner(&self) -> bool {
        self.new_vault.is_some()
    }
}
