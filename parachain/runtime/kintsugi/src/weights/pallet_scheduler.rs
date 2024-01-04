
//! Autogenerated weights for pallet_scheduler
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2024-01-04, STEPS: `2`, REPEAT: `1`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `nakul-GF65-Thin-10UE`, CPU: `Intel(R) Core(TM) i7-10750H CPU @ 2.60GHz`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("kintsugi-dev"), DB CACHE: 1024

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
// kintsugi-dev
// --output
// parachain/runtime/kintsugi/src/weights/

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weights for pallet_scheduler using the Substrate node and recommended hardware.
pub struct WeightInfo<T>(PhantomData<T>);

impl<T: frame_system::Config> pallet_scheduler::WeightInfo for WeightInfo<T> {

	/// Storage: Scheduler IncompleteSince (r:1 w:1)
	/// Proof: Scheduler IncompleteSince (max_values: Some(1), max_size: Some(4), added: 499, mode: MaxEncodedLen)
	fn service_agendas_base	() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `31`
		//  Estimated: `1489`
		// Minimum execution time: 18_609_000 picoseconds.
		Weight::from_parts(18_609_000, 1489)
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	/// Storage: Scheduler Agenda (r:1 w:1)
	/// Proof: Scheduler Agenda (max_values: None, max_size: Some(23383), added: 25858, mode: MaxEncodedLen)
	/// The range of component `s` is `[0, 30]`.
	fn service_agenda_base	(_s: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `4 + s * (179 ±0)`
		//  Estimated: `26848`
		// Minimum execution time: 17_666_000 picoseconds.
		Weight::from_parts(99_643_000, 26848)
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	fn service_task_base	() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 30_641_000 picoseconds.
		Weight::from_parts(30_641_000, 0)
	}
	/// Storage: Preimage PreimageFor (r:1 w:1)
	/// Proof: Preimage PreimageFor (max_values: None, max_size: Some(4194344), added: 4196819, mode: Measured)
	/// Storage: Preimage StatusFor (r:1 w:1)
	/// Proof: Preimage StatusFor (max_values: None, max_size: Some(91), added: 2566, mode: MaxEncodedLen)
	/// The range of component `s` is `[128, 4194304]`.
	fn service_task_fetched	(_s: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `172 + s * (1 ±0)`
		//  Estimated: `4197945`
		// Minimum execution time: 90_963_000 picoseconds.
		Weight::from_parts(14_972_451_000, 4197945)
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
	/// Storage: Scheduler Lookup (r:0 w:1)
	/// Proof: Scheduler Lookup (max_values: None, max_size: Some(48), added: 2523, mode: MaxEncodedLen)
	fn service_task_named	() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 43_844_000 picoseconds.
		Weight::from_parts(43_844_000, 0)
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	fn service_task_periodic	() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 31_539_000 picoseconds.
		Weight::from_parts(31_539_000, 0)
	}
	fn execute_dispatch_signed	() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 23_736_000 picoseconds.
		Weight::from_parts(23_736_000, 0)
	}
	fn execute_dispatch_unsigned	() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 28_886_000 picoseconds.
		Weight::from_parts(28_886_000, 0)
	}
	/// Storage: Scheduler Agenda (r:1 w:1)
	/// Proof: Scheduler Agenda (max_values: None, max_size: Some(23383), added: 25858, mode: MaxEncodedLen)
	/// The range of component `s` is `[0, 29]`.
	fn schedule	(_s: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `4 + s * (179 ±0)`
		//  Estimated: `26848`
		// Minimum execution time: 77_926_000 picoseconds.
		Weight::from_parts(190_422_000, 26848)
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	/// Storage: Scheduler Agenda (r:1 w:1)
	/// Proof: Scheduler Agenda (max_values: None, max_size: Some(23383), added: 25858, mode: MaxEncodedLen)
	/// Storage: Scheduler Lookup (r:0 w:1)
	/// Proof: Scheduler Lookup (max_values: None, max_size: Some(48), added: 2523, mode: MaxEncodedLen)
	/// The range of component `s` is `[1, 30]`.
	fn cancel	(_s: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `78 + s * (177 ±0)`
		//  Estimated: `26848`
		// Minimum execution time: 83_261_000 picoseconds.
		Weight::from_parts(236_679_000, 26848)
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
	/// Storage: Scheduler Lookup (r:1 w:1)
	/// Proof: Scheduler Lookup (max_values: None, max_size: Some(48), added: 2523, mode: MaxEncodedLen)
	/// Storage: Scheduler Agenda (r:1 w:1)
	/// Proof: Scheduler Agenda (max_values: None, max_size: Some(23383), added: 25858, mode: MaxEncodedLen)
	/// The range of component `s` is `[0, 29]`.
	fn schedule_named	(_s: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `4 + s * (194 ±0)`
		//  Estimated: `26848`
		// Minimum execution time: 91_631_000 picoseconds.
		Weight::from_parts(184_746_000, 26848)
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
	/// Storage: Scheduler Lookup (r:1 w:1)
	/// Proof: Scheduler Lookup (max_values: None, max_size: Some(48), added: 2523, mode: MaxEncodedLen)
	/// Storage: Scheduler Agenda (r:1 w:1)
	/// Proof: Scheduler Agenda (max_values: None, max_size: Some(23383), added: 25858, mode: MaxEncodedLen)
	/// The range of component `s` is `[1, 30]`.
	fn cancel_named	(_s: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `98 + s * (192 ±0)`
		//  Estimated: `26848`
		// Minimum execution time: 146_572_000 picoseconds.
		Weight::from_parts(226_600_000, 26848)
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
}