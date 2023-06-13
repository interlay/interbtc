#[cfg(any(feature = "with-interlay-runtime", feature = "with-kintsugi-runtime",))]
mod annuity;

#[cfg(any(feature = "with-interlay-runtime", feature = "with-kintsugi-runtime",))]
mod btc_relay;

#[cfg(any(feature = "with-interlay-runtime", feature = "with-kintsugi-runtime",))]
mod clients_info;

#[cfg(any(feature = "with-interlay-runtime", feature = "with-kintsugi-runtime",))]
mod escrow;

#[cfg(any(feature = "with-interlay-runtime", feature = "with-kintsugi-runtime",))]
mod fee_pool;

#[cfg(any(feature = "with-interlay-runtime", feature = "with-kintsugi-runtime",))]
mod governance;

#[cfg(any(feature = "with-interlay-runtime", feature = "with-kintsugi-runtime",))]
mod issue;

#[cfg(any(feature = "with-interlay-runtime", feature = "with-kintsugi-runtime",))]
mod loans;

#[cfg(any(feature = "with-interlay-runtime", feature = "with-kintsugi-runtime",))]
mod multisig;

#[cfg(any(feature = "with-interlay-runtime", feature = "with-kintsugi-runtime",))]
mod nomination;

#[cfg(any(feature = "with-interlay-runtime", feature = "with-kintsugi-runtime",))]
mod redeem;

#[cfg(any(feature = "with-interlay-runtime", feature = "with-kintsugi-runtime",))]
mod replace;

#[cfg(any(feature = "with-interlay-runtime", feature = "with-kintsugi-runtime",))]
mod vault_registry;
