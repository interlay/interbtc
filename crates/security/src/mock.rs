/// Mocking the test environment
use crate::{Config, Error, Module};
use frame_support::{impl_outer_event, impl_outer_origin, parameter_types};
use mocktopus::mocking::clear_mocks;
use sp_core::H256;
use sp_io;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};

impl_outer_origin! {
    pub enum Origin for Test {}
}

mod security {
    pub use crate::Event;
}

impl_outer_event! {
    pub enum TestEvent for Test {
        frame_system<T>,
        security,
    }
}

pub struct PalletInfo;

impl frame_support::traits::PalletInfo for PalletInfo {
    fn index<P: 'static>() -> Option<usize> {
        Some(0)
    }

    fn name<P: 'static>() -> Option<&'static str> {
        Some("security")
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
}

pub type TestError = Error<Test>;

pub type Security = Module<Test>;
pub type System = frame_system::Module<Test>;

pub struct ExtBuilder;

impl ExtBuilder {
    pub fn build() -> sp_io::TestExternalities {
        let storage = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();

        storage.into()
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
