//! Runtime API definition for the Exchange Rate Oracle.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use frame_support::dispatch::DispatchError;

sp_api::decl_runtime_apis! {
    pub trait ExchangeRateOracleApi<PolkaBTC, DOT> where
        PolkaBTC: Codec,
        DOT: Codec,
    {
        fn btc_to_dots(
            amount: PolkaBTC
        ) -> Result<DOT, DispatchError>;
    }
}
