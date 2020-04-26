use frame_support::traits::Currency;

pub(crate) type DOT<T> = <T as collateral::Trait>::DOT;
pub(crate) type DOTBalance<T> = <DOT<T> as Currency<<T as system::Trait>::AccountId>>::Balance;

pub(crate) type PolkaBTC<T> = <T as treasury::Trait>::PolkaBTC;
pub(crate) type PolkaBTCBalance<T> =
    <PolkaBTC<T> as Currency<<T as system::Trait>::AccountId>>::Balance;
