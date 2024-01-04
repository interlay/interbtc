
//! Autogenerated weights for frame_system
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

/// Weights for frame_system using the Substrate node and recommended hardware.
pub struct WeightInfo<T>(PhantomData<T>);

impl<T: frame_system::Config> frame_system::WeightInfo for WeightInfo<T> {

	/// The range of component `b` is `[0, 3932160]`.
	fn remark	(_b: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 8_974_000 picoseconds.
		Weight::from_parts(1_911_064_000, 0)
	}
	/// The range of component `b` is `[0, 3932160]`.
	fn remark_with_event	(_b: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 19_399_000 picoseconds.
		Weight::from_parts(5_519_412_000, 0)
	}
	/// Storage: System Digest (r:1 w:1)
	/// Proof Skipped: System Digest (max_values: Some(1), max_size: None, mode: Measured)
	/// Storage: unknown `0x3a686561707061676573` (r:0 w:1)
	/// Proof Skipped: unknown `0x3a686561707061676573` (r:0 w:1)
	fn set_heap_pages	() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `1485`
		// Minimum execution time: 33_497_000 picoseconds.
		Weight::from_parts(33_497_000, 1485)
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
	/// Storage: Skipped Metadata (r:0 w:0)
	/// Proof Skipped: Skipped Metadata (max_values: None, max_size: None, mode: Measured)
	/// The range of component `i` is `[0, 1000]`.
	fn set_storage	(_i: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 8_849_000 picoseconds.
		Weight::from_parts(1_073_399_000, 0)
			.saturating_add(T::DbWeight::get().writes(1000_u64))
	}
	/// Storage: Skipped Metadata (r:0 w:0)
	/// Proof Skipped: Skipped Metadata (max_values: None, max_size: None, mode: Measured)
	/// The range of component `i` is `[0, 1000]`.
	fn kill_storage	(_i: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 9_313_000 picoseconds.
		Weight::from_parts(640_607_000, 0)
			.saturating_add(T::DbWeight::get().writes(1000_u64))
	}
	/// Storage: Skipped Metadata (r:0 w:0)
	/// Proof Skipped: Skipped Metadata (max_values: None, max_size: None, mode: Measured)
	/// The range of component `p` is `[0, 1000]`.
	fn kill_prefix	(_p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `124 + p * (69 ±0)`
		//  Estimated: `69676`
		// Minimum execution time: 13_911_000 picoseconds.
		Weight::from_parts(1_150_870_000, 69676)
			.saturating_add(T::DbWeight::get().reads(1000_u64))
			.saturating_add(T::DbWeight::get().writes(1000_u64))
	}
}