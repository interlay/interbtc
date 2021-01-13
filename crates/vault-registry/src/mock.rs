use frame_support::traits::StorageMapShim;
/// Mocking the test environment
use frame_support::{impl_outer_event, impl_outer_origin, parameter_types};
use mocktopus::mocking::clear_mocks;
use sp_arithmetic::{FixedPointNumber, FixedU128};
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};

use mocktopus::mocking::{MockResult, Mockable};

use crate::ext;
use crate::{Config, Error, GenesisConfig, Module};

impl_outer_origin! {
    pub enum Origin for Test {}
}

mod vault_registry {
    pub use crate::Event;
}

impl_outer_event! {
    pub enum TestEvent for Test {
        frame_system<T>,
        vault_registry<T>,
        pallet_balances Instance1<T>,
        pallet_balances Instance2<T>,
        collateral<T>,
        treasury<T>,
        exchange_rate_oracle<T>,
        security,
    }
}

pub type AccountId = u64;
pub type Balance = u128;
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
    type PalletInfo = ();
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

/// DOT
impl pallet_balances::Config<pallet_balances::Instance1> for Test {
    type MaxLocks = MaxLocks;
    /// The type for recording an account's balance.
    type Balance = Balance;
    /// The ubiquitous event type.
    type Event = TestEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = StorageMapShim<
        pallet_balances::Account<Test, pallet_balances::Instance1>,
        frame_system::CallOnCreatedAccount<Test>,
        frame_system::CallKillAccount<Test>,
        AccountId,
        pallet_balances::AccountData<Balance>,
    >;
    type WeightInfo = ();
}

/// PolkaBTC
impl pallet_balances::Config<pallet_balances::Instance2> for Test {
    type MaxLocks = MaxLocks;
    type Balance = Balance;
    type Event = TestEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = StorageMapShim<
        pallet_balances::Account<Test, pallet_balances::Instance2>,
        frame_system::CallOnCreatedAccount<Test>,
        frame_system::CallKillAccount<Test>,
        AccountId,
        pallet_balances::AccountData<Balance>,
    >;
    type WeightInfo = ();
}

impl collateral::Config for Test {
    type Event = TestEvent;
    type DOT = pallet_balances::Module<Test, pallet_balances::Instance1>;
}

impl treasury::Config for Test {
    type Event = TestEvent;
    type PolkaBTC = pallet_balances::Module<Test, pallet_balances::Instance2>;
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

impl Config for Test {
    type Event = TestEvent;
    type RandomnessSource = pallet_randomness_collective_flip::Module<Test>;
    type UnsignedFixedPoint = FixedU128;
    type WeightInfo = ();
}

impl security::Config for Test {
    type Event = TestEvent;
}

pub type TestError = Error<Test>;
pub type SecurityError = security::Error<Test>;
pub type CollateralError = collateral::Error<Test>;

pub type System = frame_system::Module<Test>;
pub type VaultRegistry = Module<Test>;

pub struct ExtBuilder;

pub const DEFAULT_ID: u64 = 3;
pub const OTHER_ID: u64 = 4;
pub const RICH_ID: u64 = 5;
pub const DEFAULT_COLLATERAL: u128 = 100;
pub const RICH_COLLATERAL: u128 = DEFAULT_COLLATERAL + 50;
pub const MULTI_VAULT_TEST_IDS: [u64; 4] = [100, 101, 102, 103];
pub const MULTI_VAULT_TEST_COLLATERAL: u128 = 100000;
impl ExtBuilder {
    pub fn build_with(
        conf: pallet_balances::GenesisConfig<Test, pallet_balances::Instance1>,
    ) -> sp_io::TestExternalities {
        let mut storage = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();

        conf.assimilate_storage(&mut storage).unwrap();

        pallet_balances::GenesisConfig::<Test, pallet_balances::Instance2> { balances: vec![] }
            .assimilate_storage(&mut storage)
            .unwrap();

        // Parameters to be set in tests
        GenesisConfig::<Test> {
            minimum_collateral_vault: 0,
            punishment_delay: 0,
            secure_collateral_threshold: FixedU128::one(),
            auction_collateral_threshold: FixedU128::one(),
            premium_redeem_threshold: FixedU128::one(),
            liquidation_collateral_threshold: FixedU128::one(),
            liquidation_vault_account_id: 0,
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        sp_io::TestExternalities::from(storage)
    }
    pub fn build() -> sp_io::TestExternalities {
        ExtBuilder::build_with(
            pallet_balances::GenesisConfig::<Test, pallet_balances::Instance1> {
                balances: vec![
                    (DEFAULT_ID, DEFAULT_COLLATERAL),
                    (OTHER_ID, DEFAULT_COLLATERAL),
                    (RICH_ID, RICH_COLLATERAL),
                    (MULTI_VAULT_TEST_IDS[0], MULTI_VAULT_TEST_COLLATERAL),
                    (MULTI_VAULT_TEST_IDS[1], MULTI_VAULT_TEST_COLLATERAL),
                    (MULTI_VAULT_TEST_IDS[2], MULTI_VAULT_TEST_COLLATERAL),
                    (MULTI_VAULT_TEST_IDS[3], MULTI_VAULT_TEST_COLLATERAL),
                ],
            },
        )
    }
}

pub fn run_test<T>(test: T) -> ()
where
    T: FnOnce() -> (),
{
    clear_mocks();
    ext::oracle::dots_to_btc::<Test>.mock_safe(|v| MockResult::Return(Ok(v)));
    ext::oracle::btc_to_dots::<Test>.mock_safe(|v| MockResult::Return(Ok(v)));
    ExtBuilder::build().execute_with(|| {
        System::set_block_number(1);
        test()
    })
}
