//! The testnet runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

use testnet_common_runtime::construct_testnet;

pub mod kintsugi_constants {
    use primitives::TokenSymbol;
    pub use primitives::{Balance, CurrencyId, CurrencyId::Token, KBTC, KINT, KSM};
    use sp_runtime::{RuntimeString};

    pub const SPEC_NAME:&str = "testnet-interlay-parachain";
    pub const IMPL_NAME:&str = "testnet-interlay-parachain";

    pub const NATIVE_TOKEN_ID: TokenSymbol = KINT;
    pub const NATIVE_CURRENCY_ID: CurrencyId = Token(NATIVE_TOKEN_ID);
    pub const PARENT_TOKEN_ID: TokenSymbol = KSM;
    pub const PARENT_CURRENCY_ID: CurrencyId = Token(PARENT_TOKEN_ID);
    pub const WRAPPED_CURRENCY_ID: CurrencyId = Token(KBTC);

    // https://github.com/paritytech/polkadot/blob/c4ee9d463adccfa3bf436433e3e26d0de5a4abbc/runtime/kusama/src/constants.rs#L18
    pub const UNITS: Balance = NATIVE_TOKEN_ID.one();
    pub const CENTS: Balance = UNITS / 30_000;
    pub const GRAND: Balance = CENTS * 100_000;
    pub const MILLICENTS: Balance = CENTS / 1_000;

    pub const fn deposit(items: u32, bytes: u32) -> Balance {
        items as Balance * 2_000 * CENTS + (bytes as Balance) * 100 * MILLICENTS
    }
}

construct_testnet!(kintsugi_constants);
