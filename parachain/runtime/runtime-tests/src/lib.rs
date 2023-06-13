#![cfg(test)]
#![allow(dead_code)]
#![feature(exclusive_range_pattern)]

mod parachain;
mod relaychain;

#[cfg(any(feature = "with-interlay-runtime", feature = "with-kintsugi-runtime",))]
mod setup;

#[cfg(any(feature = "with-interlay-runtime", feature = "with-kintsugi-runtime",))]
mod utils;
