//! Runtime API definition for the Vault Registry

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use frame_support::dispatch::DispatchError;
use module_exchange_rate_oracle_rpc_runtime_api::BalanceWrapper;
use sp_std::vec::Vec;

sp_api::decl_runtime_apis! {
    pub trait VaultRegistryApi<AccountId, Issuing, Backing, UnsignedFixedPoint> where
        AccountId: Codec,
        Issuing: Codec,
        Backing: Codec,
        UnsignedFixedPoint: Codec
    {
        /// Get the total collateralization of the system
        fn get_total_collateralization() -> Result<UnsignedFixedPoint, DispatchError>;

        /// Get the first available vault with sufficient collateral to fulfil an issue request
        /// with the specified amount of Issuing.
        fn get_first_vault_with_sufficient_collateral(amount: BalanceWrapper<Issuing>) -> Result<AccountId, DispatchError>;

        /// Get the first available vault with sufficient tokens to fulfil a redeem request
        fn get_first_vault_with_sufficient_tokens(amount: BalanceWrapper<Issuing>) -> Result<AccountId, DispatchError>;

        /// Get all vaults below the premium redeem threshold, ordered in descending order of this amount
        fn get_premium_redeem_vaults() -> Result<Vec<(AccountId, BalanceWrapper<Issuing>)>, DispatchError>;

        /// Get all vaults with non-zero issuable tokens, ordered in descending order of this amount
        fn get_vaults_with_issuable_tokens() -> Result<Vec<(AccountId, BalanceWrapper<Issuing>)>, DispatchError>;

        /// Get all vaults with non-zero redeemable tokens, ordered in descending order of this amount
        fn get_vaults_with_redeemable_tokens() -> Result<Vec<(AccountId, BalanceWrapper<Issuing>)>, DispatchError>;

        /// Get the amount of tokens a vault can issue
        fn get_issuable_tokens_from_vault(vault: AccountId) -> Result<BalanceWrapper<Issuing>, DispatchError>;

        /// Get the collateralization rate of a vault
        fn get_collateralization_from_vault(vault: AccountId, only_issued: bool) -> Result<UnsignedFixedPoint, DispatchError>;

        /// Get the collateralization rate of a vault and collateral
        fn get_collateralization_from_vault_and_collateral(vault: AccountId, collateral: BalanceWrapper<Backing>, only_issued: bool) -> Result<UnsignedFixedPoint, DispatchError>;

        /// Get the minimum amount of collateral required for the given amount of btc
        /// with the current threshold and exchange rate
        fn get_required_collateral_for_issuing(amount_btc: BalanceWrapper<Issuing>) -> Result<BalanceWrapper<Backing>, DispatchError>;

        /// Get the amount of collateral required for the given vault to be at the
        /// current SecureCollateralThreshold with the current exchange rate
        fn get_required_collateral_for_vault(vault_id: AccountId) -> Result<BalanceWrapper<Backing>, DispatchError>;

        /// Simple check to validate whether a vault is below the `AuctionThreshold`
        fn is_vault_below_auction_threshold(vault: AccountId) -> Result<bool, DispatchError>;
    }
}
