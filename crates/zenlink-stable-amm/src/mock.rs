// Copyright 2021-2022 Zenlink.
// Licensed under Apache 2.0.

#[cfg(feature = "std")]
use std::marker::PhantomData;

use super::*;
use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

use frame_support::{
	assert_ok, pallet_prelude::GenesisBuild, parameter_types, traits::Contains, PalletId,
};
use frame_system::RawOrigin;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup, Zero},
	RuntimeDebug,
};

use crate as stable_amm;
use crate::{
	traits::{StablePoolLpCurrencyIdGenerate, ValidateCurrency},
	Config, Pallet,
};
use orml_traits::{parameter_type_with_key, MultiCurrency};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

parameter_types! {
	pub const ExistentialDeposit: u64 = 1;

	pub const BlockHashCount: u64 = 250;
	pub const StableAmmPalletId: PalletId = PalletId(*b"/zlkSAmm");
	pub const MaxReserves: u32 = 50;
	pub const MaxLocks:u32 = 50;
	pub const MinimumPeriod: Moment = SLOT_DURATION / 2;
	pub const PoolCurrencySymbolLimit: u32 = 50;
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

impl pallet_timestamp::Config for Test {
	type MinimumPeriod = MinimumPeriod;
	type Moment = u64;
	type OnTimestampSet = ();
	type WeightInfo = ();
}

impl Config for Test {
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

frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>} = 0,
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent} = 1,

		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>} = 8,
		StableAMM: stable_amm::{Pallet, Call, Storage, Event<T>} = 9,
		Tokens: orml_tokens::{Pallet, Storage, Event<T>, Config<T>} = 11,
	}
);

pub type StableAmm = Pallet<Test>;
pub const ALICE: u128 = 1;
pub const BOB: u128 = 2;
pub const CHARLIE: u128 = 3;

pub const TOKEN1_SYMBOL: u8 = 1;
pub const TOKEN2_SYMBOL: u8 = 2;
pub const TOKEN3_SYMBOL: u8 = 3;
pub const TOKEN4_SYMBOL: u8 = 4;

pub const STABLE_LP_DECIMAL: u32 = 18;
pub const TOKEN1_DECIMAL: u32 = 18;
pub const TOKEN2_DECIMAL: u32 = 18;
pub const TOKEN3_DECIMAL: u32 = 6;
pub const TOKEN4_DECIMAL: u32 = 6;

pub const TOKEN1_UNIT: u128 = 1_000_000_000_000_000_000;
pub const TOKEN2_UNIT: u128 = 1_000_000_000_000_000_000;
pub const TOKEN3_UNIT: u128 = 1_000_000;
pub const TOKEN4_UNIT: u128 = 1_000_000;

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap().into();
	pallet_balances::GenesisConfig::<Test> { balances: vec![(ALICE, u128::MAX)] }
		.assimilate_storage(&mut t)
		.unwrap();

	orml_tokens::GenesisConfig::<Test> {
		balances: vec![
			(ALICE, CurrencyId::Token(TOKEN1_SYMBOL), TOKEN1_UNIT * 1_00_000_000),
			(ALICE, CurrencyId::Token(TOKEN2_SYMBOL), TOKEN2_UNIT * 1_00_000_000),
			(ALICE, CurrencyId::Token(TOKEN3_SYMBOL), TOKEN3_UNIT * 1_00_000_000),
			(ALICE, CurrencyId::Token(TOKEN4_SYMBOL), TOKEN4_UNIT * 1_00_000_000),
			(BOB, CurrencyId::Token(TOKEN1_SYMBOL), TOKEN1_UNIT * 1_00),
			(BOB, CurrencyId::Token(TOKEN2_SYMBOL), TOKEN2_UNIT * 1_00),
			(BOB, CurrencyId::Token(TOKEN3_SYMBOL), TOKEN3_UNIT * 1_00),
			(BOB, CurrencyId::Token(TOKEN4_SYMBOL), TOKEN4_UNIT * 1_00),
			(CHARLIE, CurrencyId::Token(TOKEN1_SYMBOL), TOKEN1_UNIT * 1_00_000_000),
			(CHARLIE, CurrencyId::Token(TOKEN2_SYMBOL), TOKEN2_UNIT * 1_00_000_000),
			(CHARLIE, CurrencyId::Token(TOKEN3_SYMBOL), TOKEN3_UNIT * 1_00_000_000),
			(CHARLIE, CurrencyId::Token(TOKEN4_SYMBOL), TOKEN4_UNIT * 1_00_000_000),
		],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	t.into()
}

pub const DAYS: u64 = 86400;
pub const POOL0ACCOUNTID: AccountId = 33865947477506447919519395693;
pub const POOL1ACCOUNTID: AccountId = 113094109991770785513063346029;

pub fn mine_block() {
	let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();

	System::set_block_number(System::block_number() + 1);
	set_block_timestamp(now);
}

pub fn mine_block_with_timestamp(timestamp: u64) {
	System::set_block_number(System::block_number() + 1);
	set_block_timestamp(timestamp);
}

// timestamp in second
pub fn set_block_timestamp(timestamp: u64) {
	Timestamp::set_timestamp(timestamp * 1000);
}

pub fn get_user_token_balances(currencies: &[CurrencyId], user: &AccountId) -> Vec<Balance> {
	let mut res = Vec::new();
	for currency_id in currencies.iter() {
		res.push(<Test as Config>::MultiCurrency::free_balance(*currency_id, user));
	}
	res
}

pub fn get_user_balance(currency_id: CurrencyId, user: &AccountId) -> Balance {
	<Test as Config>::MultiCurrency::free_balance(currency_id, user)
}

pub type MockPool = Pool<PoolId, CurrencyId, AccountId, BoundedVec<u8, PoolCurrencySymbolLimit>>;

impl MockPool {
	pub fn get_pool_info(
		&self,
	) -> BasePool<CurrencyId, AccountId, BoundedVec<u8, PoolCurrencySymbolLimit>> {
		match self {
			MockPool::Base(bp) => (*bp).clone(),
			MockPool::Meta(mp) => mp.info.clone(),
		}
	}
}

pub fn mint_more_currencies(
	accounts: Vec<AccountId>,
	currencies: Vec<CurrencyId>,
	balances: Vec<Balance>,
) {
	assert_eq!(currencies.len(), balances.len());
	for account in accounts.iter() {
		for (i, currency_id) in currencies.iter().enumerate() {
			assert_ok!(Tokens::set_balance(
				RawOrigin::Root.into(),
				*account,
				*currency_id,
				balances[i],
				0,
			));
		}
	}
}
