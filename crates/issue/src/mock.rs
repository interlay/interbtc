/// Mocking the test environment
use crate::{Config, Error, GenesisConfig, Module};
use frame_support::{assert_ok, impl_outer_event, impl_outer_origin, parameter_types};
use pallet_balances as balances;
use sp_arithmetic::{FixedI128, FixedPointNumber, FixedU128};
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};

use mocktopus::mocking::clear_mocks;

impl_outer_origin! {
    pub enum Origin for Test {}
}

mod issue {
    pub use crate::Event;
}

impl_outer_event! {
    pub enum TestEvent for Test {
        frame_system<T>,
        issue<T>,
        balances<T>,
        vault_registry<T>,
        collateral<T>,
        btc_relay,
        treasury<T>,
        exchange_rate_oracle<T>,
        fee<T>,
        sla<T>,
        refund<T>,
        security,
    }
}

pub struct PalletInfo;

impl frame_support::traits::PalletInfo for PalletInfo {
    fn index<P: 'static>() -> Option<usize> {
        Some(0)
    }

    fn name<P: 'static>() -> Option<&'static str> {
        Some("issue")
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

impl vault_registry::Config for Test {
    type Event = TestEvent;
    type RandomnessSource = pallet_randomness_collective_flip::Module<Test>;
    type UnsignedFixedPoint = FixedU128;
    type WeightInfo = ();
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

impl treasury::Config for Test {
    type PolkaBTC = Balances;
    type Event = TestEvent;
}

impl refund::Config for Test {
    type Event = TestEvent;
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

impl exchange_rate_oracle::Config for Test {
    type Event = TestEvent;
    type UnsignedFixedPoint = FixedU128;
    type WeightInfo = ();
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

impl Config for Test {
    type Event = TestEvent;
    type WeightInfo = ();
}

pub type TestError = Error<Test>;
pub type SecurityError = security::Error<Test>;
pub type VaultRegistryError = vault_registry::Error<Test>;

pub type System = frame_system::Module<Test>;
pub type Balances = pallet_balances::Module<Test>;
pub type Issue = Module<Test>;

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
pub const CAROL: AccountId = 3;

pub const ALICE_BALANCE: u64 = 1_000_000;
pub const BOB_BALANCE: u64 = 1_000_000;
pub const CAROL_BALANCE: u64 = 1_000_000;

pub struct ExtBuilder;

impl ExtBuilder {
    pub fn build_with(conf: pallet_balances::GenesisConfig<Test>) -> sp_io::TestExternalities {
        let mut storage = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();

        conf.assimilate_storage(&mut storage).unwrap();

        fee::GenesisConfig::<Test> {
            issue_fee: FixedU128::checked_from_rational(5, 1000).unwrap(), // 0.5%
            issue_griefing_collateral: FixedU128::checked_from_rational(5, 100000).unwrap(), // 0.005%
            refund_fee: FixedU128::checked_from_rational(5, 1000).unwrap(),                  // 0.5%
            redeem_fee: FixedU128::checked_from_rational(5, 1000).unwrap(),                  // 0.5%
            premium_redeem_fee: FixedU128::checked_from_rational(5, 100).unwrap(),           // 5%
            auction_redeem_fee: FixedU128::checked_from_rational(5, 100).unwrap(),           // 5%
            punishment_fee: FixedU128::checked_from_rational(1, 10).unwrap(),                // 10%
            replace_griefing_collateral: FixedU128::checked_from_rational(1, 10).unwrap(),   // 10%
            fee_pool_account_id: 0,
            maintainer_account_id: 1,
            epoch_period: 5,
            vault_rewards: FixedU128::checked_from_rational(77, 100).unwrap(),
            vault_rewards_issued: FixedU128::checked_from_rational(90, 100).unwrap(),
            vault_rewards_locked: FixedU128::checked_from_rational(10, 100).unwrap(),
            relayer_rewards: FixedU128::checked_from_rational(3, 100).unwrap(),
            maintainer_rewards: FixedU128::checked_from_rational(20, 100).unwrap(),
            collator_rewards: FixedU128::checked_from_integer(0).unwrap(),
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        GenesisConfig::<Test> { issue_period: 10 }
            .assimilate_storage(&mut storage)
            .unwrap();

        storage.into()
    }

    pub fn build() -> sp_io::TestExternalities {
        ExtBuilder::build_with(pallet_balances::GenesisConfig::<Test> {
            balances: vec![
                (ALICE, ALICE_BALANCE),
                (BOB, BOB_BALANCE),
                (CAROL, CAROL_BALANCE),
            ],
        })
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
