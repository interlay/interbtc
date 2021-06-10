use crate as refund;
use crate::Config;
use frame_support::{assert_ok, parameter_types, traits::StorageMapShim, PalletId};
use mocktopus::mocking::clear_mocks;
use sp_core::H256;
use sp_runtime::{
    testing::{Header, TestXt},
    traits::{BlakeTwo256, IdentityLookup, One},
    FixedI128, FixedU128,
};

pub const VAULT: AccountId = 1;
pub const USER: AccountId = 2;

type TestExtrinsic = TestXt<Call, ()>;
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
        Collateral: pallet_balances::<Instance1>::{Pallet, Call, Storage, Config<T>, Event<T>},
        Wrapped: pallet_balances::<Instance2>::{Pallet, Call, Storage, Config<T>, Event<T>},

        CollateralCurrency: currency::<Instance1>::{Pallet, Call, Storage, Event<T>},
        WrappedCurrency: currency::<Instance2>::{Pallet, Call, Storage, Event<T>},

        CollateralVaultRewards: reward::<Instance1>::{Pallet, Call, Storage, Event<T>},
        WrappedVaultRewards: reward::<Instance2>::{Pallet, Call, Storage, Event<T>},
        CollateralRelayerRewards: reward::<Instance3>::{Pallet, Call, Storage, Event<T>},
        WrappedRelayerRewards: reward::<Instance4>::{Pallet, Call, Storage, Event<T>},

        // Operational
        BTCRelay: btc_relay::{Pallet, Call, Config<T>, Storage, Event<T>},
        Security: security::{Pallet, Call, Storage, Event<T>},
        VaultRegistry: vault_registry::{Pallet, Call, Config<T>, Storage, Event<T>},
        ExchangeRateOracle: exchange_rate_oracle::{Pallet, Call, Config<T>, Storage, Event<T>},
        Fee: fee::{Pallet, Call, Config<T>, Storage, Event<T>},
        Sla: sla::{Pallet, Call, Config<T>, Storage, Event<T>},
        Refund: refund::{Pallet, Call, Config<T>, Storage, Event<T>},
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
    type AccountData = ();
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

/// Collateral currency - e.g. DOT/KSM
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

/// Wrapped currency - e.g. InterBTC
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
    pub const CollateralName: &'static [u8] = b"Polkadot";
    pub const CollateralSymbol: &'static [u8] = b"DOT";
    pub const CollateralDecimals: u8 = 10;
}

impl currency::Config<currency::Collateral> for Test {
    type Event = TestEvent;
    type Balance = Balance;
    type Currency = Collateral;
    type Name = CollateralName;
    type Symbol = CollateralSymbol;
    type Decimals = CollateralDecimals;
}

parameter_types! {
    pub const WrappedName: &'static [u8] = b"Bitcoin";
    pub const WrappedSymbol: &'static [u8] = b"BTC";
    pub const WrappedDecimals: u8 = 8;
}

impl currency::Config<currency::Wrapped> for Test {
    type Event = TestEvent;
    type Balance = Balance;
    type Currency = Wrapped;
    type Name = WrappedName;
    type Symbol = WrappedSymbol;
    type Decimals = WrappedDecimals;
}

impl reward::Config<reward::CollateralVault> for Test {
    type Event = TestEvent;
    type SignedFixedPoint = FixedI128;
}

impl reward::Config<reward::WrappedVault> for Test {
    type Event = TestEvent;
    type SignedFixedPoint = FixedI128;
}

impl reward::Config<reward::CollateralRelayer> for Test {
    type Event = TestEvent;
    type SignedFixedPoint = FixedI128;
}

impl reward::Config<reward::WrappedRelayer> for Test {
    type Event = TestEvent;
    type SignedFixedPoint = FixedI128;
}

impl Config for Test {
    type Event = TestEvent;
    type WeightInfo = ();
}

parameter_types! {
    pub const FeePalletId: PalletId = PalletId(*b"mod/fees");
}

impl fee::Config for Test {
    type PalletId = FeePalletId;
    type Event = TestEvent;
    type WeightInfo = ();
    type SignedFixedPoint = FixedI128;
    type SignedInner = i128;
    type UnsignedFixedPoint = FixedU128;
    type UnsignedInner = Balance;
    type CollateralVaultRewards = CollateralVaultRewards;
    type WrappedVaultRewards = WrappedVaultRewards;
    type CollateralRelayerRewards = CollateralRelayerRewards;
    type WrappedRelayerRewards = WrappedRelayerRewards;
}

impl sla::Config for Test {
    type Event = TestEvent;
    type SignedFixedPoint = FixedI128;
    type SignedInner = i128;
    type Balance = Balance;
    type CollateralVaultRewards = CollateralVaultRewards;
    type WrappedVaultRewards = WrappedVaultRewards;
    type CollateralRelayerRewards = CollateralRelayerRewards;
    type WrappedRelayerRewards = WrappedRelayerRewards;
}

impl btc_relay::Config for Test {
    type Event = TestEvent;
    type WeightInfo = ();
}

impl security::Config for Test {
    type Event = TestEvent;
}

parameter_types! {
    pub const VaultPalletId: PalletId = PalletId(*b"mod/vreg");
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Test
where
    Call: From<C>,
{
    type OverarchingCall = Call;
    type Extrinsic = TestExtrinsic;
}

impl vault_registry::Config for Test {
    type PalletId = VaultPalletId;
    type Event = TestEvent;
    type RandomnessSource = pallet_randomness_collective_flip::Pallet<Test>;
    type Balance = Balance;
    type SignedFixedPoint = FixedI128;
    type UnsignedFixedPoint = FixedU128;
    type WeightInfo = ();
}

parameter_types! {
    pub const GetCollateralDecimals: u8 = 10;
    pub const GetWrappedDecimals: u8 = 8;
}

impl exchange_rate_oracle::Config for Test {
    type Event = TestEvent;
    type Balance = Balance;
    type UnsignedFixedPoint = FixedU128;
    type WeightInfo = ();
    type GetCollateralDecimals = GetCollateralDecimals;
    type GetWrappedDecimals = GetWrappedDecimals;
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

pub type TestEvent = Event;

pub struct ExtBuilder;

impl ExtBuilder {
    pub fn build() -> sp_io::TestExternalities {
        let storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

        sp_io::TestExternalities::from(storage)
    }
}

pub fn run_test<T>(test: T)
where
    T: FnOnce(),
{
    clear_mocks();
    ExtBuilder::build().execute_with(|| {
        assert_ok!(<exchange_rate_oracle::Pallet<Test>>::_set_exchange_rate(
            FixedU128::one()
        ));
        System::set_block_number(1);
        Security::set_active_block_number(1);
        test();
    });
}
