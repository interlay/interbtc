/// Mocking the test environment
use crate::{Error, GenesisConfig, Module, Trait};
use frame_support::{
    impl_outer_event, impl_outer_origin, parameter_types,
    weights::{
        constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight},
        Weight,
    },
};
use pallet_balances as balances;
use sp_arithmetic::{FixedI128, FixedPointNumber, FixedU128};
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
        frame_system<T>,
        test_events<T>,
        balances<T>,
        collateral<T>,
        vault_registry<T>,
        treasury<T>,
        exchange_rate_oracle<T>,
        fee<T>,
        sla<T>,
        btc_relay,
        redeem<T>,
        replace<T>,
        refund<T>,
        security,
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
    pub const AvailableBlockRatio: Perbill = Perbill::one();
}

impl frame_system::Trait for Test {
    type AccountId = AccountId;
    type Call = ();
    type Lookup = IdentityLookup<Self::AccountId>;
    type Index = u64;
    type BlockNumber = BlockNumber;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type Header = Header;
    type Event = TestEvent;
    type Origin = Origin;
    type BlockHashCount = BlockHashCount;
    type MaximumBlockWeight = MaximumBlockWeight;
    type MaximumBlockLength = MaximumBlockLength;
    type AvailableBlockRatio = AvailableBlockRatio;
    type BlockExecutionWeight = BlockExecutionWeight;
    type DbWeight = RocksDbWeight;
    type ExtrinsicBaseWeight = ExtrinsicBaseWeight;
    type Version = ();
    type PalletInfo = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type AccountData = balances::AccountData<u64>;
    type BaseCallFilter = ();
    type MaximumExtrinsicWeight = MaximumBlockWeight;
    type SystemWeightInfo = ();
}

parameter_types! {
    pub const ExistentialDeposit: u64 = 1;
    pub const MaxLocks: u32 = 50;
}

impl pallet_balances::Trait for Test {
    type MaxLocks = MaxLocks;
    type Balance = Balance;
    type Event = TestEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
}

parameter_types! {
    pub const MinimumPeriod: u64 = 5;
}

impl pallet_timestamp::Trait for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

impl security::Trait for Test {
    type Event = TestEvent;
}

impl vault_registry::Trait for Test {
    type Event = TestEvent;
    type RandomnessSource = pallet_randomness_collective_flip::Module<Test>;
    type UnsignedFixedPoint = FixedU128;
    type WeightInfo = ();
}

impl treasury::Trait for Test {
    type PolkaBTC = Balances;
    type Event = TestEvent;
}

impl exchange_rate_oracle::Trait for Test {
    type Event = TestEvent;
    type WeightInfo = ();
}

impl fee::Trait for Test {
    type Event = TestEvent;
    type UnsignedFixedPoint = FixedU128;
    type WeightInfo = ();
}

impl sla::Trait for Test {
    type Event = TestEvent;
    type SignedFixedPoint = FixedI128;
}

impl refund::Trait for Test {
    type Event = TestEvent;
    type WeightInfo = ();
}

impl collateral::Trait for Test {
    type Event = TestEvent;
    type DOT = Balances;
}

impl btc_relay::Trait for Test {
    type Event = TestEvent;
    type WeightInfo = ();
}

impl redeem::Trait for Test {
    type Event = TestEvent;
    type WeightInfo = ();
}

impl replace::Trait for Test {
    type Event = TestEvent;
    type WeightInfo = ();
}

parameter_types! {
    pub const MaturityPeriod: u64 = 10;
    pub const MinimumDeposit: u64 = 10;
    pub const MinimumStake: u64 = 10;
    pub const MinimumParticipants: u64 = 3;
    pub const VoteThreshold: u64 = 50;
    pub const VotingPeriod: u64 = 100;
    pub const MaximumMessageSize: u32 = 32;
}

impl Trait for Test {
    type Event = TestEvent;
    type WeightInfo = ();
    type MaturityPeriod = MaturityPeriod;
    type MinimumDeposit = MinimumDeposit;
    type MinimumStake = MinimumStake;
    type MinimumParticipants = MinimumParticipants;
    type VoteThreshold = VoteThreshold;
    type VotingPeriod = VotingPeriod;
    type MaximumMessageSize = MaximumMessageSize;
}

pub type System = frame_system::Module<Test>;
pub type Balances = balances::Module<Test>;
pub type Staking = Module<Test>;

pub type TestError = Error<Test>;
pub type RedeemError = redeem::Error<Test>;

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
pub const CAROL: AccountId = 3;
pub const DAVE: AccountId = 4;
pub const EVE: AccountId = 5;

pub const ALICE_BALANCE: u64 = 1_000_000;
pub const BOB_BALANCE: u64 = 1_000_000;
pub const CAROL_BALANCE: u64 = 1_000_000;
pub const DAVE_BALANCE: u64 = 1_000_000;
pub const EVE_BALANCE: u64 = 1_000_000;

pub struct ExtBuilder;

impl ExtBuilder {
    pub fn build_with<F>(conf: F) -> sp_io::TestExternalities
    where
        F: FnOnce(&mut sp_core::storage::Storage),
    {
        let mut storage = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();

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

        conf(&mut storage);

        storage.into()
    }

    pub fn build() -> sp_io::TestExternalities {
        ExtBuilder::build_with(|storage| {
            pallet_balances::GenesisConfig::<Test> {
                balances: vec![
                    (ALICE, ALICE_BALANCE),
                    (BOB, BOB_BALANCE),
                    (CAROL, CAROL_BALANCE),
                    (DAVE, DAVE_BALANCE),
                    (EVE, EVE_BALANCE),
                ],
            }
            .assimilate_storage(storage)
            .unwrap();

            GenesisConfig::<Test> { gov_id: CAROL }
                .assimilate_storage(storage)
                .unwrap();
        })
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
