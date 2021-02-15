/// Mocking the test environment
use crate::{Config, Error, Module};
use frame_support::{assert_ok, impl_outer_event, impl_outer_origin, parameter_types};
use mocktopus::mocking::clear_mocks;
use pallet_balances as balances;
use sp_core::H256;
use sp_runtime::FixedPointNumber;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};
use sp_runtime::{FixedI128, FixedU128};

pub type Balances = pallet_balances::Module<Test>;

pub type Refund = Module<Test>;
pub const VAULT: AccountId = 1;
pub const USER: AccountId = 2;

impl_outer_origin! {
    pub enum Origin for Test {}
}

mod refund {
    pub use crate::Event;
}

impl_outer_event! {
    pub enum TestEvent for Test {
        frame_system<T>,
        refund<T>,
        balances<T>,
        collateral<T>,
        btc_relay,
        treasury<T>,
        fee<T>,
        vault_registry<T>,
        exchange_rate_oracle<T>,
        sla<T>,
        security,
    }
}

pub struct PalletInfo;

impl frame_support::traits::PalletInfo for PalletInfo {
    fn index<P: 'static>() -> Option<usize> {
        Some(0)
    }

    fn name<P: 'static>() -> Option<&'static str> {
        Some("refund")
    }
}

// For testing the pallet, we construct most of a mock runtime. This means
// first constructing a configuration type (`Test`) which `impl`s each of the
// configuration traits of pallets we want to use.

pub type AccountId = u64;
#[allow(dead_code)]
pub type Balance = u64;
pub type BlockNumber = u64;

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

impl treasury::Config for Test {
    type Event = TestEvent;
    type PolkaBTC = Balances;
}

impl fee::Config for Test {
    type Event = TestEvent;
    type UnsignedFixedPoint = FixedU128;
    type WeightInfo = ();
}

impl sla::Config for Test {
    type Event = TestEvent;
    type SignedFixedPoint = FixedI128;
}

impl collateral::Config for Test {
    type Event = TestEvent;
    type DOT = Balances;
}

impl btc_relay::Config for Test {
    type Event = TestEvent;
    type WeightInfo = ();
}

impl security::Config for Test {
    type Event = TestEvent;
}

impl vault_registry::Config for Test {
    type Event = TestEvent;
    type RandomnessSource = pallet_randomness_collective_flip::Module<Test>;
    type UnsignedFixedPoint = FixedU128;
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

#[allow(dead_code)]
pub type TestError = Error<Test>;

#[allow(dead_code)]
pub type System = frame_system::Module<Test>;

#[allow(dead_code)]
pub type SLA = Module<Test>;

pub struct ExtBuilder;

impl ExtBuilder {
    pub fn build() -> sp_io::TestExternalities {
        let storage = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
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
        assert_ok!(<exchange_rate_oracle::Module<Test>>::_set_exchange_rate(
            FixedU128::one()
        ));
        System::set_block_number(1);
        test();
    });
}
