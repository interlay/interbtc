//! Runtime API definition for the Refund Module.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use frame_support::dispatch::DispatchError;

sp_api::decl_runtime_apis! {
    pub trait BtcRelayApi<H256Le> where
        H256Le: Codec,
    {
        /// Verify that the block with the given block hash is relayed, has sufficient
        /// confirmations and is part of the main chain
        fn verify_block_header_inclusion(block_hash: H256Le) -> Result<(), DispatchError>;
    }
}
