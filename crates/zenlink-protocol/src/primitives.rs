// Copyright 2021-2022 Zenlink.
// Licensed under Apache 2.0.

use super::*;
use scale_info::TypeInfo;
use sp_std::marker::PhantomData;

pub type AssetBalance = u128;

// 0.3% exchange fee rate
pub const DEFAULT_FEE_RATE: u128 = 3;
pub const FEE_ADJUSTMENT: u128 = 1000;

/// Native currency
pub const NATIVE: u8 = 0;
/// Swap module asset
pub const LIQUIDITY: u8 = 1;
/// Other asset type on this chain
pub const LOCAL: u8 = 2;
/// Reserved for future
pub const RESERVED: u8 = 3;

/// AssetId use to locate assets in framed base chain.
#[derive(
	Encode,
	Decode,
	Eq,
	PartialEq,
	Copy,
	Clone,
	RuntimeDebug,
	PartialOrd,
	Ord,
	TypeInfo,
	MaxEncodedLen,
)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Default))]
pub struct AssetId {
	/// Parachain ID
	pub chain_id: u32,
	/// Pallet ID
	pub asset_type: u8,
	/// Index of asset within that pallet
	pub asset_index: u64,
}

pub trait AssetInfo {
	fn is_support(&self) -> bool;
}

impl AssetInfo for AssetId {
	fn is_support(&self) -> bool {
		matches!(self.asset_type, NATIVE | LIQUIDITY | LOCAL | RESERVED)
	}
}

impl AssetId {
	pub fn is_native(&self, self_chain_id: u32) -> bool {
		self.chain_id == self_chain_id && self.asset_type == NATIVE && self.asset_index == 0
	}

	pub fn is_foreign(&self, self_chain_id: u32) -> bool {
		self.chain_id != self_chain_id
	}
}

pub struct PairLpGenerate<T>(PhantomData<T>);
impl<T: Config> GenerateLpAssetId<AssetId> for PairLpGenerate<T> {
	fn generate_lp_asset_id(asset_0: AssetId, asset_1: AssetId) -> Option<AssetId> {
		let currency_0 = (asset_0.asset_index & 0x0000_0000_0000_ffff) << 16;
		let currency_1 = (asset_1.asset_index & 0x0000_0000_0000_ffff) << 32;
		let discr = 6u64 << 8;
		let index = currency_0 + currency_1 + discr;
		Some(AssetId { chain_id: T::SelfParaId::get(), asset_type: LOCAL, asset_index: index })
	}
}

impl Into<MultiLocation> for AssetId {
	fn into(self) -> MultiLocation {
		MultiLocation::new(
			1,
			Junctions::X3(
				Junction::Parachain(self.chain_id),
				Junction::PalletInstance(self.asset_type),
				Junction::GeneralIndex { 0: self.asset_index as u128 },
			),
		)
	}
}

pub struct AssetIdConverter;
impl ConvertMultiLocation<AssetId> for AssetIdConverter {
	fn chain_id(asset_id: &AssetId) -> u32 {
		asset_id.chain_id
	}

	fn make_x3_location(asset_id: &AssetId) -> MultiLocation {
		asset_id.clone().into()
	}
}

/// Status for TradingPair
#[derive(Clone, Copy, Encode, Decode, RuntimeDebug, PartialEq, Eq, MaxEncodedLen, TypeInfo)]
pub enum PairStatus<Balance, BlockNumber, Account> {
	/// Pair is Trading,
	/// can add/remove liquidity and swap.
	Trading(PairMetadata<Balance, Account>),
	/// pair is Bootstrap,
	/// can add liquidity.
	Bootstrap(BootstrapParameter<Balance, BlockNumber, Account>),
	/// nothing in pair
	Disable,
}

impl<Balance, BlockNumber, Account> Default for PairStatus<Balance, BlockNumber, Account> {
	fn default() -> Self {
		Self::Disable
	}
}

impl<BlockNumber, Account> PairStatus<AssetBalance, BlockNumber, Account> {
	pub fn fee_rate(&self) -> AssetBalance {
		match self {
			Self::Trading(pair) => pair.fee_rate,
			_ => DEFAULT_FEE_RATE,
		}
	}
}

/// Parameters of pair in Bootstrap status
#[derive(Encode, Decode, Clone, Copy, RuntimeDebug, PartialEq, Eq, MaxEncodedLen, TypeInfo)]
pub struct BootstrapParameter<Balance, BlockNumber, Account> {
	/// target supply that trading pair could to normal.
	pub target_supply: (Balance, Balance),
	/// max supply in this bootstrap pair
	pub capacity_supply: (Balance, Balance),
	/// accumulated supply in this bootstrap pair.
	pub accumulated_supply: (Balance, Balance),
	/// bootstrap pair end block number.
	pub end_block_number: BlockNumber,
	/// bootstrap pair account.
	pub pair_account: Account,
}

#[derive(Encode, Decode, Clone, Copy, RuntimeDebug, PartialEq, Eq, MaxEncodedLen, TypeInfo)]
pub struct PairMetadata<Balance, Account> {
	pub pair_account: Account,
	pub total_supply: Balance,
	pub fee_rate: Balance,
}
