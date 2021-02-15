/// Mocking the test environment
use crate::{Config, Error, GenesisConfig, Module};
use frame_support::{impl_outer_event, impl_outer_origin, parameter_types};
use mocktopus::mocking::clear_mocks;
use sp_arithmetic::{FixedI128, FixedU128};
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};

impl_outer_origin! {
    pub enum Origin for Test {}
}

mod btc_relay {
    pub use crate::Event;
}

impl_outer_event! {
    pub enum TestEvent for Test {
        frame_system<T>,
        sla<T>,
        treasury<T>,
        collateral<T>,
        pallet_balances<T>,
        btc_relay,
        exchange_rate_oracle<T>,
        vault_registry<T>,
        security,
    }
}

pub struct PalletInfo;

impl frame_support::traits::PalletInfo for PalletInfo {
    fn index<P: 'static>() -> Option<usize> {
        Some(0)
    }

    fn name<P: 'static>() -> Option<&'static str> {
        Some("btc-relay")
    }
}

pub const BITCOIN_CONFIRMATIONS: u32 = 6;
pub const PARACHAIN_CONFIRMATIONS: u64 = 20;
pub type AccountId = u64;
pub type Balance = u64;
pub type Balances = pallet_balances::Module<Test>;
pub type BlockNumber = u64;

// For testing the pallet, we construct most of a mock runtime. This means
// first constructing a configuration type (`Test`) which `impl`s each of the
// configuration traits of modules we want to use.
#[derive(Clone, Eq, PartialEq)]
pub struct Test;

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub BlockWeights: frame_system::limits::BlockWeights =
        frame_system::limits::BlockWeights::simple_max(1024);
}

impl frame_system::Config for Test {
    type BaseCallFilter = ();
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type Origin = Origin;
    type Index = u64;
    type BlockNumber = BlockNumber;
    type Call = ();
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
    type SS58Prefix = ();
}

impl Config for Test {
    type Event = TestEvent;
    type WeightInfo = ();
}

impl sla::Config for Test {
    type Event = TestEvent;
    type SignedFixedPoint = FixedI128;
}

impl treasury::Config for Test {
    type Event = TestEvent;
    type PolkaBTC = Balances;
}

impl collateral::Config for Test {
    type Event = TestEvent;
    type DOT = Balances;
}

impl vault_registry::Config for Test {
    type Event = TestEvent;
    type UnsignedFixedPoint = FixedU128;
    type RandomnessSource = pallet_randomness_collective_flip::Module<Test>;
    type WeightInfo = ();
}

impl exchange_rate_oracle::Config for Test {
    type Event = TestEvent;
    type UnsignedFixedPoint = FixedU128;
    type WeightInfo = ();
}

parameter_types! {
    pub const MinimumPeriod: u64 = 5;
}

impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

parameter_types! {
    pub const ExistentialDeposit: u64 = 1;
    pub const MaxLocks: u32 = 50;
}

impl pallet_balances::Config for Test {
    type MaxLocks = MaxLocks;
    type Balance = Balance;
    type Event = TestEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
}

impl security::Config for Test {
    type Event = TestEvent;
}

pub type TestError = Error<Test>;
pub type SecurityError = security::Error<Test>;

pub type System = frame_system::Module<Test>;
pub type BTCRelay = Module<Test>;

pub struct ExtBuilder;

impl ExtBuilder {
    pub fn build() -> sp_io::TestExternalities {
        let mut storage = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();

        GenesisConfig::<Test> {
            bitcoin_confirmations: BITCOIN_CONFIRMATIONS,
            parachain_confirmations: PARACHAIN_CONFIRMATIONS,
            disable_difficulty_check: false,
            disable_inclusion_check: false,
            disable_op_return_check: false,
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        sp_io::TestExternalities::from(storage)
    }
}

pub fn run_test<T>(test: T) -> ()
where
    T: FnOnce() -> (),
{
    clear_mocks();
    ExtBuilder::build().execute_with(|| {
        System::set_block_number(1);
        test();
    });
}
