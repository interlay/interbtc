use crate::Trait;
use frame_support::traits::Currency;
use sp_arithmetic::FixedPointNumber;

pub(crate) type DOT<T> =
    <<T as collateral::Trait>::DOT as Currency<<T as frame_system::Trait>::AccountId>>::Balance;

pub(crate) type PolkaBTC<T> =
    <<T as treasury::Trait>::PolkaBTC as Currency<<T as frame_system::Trait>::AccountId>>::Balance;

pub(crate) type UnsignedFixedPoint<T> = <T as Trait>::UnsignedFixedPoint;

// TODO: concrete type is the same, circumvent this conversion
pub(crate) type Inner<T> = <<T as Trait>::UnsignedFixedPoint as FixedPointNumber>::Inner;
