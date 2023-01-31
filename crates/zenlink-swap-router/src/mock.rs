// Copyright 2021-2022 Zenlink.
// Licensed under Apache 2.0.

#[cfg(feature = "std")]
use std::marker::PhantomData;

use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};

use frame_support::{
	dispatch::{DispatchError, DispatchResult},
	pallet_prelude::GenesisBuild,
	parameter_types,
	traits::Contains,
	PalletId,
};
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup, Zero},
	RuntimeDebug,
};

use crate as router;
use crate::{Config, Pallet};
use orml_traits::{parameter_type_with_key, MultiCurrency};
use zenlink_protocol::{
	AssetBalance, AssetId, AssetIdConverter, LocalAssetHandler, PairLpGenerate, ZenlinkMultiAssets,
	LOCAL,
};
use zenlink_stable_amm::traits::{StablePoolLpCurrencyIdGenerate, ValidateCurrency};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

parameter_types! {
	pub const ExistentialDeposit: u64 = 1;

	pub const BlockHashCount: u64 = 250;
	pub const StableAmmPalletId: PalletId = PalletId(*b"/zlkSAmm");
	pub const ZenlinkPalletId: PalletId = PalletId(*b"/zenlink");
	pub const MaxReserves: u32 = 50;
	pub const MaxLocks:u32 = 50;
	pub const MinimumPeriod: Moment = SLOT_DURATION / 2;
	pub const PoolCurrencySymbolLimit: u32 = 50;
	pub SelfParaId: u32 = CHAIN_ID;
}

parameter_type_with_key! {
	pub ExistentialDeposits: |_currency_id: CurrencyId| -> u128 {
		0
	};
}

pub type AccountId = u128;
pub type TokenSymbol = u8;
pub type PoolId = u32;

pub struct MockDustRemovalWhitelist;
impl Contains<AccountId> for MockDustRemovalWhitelist {
	fn contains(_a: &AccountId) -> bool {
		true
	}
}

#[derive(
	Encode,
	Decode,
	Eq,
	PartialEq,
	Copy,
	Clone,
	RuntimeDebug,
	PartialOrd,
	MaxEncodedLen,
	Ord,
	TypeInfo,
)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum CurrencyId {
	Forbidden(TokenSymbol),
	Token(TokenSymbol),
	StableLP(PoolType),
	StableLPV2(PoolId),
	ZenlinkLp(TokenSymbol, TokenSymbol),
}

#[derive(
	Encode,
	Decode,
	Eq,
	PartialEq,
	Copy,
	Clone,
	RuntimeDebug,
	PartialOrd,
	MaxEncodedLen,
	Ord,
	TypeInfo,
)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum PoolToken {
	Token(TokenSymbol),
	StablePoolLp(PoolId),
}

#[derive(
	Encode,
	Decode,
	Eq,
	PartialEq,
	Copy,
	Clone,
	RuntimeDebug,
	PartialOrd,
	MaxEncodedLen,
	Ord,
	TypeInfo,
)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum PoolType {
	P2(PoolToken, PoolToken),
	P3(PoolToken, PoolToken, PoolToken),
	P4(PoolToken, PoolToken, PoolToken, PoolToken),
	P5(PoolToken, PoolToken, PoolToken, PoolToken, PoolToken),
	P6(PoolToken, PoolToken, PoolToken, PoolToken, PoolToken, PoolToken),
}

impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type RuntimeOrigin = RuntimeOrigin;
	type Index = u64;
	type RuntimeCall = RuntimeCall;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u128;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = BlockHashCount;
	type DbWeight = ();
	type Version = ();
	type AccountData = pallet_balances::AccountData<u128>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type PalletInfo = PalletInfo;
	type BlockWeights = ();
	type BlockLength = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl orml_tokens::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type Amount = i128;
	type CurrencyId = CurrencyId;
	type WeightInfo = ();
	type ExistentialDeposits = ExistentialDeposits;
	type MaxLocks = ();
	type DustRemovalWhitelist = MockDustRemovalWhitelist;
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
	type CurrencyHooks = ();
}

impl pallet_balances::Config for Test {
	type Balance = u128;
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = frame_system::Pallet<Test>;
	type WeightInfo = ();
	type MaxLocks = ();
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
}

pub type Moment = u64;
pub const MILLISECS_PER_BLOCK: Moment = 12000;
pub const SLOT_DURATION: Moment = MILLISECS_PER_BLOCK;
pub const CHAIN_ID: u32 = 200u32;
pub type Balance = u128;

impl pallet_timestamp::Config for Test {
	type MinimumPeriod = MinimumPeriod;
	type Moment = u64;
	type OnTimestampSet = ();
	type WeightInfo = ();
}

impl zenlink_stable_amm::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type CurrencyId = CurrencyId;
	type MultiCurrency = Tokens;
	type PoolId = PoolId;
	type EnsurePoolAsset = EnsurePoolAssetImpl<Tokens>;
	type LpGenerate = PoolLpGenerate;
	type TimeProvider = Timestamp;
	type PoolCurrencySymbolLimit = PoolCurrencySymbolLimit;
	type PalletId = StableAmmPalletId;
	type WeightInfo = ();
}

impl zenlink_protocol::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type MultiAssetsHandler = ZenlinkMultiAssets<Zenlink, Balances, LocalAssetAdaptor<Tokens>>;
	type PalletId = ZenlinkPalletId;
	type AssetId = AssetId;
	type LpGenerate = PairLpGenerate<Self>;
	type TargetChains = ();
	type SelfParaId = SelfParaId;
	type XcmExecutor = ();
	type AccountIdConverter = ();
	type AssetIdConverter = AssetIdConverter;
	type WeightInfo = ();
}

impl Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type StablePoolId = PoolId;
	type Balance = Balance;
	type StableCurrencyId = CurrencyId;
	type NormalCurrencyId = AssetId;
	type NormalAmm = Zenlink;
	type StableAMM = StableAMM;
	type WeightInfo = ();
}

pub struct EnsurePoolAssetImpl<Local>(PhantomData<Local>);

pub struct PoolLpGenerate;

impl StablePoolLpCurrencyIdGenerate<CurrencyId, PoolId> for PoolLpGenerate {
	fn generate_by_pool_id(pool_id: PoolId) -> CurrencyId {
		return CurrencyId::StableLPV2(pool_id)
	}
}

impl<Local> ValidateCurrency<CurrencyId> for EnsurePoolAssetImpl<Local>
where
	Local: MultiCurrency<AccountId, Balance = u128, CurrencyId = CurrencyId>,
{
	fn validate_pooled_currency(currencies: &[CurrencyId]) -> bool {
		for currency in currencies.iter() {
			if let CurrencyId::Forbidden(_) = *currency {
				return false
			}
		}
		true
	}

	fn validate_pool_lp_currency(currency_id: CurrencyId) -> bool {
		if let CurrencyId::Token(_) = currency_id {
			return false
		}

		if Local::total_issuance(currency_id) > Zero::zero() {
			return false
		}
		true
	}
}

pub fn asset_id_to_currency_id(asset_id: &AssetId) -> Result<CurrencyId, ()> {
	let discr = (asset_id.asset_index & 0x0000_0000_0000_ff00) >> 8;
	return if discr == 6 {
		let token0_id = ((asset_id.asset_index & 0x0000_0000_ffff_0000) >> 16) as u8;
		let token1_id = ((asset_id.asset_index & 0x0000_ffff_0000_0000) >> 16) as u8;
		Ok(CurrencyId::ZenlinkLp(token0_id, token1_id))
	} else {
		let token_id = asset_id.asset_index as u8;

		Ok(CurrencyId::Token(token_id))
	}
}

pub struct LocalAssetAdaptor<Local>(PhantomData<Local>);

impl<Local> LocalAssetHandler<AccountId> for LocalAssetAdaptor<Local>
where
	Local: MultiCurrency<AccountId, Balance = u128, CurrencyId = CurrencyId>,
{
	fn local_balance_of(asset_id: AssetId, who: &AccountId) -> AssetBalance {
		asset_id_to_currency_id(&asset_id)
			.map_or(AssetBalance::default(), |currency_id| Local::free_balance(currency_id, who))
	}

	fn local_total_supply(asset_id: AssetId) -> AssetBalance {
		asset_id_to_currency_id(&asset_id)
			.map_or(AssetBalance::default(), |currency_id| Local::total_issuance(currency_id))
	}

	fn local_is_exists(asset_id: AssetId) -> bool {
		asset_id_to_currency_id(&asset_id).map_or(false, |currency_id| {
			Local::total_issuance(currency_id) > AssetBalance::default()
		})
	}

	fn local_transfer(
		asset_id: AssetId,
		origin: &AccountId,
		target: &AccountId,
		amount: AssetBalance,
	) -> DispatchResult {
		asset_id_to_currency_id(&asset_id).map_or(Err(DispatchError::CannotLookup), |currency_id| {
			Local::transfer(currency_id, origin, target, amount)
		})
	}

	fn local_deposit(
		asset_id: AssetId,
		origin: &AccountId,
		amount: AssetBalance,
	) -> Result<AssetBalance, DispatchError> {
		asset_id_to_currency_id(&asset_id).map_or(Ok(AssetBalance::default()), |currency_id| {
			Local::deposit(currency_id, origin, amount).map(|_| amount)
		})
	}

	fn local_withdraw(
		asset_id: AssetId,
		origin: &AccountId,
		amount: AssetBalance,
	) -> Result<AssetBalance, DispatchError> {
		asset_id_to_currency_id(&asset_id).map_or(Ok(AssetBalance::default()), |currency_id| {
			Local::withdraw(currency_id, origin, amount).map(|_| amount)
		})
	}
}

frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>} = 0,
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent} = 1,

		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>} = 8,
		StableAMM: zenlink_stable_amm::{Pallet, Call, Storage, Event<T>} = 9,
		Tokens: orml_tokens::{Pallet, Storage, Event<T>, Config<T>} = 11,
		Zenlink: zenlink_protocol::{Pallet, Call, Storage, Event<T>} = 12,
		Router: router::{Pallet, Call, Event<T>} = 13,
	}
);

pub type RouterPallet = Pallet<Test>;
pub const USER1: u128 = 1;
pub const USER2: u128 = 2;
pub const USER3: u128 = 3;

pub const TOKEN1_SYMBOL: u8 = 1;
pub const TOKEN2_SYMBOL: u8 = 2;
pub const TOKEN3_SYMBOL: u8 = 3;
pub const TOKEN4_SYMBOL: u8 = 4;

pub const TOKEN1_DECIMAL: u32 = 18;
pub const TOKEN2_DECIMAL: u32 = 18;
pub const TOKEN3_DECIMAL: u32 = 6;
pub const TOKEN4_DECIMAL: u32 = 6;

pub const TOKEN1_UNIT: u128 = 1_000_000_000_000_000_000;
pub const TOKEN2_UNIT: u128 = 1_000_000_000_000_000_000;
pub const TOKEN3_UNIT: u128 = 1_000_000;
pub const TOKEN4_UNIT: u128 = 1_000_000;

pub const TOKEN1_ASSET_ID: AssetId =
	AssetId { chain_id: CHAIN_ID, asset_type: LOCAL, asset_index: 1 };

pub const TOKEN2_ASSET_ID: AssetId =
	AssetId { chain_id: CHAIN_ID, asset_type: LOCAL, asset_index: 2 };

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap().into();
	pallet_balances::GenesisConfig::<Test> { balances: vec![(USER1, u128::MAX)] }
		.assimilate_storage(&mut t)
		.unwrap();

	orml_tokens::GenesisConfig::<Test> {
		balances: vec![
			(USER1, CurrencyId::Token(TOKEN1_SYMBOL), TOKEN1_UNIT * 1_00_000_000),
			(USER1, CurrencyId::Token(TOKEN2_SYMBOL), TOKEN2_UNIT * 1_00_000_000),
			(USER1, CurrencyId::Token(TOKEN3_SYMBOL), TOKEN3_UNIT * 1_00_000_000),
			(USER1, CurrencyId::Token(TOKEN4_SYMBOL), TOKEN4_UNIT * 1_00_000_000),
			(USER2, CurrencyId::Token(TOKEN1_SYMBOL), TOKEN1_UNIT * 1_00),
			(USER2, CurrencyId::Token(TOKEN2_SYMBOL), TOKEN2_UNIT * 1_00),
			(USER2, CurrencyId::Token(TOKEN3_SYMBOL), TOKEN3_UNIT * 1_00),
			(USER2, CurrencyId::Token(TOKEN4_SYMBOL), TOKEN4_UNIT * 1_00),
			(USER3, CurrencyId::Token(TOKEN1_SYMBOL), TOKEN1_UNIT * 1_00_000_000),
			(USER3, CurrencyId::Token(TOKEN2_SYMBOL), TOKEN2_UNIT * 1_00_000_000),
			(USER3, CurrencyId::Token(TOKEN3_SYMBOL), TOKEN3_UNIT * 1_00_000_000),
			(USER3, CurrencyId::Token(TOKEN4_SYMBOL), TOKEN4_UNIT * 1_00_000_000),
		],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	t.into()
}
