
//! Autogenerated weights for nomination
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

/// Weights for nomination using the Substrate node and recommended hardware.
pub struct WeightInfo<T>(PhantomData<T>);

impl<T: frame_system::Config> nomination::WeightInfo for WeightInfo<T> {

	/// Storage: Nomination NominationEnabled (r:0 w:1)
	/// Proof: Nomination NominationEnabled (max_values: Some(1), max_size: Some(1), added: 496, mode: MaxEncodedLen)
	fn set_nomination_enabled	() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 13_099_000 picoseconds.
		Weight::from_parts(13_099_000, 0)
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	/// Storage: Nomination NominationLimit (r:0 w:1)
	/// Proof: Nomination NominationLimit (max_values: None, max_size: Some(86), added: 2561, mode: MaxEncodedLen)
	fn set_nomination_limit	() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 23_809_000 picoseconds.
		Weight::from_parts(23_809_000, 0)
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	/// Storage: Nomination NominationEnabled (r:1 w:0)
	/// Proof: Nomination NominationEnabled (max_values: Some(1), max_size: Some(1), added: 496, mode: MaxEncodedLen)
	/// Storage: VaultRegistry Vaults (r:1 w:0)
	/// Proof: VaultRegistry Vaults (max_values: None, max_size: Some(260), added: 2735, mode: MaxEncodedLen)
	/// Storage: Nomination Vaults (r:1 w:1)
	/// Proof: Nomination Vaults (max_values: None, max_size: Some(71), added: 2546, mode: MaxEncodedLen)
	fn opt_in_to_nomination	() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `642`
		//  Estimated: `3725`
		// Minimum execution time: 73_518_000 picoseconds.
		Weight::from_parts(73_518_000, 3725)
			.saturating_add(T::DbWeight::get().reads(3_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	/// Storage: Nomination Vaults (r:1 w:1)
	/// Proof: Nomination Vaults (max_values: None, max_size: Some(71), added: 2546, mode: MaxEncodedLen)
	/// Storage: VaultStaking Nonce (r:1 w:1)
	/// Proof: VaultStaking Nonce (max_values: None, max_size: Some(74), added: 2549, mode: MaxEncodedLen)
	/// Storage: VaultStaking TotalCurrentStake (r:2 w:2)
	/// Proof: VaultStaking TotalCurrentStake (max_values: None, max_size: Some(106), added: 2581, mode: MaxEncodedLen)
	/// Storage: VaultStaking Stake (r:2 w:2)
	/// Proof: VaultStaking Stake (max_values: None, max_size: Some(138), added: 2613, mode: MaxEncodedLen)
	/// Storage: VaultStaking SlashPerToken (r:2 w:0)
	/// Proof: VaultStaking SlashPerToken (max_values: None, max_size: Some(106), added: 2581, mode: MaxEncodedLen)
	/// Storage: VaultStaking SlashTally (r:2 w:2)
	/// Proof: VaultStaking SlashTally (max_values: None, max_size: Some(138), added: 2613, mode: MaxEncodedLen)
	/// Storage: VaultRegistry Vaults (r:1 w:0)
	/// Proof: VaultRegistry Vaults (max_values: None, max_size: Some(260), added: 2735, mode: MaxEncodedLen)
	/// Storage: VaultRegistry MinimumCollateralVault (r:1 w:0)
	/// Proof: VaultRegistry MinimumCollateralVault (max_values: None, max_size: Some(43), added: 2518, mode: MaxEncodedLen)
	/// Storage: VaultRegistry SecureCollateralThreshold (r:1 w:0)
	/// Proof: VaultRegistry SecureCollateralThreshold (max_values: None, max_size: Some(54), added: 2529, mode: MaxEncodedLen)
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
	/// Storage: VaultStaking RewardPerToken (r:4 w:2)
	/// Proof: VaultStaking RewardPerToken (max_values: None, max_size: Some(117), added: 2592, mode: MaxEncodedLen)
	/// Storage: VaultStaking TotalStake (r:2 w:2)
	/// Proof: VaultStaking TotalStake (max_values: None, max_size: Some(106), added: 2581, mode: MaxEncodedLen)
	/// Storage: VaultStaking RewardTally (r:4 w:4)
	/// Proof: VaultStaking RewardTally (max_values: None, max_size: Some(149), added: 2624, mode: MaxEncodedLen)
	/// Storage: VaultRewards TotalStake (r:1 w:0)
	/// Proof: VaultRewards TotalStake (max_values: None, max_size: Some(43), added: 2518, mode: MaxEncodedLen)
	/// Storage: VaultRegistry TotalUserVaultCollateral (r:1 w:1)
	/// Proof: VaultRegistry TotalUserVaultCollateral (max_values: None, max_size: Some(54), added: 2529, mode: MaxEncodedLen)
	fn opt_out_of_nomination	() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `4500`
		//  Estimated: `11486`
		// Minimum execution time: 1_003_883_000 picoseconds.
		Weight::from_parts(1_003_883_000, 11486)
			.saturating_add(T::DbWeight::get().reads(53_u64))
			.saturating_add(T::DbWeight::get().writes(26_u64))
	}
	/// Storage: VaultStaking Nonce (r:1 w:0)
	/// Proof: VaultStaking Nonce (max_values: None, max_size: Some(74), added: 2549, mode: MaxEncodedLen)
	/// Storage: VaultStaking TotalCurrentStake (r:1 w:1)
	/// Proof: VaultStaking TotalCurrentStake (max_values: None, max_size: Some(106), added: 2581, mode: MaxEncodedLen)
	/// Storage: VaultStaking Stake (r:2 w:1)
	/// Proof: VaultStaking Stake (max_values: None, max_size: Some(138), added: 2613, mode: MaxEncodedLen)
	/// Storage: VaultStaking SlashPerToken (r:1 w:0)
	/// Proof: VaultStaking SlashPerToken (max_values: None, max_size: Some(106), added: 2581, mode: MaxEncodedLen)
	/// Storage: VaultStaking SlashTally (r:2 w:1)
	/// Proof: VaultStaking SlashTally (max_values: None, max_size: Some(138), added: 2613, mode: MaxEncodedLen)
	/// Storage: Nomination NominationLimit (r:1 w:0)
	/// Proof: Nomination NominationLimit (max_values: None, max_size: Some(86), added: 2561, mode: MaxEncodedLen)
	/// Storage: Nomination NominationEnabled (r:1 w:0)
	/// Proof: Nomination NominationEnabled (max_values: Some(1), max_size: Some(1), added: 496, mode: MaxEncodedLen)
	/// Storage: Nomination Vaults (r:1 w:0)
	/// Proof: Nomination Vaults (max_values: None, max_size: Some(71), added: 2546, mode: MaxEncodedLen)
	/// Storage: Loans UnderlyingAssetId (r:1 w:0)
	/// Proof: Loans UnderlyingAssetId (max_values: None, max_size: Some(38), added: 2513, mode: MaxEncodedLen)
	/// Storage: Loans RewardSupplyState (r:1 w:1)
	/// Proof: Loans RewardSupplyState (max_values: None, max_size: Some(47), added: 2522, mode: MaxEncodedLen)
	/// Storage: Loans RewardSupplySpeed (r:1 w:0)
	/// Proof: Loans RewardSupplySpeed (max_values: None, max_size: Some(43), added: 2518, mode: MaxEncodedLen)
	/// Storage: Loans RewardSupplierIndex (r:2 w:2)
	/// Proof: Loans RewardSupplierIndex (max_values: None, max_size: Some(91), added: 2566, mode: MaxEncodedLen)
	/// Storage: Loans Markets (r:2 w:0)
	/// Proof: Loans Markets (max_values: None, max_size: Some(160), added: 2635, mode: MaxEncodedLen)
	/// Storage: Loans RewardAccrued (r:2 w:2)
	/// Proof: Loans RewardAccrued (max_values: None, max_size: Some(64), added: 2539, mode: MaxEncodedLen)
	/// Storage: Tokens Accounts (r:3 w:2)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(115), added: 2590, mode: MaxEncodedLen)
	/// Storage: System Account (r:2 w:0)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	/// Storage: Loans AccountDeposits (r:1 w:0)
	/// Proof: Loans AccountDeposits (max_values: None, max_size: Some(91), added: 2566, mode: MaxEncodedLen)
	/// Storage: VaultCapacity Stake (r:1 w:1)
	/// Proof: VaultCapacity Stake (max_values: None, max_size: Some(43), added: 2518, mode: MaxEncodedLen)
	/// Storage: VaultCapacity RewardPerToken (r:2 w:0)
	/// Proof: VaultCapacity RewardPerToken (max_values: None, max_size: Some(59), added: 2534, mode: MaxEncodedLen)
	/// Storage: VaultCapacity RewardTally (r:2 w:2)
	/// Proof: VaultCapacity RewardTally (max_values: None, max_size: Some(70), added: 2545, mode: MaxEncodedLen)
	/// Storage: VaultCapacity TotalRewards (r:2 w:2)
	/// Proof: VaultCapacity TotalRewards (max_values: None, max_size: Some(43), added: 2518, mode: MaxEncodedLen)
	/// Storage: VaultRewards TotalStake (r:1 w:1)
	/// Proof: VaultRewards TotalStake (max_values: None, max_size: Some(43), added: 2518, mode: MaxEncodedLen)
	/// Storage: VaultRewards RewardCurrencies (r:1 w:1)
	/// Proof: VaultRewards RewardCurrencies (max_values: None, max_size: Some(50), added: 2525, mode: MaxEncodedLen)
	/// Storage: VaultRewards RewardPerToken (r:2 w:2)
	/// Proof: VaultRewards RewardPerToken (max_values: None, max_size: Some(70), added: 2545, mode: MaxEncodedLen)
	/// Storage: VaultRewards TotalRewards (r:2 w:2)
	/// Proof: VaultRewards TotalRewards (max_values: None, max_size: Some(43), added: 2518, mode: MaxEncodedLen)
	/// Storage: VaultRewards Stake (r:1 w:1)
	/// Proof: VaultRewards Stake (max_values: None, max_size: Some(97), added: 2572, mode: MaxEncodedLen)
	/// Storage: VaultRewards RewardTally (r:2 w:2)
	/// Proof: VaultRewards RewardTally (max_values: None, max_size: Some(124), added: 2599, mode: MaxEncodedLen)
	/// Storage: Fee Commission (r:1 w:0)
	/// Proof: Fee Commission (max_values: None, max_size: Some(86), added: 2561, mode: MaxEncodedLen)
	/// Storage: VaultStaking RewardPerToken (r:2 w:2)
	/// Proof: VaultStaking RewardPerToken (max_values: None, max_size: Some(117), added: 2592, mode: MaxEncodedLen)
	/// Storage: VaultStaking TotalRewards (r:2 w:2)
	/// Proof: VaultStaking TotalRewards (max_values: None, max_size: Some(117), added: 2592, mode: MaxEncodedLen)
	/// Storage: VaultStaking TotalStake (r:1 w:1)
	/// Proof: VaultStaking TotalStake (max_values: None, max_size: Some(106), added: 2581, mode: MaxEncodedLen)
	/// Storage: VaultStaking RewardTally (r:2 w:2)
	/// Proof: VaultStaking RewardTally (max_values: None, max_size: Some(149), added: 2624, mode: MaxEncodedLen)
	/// Storage: VaultRegistry Vaults (r:1 w:0)
	/// Proof: VaultRegistry Vaults (max_values: None, max_size: Some(260), added: 2735, mode: MaxEncodedLen)
	/// Storage: VaultRegistry SecureCollateralThreshold (r:1 w:0)
	/// Proof: VaultRegistry SecureCollateralThreshold (max_values: None, max_size: Some(54), added: 2529, mode: MaxEncodedLen)
	/// Storage: Timestamp Now (r:1 w:0)
	/// Proof: Timestamp Now (max_values: Some(1), max_size: Some(8), added: 503, mode: MaxEncodedLen)
	/// Storage: Loans LastAccruedInterestTime (r:1 w:1)
	/// Proof: Loans LastAccruedInterestTime (max_values: None, max_size: Some(35), added: 2510, mode: MaxEncodedLen)
	/// Storage: Tokens TotalIssuance (r:1 w:0)
	/// Proof: Tokens TotalIssuance (max_values: None, max_size: Some(35), added: 2510, mode: MaxEncodedLen)
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
	/// Storage: VaultCapacity TotalStake (r:1 w:1)
	/// Proof: VaultCapacity TotalStake (max_values: None, max_size: Some(32), added: 2507, mode: MaxEncodedLen)
	/// Storage: VaultCapacity RewardCurrencies (r:1 w:0)
	/// Proof: VaultCapacity RewardCurrencies (max_values: None, max_size: Some(39), added: 2514, mode: MaxEncodedLen)
	/// Storage: VaultRegistry TotalUserVaultCollateral (r:1 w:1)
	/// Proof: VaultRegistry TotalUserVaultCollateral (max_values: None, max_size: Some(54), added: 2529, mode: MaxEncodedLen)
	/// Storage: VaultRegistry SystemCollateralCeiling (r:1 w:0)
	/// Proof: VaultRegistry SystemCollateralCeiling (max_values: None, max_size: Some(54), added: 2529, mode: MaxEncodedLen)
	fn deposit_collateral	() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `5681`
		//  Estimated: `8760`
		// Minimum execution time: 929_215_000 picoseconds.
		Weight::from_parts(929_215_000, 8760)
			.saturating_add(T::DbWeight::get().reads(63_u64))
			.saturating_add(T::DbWeight::get().writes(34_u64))
	}
	/// Storage: VaultStaking Nonce (r:1 w:0)
	/// Proof: VaultStaking Nonce (max_values: None, max_size: Some(74), added: 2549, mode: MaxEncodedLen)
	/// Storage: VaultRegistry Vaults (r:1 w:0)
	/// Proof: VaultRegistry Vaults (max_values: None, max_size: Some(260), added: 2735, mode: MaxEncodedLen)
	/// Storage: VaultStaking TotalCurrentStake (r:1 w:1)
	/// Proof: VaultStaking TotalCurrentStake (max_values: None, max_size: Some(106), added: 2581, mode: MaxEncodedLen)
	/// Storage: VaultRegistry MinimumCollateralVault (r:1 w:0)
	/// Proof: VaultRegistry MinimumCollateralVault (max_values: None, max_size: Some(43), added: 2518, mode: MaxEncodedLen)
	/// Storage: VaultRegistry SecureCollateralThreshold (r:1 w:0)
	/// Proof: VaultRegistry SecureCollateralThreshold (max_values: None, max_size: Some(54), added: 2529, mode: MaxEncodedLen)
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
	/// Storage: System Account (r:2 w:0)
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
	/// Storage: Nomination NominationEnabled (r:1 w:0)
	/// Proof: Nomination NominationEnabled (max_values: Some(1), max_size: Some(1), added: 496, mode: MaxEncodedLen)
	/// Storage: Nomination Vaults (r:1 w:0)
	/// Proof: Nomination Vaults (max_values: None, max_size: Some(71), added: 2546, mode: MaxEncodedLen)
	/// Storage: VaultCapacity Stake (r:1 w:1)
	/// Proof: VaultCapacity Stake (max_values: None, max_size: Some(43), added: 2518, mode: MaxEncodedLen)
	/// Storage: VaultCapacity RewardPerToken (r:2 w:0)
	/// Proof: VaultCapacity RewardPerToken (max_values: None, max_size: Some(59), added: 2534, mode: MaxEncodedLen)
	/// Storage: VaultCapacity RewardTally (r:2 w:2)
	/// Proof: VaultCapacity RewardTally (max_values: None, max_size: Some(70), added: 2545, mode: MaxEncodedLen)
	/// Storage: VaultCapacity TotalRewards (r:2 w:2)
	/// Proof: VaultCapacity TotalRewards (max_values: None, max_size: Some(43), added: 2518, mode: MaxEncodedLen)
	/// Storage: VaultRewards TotalStake (r:1 w:1)
	/// Proof: VaultRewards TotalStake (max_values: None, max_size: Some(43), added: 2518, mode: MaxEncodedLen)
	/// Storage: VaultRewards RewardCurrencies (r:1 w:1)
	/// Proof: VaultRewards RewardCurrencies (max_values: None, max_size: Some(50), added: 2525, mode: MaxEncodedLen)
	/// Storage: VaultRewards RewardPerToken (r:2 w:2)
	/// Proof: VaultRewards RewardPerToken (max_values: None, max_size: Some(70), added: 2545, mode: MaxEncodedLen)
	/// Storage: VaultRewards TotalRewards (r:2 w:2)
	/// Proof: VaultRewards TotalRewards (max_values: None, max_size: Some(43), added: 2518, mode: MaxEncodedLen)
	/// Storage: VaultRewards Stake (r:1 w:1)
	/// Proof: VaultRewards Stake (max_values: None, max_size: Some(97), added: 2572, mode: MaxEncodedLen)
	/// Storage: VaultRewards RewardTally (r:2 w:2)
	/// Proof: VaultRewards RewardTally (max_values: None, max_size: Some(124), added: 2599, mode: MaxEncodedLen)
	/// Storage: Fee Commission (r:1 w:0)
	/// Proof: Fee Commission (max_values: None, max_size: Some(86), added: 2561, mode: MaxEncodedLen)
	/// Storage: VaultStaking RewardPerToken (r:2 w:2)
	/// Proof: VaultStaking RewardPerToken (max_values: None, max_size: Some(117), added: 2592, mode: MaxEncodedLen)
	/// Storage: VaultStaking TotalRewards (r:2 w:2)
	/// Proof: VaultStaking TotalRewards (max_values: None, max_size: Some(117), added: 2592, mode: MaxEncodedLen)
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
	/// Storage: VaultCapacity TotalStake (r:1 w:1)
	/// Proof: VaultCapacity TotalStake (max_values: None, max_size: Some(32), added: 2507, mode: MaxEncodedLen)
	/// Storage: VaultCapacity RewardCurrencies (r:1 w:0)
	/// Proof: VaultCapacity RewardCurrencies (max_values: None, max_size: Some(39), added: 2514, mode: MaxEncodedLen)
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
	/// Storage: VaultRegistry TotalUserVaultCollateral (r:1 w:1)
	/// Proof: VaultRegistry TotalUserVaultCollateral (max_values: None, max_size: Some(54), added: 2529, mode: MaxEncodedLen)
	fn withdraw_collateral	() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `6006`
		//  Estimated: `8760`
		// Minimum execution time: 758_172_000 picoseconds.
		Weight::from_parts(758_172_000, 8760)
			.saturating_add(T::DbWeight::get().reads(60_u64))
			.saturating_add(T::DbWeight::get().writes(34_u64))
	}
}