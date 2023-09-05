use crate::{self as supply, Config, OnInflation};
use frame_support::{parameter_types, traits::Everything, PalletId};
pub use primitives::{CurrencyId, UnsignedFixedPoint};
use sp_core::H256;
pub use sp_runtime::FixedPointNumber;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
    pub enum Test
    {
        System: frame_system::{Pallet, Call, Storage, Config<T>, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Event<T>},
        Supply: supply::{Pallet, Call, Storage, Config<T>, Event<T>},
    }
);

pub type AccountId = u64;
pub type Balance = u128;
pub type BlockNumber = u64;

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

impl frame_system::Config for Test {
    type BaseCallFilter = Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Nonce = u64;
    type Block = Block;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
    pub const ExistentialDeposit: u64 = 1;
}

impl pallet_balances::Config for Test {
    type Balance = Balance;
    type DustRemoval = ();
    type RuntimeEvent = RuntimeEvent;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = ();
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type RuntimeHoldReason = ();
    type FreezeIdentifier = ();
    type MaxFreezes = ();
    type MaxHolds = ();
}

pub const MILLISECS_PER_BLOCK: u64 = 12000;

pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;
pub const YEARS: BlockNumber = DAYS * 365;

parameter_types! {
    pub const SupplyPalletId: PalletId = PalletId(*b"mod/supl");
    pub const InflationPeriod: BlockNumber = YEARS;
}

pub struct MockOnInflation;

impl OnInflation<AccountId> for MockOnInflation {
    type Currency = Balances;
    fn on_inflation(_: &AccountId, _: Balance) {}
}

impl Config for Test {
    type SupplyPalletId = SupplyPalletId;
    type RuntimeEvent = RuntimeEvent;
    type UnsignedFixedPoint = UnsignedFixedPoint;
    type Currency = Balances;
    type InflationPeriod = InflationPeriod;
    type OnInflation = MockOnInflation;
    type WeightInfo = ();
}

pub struct ExtBuilder;

impl ExtBuilder {
    pub fn build() -> sp_io::TestExternalities {
        let mut storage = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

        supply::GenesisConfig::<Test> {
            initial_supply: 10_000_000,
            start_height: 100,
            inflation: UnsignedFixedPoint::checked_from_rational(2, 100).unwrap(), // 2%
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        storage.into()
    }
}

pub fn run_test<T>(test: T)
where
    T: FnOnce(),
{
    ExtBuilder::build().execute_with(|| {
        System::set_block_number(1);
        test();
    });
}
