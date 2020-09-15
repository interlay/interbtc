//! Runtime API definition for the Staked Relayers.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use frame_support::dispatch::DispatchResult;
use sp_std::prelude::Vec;

sp_api::decl_runtime_apis! {
    pub trait StakedRelayersApi<AccountId> where
        AccountId: Codec,
    {
        fn is_transaction_invalid(
            vault_id: AccountId, raw_tx: Vec<u8>
        ) -> DispatchResult;
    }
}
