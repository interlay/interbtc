//! Runtime API definition for the Vault Registry

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use frame_support::dispatch::DispatchResult;
use sp_std::prelude::Vec;

sp_api::decl_runtime_apis! {
    pub trait VaultRegistryApi<AccountId, PolkaBTC> where
        AccountId: Codec,
        PolkaBTC: Codec,
    {
        /// Get the first available vault with sufficient collateral to fulfil an issue request
        /// with the specified amount of PolkaBTC.
        fn get_first_vault_with_sufficient_collateral(amount: PolkaBTC) -> DispatchResult;

        /// Get the amount of tokens a vault can issue
        fn get_issuable_tokens_from_vault(vault: AccountId) -> DispatchResult;
    }
}
