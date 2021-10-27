//! Runtime API definition for the Refund Module.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use sp_std::vec::Vec;

sp_api::decl_runtime_apis! {
    pub trait RefundApi<AccountId, H256, RefundRequest> where
        AccountId: Codec,
        H256: Codec,
        RefundRequest: Codec,
    {
        /// Get all refund requests for a particular account
        fn get_refund_requests(account_id: AccountId) -> Vec<H256>;

        /// Get the refund request corresponding to a particular issue ID
        fn get_refund_requests_by_issue_id(issue_id: H256) -> Option<H256>;

        /// Get all refund requests for a particular vault
        fn get_vault_refund_requests(vault_id: AccountId) -> Vec<H256>;
    }
}
