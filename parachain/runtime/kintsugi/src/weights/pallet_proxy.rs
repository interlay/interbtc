
//! Autogenerated weights for pallet_proxy
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-08-07, STEPS: `50`, REPEAT: `10`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `interlay-rust-runner-2mz2v-kcxvd`, CPU: `AMD EPYC 7502P 32-Core Processor`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("kintsugi-dev"), DB CACHE: 1024

// Executed Command:
// target/release/interbtc-parachain
// benchmark
// pallet
// --pallet
// *
// --extrinsic
// *
// --chain
// kintsugi-dev
// --execution=wasm
// --wasm-execution=compiled
// --steps
// 50
// --repeat
// 10
// --output
// parachain/runtime/kintsugi/src/weights/
// --template
// .deploy/runtime-weight-template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weights for pallet_proxy using the Substrate node and recommended hardware.
pub struct WeightInfo<T>(PhantomData<T>);

impl<T: frame_system::Config> pallet_proxy::WeightInfo for WeightInfo<T> {

	/// Storage: Proxy Proxies (r:1 w:0)
	/// Proof: Proxy Proxies (max_values: None, max_size: Some(1241), added: 3716, mode: MaxEncodedLen)
	/// The range of component `p` is `[1, 31]`.
	fn proxy	(p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `251 + p * (37 ±0)`
		//  Estimated: `4706`
		// Minimum execution time: 31_042_000 picoseconds.
		Weight::from_parts(31_696_042, 4706)
			// Standard Error: 1_810
			.saturating_add(Weight::from_parts(29_815, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(1_u64))
	}
	/// Storage: Proxy Proxies (r:1 w:0)
	/// Proof: Proxy Proxies (max_values: None, max_size: Some(1241), added: 3716, mode: MaxEncodedLen)
	/// Storage: Proxy Announcements (r:1 w:1)
	/// Proof: Proxy Announcements (max_values: None, max_size: Some(2233), added: 4708, mode: MaxEncodedLen)
	/// Storage: Tokens Accounts (r:1 w:1)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(115), added: 2590, mode: MaxEncodedLen)
	/// The range of component `a` is `[0, 31]`.
	/// The range of component `p` is `[1, 31]`.
	fn proxy_announced	(a: u32, p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1011 + a * (68 ±0) + p * (37 ±0)`
		//  Estimated: `5698`
		// Minimum execution time: 68_858_000 picoseconds.
		Weight::from_parts(71_304_525, 5698)
			// Standard Error: 4_860
			.saturating_add(Weight::from_parts(292_426, 0).saturating_mul(a.into()))
			// Standard Error: 5_028
			.saturating_add(Weight::from_parts(20_095, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(3_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
	/// Storage: Proxy Announcements (r:1 w:1)
	/// Proof: Proxy Announcements (max_values: None, max_size: Some(2233), added: 4708, mode: MaxEncodedLen)
	/// Storage: Tokens Accounts (r:1 w:1)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(115), added: 2590, mode: MaxEncodedLen)
	/// The range of component `a` is `[0, 31]`.
	/// The range of component `p` is `[1, 31]`.
	fn remove_announcement	(a: u32, _p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `926 + a * (68 ±0)`
		//  Estimated: `5698`
		// Minimum execution time: 45_691_000 picoseconds.
		Weight::from_parts(46_976_313, 5698)
			// Standard Error: 37_400
			.saturating_add(Weight::from_parts(429_525, 0).saturating_mul(a.into()))
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
	/// Storage: Proxy Announcements (r:1 w:1)
	/// Proof: Proxy Announcements (max_values: None, max_size: Some(2233), added: 4708, mode: MaxEncodedLen)
	/// Storage: Tokens Accounts (r:1 w:1)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(115), added: 2590, mode: MaxEncodedLen)
	/// The range of component `a` is `[0, 31]`.
	/// The range of component `p` is `[1, 31]`.
	fn reject_announcement	(a: u32, p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `926 + a * (68 ±0)`
		//  Estimated: `5698`
		// Minimum execution time: 45_441_000 picoseconds.
		Weight::from_parts(47_659_912, 5698)
			// Standard Error: 3_967
			.saturating_add(Weight::from_parts(304_018, 0).saturating_mul(a.into()))
			// Standard Error: 4_105
			.saturating_add(Weight::from_parts(5_528, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
	/// Storage: Proxy Proxies (r:1 w:0)
	/// Proof: Proxy Proxies (max_values: None, max_size: Some(1241), added: 3716, mode: MaxEncodedLen)
	/// Storage: Proxy Announcements (r:1 w:1)
	/// Proof: Proxy Announcements (max_values: None, max_size: Some(2233), added: 4708, mode: MaxEncodedLen)
	/// Storage: Tokens Accounts (r:1 w:1)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(115), added: 2590, mode: MaxEncodedLen)
	/// The range of component `a` is `[0, 31]`.
	/// The range of component `p` is `[1, 31]`.
	fn announce	(a: u32, p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `943 + a * (68 ±0) + p * (37 ±0)`
		//  Estimated: `5698`
		// Minimum execution time: 69_500_000 picoseconds.
		Weight::from_parts(71_450_204, 5698)
			// Standard Error: 3_637
			.saturating_add(Weight::from_parts(294_087, 0).saturating_mul(a.into()))
			// Standard Error: 3_763
			.saturating_add(Weight::from_parts(33_253, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(3_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
	/// Storage: Proxy Proxies (r:1 w:1)
	/// Proof: Proxy Proxies (max_values: None, max_size: Some(1241), added: 3716, mode: MaxEncodedLen)
	/// Storage: Tokens Accounts (r:1 w:1)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(115), added: 2590, mode: MaxEncodedLen)
	/// The range of component `p` is `[1, 31]`.
	fn add_proxy	(p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `787 + p * (37 ±0)`
		//  Estimated: `4706`
		// Minimum execution time: 60_491_000 picoseconds.
		Weight::from_parts(61_796_024, 4706)
			// Standard Error: 5_264
			.saturating_add(Weight::from_parts(83_677, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
	/// Storage: Proxy Proxies (r:1 w:1)
	/// Proof: Proxy Proxies (max_values: None, max_size: Some(1241), added: 3716, mode: MaxEncodedLen)
	/// Storage: Tokens Accounts (r:1 w:1)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(115), added: 2590, mode: MaxEncodedLen)
	/// The range of component `p` is `[1, 31]`.
	fn remove_proxy	(p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `787 + p * (37 ±0)`
		//  Estimated: `4706`
		// Minimum execution time: 52_626_000 picoseconds.
		Weight::from_parts(54_526_510, 4706)
			// Standard Error: 4_342
			.saturating_add(Weight::from_parts(94_311, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
	/// Storage: Proxy Proxies (r:1 w:1)
	/// Proof: Proxy Proxies (max_values: None, max_size: Some(1241), added: 3716, mode: MaxEncodedLen)
	/// Storage: Tokens Accounts (r:1 w:1)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(115), added: 2590, mode: MaxEncodedLen)
	/// The range of component `p` is `[1, 31]`.
	fn remove_proxies	(p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `787 + p * (37 ±0)`
		//  Estimated: `4706`
		// Minimum execution time: 43_587_000 picoseconds.
		Weight::from_parts(44_521_020, 4706)
			// Standard Error: 2_924
			.saturating_add(Weight::from_parts(22_119, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
	/// Storage: Proxy Proxies (r:1 w:1)
	/// Proof: Proxy Proxies (max_values: None, max_size: Some(1241), added: 3716, mode: MaxEncodedLen)
	/// Storage: Tokens Accounts (r:1 w:1)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(115), added: 2590, mode: MaxEncodedLen)
	/// The range of component `p` is `[1, 31]`.
	fn create_pure	(p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `799`
		//  Estimated: `4706`
		// Minimum execution time: 64_509_000 picoseconds.
		Weight::from_parts(65_543_540, 4706)
			// Standard Error: 77_269
			.saturating_add(Weight::from_parts(318_410, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
	/// Storage: Proxy Proxies (r:1 w:1)
	/// Proof: Proxy Proxies (max_values: None, max_size: Some(1241), added: 3716, mode: MaxEncodedLen)
	/// Storage: Tokens Accounts (r:1 w:1)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(115), added: 2590, mode: MaxEncodedLen)
	/// The range of component `p` is `[0, 30]`.
	fn kill_pure	(_p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `857 + p * (37 ±0)`
		//  Estimated: `4706`
		// Minimum execution time: 45_491_000 picoseconds.
		Weight::from_parts(49_718_434, 4706)
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
}