use currency::Amount;
use frame_support::traits::Get;
pub use primitives::refund::RefundRequest;

pub(crate) type BalanceOf<T> = <T as vault_registry::Config>::Balance;

pub(crate) type Wrapped<T> = BalanceOf<T>;

pub type DefaultRefundRequest<T> = RefundRequest<<T as frame_system::Config>::AccountId, BalanceOf<T>>;

pub(crate) trait RefundRequestExt<T: crate::Config> {
    fn amount_wrapped(&self) -> Amount<T>;
    fn fee(&self) -> Amount<T>;
    fn amount_btc(&self) -> Amount<T>;
}

impl<T: crate::Config> RefundRequestExt<T> for RefundRequest<T::AccountId, BalanceOf<T>> {
    fn amount_wrapped(&self) -> Amount<T> {
        Amount::new(self.amount_wrapped, T::GetWrappedCurrencyId::get())
    }
    fn fee(&self) -> Amount<T> {
        Amount::new(self.fee, T::GetWrappedCurrencyId::get())
    }
    fn amount_btc(&self) -> Amount<T> {
        Amount::new(self.amount_btc, T::GetWrappedCurrencyId::get())
    }
}
