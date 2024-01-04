
//! Autogenerated weights for orml_asset_registry
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

/// Weights for orml_asset_registry using the Substrate node and recommended hardware.
pub struct WeightInfo<T>(PhantomData<T>);

impl<T: frame_system::Config> orml_asset_registry::WeightInfo for WeightInfo<T> {

	/// Storage: AssetRegistry LastAssetId (r:1 w:1)
	/// Proof Skipped: AssetRegistry LastAssetId (max_values: Some(1), max_size: None, mode: Measured)
	/// Storage: AssetRegistry Metadata (r:1 w:1)
	/// Proof Skipped: AssetRegistry Metadata (max_values: None, max_size: None, mode: Measured)
	/// Storage: AssetRegistry LocationToAssetId (r:1 w:1)
	/// Proof Skipped: AssetRegistry LocationToAssetId (max_values: None, max_size: None, mode: Measured)
	fn register_asset	() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `107`
		//  Estimated: `3572`
		// Minimum execution time: 42_033_000 picoseconds.
		Weight::from_parts(42_033_000, 3572)
			.saturating_add(T::DbWeight::get().reads(3_u64))
			.saturating_add(T::DbWeight::get().writes(3_u64))
	}
	/// Storage: AssetRegistry Metadata (r:1 w:1)
	/// Proof Skipped: AssetRegistry Metadata (max_values: None, max_size: None, mode: Measured)
	/// Storage: AssetRegistry LocationToAssetId (r:1 w:2)
	/// Proof Skipped: AssetRegistry LocationToAssetId (max_values: None, max_size: None, mode: Measured)
	fn update_asset	() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `774`
		//  Estimated: `4239`
		// Minimum execution time: 42_748_000 picoseconds.
		Weight::from_parts(42_748_000, 4239)
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(3_u64))
	}
	fn set_asset_location	() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 2_761_000 picoseconds.
		Weight::from_parts(2_761_000, 0)
	}
}