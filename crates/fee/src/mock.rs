use crate as fee;
use crate::{Config, Error};
use frame_support::{parameter_types, traits::StorageMapShim};
use mocktopus::mocking::clear_mocks;
use sp_arithmetic::{FixedI128, FixedU128};
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    ModuleId,
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
        Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},

        // Tokens & Balances
        Backing: pallet_balances::<Instance1>::{Pallet, Call, Storage, Config<T>, Event<T>},
        Issuing: pallet_balances::<Instance2>::{Pallet, Call, Storage, Config<T>, Event<T>},

        BackingCurrency: currency::<Instance1>::{Pallet, Call, Storage, Event<T>},
        IssuingCurrency: currency::<Instance2>::{Pallet, Call, Storage, Event<T>},

        BackingVaultRewards: reward::<Instance1>::{Pallet, Call, Storage, Event<T>},
        IssuingVaultRewards: reward::<Instance2>::{Pallet, Call, Storage, Event<T>},
        BackingRelayerRewards: reward::<Instance3>::{Pallet, Call, Storage, Event<T>},
        IssuingRelayerRewards: reward::<Instance4>::{Pallet, Call, Storage, Event<T>},

        // Operational
        Security: security::{Pallet, Call, Storage, Event<T>},
        Fee: fee::{Pallet, Call, Config<T>, Storage, Event<T>},
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
    type BaseCallFilter = ();
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
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
    pub const MaxLocks: u32 = 50;
}

/// Backing currency - e.g. DOT/KSM
impl pallet_balances::Config<pallet_balances::Instance1> for Test {
    type MaxLocks = MaxLocks;
    type Balance = Balance;
    type Event = TestEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = StorageMapShim<
        pallet_balances::Account<Test, pallet_balances::Instance1>,
        frame_system::Provider<Test>,
        AccountId,
        pallet_balances::AccountData<Balance>,
    >;
    type WeightInfo = ();
}

/// Issuing currency - e.g. PolkaBTC
impl pallet_balances::Config<pallet_balances::Instance2> for Test {
    type MaxLocks = MaxLocks;
    type Balance = Balance;
    type Event = TestEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = StorageMapShim<
        pallet_balances::Account<Test, pallet_balances::Instance2>,
        frame_system::Provider<Test>,
        AccountId,
        pallet_balances::AccountData<Balance>,
    >;
    type WeightInfo = ();
}

parameter_types! {
    pub const BackingName: &'static [u8] = b"Polkadot";
    pub const BackingSymbol: &'static [u8] = b"DOT";
    pub const BackingDecimals: u8 = 10;
}

impl currency::Config<currency::Backing> for Test {
    type Event = TestEvent;
    type Currency = Backing;
    type Name = BackingName;
    type Symbol = BackingSymbol;
    type Decimals = BackingDecimals;
}

parameter_types! {
    pub const IssuingName: &'static [u8] = b"Bitcoin";
    pub const IssuingSymbol: &'static [u8] = b"BTC";
    pub const IssuingDecimals: u8 = 8;
}

impl currency::Config<currency::Issuing> for Test {
    type Event = TestEvent;
    type Currency = Issuing;
    type Name = IssuingName;
    type Symbol = IssuingSymbol;
    type Decimals = IssuingDecimals;
}

impl reward::Config<reward::BackingVault> for Test {
    type Event = TestEvent;
    type SignedFixedPoint = FixedI128;
}

impl reward::Config<reward::IssuingVault> for Test {
    type Event = TestEvent;
    type SignedFixedPoint = FixedI128;
}

impl reward::Config<reward::BackingRelayer> for Test {
    type Event = TestEvent;
    type SignedFixedPoint = FixedI128;
}

impl reward::Config<reward::IssuingRelayer> for Test {
    type Event = TestEvent;
    type SignedFixedPoint = FixedI128;
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

impl security::Config for Test {
    type Event = TestEvent;
}

parameter_types! {
    pub const FeeModuleId: ModuleId = ModuleId(*b"mod/fees");
}

impl Config for Test {
    type ModuleId = FeeModuleId;
    type Event = TestEvent;
    type WeightInfo = ();
    type SignedFixedPoint = FixedI128;
    type SignedInner = i128;
    type UnsignedFixedPoint = FixedU128;
    type UnsignedInner = Balance;
    type BackingVaultRewards = BackingVaultRewards;
    type IssuingVaultRewards = IssuingVaultRewards;
    type BackingRelayerRewards = BackingRelayerRewards;
    type IssuingRelayerRewards = IssuingRelayerRewards;
}

pub type TestEvent = Event;

#[allow(dead_code)]
pub type TestError = Error<Test>;

pub struct ExtBuilder;

impl ExtBuilder {
    pub fn build() -> sp_io::TestExternalities {
        let storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

        storage.into()
    }
}

pub fn run_test<T>(test: T)
where
    T: FnOnce(),
{
    clear_mocks();
    ExtBuilder::build().execute_with(|| {
        System::set_block_number(1);
        Security::set_active_block_number(1);
        test();
    });
}
