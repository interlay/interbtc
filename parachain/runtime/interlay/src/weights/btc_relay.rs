
//! Autogenerated weights for btc_relay
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-09-27, STEPS: `2`, REPEAT: `1`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `Nakuls-MacBook-Pro.local`, CPU: `<UNKNOWN>`
//! EXECUTION: , WASM-EXECUTION: Compiled, CHAIN: Some("interlay-dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/interbtc-parachain
// benchmark
// pallet
// --pallet
// btc-relay
// --extrinsic
// *
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
// interlay_bench.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weights for btc_relay using the Substrate node and recommended hardware.
pub struct WeightInfo<T>(PhantomData<T>);

impl<T: frame_system::Config> btc_relay::WeightInfo for WeightInfo<T> {

	/// Storage: `BTCRelay::BestBlock` (r:1 w:1)
	/// Proof: `BTCRelay::BestBlock` (`max_values`: Some(1), `max_size`: Some(32), added: 527, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::DisableDifficultyCheck` (r:1 w:0)
	/// Proof: `BTCRelay::DisableDifficultyCheck` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::ChainCounter` (r:1 w:1)
	/// Proof: `BTCRelay::ChainCounter` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::Chains` (r:1 w:1)
	/// Proof: `BTCRelay::Chains` (`max_values`: None, `max_size`: Some(24), added: 2499, mode: `MaxEncodedLen`)
	/// Storage: `Security::ActiveBlockCount` (r:1 w:0)
	/// Proof: `Security::ActiveBlockCount` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::ChainWork` (r:1 w:0)
	/// Proof: `BTCRelay::ChainWork` (`max_values`: None, `max_size`: Some(80), added: 2555, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::ChainsHashes` (r:0 w:1)
	/// Proof: `BTCRelay::ChainsHashes` (`max_values`: None, `max_size`: Some(72), added: 2547, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::StartBlockHeight` (r:0 w:1)
	/// Proof: `BTCRelay::StartBlockHeight` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::BestBlockHeight` (r:0 w:1)
	/// Proof: `BTCRelay::BestBlockHeight` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::ChainsIndex` (r:0 w:1)
	/// Proof: `BTCRelay::ChainsIndex` (`max_values`: None, `max_size`: Some(32), added: 2507, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::BlockHeaders` (r:0 w:1)
	/// Proof: `BTCRelay::BlockHeaders` (`max_values`: None, `max_size`: Some(200), added: 2675, mode: `MaxEncodedLen`)
	fn initialize	() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `403`
		//  Estimated: `3545`
		// Minimum execution time: 44_000_000 picoseconds.
		Weight::from_parts(44_000_000, 3545)
			.saturating_add(T::DbWeight::get().reads(6_u64))
			.saturating_add(T::DbWeight::get().writes(8_u64))
	}
	/// Storage: `BTCRelay::BlockHeaders` (r:2 w:0)
	/// Proof: `BTCRelay::BlockHeaders` (`max_values`: None, `max_size`: Some(200), added: 2675, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::ChainWork` (r:1 w:1)
	/// Proof: `BTCRelay::ChainWork` (`max_values`: None, `max_size`: Some(80), added: 2555, mode: `MaxEncodedLen`)
	fn set_chainwork_for_block	() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1049`
		//  Estimated: `6340`
		// Minimum execution time: 27_000_000 picoseconds.
		Weight::from_parts(27_000_000, 6340)
			.saturating_add(T::DbWeight::get().reads(3_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	/// Storage: `BTCRelay::ChainCounter` (r:1 w:0)
	/// Proof: `BTCRelay::ChainCounter` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::BlockHeaders` (r:2 w:1)
	/// Proof: `BTCRelay::BlockHeaders` (`max_values`: None, `max_size`: Some(200), added: 2675, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::ChainsIndex` (r:1 w:1)
	/// Proof: `BTCRelay::ChainsIndex` (`max_values`: None, `max_size`: Some(32), added: 2507, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::DisableDifficultyCheck` (r:1 w:0)
	/// Proof: `BTCRelay::DisableDifficultyCheck` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::ChainsHashes` (r:1 w:1)
	/// Proof: `BTCRelay::ChainsHashes` (`max_values`: None, `max_size`: Some(72), added: 2547, mode: `MaxEncodedLen`)
	/// Storage: `Security::ActiveBlockCount` (r:1 w:0)
	/// Proof: `Security::ActiveBlockCount` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::ChainWork` (r:1 w:1)
	/// Proof: `BTCRelay::ChainWork` (`max_values`: None, `max_size`: Some(80), added: 2555, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::BestBlock` (r:0 w:1)
	/// Proof: `BTCRelay::BestBlock` (`max_values`: Some(1), `max_size`: Some(32), added: 527, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::BestBlockHeight` (r:0 w:1)
	/// Proof: `BTCRelay::BestBlockHeight` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	fn store_block_header_when_adding_chainwork	() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `979`
		//  Estimated: `6340`
		// Minimum execution time: 52_000_000 picoseconds.
		Weight::from_parts(52_000_000, 6340)
			.saturating_add(T::DbWeight::get().reads(8_u64))
			.saturating_add(T::DbWeight::get().writes(6_u64))
	}
	/// Storage: `BTCRelay::ChainCounter` (r:1 w:0)
	/// Proof: `BTCRelay::ChainCounter` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::BlockHeaders` (r:2 w:1)
	/// Proof: `BTCRelay::BlockHeaders` (`max_values`: None, `max_size`: Some(200), added: 2675, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::ChainsIndex` (r:1 w:1)
	/// Proof: `BTCRelay::ChainsIndex` (`max_values`: None, `max_size`: Some(32), added: 2507, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::DisableDifficultyCheck` (r:1 w:0)
	/// Proof: `BTCRelay::DisableDifficultyCheck` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::ChainsHashes` (r:1 w:1)
	/// Proof: `BTCRelay::ChainsHashes` (`max_values`: None, `max_size`: Some(72), added: 2547, mode: `MaxEncodedLen`)
	/// Storage: `Security::ActiveBlockCount` (r:1 w:0)
	/// Proof: `Security::ActiveBlockCount` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::ChainWork` (r:1 w:0)
	/// Proof: `BTCRelay::ChainWork` (`max_values`: None, `max_size`: Some(80), added: 2555, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::BestBlock` (r:0 w:1)
	/// Proof: `BTCRelay::BestBlock` (`max_values`: Some(1), `max_size`: Some(32), added: 527, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::BestBlockHeight` (r:0 w:1)
	/// Proof: `BTCRelay::BestBlockHeight` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	fn store_block_header	() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `874`
		//  Estimated: `6340`
		// Minimum execution time: 47_000_000 picoseconds.
		Weight::from_parts(47_000_000, 6340)
			.saturating_add(T::DbWeight::get().reads(8_u64))
			.saturating_add(T::DbWeight::get().writes(5_u64))
	}
	/// Storage: `BTCRelay::ChainCounter` (r:1 w:1)
	/// Proof: `BTCRelay::ChainCounter` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::BlockHeaders` (r:2 w:1)
	/// Proof: `BTCRelay::BlockHeaders` (`max_values`: None, `max_size`: Some(200), added: 2675, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::ChainsIndex` (r:6 w:1)
	/// Proof: `BTCRelay::ChainsIndex` (`max_values`: None, `max_size`: Some(32), added: 2507, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::DisableDifficultyCheck` (r:1 w:0)
	/// Proof: `BTCRelay::DisableDifficultyCheck` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::Chains` (r:7 w:1)
	/// Proof: `BTCRelay::Chains` (`max_values`: None, `max_size`: Some(24), added: 2499, mode: `MaxEncodedLen`)
	/// Storage: `Security::ActiveBlockCount` (r:1 w:0)
	/// Proof: `Security::ActiveBlockCount` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::ChainWork` (r:1 w:0)
	/// Proof: `BTCRelay::ChainWork` (`max_values`: None, `max_size`: Some(80), added: 2555, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::BestBlock` (r:1 w:0)
	/// Proof: `BTCRelay::BestBlock` (`max_values`: Some(1), `max_size`: Some(32), added: 527, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::ChainsHashes` (r:0 w:1)
	/// Proof: `BTCRelay::ChainsHashes` (`max_values`: None, `max_size`: Some(72), added: 2547, mode: `MaxEncodedLen`)
	/// The range of component `f` is `[1, 6]`.
	fn store_block_header_new_fork_sorted	(_f: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `796 + f * (110 ±0)`
		//  Estimated: `18483`
		// Minimum execution time: 71_000_000 picoseconds.
		Weight::from_parts(93_000_000, 18483)
			.saturating_add(T::DbWeight::get().reads(20_u64))
			.saturating_add(T::DbWeight::get().writes(5_u64))
	}
	/// Storage: `BTCRelay::ChainCounter` (r:1 w:1)
	/// Proof: `BTCRelay::ChainCounter` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::BlockHeaders` (r:2 w:1)
	/// Proof: `BTCRelay::BlockHeaders` (`max_values`: None, `max_size`: Some(200), added: 2675, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::ChainsIndex` (r:2 w:1)
	/// Proof: `BTCRelay::ChainsIndex` (`max_values`: None, `max_size`: Some(32), added: 2507, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::DisableDifficultyCheck` (r:1 w:0)
	/// Proof: `BTCRelay::DisableDifficultyCheck` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::Chains` (r:7 w:6)
	/// Proof: `BTCRelay::Chains` (`max_values`: None, `max_size`: Some(24), added: 2499, mode: `MaxEncodedLen`)
	/// Storage: `Security::ActiveBlockCount` (r:1 w:0)
	/// Proof: `Security::ActiveBlockCount` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::ChainWork` (r:1 w:0)
	/// Proof: `BTCRelay::ChainWork` (`max_values`: None, `max_size`: Some(80), added: 2555, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::BestBlock` (r:1 w:0)
	/// Proof: `BTCRelay::BestBlock` (`max_values`: Some(1), `max_size`: Some(32), added: 527, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::ChainsHashes` (r:0 w:1)
	/// Proof: `BTCRelay::ChainsHashes` (`max_values`: None, `max_size`: Some(72), added: 2547, mode: `MaxEncodedLen`)
	/// The range of component `f` is `[1, 6]`.
	fn store_block_header_new_fork_unsorted	(_f: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `832 + f * (108 ±0)`
		//  Estimated: `18483`
		// Minimum execution time: 54_000_000 picoseconds.
		Weight::from_parts(94_000_000, 18483)
			.saturating_add(T::DbWeight::get().reads(16_u64))
			.saturating_add(T::DbWeight::get().writes(10_u64))
	}
	/// Storage: `BTCRelay::ChainCounter` (r:1 w:0)
	/// Proof: `BTCRelay::ChainCounter` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::BlockHeaders` (r:20 w:18)
	/// Proof: `BTCRelay::BlockHeaders` (`max_values`: None, `max_size`: Some(200), added: 2675, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::ChainsIndex` (r:3 w:2)
	/// Proof: `BTCRelay::ChainsIndex` (`max_values`: None, `max_size`: Some(32), added: 2507, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::DisableDifficultyCheck` (r:1 w:0)
	/// Proof: `BTCRelay::DisableDifficultyCheck` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::ChainsHashes` (r:13 w:24)
	/// Proof: `BTCRelay::ChainsHashes` (`max_values`: None, `max_size`: Some(72), added: 2547, mode: `MaxEncodedLen`)
	/// Storage: `Security::ActiveBlockCount` (r:1 w:0)
	/// Proof: `Security::ActiveBlockCount` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::ChainWork` (r:3 w:0)
	/// Proof: `BTCRelay::ChainWork` (`max_values`: None, `max_size`: Some(80), added: 2555, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::Chains` (r:6 w:0)
	/// Proof: `BTCRelay::Chains` (`max_values`: None, `max_size`: Some(24), added: 2499, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::StableBitcoinConfirmations` (r:1 w:0)
	/// Proof: `BTCRelay::StableBitcoinConfirmations` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::BestBlock` (r:0 w:1)
	/// Proof: `BTCRelay::BestBlock` (`max_values`: Some(1), `max_size`: Some(32), added: 527, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::BestBlockHeight` (r:0 w:1)
	/// Proof: `BTCRelay::BestBlockHeight` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// The range of component `f` is `[3, 6]`.
	fn store_block_header_reorganize_chains	(_f: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `4754 + f * (235 ±0)`
		//  Estimated: `54490`
		// Minimum execution time: 322_000_000 picoseconds.
		Weight::from_parts(345_000_000, 54490)
			.saturating_add(T::DbWeight::get().reads(49_u64))
			.saturating_add(T::DbWeight::get().writes(46_u64))
	}
	/// Storage: `BTCRelay::ChainCounter` (r:1 w:0)
	/// Proof: `BTCRelay::ChainCounter` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::BlockHeaders` (r:20 w:18)
	/// Proof: `BTCRelay::BlockHeaders` (`max_values`: None, `max_size`: Some(200), added: 2675, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::ChainsIndex` (r:3 w:2)
	/// Proof: `BTCRelay::ChainsIndex` (`max_values`: None, `max_size`: Some(32), added: 2507, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::DisableDifficultyCheck` (r:1 w:0)
	/// Proof: `BTCRelay::DisableDifficultyCheck` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::ChainsHashes` (r:13 w:24)
	/// Proof: `BTCRelay::ChainsHashes` (`max_values`: None, `max_size`: Some(72), added: 2547, mode: `MaxEncodedLen`)
	/// Storage: `Security::ActiveBlockCount` (r:1 w:0)
	/// Proof: `Security::ActiveBlockCount` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::ChainWork` (r:2 w:1)
	/// Proof: `BTCRelay::ChainWork` (`max_values`: None, `max_size`: Some(80), added: 2555, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::Chains` (r:6 w:0)
	/// Proof: `BTCRelay::Chains` (`max_values`: None, `max_size`: Some(24), added: 2499, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::BestBlock` (r:0 w:1)
	/// Proof: `BTCRelay::BestBlock` (`max_values`: Some(1), `max_size`: Some(32), added: 527, mode: `MaxEncodedLen`)
	/// Storage: `BTCRelay::BestBlockHeight` (r:0 w:1)
	/// Proof: `BTCRelay::BestBlockHeight` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// The range of component `f` is `[3, 6]`.
	fn store_block_header_reorganize_chains_based_on_chainwork	(_f: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `5196 + f * (298 ±0)`
		//  Estimated: `54490`
		// Minimum execution time: 328_000_000 picoseconds.
		Weight::from_parts(358_000_000, 54490)
			.saturating_add(T::DbWeight::get().reads(47_u64))
			.saturating_add(T::DbWeight::get().writes(47_u64))
	}
}