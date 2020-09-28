//! Runtime API definition for the Vault Registry

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use frame_support::dispatch::DispatchError;

sp_api::decl_runtime_apis! {
    pub trait VaultRegistryApi<AccountId, PolkaBTC> where
        AccountId: Codec,
        PolkaBTC: Codec,
    {
        /// Get the first available vault with sufficient collateral to fulfil an issue request
        /// with the specified amount of PolkaBTC.
        fn get_first_vault_with_sufficient_collateral(amount: PolkaBTC) -> Result<AccountId, DispatchError>;

        /// Get the amount of tokens a vault can issue
        fn get_issuable_tokens_from_vault(vault: AccountId) -> Result<PolkaBTC, DispatchError>;

        /// Get the collateralization rate of a vault scaled by GRANULARITY
        fn get_collateralization_from_vault(vault: AccountId) -> Result<u128, DispatchError>;
    }
}
