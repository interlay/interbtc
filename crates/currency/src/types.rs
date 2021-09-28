use frame_support::dispatch::DispatchError;

use crate::Config;

pub type CurrencyId<T> = <T as orml_tokens::Config>::CurrencyId;

pub(crate) type BalanceOf<T> = <T as Config>::Balance;

pub(crate) type SignedFixedPoint<T> = <T as Config>::SignedFixedPoint;

pub(crate) type UnsignedFixedPoint<T> = <T as Config>::UnsignedFixedPoint;

pub(crate) type SignedInner<T> = <T as Config>::SignedInner;
pub trait CurrencyConversion<Amount, CurrencyId> {
    fn convert(amount: &Amount, to: CurrencyId) -> Result<Amount, DispatchError>;
}
