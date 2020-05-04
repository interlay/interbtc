/// Mocking the test environment
use crate::{Error, GenesisConfig, Module, Trait};
use frame_support::{impl_outer_event, impl_outer_origin, parameter_types, weights::Weight};
use sp_core::H256;
use sp_io;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    Perbill,
};

use mocktopus::mocking::clear_mocks;

impl_outer_origin! {
    pub enum Origin for Test {}
}

mod test_events {
    pub use crate::Event;
}

impl_outer_event! {
    pub enum TestEvent for Test {
        system<T>,
        test_events<T>,
        balances<T>,
        collateral<T>,
        vault_registry<T>,
        treasury<T>,
        exchange_rate_oracle<T>,
        btc_relay,
    }
}

// For testing the pallet, we construct most of a mock runtime. This means
// first constructing a configuration type (`Test`) which `impl`s each of the
// configuration traits of pallets we want to use.

pub type AccountId = u64;
pub type Balance = u64;
pub type BlockNumber = u64;

#[derive(Clone, Eq, PartialEq)]
pub struct Test;

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaximumBlockWeight: Weight = 1024;
    pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
}
impl system::Trait for Test {
    type Origin = Origin;
    type Call = ();
    type Index = u64;
    type BlockNumber = BlockNumber;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = TestEvent;
    type BlockHashCount = BlockHashCount;
    type MaximumBlockWeight = MaximumBlockWeight;
    type MaximumBlockLength = MaximumBlockLength;
    type AvailableBlockRatio = AvailableBlockRatio;
    type Version = ();
    type ModuleToIndex = ();
    type AccountData = balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
}

parameter_types! {
    pub const ExistentialDeposit: u64 = 1;
}
impl balances::Trait for Test {
    type Balance = Balance;
    type Event = TestEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
}

parameter_types! {
    pub const MinimumPeriod: u64 = 5;
}
impl timestamp::Trait for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
}

impl security::Trait for Test {}

impl vault_registry::Trait for Test {
    type Event = TestEvent;
}

impl treasury::Trait for Test {
    type PolkaBTC = Balances;
    type Event = TestEvent;
}

impl exchange_rate_oracle::Trait for Test {
    type Event = TestEvent;
}

impl collateral::Trait for Test {
    type Event = TestEvent;
    type DOT = Balances;
}

impl btc_relay::Trait for Test {
    type Event = TestEvent;
}

parameter_types! {
    pub const MaturityPeriod: u64 = 10;
    pub const MinimumDeposit: u64 = 10;
    pub const MinimumStake: u64 = 10;
    pub const VoteThreshold: u64 = 50;
}
impl Trait for Test {
    type Event = TestEvent;
    type MaturityPeriod = MaturityPeriod;
    type MinimumDeposit = MinimumDeposit;
    type MinimumStake = MinimumStake;
    type VoteThreshold = VoteThreshold;
}

pub type System = system::Module<Test>;
pub type Balances = balances::Module<Test>;
pub type Staking = Module<Test>;

pub type TestError = Error<Test>;

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
pub const CAROL: AccountId = 3;

pub const ALICE_BALANCE: u64 = 1_000_000;
pub const BOB_BALANCE: u64 = 1_000_000;
pub const CAROL_BALANCE: u64 = 1_000_000;

pub struct ExtBuilder;

impl ExtBuilder {
    pub fn build() -> sp_io::TestExternalities {
        let mut storage = system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();

        balances::GenesisConfig::<Test> {
            balances: vec![
                (ALICE, ALICE_BALANCE),
                (BOB, BOB_BALANCE),
                (CAROL, CAROL_BALANCE),
            ],
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        GenesisConfig::<Test> { gov_id: CAROL }
            .assimilate_storage(&mut storage)
            .unwrap();

        storage.into()
    }
}

pub fn run_test<T>(test: T) -> ()
where
    T: FnOnce() -> (),
{
    clear_mocks();
    ExtBuilder::build().execute_with(test);
}
