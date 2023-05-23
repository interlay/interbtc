
//! Autogenerated weights for annuity
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-04-14, STEPS: `100`, REPEAT: `10`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `enterprise`, CPU: `Intel(R) Core(TM) i7-9700K CPU @ 3.60GHz`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("interlay-dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/interbtc-parachain
// benchmark
// pallet
// --pallet
// annuity
// --extrinsic
// *
// --chain
// interlay-dev
// --execution=wasm
// --wasm-execution=compiled
// --steps
// 100
// --repeat
// 10
// --output
// parachain/runtime/interlay/src/weights
// --template
// .deploy/runtime-weight-template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weights for annuity using the Substrate node and recommended hardware.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> annuity::WeightInfo for WeightInfo<T> {
	/// Storage: EscrowAnnuity RewardPerBlock (r:1 w:0)
	/// Proof: EscrowAnnuity RewardPerBlock (max_values: Some(1), max_size: Some(16), added: 511, mode: MaxEncodedLen)
	/// Storage: EscrowAnnuity RewardPerWrapped (r:1 w:0)
	/// Proof: EscrowAnnuity RewardPerWrapped (max_values: Some(1), max_size: Some(16), added: 511, mode: MaxEncodedLen)
	fn on_initialize() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `957`
		//  Estimated: `1022`
		// Minimum execution time: 21_418_000 picoseconds.
		Weight::from_parts(21_953_000, 1022)
			.saturating_add(T::DbWeight::get().reads(2_u64))
	}
	/// Storage: EscrowRewards Stake (r:1 w:0)
	/// Proof: EscrowRewards Stake (max_values: None, max_size: Some(64), added: 2539, mode: MaxEncodedLen)
	/// Storage: EscrowRewards RewardPerToken (r:1 w:0)
	/// Proof: EscrowRewards RewardPerToken (max_values: None, max_size: Some(59), added: 2534, mode: MaxEncodedLen)
	/// Storage: EscrowRewards RewardTally (r:1 w:1)
	/// Proof: EscrowRewards RewardTally (max_values: None, max_size: Some(91), added: 2566, mode: MaxEncodedLen)
	/// Storage: EscrowRewards TotalRewards (r:1 w:1)
	/// Proof: EscrowRewards TotalRewards (max_values: None, max_size: Some(43), added: 2518, mode: MaxEncodedLen)
	/// Storage: Tokens Accounts (r:2 w:2)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(115), added: 2590, mode: MaxEncodedLen)
	/// Storage: System Account (r:1 w:0)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	fn withdraw_rewards() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1867`
		//  Estimated: `17940`
		// Minimum execution time: 86_386_000 picoseconds.
		Weight::from_parts(96_129_000, 17940)
			.saturating_add(T::DbWeight::get().reads(7_u64))
			.saturating_add(T::DbWeight::get().writes(4_u64))
	}
	/// Storage: Tokens Accounts (r:1 w:0)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(115), added: 2590, mode: MaxEncodedLen)
	/// Storage: EscrowAnnuity RewardPerBlock (r:0 w:1)
	/// Proof: EscrowAnnuity RewardPerBlock (max_values: Some(1), max_size: Some(16), added: 511, mode: MaxEncodedLen)
	fn update_rewards() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `991`
		//  Estimated: `2590`
		// Minimum execution time: 18_395_000 picoseconds.
		Weight::from_parts(21_653_000, 2590)
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	/// Storage: EscrowAnnuity RewardPerWrapped (r:0 w:1)
	/// Proof: EscrowAnnuity RewardPerWrapped (max_values: Some(1), max_size: Some(16), added: 511, mode: MaxEncodedLen)
	fn set_reward_per_wrapped() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `711`
		//  Estimated: `0`
		// Minimum execution time: 8_859_000 picoseconds.
		Weight::from_parts(10_114_000, 0)
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
}