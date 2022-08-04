//! Runtime API definition for the Escrow Module.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use module_oracle_rpc_runtime_api::BalanceWrapper;

sp_api::decl_runtime_apis! {
    pub trait EscrowApi<AccountId, BlockNumber, Balance> where
        AccountId: Codec,
        BlockNumber: Codec,
        Balance: Codec,
    {
        /// Get a given user's escrowed balance
        fn balance_at(account_id: AccountId, height: Option<BlockNumber>) -> BalanceWrapper<Balance>;

        /// Get the total voting supply in the system
        fn total_supply(height: Option<BlockNumber>) -> BalanceWrapper<Balance>;
    }
}
