use currency::Amount;
pub use primitives::refund::RefundRequest;
use primitives::VaultId;
use vault_registry::types::CurrencyId;

pub(crate) type BalanceOf<T> = <T as vault_registry::Config>::Balance;

pub(crate) type DefaultVaultId<T> = VaultId<<T as frame_system::Config>::AccountId, CurrencyId<T>>;

pub type DefaultRefundRequest<T> = RefundRequest<<T as frame_system::Config>::AccountId, BalanceOf<T>, CurrencyId<T>>;

pub trait RefundRequestExt<T: crate::Config> {
    fn fee(&self) -> Amount<T>;
    fn amount_btc(&self) -> Amount<T>;
}

impl<T: crate::Config> RefundRequestExt<T> for RefundRequest<T::AccountId, BalanceOf<T>, CurrencyId<T>> {
    fn fee(&self) -> Amount<T> {
        Amount::new(self.fee, self.vault.wrapped_currency())
    }
    fn amount_btc(&self) -> Amount<T> {
        Amount::new(self.amount_btc, self.vault.wrapped_currency())
    }
}
