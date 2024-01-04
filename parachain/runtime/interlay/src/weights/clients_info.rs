
//! Autogenerated weights for clients_info
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

/// Weights for clients_info using the Substrate node and recommended hardware.
pub struct WeightInfo<T>(PhantomData<T>);

impl<T: frame_system::Config> clients_info::WeightInfo for WeightInfo<T> {

	/// Storage: ClientsInfo CurrentClientReleases (r:0 w:1)
	/// Proof: ClientsInfo CurrentClientReleases (max_values: None, max_size: Some(562), added: 3037, mode: MaxEncodedLen)
	/// The range of component `n` is `[0, 255]`.
	/// The range of component `u` is `[0, 255]`.
	fn set_current_client_release	(_n: u32, u: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 28_504_000 picoseconds.
		Weight::from_parts(29_112_000, 0)
			// Standard Error: 40
			.saturating_add(Weight::from_parts(1_266, 0).saturating_mul(u.into()))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	/// Storage: ClientsInfo PendingClientReleases (r:0 w:1)
	/// Proof: ClientsInfo PendingClientReleases (max_values: None, max_size: Some(562), added: 3037, mode: MaxEncodedLen)
	/// The range of component `n` is `[0, 255]`.
	/// The range of component `u` is `[0, 255]`.
	fn set_pending_client_release	(_n: u32, u: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 27_988_000 picoseconds.
		Weight::from_parts(28_117_000, 0)
			// Standard Error: 1_582
			.saturating_add(Weight::from_parts(3_392, 0).saturating_mul(u.into()))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
}