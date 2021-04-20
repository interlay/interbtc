use crate as replace;
use crate::{Config, Error};
use frame_support::{assert_ok, parameter_types, traits::StorageMapShim};
use mocktopus::mocking::clear_mocks;
use sp_arithmetic::{FixedI128, FixedPointNumber, FixedU128};
use sp_core::H256;
use sp_runtime::{
    testing::{Header, TestXt},
    traits::{BlakeTwo256, IdentityLookup},
    ModuleId,
};

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
        Backing: pallet_balances::<Instance1>::{Pallet, Call, Storage, Config<T>, Event<T>},
        Issuing: pallet_balances::<Instance2>::{Pallet, Call, Storage, Config<T>, Event<T>},

        Collateral: currency::<Instance1>::{Pallet, Call, Storage, Event<T>},
        Treasury: currency::<Instance2>::{Pallet, Call, Storage, Event<T>},

        // Operational
        BTCRelay: btc_relay::{Pallet, Call, Config<T>, Storage, Event<T>},
        Security: security::{Pallet, Call, Storage, Event<T>},
        VaultRegistry: vault_registry::{Pallet, Call, Config<T>, Storage, Event<T>},
        ExchangeRateOracle: exchange_rate_oracle::{Pallet, Call, Config<T>, Storage, Event<T>},
        Replace: replace::{Pallet, Call, Config<T>, Storage, Event<T>},
        Fee: fee::{Pallet, Call, Config<T>, Storage, Event<T>},
        Sla: sla::{Pallet, Call, Config<T>, Storage, Event<T>},
        Nomination: nomination::{Pallet, Call, Config<T>, Storage, Event<T>},
    }
);

pub type AccountId = u64;
pub type Balance = u64;
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

impl currency::Config<currency::Collateral> for Test {
    type Event = TestEvent;
    type Currency = pallet_balances::Pallet<Test, pallet_balances::Instance1>;
    type Name = BackingName;
    type Symbol = BackingSymbol;
    type Decimals = BackingDecimals;
}

parameter_types! {
    pub const IssuingName: &'static [u8] = b"Bitcoin";
    pub const IssuingSymbol: &'static [u8] = b"BTC";
    pub const IssuingDecimals: u8 = 8;
}

impl currency::Config<currency::Treasury> for Test {
    type Event = TestEvent;
    type Currency = pallet_balances::Pallet<Test, pallet_balances::Instance2>;
    type Name = IssuingName;
    type Symbol = IssuingSymbol;
    type Decimals = IssuingDecimals;
}

parameter_types! {
    pub const VaultModuleId: ModuleId = ModuleId(*b"mod/vreg");
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Test
where
    Call: From<C>,
{
    type OverarchingCall = Call;
    type Extrinsic = TestExtrinsic;
}

impl vault_registry::Config for Test {
    type ModuleId = VaultModuleId;
    type Event = TestEvent;
    type UnsignedFixedPoint = FixedU128;
    type RandomnessSource = pallet_randomness_collective_flip::Pallet<Test>;
    type WeightInfo = ();
}

impl btc_relay::Config for Test {
    type Event = TestEvent;
    type WeightInfo = ();
}

impl security::Config for Test {
    type Event = TestEvent;
}

impl sla::Config for Test {
    type Event = TestEvent;
    type SignedFixedPoint = FixedI128;
}

impl nomination::Config for Test {
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

impl exchange_rate_oracle::Config for Test {
    type Event = TestEvent;
    type UnsignedFixedPoint = FixedU128;
    type WeightInfo = ();
}

parameter_types! {
    pub const FeeModuleId: ModuleId = ModuleId(*b"mod/fees");
}

impl fee::Config for Test {
    type ModuleId = FeeModuleId;
    type Event = TestEvent;
    type UnsignedFixedPoint = FixedU128;
    type WeightInfo = ();
}

impl Config for Test {
    type Event = TestEvent;
    type WeightInfo = ();
}

pub type TestEvent = Event;
pub type TestError = Error<Test>;

pub const OLD_VAULT: AccountId = 1;
pub const NEW_VAULT: AccountId = 2;
pub const CAROL: AccountId = 3;

pub const OLD_VAULT_BALANCE: u64 = 1_000_000;
pub const NEW_VAULT_BALANCE: u64 = 1_000_000;
pub const CAROL_BALANCE: u64 = 1_000_000;

pub struct ExtBuilder;

impl ExtBuilder {
    pub fn build_with(
        backing_balances: pallet_balances::GenesisConfig<Test, pallet_balances::Instance1>,
    ) -> sp_io::TestExternalities {
        let mut storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

        backing_balances.assimilate_storage(&mut storage).unwrap();

        fee::GenesisConfig::<Test> {
            issue_fee: FixedU128::checked_from_rational(5, 1000).unwrap(), // 0.5%
            issue_griefing_collateral: FixedU128::checked_from_rational(5, 100000).unwrap(), // 0.005%
            refund_fee: FixedU128::checked_from_rational(5, 1000).unwrap(), // 0.5%
            redeem_fee: FixedU128::checked_from_rational(5, 1000).unwrap(), // 0.5%
            premium_redeem_fee: FixedU128::checked_from_rational(5, 100).unwrap(), // 5%
            auction_redeem_fee: FixedU128::checked_from_rational(5, 100).unwrap(), // 5%
            punishment_fee: FixedU128::checked_from_rational(1, 10).unwrap(), // 10%
            replace_griefing_collateral: FixedU128::checked_from_rational(1, 10).unwrap(), // 10%
            maintainer_account_id: 1,
            epoch_period: 5,
            vault_rewards: FixedU128::checked_from_rational(77, 100).unwrap(),
            vault_rewards_issued: FixedU128::checked_from_rational(90, 100).unwrap(),
            vault_rewards_locked: FixedU128::checked_from_rational(10, 100).unwrap(),
            relayer_rewards: FixedU128::checked_from_rational(3, 100).unwrap(),
            maintainer_rewards: FixedU128::checked_from_rational(20, 100).unwrap(),
            collator_rewards: FixedU128::checked_from_integer(0).unwrap(),
            nomination_rewards: FixedU128::checked_from_rational(0, 100).unwrap(),
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        replace::GenesisConfig::<Test> {
            replace_period: 10,
            replace_btc_dust_value: 2,
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        storage.into()
    }

    pub fn build() -> sp_io::TestExternalities {
        ExtBuilder::build_with(pallet_balances::GenesisConfig::<Test, pallet_balances::Instance1> {
            balances: vec![
                (OLD_VAULT, OLD_VAULT_BALANCE),
                (NEW_VAULT, NEW_VAULT_BALANCE),
                (CAROL, CAROL_BALANCE),
            ],
        })
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
