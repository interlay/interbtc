//! Runtime API definition for the Relay Pallet.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use frame_support::dispatch::DispatchResult;
use sp_std::prelude::Vec;

sp_api::decl_runtime_apis! {
    pub trait RelayApi<AccountId> where
        AccountId: Codec,
    {
        fn is_transaction_invalid(
            vault_id: AccountId, raw_tx: Vec<u8>
        ) -> DispatchResult;
    }
}
