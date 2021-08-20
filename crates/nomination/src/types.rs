pub(crate) type BalanceOf<T> = <T as vault_registry::Config>::Balance;

pub(crate) type Collateral<T> = BalanceOf<T>;

pub(crate) type SignedFixedPoint<T> = <T as currency::Config>::SignedFixedPoint;

pub(crate) type SignedInner<T> = <T as currency::Config>::SignedInner;
