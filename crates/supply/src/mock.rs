use crate::{self as supply, Config};
use frame_support::{
    parameter_types,
    traits::{Everything, GenesisBuild},
    PalletId,
};
pub use primitives::CurrencyId;
use primitives::UnsignedFixedPoint;
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    FixedPointNumber,
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Storage, Config, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Event<T>},
        Supply: supply::{Pallet, Call, Storage, Config<T>, Event<T>},
    }
);

pub type AccountId = u64;
pub type Balance = u128;
pub type BlockNumber = u64;
pub type Index = u64;

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

impl frame_system::Config for Test {
    type BaseCallFilter = Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type Origin = Origin;
    type Call = Call;
    type Index = Index;
    type BlockNumber = BlockNumber;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = TestEvent;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
}

parameter_types! {
    pub const ExistentialDeposit: u64 = 1;
}

impl pallet_balances::Config for Test {
    type Balance = Balance;
    type DustRemoval = ();
    type Event = Event;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = ();
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
}

pub const MILLISECS_PER_BLOCK: u64 = 12000;

pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;
pub const YEARS: BlockNumber = DAYS * 365;

parameter_types! {
    pub const SupplyPalletId: PalletId = PalletId(*b"mod/annu");
    pub const InflationPeriod: BlockNumber = YEARS;
}

impl Config for Test {
    type SupplyPalletId = SupplyPalletId;
    type Event = TestEvent;
    type UnsignedFixedPoint = UnsignedFixedPoint;
    type Currency = Balances;
    type InflationPeriod = InflationPeriod;
    type OnInflation = ();
}

pub type TestEvent = Event;
// pub type TestError = Error<Test>;

// pub const ALICE: AccountId = 1;
// pub const BOB: AccountId = 2;

pub struct ExtBuilder;

impl ExtBuilder {
    pub fn build() -> sp_io::TestExternalities {
        let mut storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

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
