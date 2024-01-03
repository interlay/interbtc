//! Runtime API definition for the Redeem Module.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use frame_support::dispatch::DispatchError;
use oracle_rpc_runtime_api::BalanceWrapper;
use sp_std::vec::Vec;

sp_api::decl_runtime_apis! {
    pub trait RedeemApi<VaultId, Balance, AccountId, H256, RedeemRequest> where
        VaultId: Codec,
        Balance: Codec,
        AccountId: Codec,
        H256: Codec,
        RedeemRequest: Codec,
    {
        /// Get all redeem requests for a particular account
        fn get_redeem_requests(account_id: AccountId) -> Vec<H256>;

        /// Get all redeem requests for a particular vault
        fn get_vault_redeem_requests(vault_id: AccountId) -> Vec<H256>;

        /// Get all vaults below the premium redeem threshold, ordered in descending order of this amount
        fn get_premium_redeem_vaults() -> Result<Vec<(VaultId, BalanceWrapper<Balance>)>, DispatchError>;
    }
}
