//! Runtime API definition for the Issue Module.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use sp_std::vec::Vec;

sp_api::decl_runtime_apis! {
    pub trait IssueApi<AccountId, H256, IssueRequest> where
        AccountId: Codec,
        H256: Codec,
        IssueRequest: Codec,
    {
        /// Get all issue requests for a particular account
        fn get_issue_requests(account_id: AccountId) -> Vec<H256>;

        /// Get all issue requests for a particular vault
        fn get_vault_issue_requests(vault_id: AccountId) -> Vec<H256>;
    }
}
