use codec::{Decode, Encode};
use frame_support::traits::Currency;
use sp_core::H160;

pub(crate) type DOT<T> =
    <<T as collateral::Trait>::DOT as Currency<<T as system::Trait>::AccountId>>::Balance;

pub(crate) type PolkaBTC<T> =
    <<T as treasury::Trait>::PolkaBTC as Currency<<T as system::Trait>::AccountId>>::Balance;

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Redeem<AccountId, BlockNumber, PolkaBTC, DOT> {
    pub(crate) vault: AccountId,
    pub(crate) opentime: BlockNumber,
    pub(crate) amount_polka_btc: PolkaBTC,
    pub(crate) amount_btc: PolkaBTC,
    pub(crate) amount_dot: DOT,
    pub(crate) premium_dot: DOT,
    pub(crate) redeemer: AccountId,
    pub(crate) btc_address: H160,
}
