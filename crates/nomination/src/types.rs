use sp_arithmetic::FixedPointNumber;

pub(crate) type BalanceOf<T> = <T as vault_registry::Config>::Balance;

pub(crate) type Collateral<T> = BalanceOf<T>;

pub(crate) type UnsignedFixedPoint<T> = <T as vault_registry::Config>::UnsignedFixedPoint;

pub(crate) type SignedFixedPoint<T> = <T as vault_registry::Config>::SignedFixedPoint;

pub(crate) type SignedInner<T> = <<T as vault_registry::Config>::SignedFixedPoint as FixedPointNumber>::Inner;
