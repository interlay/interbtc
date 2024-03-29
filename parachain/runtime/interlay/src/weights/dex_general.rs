
//! Autogenerated weights for dex_general
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-08-07, STEPS: `50`, REPEAT: `10`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `interlay-rust-runner-2mz2v-jrrg4`, CPU: `AMD EPYC 7502P 32-Core Processor`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("interlay-dev"), DB CACHE: 1024

// Executed Command:
// target/release/interbtc-parachain
// benchmark
// pallet
// --pallet
// *
// --extrinsic
// *
// --chain
// interlay-dev
// --execution=wasm
// --wasm-execution=compiled
// --steps
// 50
// --repeat
// 10
// --output
// parachain/runtime/interlay/src/weights/
// --template
// .deploy/runtime-weight-template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weights for dex_general using the Substrate node and recommended hardware.
pub struct WeightInfo<T>(PhantomData<T>);

impl<T: frame_system::Config> dex_general::WeightInfo for WeightInfo<T> {

	/// Storage: DexGeneral FeeMeta (r:1 w:1)
	/// Proof: DexGeneral FeeMeta (max_values: Some(1), max_size: Some(34), added: 529, mode: MaxEncodedLen)
	fn set_fee_receiver	() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `4`
		//  Estimated: `1519`
		// Minimum execution time: 13_066_000 picoseconds.
		Weight::from_parts(13_848_000, 1519)
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	/// Storage: DexGeneral FeeMeta (r:1 w:1)
	/// Proof: DexGeneral FeeMeta (max_values: Some(1), max_size: Some(34), added: 529, mode: MaxEncodedLen)
	fn set_fee_point	() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `4`
		//  Estimated: `1519`
		// Minimum execution time: 21_603_000 picoseconds.
		Weight::from_parts(22_054_000, 1519)
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	/// Storage: DexGeneral PairStatuses (r:1 w:1)
	/// Proof: DexGeneral PairStatuses (max_values: None, max_size: Some(163), added: 2638, mode: MaxEncodedLen)
	/// Storage: DexGeneral LiquidityPairs (r:0 w:1)
	/// Proof: DexGeneral LiquidityPairs (max_values: None, max_size: Some(50), added: 2525, mode: MaxEncodedLen)
	fn create_pair	() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `4`
		//  Estimated: `3628`
		// Minimum execution time: 31_944_000 picoseconds.
		Weight::from_parts(32_956_000, 3628)
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
	/// Storage: DexGeneral PairStatuses (r:1 w:1)
	/// Proof: DexGeneral PairStatuses (max_values: None, max_size: Some(163), added: 2638, mode: MaxEncodedLen)
	/// Storage: DexGeneral BootstrapLimits (r:0 w:1)
	/// Proof: DexGeneral BootstrapLimits (max_values: None, max_size: Some(27032), added: 29507, mode: MaxEncodedLen)
	/// Storage: DexGeneral BootstrapRewards (r:0 w:1)
	/// Proof: DexGeneral BootstrapRewards (max_values: None, max_size: Some(27032), added: 29507, mode: MaxEncodedLen)
	/// The range of component `r` is `[1, 10]`.
	/// The range of component `l` is `[1, 10]`.
	fn bootstrap_create	(r: u32, l: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `4`
		//  Estimated: `3628`
		// Minimum execution time: 37_816_000 picoseconds.
		Weight::from_parts(29_977_579, 3628)
			// Standard Error: 119_682
			.saturating_add(Weight::from_parts(499_256, 0).saturating_mul(r.into()))
			// Standard Error: 119_682
			.saturating_add(Weight::from_parts(1_107_001, 0).saturating_mul(l.into()))
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(3_u64))
	}
	/// Storage: DexGeneral BootstrapLimits (r:1 w:0)
	/// Proof: DexGeneral BootstrapLimits (max_values: None, max_size: Some(27032), added: 29507, mode: MaxEncodedLen)
	/// Storage: Tokens Accounts (r:4 w:4)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(115), added: 2590, mode: MaxEncodedLen)
	/// Storage: DexGeneral PairStatuses (r:1 w:1)
	/// Proof: DexGeneral PairStatuses (max_values: None, max_size: Some(163), added: 2638, mode: MaxEncodedLen)
	/// Storage: DexGeneral BootstrapPersonalSupply (r:1 w:1)
	/// Proof: DexGeneral BootstrapPersonalSupply (max_values: None, max_size: Some(102), added: 2577, mode: MaxEncodedLen)
	/// Storage: System Account (r:1 w:1)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	fn bootstrap_contribute	() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `941`
		//  Estimated: `30497`
		// Minimum execution time: 153_948_000 picoseconds.
		Weight::from_parts(156_744_000, 30497)
			.saturating_add(T::DbWeight::get().reads(8_u64))
			.saturating_add(T::DbWeight::get().writes(7_u64))
	}
	/// Storage: DexGeneral PairStatuses (r:1 w:0)
	/// Proof: DexGeneral PairStatuses (max_values: None, max_size: Some(163), added: 2638, mode: MaxEncodedLen)
	/// Storage: DexGeneral BootstrapPersonalSupply (r:1 w:1)
	/// Proof: DexGeneral BootstrapPersonalSupply (max_values: None, max_size: Some(102), added: 2577, mode: MaxEncodedLen)
	/// Storage: DexGeneral BootstrapEndStatus (r:1 w:0)
	/// Proof: DexGeneral BootstrapEndStatus (max_values: None, max_size: Some(163), added: 2638, mode: MaxEncodedLen)
	/// Storage: DexGeneral LiquidityPairs (r:1 w:0)
	/// Proof: DexGeneral LiquidityPairs (max_values: None, max_size: Some(50), added: 2525, mode: MaxEncodedLen)
	/// Storage: Tokens Accounts (r:2 w:2)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(115), added: 2590, mode: MaxEncodedLen)
	/// Storage: System Account (r:1 w:0)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	/// Storage: DexGeneral BootstrapRewards (r:1 w:0)
	/// Proof: DexGeneral BootstrapRewards (max_values: None, max_size: Some(27032), added: 29507, mode: MaxEncodedLen)
	fn bootstrap_claim	() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1486`
		//  Estimated: `30497`
		// Minimum execution time: 131_704_000 picoseconds.
		Weight::from_parts(132_755_000, 30497)
			.saturating_add(T::DbWeight::get().reads(8_u64))
			.saturating_add(T::DbWeight::get().writes(3_u64))
	}
	/// Storage: DexGeneral PairStatuses (r:1 w:1)
	/// Proof: DexGeneral PairStatuses (max_values: None, max_size: Some(163), added: 2638, mode: MaxEncodedLen)
	/// Storage: Tokens Accounts (r:5 w:5)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(115), added: 2590, mode: MaxEncodedLen)
	/// Storage: System Account (r:2 w:1)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	/// Storage: Tokens TotalIssuance (r:1 w:1)
	/// Proof: Tokens TotalIssuance (max_values: None, max_size: Some(35), added: 2510, mode: MaxEncodedLen)
	/// Storage: DexGeneral LiquidityPairs (r:0 w:1)
	/// Proof: DexGeneral LiquidityPairs (max_values: None, max_size: Some(50), added: 2525, mode: MaxEncodedLen)
	/// Storage: DexGeneral BootstrapEndStatus (r:0 w:1)
	/// Proof: DexGeneral BootstrapEndStatus (max_values: None, max_size: Some(163), added: 2638, mode: MaxEncodedLen)
	fn bootstrap_end	() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1200`
		//  Estimated: `13940`
		// Minimum execution time: 192_676_000 picoseconds.
		Weight::from_parts(193_828_000, 13940)
			.saturating_add(T::DbWeight::get().reads(9_u64))
			.saturating_add(T::DbWeight::get().writes(10_u64))
	}
	/// Storage: DexGeneral PairStatuses (r:1 w:1)
	/// Proof: DexGeneral PairStatuses (max_values: None, max_size: Some(163), added: 2638, mode: MaxEncodedLen)
	/// Storage: DexGeneral BootstrapRewards (r:1 w:1)
	/// Proof: DexGeneral BootstrapRewards (max_values: None, max_size: Some(27032), added: 29507, mode: MaxEncodedLen)
	/// Storage: DexGeneral BootstrapLimits (r:0 w:1)
	/// Proof: DexGeneral BootstrapLimits (max_values: None, max_size: Some(27032), added: 29507, mode: MaxEncodedLen)
	/// The range of component `r` is `[1, 10]`.
	/// The range of component `l` is `[1, 10]`.
	fn bootstrap_update	(r: u32, l: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `260 + r * (21 ±0)`
		//  Estimated: `30497`
		// Minimum execution time: 46_433_000 picoseconds.
		Weight::from_parts(45_194_974, 30497)
			// Standard Error: 144_589
			.saturating_add(Weight::from_parts(153_960, 0).saturating_mul(r.into()))
			// Standard Error: 144_589
			.saturating_add(Weight::from_parts(754_676, 0).saturating_mul(l.into()))
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(3_u64))
	}
	/// Storage: DexGeneral PairStatuses (r:1 w:1)
	/// Proof: DexGeneral PairStatuses (max_values: None, max_size: Some(163), added: 2638, mode: MaxEncodedLen)
	/// Storage: DexGeneral BootstrapPersonalSupply (r:1 w:1)
	/// Proof: DexGeneral BootstrapPersonalSupply (max_values: None, max_size: Some(102), added: 2577, mode: MaxEncodedLen)
	/// Storage: Tokens Accounts (r:4 w:4)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(115), added: 2590, mode: MaxEncodedLen)
	/// Storage: System Account (r:1 w:0)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	fn bootstrap_refund	() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1301`
		//  Estimated: `11350`
		// Minimum execution time: 122_856_000 picoseconds.
		Weight::from_parts(127_425_000, 11350)
			.saturating_add(T::DbWeight::get().reads(7_u64))
			.saturating_add(T::DbWeight::get().writes(6_u64))
	}
	/// Storage: DexGeneral PairStatuses (r:1 w:1)
	/// Proof: DexGeneral PairStatuses (max_values: None, max_size: Some(163), added: 2638, mode: MaxEncodedLen)
	/// Storage: Tokens Accounts (r:5 w:5)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(115), added: 2590, mode: MaxEncodedLen)
	/// Storage: DexGeneral LiquidityPairs (r:1 w:0)
	/// Proof: DexGeneral LiquidityPairs (max_values: None, max_size: Some(50), added: 2525, mode: MaxEncodedLen)
	/// Storage: DexGeneral KLast (r:1 w:1)
	/// Proof: DexGeneral KLast (max_values: None, max_size: Some(62), added: 2537, mode: MaxEncodedLen)
	/// Storage: DexGeneral FeeMeta (r:1 w:0)
	/// Proof: DexGeneral FeeMeta (max_values: Some(1), max_size: Some(34), added: 529, mode: MaxEncodedLen)
	/// Storage: Tokens TotalIssuance (r:1 w:1)
	/// Proof: Tokens TotalIssuance (max_values: None, max_size: Some(35), added: 2510, mode: MaxEncodedLen)
	/// Storage: System Account (r:1 w:1)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	fn add_liquidity	() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `974`
		//  Estimated: `13940`
		// Minimum execution time: 213_548_000 picoseconds.
		Weight::from_parts(214_830_000, 13940)
			.saturating_add(T::DbWeight::get().reads(11_u64))
			.saturating_add(T::DbWeight::get().writes(9_u64))
	}
	/// Storage: DexGeneral PairStatuses (r:1 w:1)
	/// Proof: DexGeneral PairStatuses (max_values: None, max_size: Some(163), added: 2638, mode: MaxEncodedLen)
	/// Storage: Tokens Accounts (r:5 w:5)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(115), added: 2590, mode: MaxEncodedLen)
	/// Storage: DexGeneral LiquidityPairs (r:1 w:0)
	/// Proof: DexGeneral LiquidityPairs (max_values: None, max_size: Some(50), added: 2525, mode: MaxEncodedLen)
	/// Storage: DexGeneral KLast (r:1 w:1)
	/// Proof: DexGeneral KLast (max_values: None, max_size: Some(62), added: 2537, mode: MaxEncodedLen)
	/// Storage: DexGeneral FeeMeta (r:1 w:0)
	/// Proof: DexGeneral FeeMeta (max_values: Some(1), max_size: Some(34), added: 529, mode: MaxEncodedLen)
	/// Storage: Tokens TotalIssuance (r:1 w:1)
	/// Proof: Tokens TotalIssuance (max_values: None, max_size: Some(35), added: 2510, mode: MaxEncodedLen)
	/// Storage: System Account (r:1 w:0)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	fn remove_liquidity	() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1443`
		//  Estimated: `13940`
		// Minimum execution time: 182_185_000 picoseconds.
		Weight::from_parts(183_037_000, 13940)
			.saturating_add(T::DbWeight::get().reads(11_u64))
			.saturating_add(T::DbWeight::get().writes(8_u64))
	}
	/// Storage: Tokens Accounts (r:20 w:20)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(115), added: 2590, mode: MaxEncodedLen)
	/// Storage: DexGeneral PairStatuses (r:9 w:0)
	/// Proof: DexGeneral PairStatuses (max_values: None, max_size: Some(163), added: 2638, mode: MaxEncodedLen)
	/// Storage: System Account (r:9 w:0)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	/// The range of component `a` is `[2, 10]`.
	fn swap_exact_assets_for_assets	(a: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `224 + a * (459 ±0)`
		//  Estimated: `3628 + a * (5180 ±21)`
		// Minimum execution time: 134_600_000 picoseconds.
		Weight::from_parts(11_419_585, 3628)
			// Standard Error: 109_224
			.saturating_add(Weight::from_parts(63_887_910, 0).saturating_mul(a.into()))
			.saturating_add(T::DbWeight::get().reads(6_u64))
			.saturating_add(T::DbWeight::get().reads((3_u64).saturating_mul(a.into())))
			.saturating_add(T::DbWeight::get().writes((2_u64).saturating_mul(a.into())))
			.saturating_add(Weight::from_parts(0, 5180).saturating_mul(a.into()))
	}
	/// Storage: Tokens Accounts (r:20 w:20)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(115), added: 2590, mode: MaxEncodedLen)
	/// Storage: DexGeneral PairStatuses (r:9 w:0)
	/// Proof: DexGeneral PairStatuses (max_values: None, max_size: Some(163), added: 2638, mode: MaxEncodedLen)
	/// Storage: System Account (r:9 w:0)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	/// The range of component `a` is `[2, 10]`.
	fn swap_assets_for_exact_assets	(a: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `224 + a * (459 ±0)`
		//  Estimated: `3628 + a * (5180 ±22)`
		// Minimum execution time: 134_940_000 picoseconds.
		Weight::from_parts(10_824_127, 3628)
			// Standard Error: 93_262
			.saturating_add(Weight::from_parts(63_943_649, 0).saturating_mul(a.into()))
			.saturating_add(T::DbWeight::get().reads(6_u64))
			.saturating_add(T::DbWeight::get().reads((3_u64).saturating_mul(a.into())))
			.saturating_add(T::DbWeight::get().writes((2_u64).saturating_mul(a.into())))
			.saturating_add(Weight::from_parts(0, 5180).saturating_mul(a.into()))
	}
	/// Storage: DexGeneral BootstrapRewards (r:1 w:1)
	/// Proof: DexGeneral BootstrapRewards (max_values: None, max_size: Some(27032), added: 29507, mode: MaxEncodedLen)
	/// Storage: Tokens Accounts (r:20 w:20)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(115), added: 2590, mode: MaxEncodedLen)
	/// Storage: System Account (r:1 w:1)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	/// The range of component `r` is `[1, 10]`.
	fn bootstrap_charge_reward	(r: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `646 + r * (88 ±0)`
		//  Estimated: `30497 + r * (5180 ±0)`
		// Minimum execution time: 92_285_000 picoseconds.
		Weight::from_parts(51_599_752, 30497)
			// Standard Error: 35_906
			.saturating_add(Weight::from_parts(43_455_215, 0).saturating_mul(r.into()))
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().reads((2_u64).saturating_mul(r.into())))
			.saturating_add(T::DbWeight::get().writes(2_u64))
			.saturating_add(T::DbWeight::get().writes((2_u64).saturating_mul(r.into())))
			.saturating_add(Weight::from_parts(0, 5180).saturating_mul(r.into()))
	}
	/// Storage: DexGeneral BootstrapRewards (r:1 w:1)
	/// Proof: DexGeneral BootstrapRewards (max_values: None, max_size: Some(27032), added: 29507, mode: MaxEncodedLen)
	fn bootstrap_withdraw_reward	() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `136`
		//  Estimated: `30497`
		// Minimum execution time: 29_279_000 picoseconds.
		Weight::from_parts(29_900_000, 30497)
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
}