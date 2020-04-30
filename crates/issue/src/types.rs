use frame_support::traits::Currency;

pub(crate) type DOT<T> =
    <<T as collateral::Trait>::DOT as Currency<<T as system::Trait>::AccountId>>::Balance;

pub(crate) type PolkaBTC<T> =
    <<T as treasury::Trait>::PolkaBTC as Currency<<T as system::Trait>::AccountId>>::Balance;
