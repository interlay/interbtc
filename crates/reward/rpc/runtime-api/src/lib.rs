//! Runtime API definition for the Reward Module.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use frame_support::dispatch::DispatchError;
use oracle_rpc_runtime_api::BalanceWrapper;

sp_api::decl_runtime_apis! {
    pub trait RewardApi<AccountId, VaultId, CurrencyId, Balance, BlockNumber, UnsignedFixedPoint> where
        AccountId: Codec,
        VaultId: Codec,
        CurrencyId: Codec,
        Balance: Codec,
        BlockNumber: Codec,
        UnsignedFixedPoint: Codec,
    {
        /// Get a given user's rewards due
        fn compute_escrow_reward(account_id: AccountId, currency_id: CurrencyId) -> Result<BalanceWrapper<Balance>, DispatchError>;

        /// Get a given vault's rewards due
        fn compute_vault_reward(vault_id: VaultId, currency_id: CurrencyId) -> Result<BalanceWrapper<Balance>, DispatchError>;

        /// Estimate staking reward rate for a one year period
        fn estimate_escrow_reward_rate(account_id: AccountId, amount: Option<Balance>, lock_time: Option<BlockNumber>) -> Result<UnsignedFixedPoint, DispatchError>;

        /// Estimate vault reward rate for a one year period
        fn estimate_vault_reward_rate(vault_id: VaultId) -> Result<UnsignedFixedPoint, DispatchError>;
    }
}
