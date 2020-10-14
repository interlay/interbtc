use codec::{Decode, Encode};
use frame_support::traits::Currency;
use sp_core::H160;

pub(crate) type DOT<T> =
    <<T as collateral::Trait>::DOT as Currency<<T as frame_system::Trait>::AccountId>>::Balance;

pub(crate) type PolkaBTC<T> =
    <<T as treasury::Trait>::PolkaBTC as Currency<<T as frame_system::Trait>::AccountId>>::Balance;

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug, serde::Serialize))]
pub struct RedeemRequest<AccountId, BlockNumber, PolkaBTC, DOT> {
    pub vault: AccountId,
    pub opentime: BlockNumber,
    pub amount_polka_btc: PolkaBTC,
    pub amount_btc: PolkaBTC,
    pub amount_dot: DOT,
    pub premium_dot: DOT,
    pub redeemer: AccountId,
    pub btc_address: H160,
}
