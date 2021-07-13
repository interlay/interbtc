pub use primitives::refund::RefundRequest;

pub(crate) type BalanceOf<T> = <T as vault_registry::Config>::Balance;

pub(crate) type Wrapped<T> = BalanceOf<T>;
