//! Runtime API definition for the Redeem Module.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use sp_std::vec::Vec;

sp_api::decl_runtime_apis! {
    pub trait RedeemApi<AccountId, H256, RedeemRequest> where
        AccountId: Codec,
        H256: Codec,
        RedeemRequest: Codec,
    {
        /// Get all redeem requests for a particular account
        fn get_redeem_requests(account_id: AccountId) -> Vec<H256>;

        /// Get all redeem requests for a particular vault
        fn get_vault_redeem_requests(vault_id: AccountId) -> Vec<H256>;
    }
}
