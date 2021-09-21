//! Runtime API definition for the Replace Module.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use sp_std::vec::Vec;

sp_api::decl_runtime_apis! {
    pub trait ReplaceApi<VaultId, H256, ReplaceRequest> where
        VaultId: Codec,
        H256: Codec,
        ReplaceRequest: Codec,
    {
        /// Get all replace requests from a particular vault
        fn get_old_vault_replace_requests(vault_id: VaultId) -> Vec<(H256, ReplaceRequest)>;

        /// Get all replace requests to a particular vault
        fn get_new_vault_replace_requests(vault_id: VaultId) -> Vec<(H256, ReplaceRequest)>;
    }
}
