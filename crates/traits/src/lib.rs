#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::dispatch::DispatchError;
use num_bigint::{BigUint, ToBigUint};

pub mod loans;
pub use loans::*;

pub trait ConvertToBigUint {
    fn get_big_uint(&self) -> BigUint;
}

impl ConvertToBigUint for u128 {
    fn get_big_uint(&self) -> BigUint {
        self.to_biguint().unwrap()
    }
}

pub trait OracleApi<Amount, CurrencyId> {
    fn convert(amount: &Amount, to: CurrencyId) -> Result<Amount, DispatchError>;
}

pub trait NominationApi<VaultId, Amount> {
    fn deposit_vault_collateral(vault_id: &VaultId, amount: &Amount) -> Result<(), DispatchError>;
    fn ensure_opted_in_to_nomination(vault_id: &VaultId) -> Result<(), DispatchError>;

    #[cfg(any(feature = "runtime-benchmarks", test))]
    fn opt_in_to_nomination(vault_id: &VaultId);
}

pub trait OnExchangeRateChange<CurrencyId> {
    fn on_exchange_rate_change(currency_id: &CurrencyId);
}

#[impl_trait_for_tuples::impl_for_tuples(3)]
impl<CurrencyId> OnExchangeRateChange<CurrencyId> for Tuple {
    fn on_exchange_rate_change(currency_id: &CurrencyId) {
        for_tuples!( #(
            Tuple::on_exchange_rate_change(currency_id);
        )* );
    }
}
