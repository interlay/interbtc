#[cfg(not(feature = "with-interlay-runtime"))]
mod kusama_cross_chain_transfer;
#[cfg(not(feature = "with-interlay-runtime"))]
pub mod kusama_test_net;
#[cfg(feature = "with-interlay-runtime")]
mod polkadot_cross_chain_transfer;
#[cfg(feature = "with-interlay-runtime")]
pub mod polkadot_test_net;
