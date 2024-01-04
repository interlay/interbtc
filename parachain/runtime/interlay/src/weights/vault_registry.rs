
//! Autogenerated weights for vault_registry
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2024-01-04, STEPS: `2`, REPEAT: `1`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `nakul-GF65-Thin-10UE`, CPU: `Intel(R) Core(TM) i7-10750H CPU @ 2.60GHz`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("interlay-dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/interbtc-parachain
// benchmark
// pallet
// --pallet
// *
// --extrinsic
// *
// --execution=wasm
// --wasm-execution=compiled
// --steps
// 2
// --repeat
// 1
// --template
// .deploy/runtime-weight-template.hbs
// --chain
// interlay-dev
// --output
// parachain/runtime/interlay/src/weights/

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weights for vault_registry using the Substrate node and recommended hardware.
pub struct WeightInfo<T>(PhantomData<T>);

impl<T: frame_system::Config> vault_registry::WeightInfo for WeightInfo<T> {

	/// Storage: VaultRegistry SecureCollateralThreshold (r:1 w:0)
	/// Proof: VaultRegistry SecureCollateralThreshold (max_values: None, max_size: Some(54), added: 2529, mode: MaxEncodedLen)
	/// Storage: VaultRegistry PremiumRedeemThreshold (r:1 w:0)
	/// Proof: VaultRegistry PremiumRedeemThreshold (max_values: None, max_size: Some(54), added: 2529, mode: MaxEncodedLen)
	/// Storage: VaultRegistry LiquidationCollateralThreshold (r:1 w:0)
	/// Proof: VaultRegistry LiquidationCollateralThreshold (max_values: None, max_size: Some(54), added: 2529, mode: MaxEncodedLen)
	/// Storage: VaultRegistry MinimumCollateralVault (r:1 w:0)
	/// Proof: VaultRegistry MinimumCollateralVault (max_values: None, max_size: Some(43), added: 2518, mode: MaxEncodedLen)
	/// Storage: VaultRegistry SystemCollateralCeiling (r:1 w:0)
	/// Proof: VaultRegistry SystemCollateralCeiling (max_values: None, max_size: Some(54), added: 2529, mode: MaxEncodedLen)
	/// Storage: VaultRegistry VaultBitcoinPublicKey (r:1 w:0)
	/// Proof: VaultRegistry VaultBitcoinPublicKey (max_values: None, max_size: Some(81), added: 2556, mode: MaxEncodedLen)
	/// Storage: VaultRegistry Vaults (r:1 w:1)
	/// Proof: VaultRegistry Vaults (max_values: None, max_size: Some(260), added: 2735, mode: MaxEncodedLen)
	/// Storage: VaultCapacity Stake (r:1 w:0)
	/// Proof: VaultCapacity Stake (max_values: None, max_size: Some(43), added: 2518, mode: MaxEncodedLen)
	/// Storage: VaultCapacity RewardPerToken (r:2 w:0)
	/// Proof: VaultCapacity RewardPerToken (max_values: None, max_size: Some(59), added: 2534, mode: MaxEncodedLen)
	/// Storage: VaultCapacity RewardTally (r:2 w:2)
	/// Proof: VaultCapacity RewardTally (max_values: None, max_size: Some(70), added: 2545, mode: MaxEncodedLen)
	/// Storage: VaultCapacity TotalRewards (r:2 w:2)
	/// Proof: VaultCapacity TotalRewards (max_values: None, max_size: Some(43), added: 2518, mode: MaxEncodedLen)
	/// Storage: VaultRewards Stake (r:1 w:0)
	/// Proof: VaultRewards Stake (max_values: None, max_size: Some(97), added: 2572, mode: MaxEncodedLen)
	/// Storage: VaultRewards RewardPerToken (r:2 w:0)
	/// Proof: VaultRewards RewardPerToken (max_values: None, max_size: Some(70), added: 2545, mode: MaxEncodedLen)
	/// Storage: VaultRewards RewardTally (r:2 w:2)
	/// Proof: VaultRewards RewardTally (max_values: None, max_size: Some(124), added: 2599, mode: MaxEncodedLen)
	/// Storage: VaultRewards TotalRewards (r:2 w:2)
	/// Proof: VaultRewards TotalRewards (max_values: None, max_size: Some(43), added: 2518, mode: MaxEncodedLen)
	/// Storage: Fee Commission (r:1 w:0)
	/// Proof: Fee Commission (max_values: None, max_size: Some(86), added: 2561, mode: MaxEncodedLen)
	/// Storage: VaultStaking Nonce (r:1 w:0)
	/// Proof: VaultStaking Nonce (max_values: None, max_size: Some(74), added: 2549, mode: MaxEncodedLen)
	/// Storage: VaultStaking TotalCurrentStake (r:1 w:1)
	/// Proof: VaultStaking TotalCurrentStake (max_values: None, max_size: Some(106), added: 2581, mode: MaxEncodedLen)
	/// Storage: VaultStaking Stake (r:1 w:1)
	/// Proof: VaultStaking Stake (max_values: None, max_size: Some(138), added: 2613, mode: MaxEncodedLen)
	/// Storage: VaultStaking SlashPerToken (r:1 w:0)
	/// Proof: VaultStaking SlashPerToken (max_values: None, max_size: Some(106), added: 2581, mode: MaxEncodedLen)
	/// Storage: VaultStaking SlashTally (r:1 w:1)
	/// Proof: VaultStaking SlashTally (max_values: None, max_size: Some(138), added: 2613, mode: MaxEncodedLen)
	/// Storage: VaultStaking TotalStake (r:1 w:1)
	/// Proof: VaultStaking TotalStake (max_values: None, max_size: Some(106), added: 2581, mode: MaxEncodedLen)
	/// Storage: VaultStaking RewardTally (r:2 w:2)
	/// Proof: VaultStaking RewardTally (max_values: None, max_size: Some(149), added: 2624, mode: MaxEncodedLen)
	/// Storage: VaultStaking RewardPerToken (r:2 w:0)
	/// Proof: VaultStaking RewardPerToken (max_values: None, max_size: Some(117), added: 2592, mode: MaxEncodedLen)
	/// Storage: VaultRewards TotalStake (r:1 w:0)
	/// Proof: VaultRewards TotalStake (max_values: None, max_size: Some(43), added: 2518, mode: MaxEncodedLen)
	/// Storage: Loans UnderlyingAssetId (r:1 w:0)
	/// Proof: Loans UnderlyingAssetId (max_values: None, max_size: Some(38), added: 2513, mode: MaxEncodedLen)
	/// Storage: Loans Markets (r:2 w:0)
	/// Proof: Loans Markets (max_values: None, max_size: Some(160), added: 2635, mode: MaxEncodedLen)
	/// Storage: Timestamp Now (r:1 w:0)
	/// Proof: Timestamp Now (max_values: Some(1), max_size: Some(8), added: 503, mode: MaxEncodedLen)
	/// Storage: Loans LastAccruedInterestTime (r:1 w:1)
	/// Proof: Loans LastAccruedInterestTime (max_values: None, max_size: Some(35), added: 2510, mode: MaxEncodedLen)
	/// Storage: Tokens TotalIssuance (r:1 w:0)
	/// Proof: Tokens TotalIssuance (max_values: None, max_size: Some(35), added: 2510, mode: MaxEncodedLen)
	/// Storage: Tokens Accounts (r:1 w:0)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(115), added: 2590, mode: MaxEncodedLen)
	/// Storage: System Account (r:1 w:0)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	/// Storage: Loans TotalBorrows (r:1 w:0)
	/// Proof: Loans TotalBorrows (max_values: None, max_size: Some(43), added: 2518, mode: MaxEncodedLen)
	/// Storage: Loans TotalReserves (r:1 w:0)
	/// Proof: Loans TotalReserves (max_values: None, max_size: Some(43), added: 2518, mode: MaxEncodedLen)
	/// Storage: Loans MinExchangeRate (r:1 w:0)
	/// Proof: Loans MinExchangeRate (max_values: Some(1), max_size: Some(16), added: 511, mode: MaxEncodedLen)
	/// Storage: Loans MaxExchangeRate (r:1 w:0)
	/// Proof: Loans MaxExchangeRate (max_values: Some(1), max_size: Some(16), added: 511, mode: MaxEncodedLen)
	/// Storage: Oracle Aggregate (r:1 w:0)
	/// Proof: Oracle Aggregate (max_values: None, max_size: Some(44), added: 2519, mode: MaxEncodedLen)
	/// Storage: VaultRegistry TotalUserVaultCollateral (r:1 w:1)
	/// Proof: VaultRegistry TotalUserVaultCollateral (max_values: None, max_size: Some(54), added: 2529, mode: MaxEncodedLen)
	fn register_vault	() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2723`
		//  Estimated: `6260`
		// Minimum execution time: 413_864_000 picoseconds.
		Weight::from_parts(413_864_000, 6260)
			.saturating_add(T::DbWeight::get().reads(47_u64))
			.saturating_add(T::DbWeight::get().writes(17_u64))
	}
	/// Storage: VaultRegistry VaultBitcoinPublicKey (r:1 w:1)
	/// Proof: VaultRegistry VaultBitcoinPublicKey (max_values: None, max_size: Some(81), added: 2556, mode: MaxEncodedLen)
	fn register_public_key	() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `365`
		//  Estimated: `3546`
		// Minimum execution time: 26_284_000 picoseconds.
		Weight::from_parts(26_284_000, 3546)
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	/// Storage: VaultRegistry Vaults (r:1 w:1)
	/// Proof: VaultRegistry Vaults (max_values: None, max_size: Some(260), added: 2735, mode: MaxEncodedLen)
	/// Storage: VaultCapacity Stake (r:1 w:0)
	/// Proof: VaultCapacity Stake (max_values: None, max_size: Some(43), added: 2518, mode: MaxEncodedLen)
	/// Storage: VaultCapacity RewardPerToken (r:2 w:0)
	/// Proof: VaultCapacity RewardPerToken (max_values: None, max_size: Some(59), added: 2534, mode: MaxEncodedLen)
	/// Storage: VaultCapacity RewardTally (r:2 w:2)
	/// Proof: VaultCapacity RewardTally (max_values: None, max_size: Some(70), added: 2545, mode: MaxEncodedLen)
	/// Storage: VaultCapacity TotalRewards (r:2 w:2)
	/// Proof: VaultCapacity TotalRewards (max_values: None, max_size: Some(43), added: 2518, mode: MaxEncodedLen)
	/// Storage: VaultRewards Stake (r:1 w:0)
	/// Proof: VaultRewards Stake (max_values: None, max_size: Some(97), added: 2572, mode: MaxEncodedLen)
	/// Storage: VaultRewards RewardPerToken (r:2 w:0)
	/// Proof: VaultRewards RewardPerToken (max_values: None, max_size: Some(70), added: 2545, mode: MaxEncodedLen)
	/// Storage: VaultRewards RewardTally (r:2 w:2)
	/// Proof: VaultRewards RewardTally (max_values: None, max_size: Some(124), added: 2599, mode: MaxEncodedLen)
	/// Storage: VaultRewards TotalRewards (r:2 w:2)
	/// Proof: VaultRewards TotalRewards (max_values: None, max_size: Some(43), added: 2518, mode: MaxEncodedLen)
	/// Storage: Fee Commission (r:1 w:0)
	/// Proof: Fee Commission (max_values: None, max_size: Some(86), added: 2561, mode: MaxEncodedLen)
	/// Storage: VaultStaking Nonce (r:1 w:0)
	/// Proof: VaultStaking Nonce (max_values: None, max_size: Some(74), added: 2549, mode: MaxEncodedLen)
	/// Storage: VaultStaking TotalCurrentStake (r:1 w:0)
	/// Proof: VaultStaking TotalCurrentStake (max_values: None, max_size: Some(106), added: 2581, mode: MaxEncodedLen)
	/// Storage: VaultStaking RewardPerToken (r:2 w:2)
	/// Proof: VaultStaking RewardPerToken (max_values: None, max_size: Some(117), added: 2592, mode: MaxEncodedLen)
	/// Storage: VaultRegistry SecureCollateralThreshold (r:1 w:0)
	/// Proof: VaultRegistry SecureCollateralThreshold (max_values: None, max_size: Some(54), added: 2529, mode: MaxEncodedLen)
	/// Storage: VaultRewards TotalStake (r:1 w:0)
	/// Proof: VaultRewards TotalStake (max_values: None, max_size: Some(43), added: 2518, mode: MaxEncodedLen)
	/// Storage: Loans UnderlyingAssetId (r:1 w:0)
	/// Proof: Loans UnderlyingAssetId (max_values: None, max_size: Some(38), added: 2513, mode: MaxEncodedLen)
	/// Storage: Loans Markets (r:2 w:0)
	/// Proof: Loans Markets (max_values: None, max_size: Some(160), added: 2635, mode: MaxEncodedLen)
	/// Storage: Timestamp Now (r:1 w:0)
	/// Proof: Timestamp Now (max_values: Some(1), max_size: Some(8), added: 503, mode: MaxEncodedLen)
	/// Storage: Loans LastAccruedInterestTime (r:1 w:1)
	/// Proof: Loans LastAccruedInterestTime (max_values: None, max_size: Some(35), added: 2510, mode: MaxEncodedLen)
	/// Storage: Tokens TotalIssuance (r:1 w:0)
	/// Proof: Tokens TotalIssuance (max_values: None, max_size: Some(35), added: 2510, mode: MaxEncodedLen)
	/// Storage: Tokens Accounts (r:1 w:0)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(115), added: 2590, mode: MaxEncodedLen)
	/// Storage: System Account (r:1 w:0)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	/// Storage: Loans TotalBorrows (r:1 w:0)
	/// Proof: Loans TotalBorrows (max_values: None, max_size: Some(43), added: 2518, mode: MaxEncodedLen)
	/// Storage: Loans TotalReserves (r:1 w:0)
	/// Proof: Loans TotalReserves (max_values: None, max_size: Some(43), added: 2518, mode: MaxEncodedLen)
	/// Storage: Loans MinExchangeRate (r:1 w:0)
	/// Proof: Loans MinExchangeRate (max_values: Some(1), max_size: Some(16), added: 511, mode: MaxEncodedLen)
	/// Storage: Loans MaxExchangeRate (r:1 w:0)
	/// Proof: Loans MaxExchangeRate (max_values: Some(1), max_size: Some(16), added: 511, mode: MaxEncodedLen)
	/// Storage: Oracle Aggregate (r:1 w:0)
	/// Proof: Oracle Aggregate (max_values: None, max_size: Some(44), added: 2519, mode: MaxEncodedLen)
	fn accept_new_issues	() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `3665`
		//  Estimated: `6260`
		// Minimum execution time: 278_933_000 picoseconds.
		Weight::from_parts(278_933_000, 6260)
			.saturating_add(T::DbWeight::get().reads(35_u64))
			.saturating_add(T::DbWeight::get().writes(12_u64))
	}
	/// Storage: VaultRegistry SecureCollateralThreshold (r:1 w:0)
	/// Proof: VaultRegistry SecureCollateralThreshold (max_values: None, max_size: Some(54), added: 2529, mode: MaxEncodedLen)
	/// Storage: VaultRegistry Vaults (r:1 w:1)
	/// Proof: VaultRegistry Vaults (max_values: None, max_size: Some(260), added: 2735, mode: MaxEncodedLen)
	/// Storage: VaultCapacity Stake (r:1 w:0)
	/// Proof: VaultCapacity Stake (max_values: None, max_size: Some(43), added: 2518, mode: MaxEncodedLen)
	/// Storage: VaultCapacity RewardPerToken (r:2 w:0)
	/// Proof: VaultCapacity RewardPerToken (max_values: None, max_size: Some(59), added: 2534, mode: MaxEncodedLen)
	/// Storage: VaultCapacity RewardTally (r:2 w:2)
	/// Proof: VaultCapacity RewardTally (max_values: None, max_size: Some(70), added: 2545, mode: MaxEncodedLen)
	/// Storage: VaultCapacity TotalRewards (r:2 w:2)
	/// Proof: VaultCapacity TotalRewards (max_values: None, max_size: Some(43), added: 2518, mode: MaxEncodedLen)
	/// Storage: VaultRewards Stake (r:1 w:0)
	/// Proof: VaultRewards Stake (max_values: None, max_size: Some(97), added: 2572, mode: MaxEncodedLen)
	/// Storage: VaultRewards RewardPerToken (r:2 w:0)
	/// Proof: VaultRewards RewardPerToken (max_values: None, max_size: Some(70), added: 2545, mode: MaxEncodedLen)
	/// Storage: VaultRewards RewardTally (r:2 w:2)
	/// Proof: VaultRewards RewardTally (max_values: None, max_size: Some(124), added: 2599, mode: MaxEncodedLen)
	/// Storage: VaultRewards TotalRewards (r:2 w:2)
	/// Proof: VaultRewards TotalRewards (max_values: None, max_size: Some(43), added: 2518, mode: MaxEncodedLen)
	/// Storage: Fee Commission (r:1 w:0)
	/// Proof: Fee Commission (max_values: None, max_size: Some(86), added: 2561, mode: MaxEncodedLen)
	/// Storage: VaultStaking Nonce (r:1 w:0)
	/// Proof: VaultStaking Nonce (max_values: None, max_size: Some(74), added: 2549, mode: MaxEncodedLen)
	/// Storage: VaultStaking TotalCurrentStake (r:1 w:0)
	/// Proof: VaultStaking TotalCurrentStake (max_values: None, max_size: Some(106), added: 2581, mode: MaxEncodedLen)
	/// Storage: VaultStaking RewardPerToken (r:2 w:2)
	/// Proof: VaultStaking RewardPerToken (max_values: None, max_size: Some(117), added: 2592, mode: MaxEncodedLen)
	/// Storage: VaultRewards TotalStake (r:1 w:0)
	/// Proof: VaultRewards TotalStake (max_values: None, max_size: Some(43), added: 2518, mode: MaxEncodedLen)
	/// Storage: Loans UnderlyingAssetId (r:1 w:0)
	/// Proof: Loans UnderlyingAssetId (max_values: None, max_size: Some(38), added: 2513, mode: MaxEncodedLen)
	/// Storage: Loans Markets (r:2 w:0)
	/// Proof: Loans Markets (max_values: None, max_size: Some(160), added: 2635, mode: MaxEncodedLen)
	/// Storage: Timestamp Now (r:1 w:0)
	/// Proof: Timestamp Now (max_values: Some(1), max_size: Some(8), added: 503, mode: MaxEncodedLen)
	/// Storage: Loans LastAccruedInterestTime (r:1 w:1)
	/// Proof: Loans LastAccruedInterestTime (max_values: None, max_size: Some(35), added: 2510, mode: MaxEncodedLen)
	/// Storage: Tokens TotalIssuance (r:1 w:0)
	/// Proof: Tokens TotalIssuance (max_values: None, max_size: Some(35), added: 2510, mode: MaxEncodedLen)
	/// Storage: Tokens Accounts (r:1 w:0)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(115), added: 2590, mode: MaxEncodedLen)
	/// Storage: System Account (r:1 w:0)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	/// Storage: Loans TotalBorrows (r:1 w:0)
	/// Proof: Loans TotalBorrows (max_values: None, max_size: Some(43), added: 2518, mode: MaxEncodedLen)
	/// Storage: Loans TotalReserves (r:1 w:0)
	/// Proof: Loans TotalReserves (max_values: None, max_size: Some(43), added: 2518, mode: MaxEncodedLen)
	/// Storage: Loans MinExchangeRate (r:1 w:0)
	/// Proof: Loans MinExchangeRate (max_values: Some(1), max_size: Some(16), added: 511, mode: MaxEncodedLen)
	/// Storage: Loans MaxExchangeRate (r:1 w:0)
	/// Proof: Loans MaxExchangeRate (max_values: Some(1), max_size: Some(16), added: 511, mode: MaxEncodedLen)
	/// Storage: Oracle Aggregate (r:1 w:0)
	/// Proof: Oracle Aggregate (max_values: None, max_size: Some(44), added: 2519, mode: MaxEncodedLen)
	fn set_custom_secure_threshold	() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `3665`
		//  Estimated: `6260`
		// Minimum execution time: 284_269_000 picoseconds.
		Weight::from_parts(284_269_000, 6260)
			.saturating_add(T::DbWeight::get().reads(35_u64))
			.saturating_add(T::DbWeight::get().writes(12_u64))
	}
	/// Storage: VaultRegistry MinimumCollateralVault (r:0 w:1)
	/// Proof: VaultRegistry MinimumCollateralVault (max_values: None, max_size: Some(43), added: 2518, mode: MaxEncodedLen)
	fn set_minimum_collateral	() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 13_971_000 picoseconds.
		Weight::from_parts(13_971_000, 0)
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	/// Storage: VaultRegistry SystemCollateralCeiling (r:0 w:1)
	/// Proof: VaultRegistry SystemCollateralCeiling (max_values: None, max_size: Some(54), added: 2529, mode: MaxEncodedLen)
	fn set_system_collateral_ceiling	() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 14_180_000 picoseconds.
		Weight::from_parts(14_180_000, 0)
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	/// Storage: VaultRegistry SecureCollateralThreshold (r:0 w:1)
	/// Proof: VaultRegistry SecureCollateralThreshold (max_values: None, max_size: Some(54), added: 2529, mode: MaxEncodedLen)
	fn set_secure_collateral_threshold	() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 22_585_000 picoseconds.
		Weight::from_parts(22_585_000, 0)
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	/// Storage: VaultRegistry PremiumRedeemThreshold (r:0 w:1)
	/// Proof: VaultRegistry PremiumRedeemThreshold (max_values: None, max_size: Some(54), added: 2529, mode: MaxEncodedLen)
	fn set_premium_redeem_threshold	() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 23_042_000 picoseconds.
		Weight::from_parts(23_042_000, 0)
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	/// Storage: VaultRegistry LiquidationCollateralThreshold (r:0 w:1)
	/// Proof: VaultRegistry LiquidationCollateralThreshold (max_values: None, max_size: Some(54), added: 2529, mode: MaxEncodedLen)
	fn set_liquidation_collateral_threshold	() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 23_949_000 picoseconds.
		Weight::from_parts(23_949_000, 0)
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	/// Storage: VaultRegistry Vaults (r:1 w:1)
	/// Proof: VaultRegistry Vaults (max_values: None, max_size: Some(260), added: 2735, mode: MaxEncodedLen)
	/// Storage: VaultRegistry LiquidationCollateralThreshold (r:1 w:0)
	/// Proof: VaultRegistry LiquidationCollateralThreshold (max_values: None, max_size: Some(54), added: 2529, mode: MaxEncodedLen)
	/// Storage: VaultStaking Nonce (r:1 w:0)
	/// Proof: VaultStaking Nonce (max_values: None, max_size: Some(74), added: 2549, mode: MaxEncodedLen)
	/// Storage: VaultStaking TotalCurrentStake (r:1 w:1)
	/// Proof: VaultStaking TotalCurrentStake (max_values: None, max_size: Some(106), added: 2581, mode: MaxEncodedLen)
	/// Storage: Loans UnderlyingAssetId (r:1 w:0)
	/// Proof: Loans UnderlyingAssetId (max_values: None, max_size: Some(38), added: 2513, mode: MaxEncodedLen)
	/// Storage: Loans Markets (r:2 w:0)
	/// Proof: Loans Markets (max_values: None, max_size: Some(160), added: 2635, mode: MaxEncodedLen)
	/// Storage: Timestamp Now (r:1 w:0)
	/// Proof: Timestamp Now (max_values: Some(1), max_size: Some(8), added: 503, mode: MaxEncodedLen)
	/// Storage: Loans LastAccruedInterestTime (r:1 w:1)
	/// Proof: Loans LastAccruedInterestTime (max_values: None, max_size: Some(35), added: 2510, mode: MaxEncodedLen)
	/// Storage: Tokens TotalIssuance (r:1 w:0)
	/// Proof: Tokens TotalIssuance (max_values: None, max_size: Some(35), added: 2510, mode: MaxEncodedLen)
	/// Storage: Tokens Accounts (r:3 w:2)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(115), added: 2590, mode: MaxEncodedLen)
	/// Storage: System Account (r:3 w:1)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	/// Storage: Loans TotalBorrows (r:1 w:0)
	/// Proof: Loans TotalBorrows (max_values: None, max_size: Some(43), added: 2518, mode: MaxEncodedLen)
	/// Storage: Loans TotalReserves (r:1 w:0)
	/// Proof: Loans TotalReserves (max_values: None, max_size: Some(43), added: 2518, mode: MaxEncodedLen)
	/// Storage: Loans MinExchangeRate (r:1 w:0)
	/// Proof: Loans MinExchangeRate (max_values: Some(1), max_size: Some(16), added: 511, mode: MaxEncodedLen)
	/// Storage: Loans MaxExchangeRate (r:1 w:0)
	/// Proof: Loans MaxExchangeRate (max_values: Some(1), max_size: Some(16), added: 511, mode: MaxEncodedLen)
	/// Storage: Oracle Aggregate (r:1 w:0)
	/// Proof: Oracle Aggregate (max_values: None, max_size: Some(44), added: 2519, mode: MaxEncodedLen)
	/// Storage: VaultStaking Stake (r:1 w:1)
	/// Proof: VaultStaking Stake (max_values: None, max_size: Some(138), added: 2613, mode: MaxEncodedLen)
	/// Storage: VaultStaking SlashPerToken (r:1 w:0)
	/// Proof: VaultStaking SlashPerToken (max_values: None, max_size: Some(106), added: 2581, mode: MaxEncodedLen)
	/// Storage: VaultStaking SlashTally (r:1 w:1)
	/// Proof: VaultStaking SlashTally (max_values: None, max_size: Some(138), added: 2613, mode: MaxEncodedLen)
	/// Storage: VaultCapacity Stake (r:1 w:0)
	/// Proof: VaultCapacity Stake (max_values: None, max_size: Some(43), added: 2518, mode: MaxEncodedLen)
	/// Storage: VaultCapacity RewardPerToken (r:2 w:0)
	/// Proof: VaultCapacity RewardPerToken (max_values: None, max_size: Some(59), added: 2534, mode: MaxEncodedLen)
	/// Storage: VaultCapacity RewardTally (r:2 w:2)
	/// Proof: VaultCapacity RewardTally (max_values: None, max_size: Some(70), added: 2545, mode: MaxEncodedLen)
	/// Storage: VaultCapacity TotalRewards (r:2 w:2)
	/// Proof: VaultCapacity TotalRewards (max_values: None, max_size: Some(43), added: 2518, mode: MaxEncodedLen)
	/// Storage: VaultRewards Stake (r:1 w:1)
	/// Proof: VaultRewards Stake (max_values: None, max_size: Some(97), added: 2572, mode: MaxEncodedLen)
	/// Storage: VaultRewards RewardPerToken (r:2 w:0)
	/// Proof: VaultRewards RewardPerToken (max_values: None, max_size: Some(70), added: 2545, mode: MaxEncodedLen)
	/// Storage: VaultRewards RewardTally (r:2 w:2)
	/// Proof: VaultRewards RewardTally (max_values: None, max_size: Some(124), added: 2599, mode: MaxEncodedLen)
	/// Storage: VaultRewards TotalRewards (r:2 w:2)
	/// Proof: VaultRewards TotalRewards (max_values: None, max_size: Some(43), added: 2518, mode: MaxEncodedLen)
	/// Storage: Fee Commission (r:1 w:0)
	/// Proof: Fee Commission (max_values: None, max_size: Some(86), added: 2561, mode: MaxEncodedLen)
	/// Storage: VaultStaking RewardPerToken (r:2 w:2)
	/// Proof: VaultStaking RewardPerToken (max_values: None, max_size: Some(117), added: 2592, mode: MaxEncodedLen)
	/// Storage: VaultStaking TotalStake (r:1 w:1)
	/// Proof: VaultStaking TotalStake (max_values: None, max_size: Some(106), added: 2581, mode: MaxEncodedLen)
	/// Storage: VaultRegistry SecureCollateralThreshold (r:1 w:0)
	/// Proof: VaultRegistry SecureCollateralThreshold (max_values: None, max_size: Some(54), added: 2529, mode: MaxEncodedLen)
	/// Storage: VaultRewards TotalStake (r:1 w:1)
	/// Proof: VaultRewards TotalStake (max_values: None, max_size: Some(43), added: 2518, mode: MaxEncodedLen)
	/// Storage: VaultStaking RewardTally (r:2 w:2)
	/// Proof: VaultStaking RewardTally (max_values: None, max_size: Some(149), added: 2624, mode: MaxEncodedLen)
	/// Storage: VaultRewards RewardCurrencies (r:1 w:0)
	/// Proof: VaultRewards RewardCurrencies (max_values: None, max_size: Some(50), added: 2525, mode: MaxEncodedLen)
	/// Storage: VaultRegistry TotalUserVaultCollateral (r:1 w:1)
	/// Proof: VaultRegistry TotalUserVaultCollateral (max_values: None, max_size: Some(54), added: 2529, mode: MaxEncodedLen)
	/// Storage: Loans RewardSupplyState (r:1 w:1)
	/// Proof: Loans RewardSupplyState (max_values: None, max_size: Some(47), added: 2522, mode: MaxEncodedLen)
	/// Storage: Loans RewardSupplySpeed (r:1 w:0)
	/// Proof: Loans RewardSupplySpeed (max_values: None, max_size: Some(43), added: 2518, mode: MaxEncodedLen)
	/// Storage: Loans RewardSupplierIndex (r:2 w:2)
	/// Proof: Loans RewardSupplierIndex (max_values: None, max_size: Some(91), added: 2566, mode: MaxEncodedLen)
	/// Storage: Loans RewardAccrued (r:2 w:2)
	/// Proof: Loans RewardAccrued (max_values: None, max_size: Some(64), added: 2539, mode: MaxEncodedLen)
	/// Storage: Loans AccountDeposits (r:1 w:0)
	/// Proof: Loans AccountDeposits (max_values: None, max_size: Some(91), added: 2566, mode: MaxEncodedLen)
	/// Storage: VaultRegistry SystemCollateralCeiling (r:1 w:0)
	/// Proof: VaultRegistry SystemCollateralCeiling (max_values: None, max_size: Some(54), added: 2529, mode: MaxEncodedLen)
	/// Storage: VaultRegistry LiquidationVault (r:1 w:1)
	/// Proof: VaultRegistry LiquidationVault (max_values: None, max_size: Some(124), added: 2599, mode: MaxEncodedLen)
	fn report_undercollateralized_vault	() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `4926`
		//  Estimated: `8799`
		// Minimum execution time: 973_462_000 picoseconds.
		Weight::from_parts(973_462_000, 8799)
			.saturating_add(T::DbWeight::get().reads(57_u64))
			.saturating_add(T::DbWeight::get().writes(30_u64))
	}
	/// Storage: VaultRegistry Vaults (r:1 w:1)
	/// Proof: VaultRegistry Vaults (max_values: None, max_size: Some(260), added: 2735, mode: MaxEncodedLen)
	fn recover_vault_id	() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `669`
		//  Estimated: `3725`
		// Minimum execution time: 23_836_000 picoseconds.
		Weight::from_parts(23_836_000, 3725)
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
}