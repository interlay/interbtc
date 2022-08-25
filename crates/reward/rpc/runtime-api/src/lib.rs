//! Runtime API definition for the Reward Module.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use frame_support::dispatch::DispatchError;
use module_oracle_rpc_runtime_api::BalanceWrapper;

sp_api::decl_runtime_apis! {
    pub trait RewardApi<RewardId, CurrencyId, Balance> where
        RewardId: Codec,
        CurrencyId: Codec,
        Balance: Codec
    {
        /// Get a given user's rewards due
        fn compute_reward(account_id: RewardId, currency_id: CurrencyId) -> Result<BalanceWrapper<Balance>, DispatchError>;
    }
}
