//! Runtime API definition for the Vault Registry

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use frame_support::dispatch::DispatchError;
use module_oracle_rpc_runtime_api::BalanceWrapper;
use sp_std::vec::Vec;

sp_api::decl_runtime_apis! {
    pub trait VaultRegistryApi<VaultId, Balance, UnsignedFixedPoint, CurrencyId, AccountId> where
        VaultId: Codec,
        Balance: Codec,
        UnsignedFixedPoint: Codec,
        CurrencyId: Codec,
        AccountId: Codec,
    {
        /// Get the vault's collateral (excluding nomination)
        fn get_vault_collateral(vault_id: VaultId) -> Result<BalanceWrapper<Balance>, DispatchError>;

        /// Get all the vaultIds registered by a vault's accountId
        fn get_vaults_by_account_id(account_id: AccountId) -> Result<Vec<VaultId>, DispatchError>;

        /// Get the vault's collateral (including nomination)
        fn get_vault_total_collateral(vault_id: VaultId) -> Result<BalanceWrapper<Balance>, DispatchError>;

        /// Get all vaults below the premium redeem threshold, ordered in descending order of this amount
        fn get_premium_redeem_vaults() -> Result<Vec<(VaultId, BalanceWrapper<Balance>)>, DispatchError>;

        /// Get all vaults with non-zero issuable tokens, ordered in descending order of this amount
        fn get_vaults_with_issuable_tokens() -> Result<Vec<(VaultId, BalanceWrapper<Balance>)>, DispatchError>;

        /// Get all vaults with non-zero redeemable tokens, ordered in descending order of this amount
        fn get_vaults_with_redeemable_tokens() -> Result<Vec<(VaultId, BalanceWrapper<Balance>)>, DispatchError>;

        /// Get the amount of tokens a vault can issue
        fn get_issuable_tokens_from_vault(vault: VaultId) -> Result<BalanceWrapper<Balance>, DispatchError>;

        /// Get the collateralization rate of a vault
        fn get_collateralization_from_vault(vault: VaultId, only_issued: bool) -> Result<UnsignedFixedPoint, DispatchError>;

        /// Get the collateralization rate of a vault and collateral
        fn get_collateralization_from_vault_and_collateral(vault: VaultId, collateral: BalanceWrapper<Balance>, only_issued: bool) -> Result<UnsignedFixedPoint, DispatchError>;

        /// Get the minimum amount of collateral required for the given amount of btc
        /// with the current threshold and exchange rate
        fn get_required_collateral_for_wrapped(amount_btc: BalanceWrapper<Balance>, currency_id: CurrencyId) -> Result<BalanceWrapper<Balance>, DispatchError>;

        /// Get the amount of collateral required for the given vault to be at the
        /// current SecureCollateralThreshold with the current exchange rate
        fn get_required_collateral_for_vault(vault_id: VaultId) -> Result<BalanceWrapper<Balance>, DispatchError>;
    }
}
