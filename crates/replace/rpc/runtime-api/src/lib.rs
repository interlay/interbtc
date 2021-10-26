//! Runtime API definition for the Replace Module.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use sp_std::vec::Vec;

sp_api::decl_runtime_apis! {
    pub trait ReplaceApi<AccountId, H256, ReplaceRequest> where
        AccountId: Codec,
        H256: Codec,
        ReplaceRequest: Codec,
    {
        /// Get all replace requests from a particular vault
        fn get_old_vault_replace_requests(vault_id: AccountId) -> Vec<H256>;

        /// Get all replace requests to a particular vault
        fn get_new_vault_replace_requests(vault_id: AccountId) -> Vec<H256>;
    }
}
