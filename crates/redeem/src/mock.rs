use crate as redeem;
use crate::{Config, Error};
use frame_support::{
    assert_ok, parameter_types,
    traits::{GenesisBuild, StorageMapShim},
    PalletId,
};
use mocktopus::mocking::clear_mocks;
use sp_arithmetic::{FixedI128, FixedPointNumber, FixedU128};
use sp_core::H256;
use sp_runtime::{
    testing::{Header, TestXt},
    traits::{BlakeTwo256, IdentityLookup, One},
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

        BackingCurrency: currency::<Instance1>::{Pallet, Call, Storage, Event<T>},
        IssuingCurrency: currency::<Instance2>::{Pallet, Call, Storage, Event<T>},

        BackingVaultRewards: reward::<Instance1>::{Pallet, Call, Storage, Event<T>},
        IssuingVaultRewards: reward::<Instance2>::{Pallet, Call, Storage, Event<T>},
        BackingRelayerRewards: reward::<Instance3>::{Pallet, Call, Storage, Event<T>},
        IssuingRelayerRewards: reward::<Instance4>::{Pallet, Call, Storage, Event<T>},

        // Operational
        BTCRelay: btc_relay::{Pallet, Call, Config<T>, Storage, Event<T>},
        Security: security::{Pallet, Call, Storage, Event<T>},
        VaultRegistry: vault_registry::{Pallet, Call, Config<T>, Storage, Event<T>},
        ExchangeRateOracle: exchange_rate_oracle::{Pallet, Call, Config<T>, Storage, Event<T>},
        Redeem: redeem::{Pallet, Call, Config<T>, Storage, Event<T>},
        Fee: fee::{Pallet, Call, Config<T>, Storage, Event<T>},
        Sla: sla::{Pallet, Call, Config<T>, Storage, Event<T>},
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
    type SignedFixedPoint = FixedI128;
    type UnsignedFixedPoint = FixedU128;
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

impl btc_relay::Config for Test {
    type Event = TestEvent;
    type WeightInfo = ();
}

impl security::Config for Test {
    type Event = TestEvent;
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
    type BackingVaultRewards = BackingVaultRewards;
    type IssuingVaultRewards = IssuingVaultRewards;
    type BackingRelayerRewards = BackingRelayerRewards;
    type IssuingRelayerRewards = IssuingRelayerRewards;
}

impl sla::Config for Test {
    type Event = TestEvent;
    type SignedFixedPoint = FixedI128;
    type SignedInner = i128;
    type Balance = Balance;
    type BackingVaultRewards = BackingVaultRewards;
    type IssuingVaultRewards = IssuingVaultRewards;
    type BackingRelayerRewards = BackingRelayerRewards;
    type IssuingRelayerRewards = IssuingRelayerRewards;
}

impl Config for Test {
    type Event = TestEvent;
    type WeightInfo = ();
}

pub type TestEvent = Event;
pub type TestError = Error<Test>;
pub type VaultRegistryError = vault_registry::Error<Test>;

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
pub const CAROL: AccountId = 3;

pub const ALICE_BALANCE: u128 = 1_005_000;
pub const BOB_BALANCE: u128 = 1_005_000;
pub const CAROL_BALANCE: u128 = 1_005_000;

pub struct ExtBuilder;

impl ExtBuilder {
    pub fn build_with(
        backing_balances: pallet_balances::GenesisConfig<Test, pallet_balances::Instance1>,
        issuing_balances: pallet_balances::GenesisConfig<Test, pallet_balances::Instance2>,
    ) -> sp_io::TestExternalities {
        let mut storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

        backing_balances.assimilate_storage(&mut storage).unwrap();
        issuing_balances.assimilate_storage(&mut storage).unwrap();

        fee::GenesisConfig::<Test> {
            issue_fee: FixedU128::checked_from_rational(5, 1000).unwrap(), // 0.5%
            issue_griefing_collateral: FixedU128::checked_from_rational(5, 100000).unwrap(), // 0.005%
            refund_fee: FixedU128::checked_from_rational(5, 1000).unwrap(), // 0.5%
            redeem_fee: FixedU128::checked_from_rational(5, 1000).unwrap(), // 0.5%
            premium_redeem_fee: FixedU128::checked_from_rational(5, 100).unwrap(), // 5%
            punishment_fee: FixedU128::checked_from_rational(1, 10).unwrap(), // 10%
            replace_griefing_collateral: FixedU128::checked_from_rational(1, 10).unwrap(), // 10%
            maintainer_account_id: 1,
            vault_rewards: FixedU128::checked_from_rational(77, 100).unwrap(),
            relayer_rewards: FixedU128::checked_from_rational(3, 100).unwrap(),
            maintainer_rewards: FixedU128::checked_from_rational(20, 100).unwrap(),
            nomination_rewards: FixedU128::checked_from_rational(0, 100).unwrap(),
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        vault_registry::GenesisConfig::<Test> {
            minimum_collateral_vault: 0,
            punishment_delay: 8,
            secure_collateral_threshold: FixedU128::checked_from_rational(200, 100).unwrap(),
            premium_redeem_threshold: FixedU128::checked_from_rational(120, 100).unwrap(),
            liquidation_collateral_threshold: FixedU128::checked_from_rational(110, 100).unwrap(),
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        sla::GenesisConfig::<Test> {
            vault_target_sla: FixedI128::from(100),
            vault_redeem_failure_sla_change: FixedI128::from(-10),
            vault_execute_issue_max_sla_change: FixedI128::from(4),
            vault_deposit_max_sla_change: FixedI128::from(4),
            vault_withdraw_max_sla_change: FixedI128::from(-4),
            vault_submit_issue_proof: FixedI128::from(0),
            vault_refund: FixedI128::from(1),
            relayer_target_sla: FixedI128::from(100),
            relayer_block_submission: FixedI128::from(1),
            relayer_duplicate_block_submission: FixedI128::from(1),
            relayer_correct_theft_report: FixedI128::from(1),
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        redeem::GenesisConfig::<Test> {
            redeem_transaction_size: 400,
            redeem_period: 10,
            redeem_btc_dust_value: 2,
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        storage.into()
    }

    pub fn build() -> sp_io::TestExternalities {
        ExtBuilder::build_with(
            pallet_balances::GenesisConfig::<Test, pallet_balances::Instance1> {
                balances: vec![(ALICE, ALICE_BALANCE), (BOB, BOB_BALANCE), (CAROL, CAROL_BALANCE)],
            },
            pallet_balances::GenesisConfig::<Test, pallet_balances::Instance2> {
                balances: vec![(ALICE, ALICE_BALANCE), (BOB, BOB_BALANCE), (CAROL, CAROL_BALANCE)],
            },
        )
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
        Security::set_active_block_number(1);
        System::set_block_number(1);
        test();
    });
}
