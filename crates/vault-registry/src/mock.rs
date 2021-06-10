use crate as vault_registry;
use crate::{ext, Config, Error};
use codec::{Decode, Encode};
use frame_support::{parameter_types, traits::GenesisBuild, PalletId};
use mocktopus::mocking::{clear_mocks, MockResult, Mockable};
use orml_tokens::CurrencyAdapter;
use orml_traits::parameter_type_with_key;
use sp_arithmetic::{FixedI128, FixedPointNumber, FixedU128};
use sp_core::H256;
use sp_runtime::{
    testing::{Header, TestXt},
    traits::{BlakeTwo256, IdentityLookup, One, Zero},
};

pub(crate) type Extrinsic = TestXt<Call, ()>;
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
        Tokens: orml_tokens::{Pallet, Storage, Config<T>, Event<T>},

        CollateralVaultRewards: reward::<Instance1>::{Pallet, Call, Storage, Event<T>},
        WrappedVaultRewards: reward::<Instance2>::{Pallet, Call, Storage, Event<T>},
        CollateralRelayerRewards: reward::<Instance3>::{Pallet, Call, Storage, Event<T>},
        WrappedRelayerRewards: reward::<Instance4>::{Pallet, Call, Storage, Event<T>},

        // Operational
        Security: security::{Pallet, Call, Storage, Event<T>},
        VaultRegistry: vault_registry::{Pallet, Call, Config<T>, Storage, Event<T>, ValidateUnsigned},
        ExchangeRateOracle: exchange_rate_oracle::{Pallet, Call, Config<T>, Storage, Event<T>},
        Sla: sla::{Pallet, Call, Config<T>, Storage, Event<T>},
    }
);

pub type AccountId = u64;
pub type Balance = u128;
pub type Amount = i128;
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

#[derive(Encode, Decode, Debug, PartialEq, PartialOrd, Ord, Eq, Clone, Copy)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum CurrencyId {
    DOT,
    INTERBTC,
}

pub const DOT: CurrencyId = CurrencyId::DOT;
pub const INTERBTC: CurrencyId = CurrencyId::INTERBTC;

parameter_types! {
    pub const GetCollateralCurrencyId: CurrencyId = DOT;
    pub const GetWrappedCurrencyId: CurrencyId = INTERBTC;
    pub const MaxLocks: u32 = 50;
}

parameter_type_with_key! {
    pub ExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
        Zero::zero()
    };
}

impl orml_tokens::Config for Test {
    type Event = Event;
    type Balance = Balance;
    type Amount = Amount;
    type CurrencyId = CurrencyId;
    type WeightInfo = ();
    type ExistentialDeposits = ExistentialDeposits;
    type OnDust = ();
    type MaxLocks = MaxLocks;
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

parameter_types! {
    pub const MinimumPeriod: u64 = 5;
}

impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
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
    pub const VaultPalletId: PalletId = PalletId(*b"mod/vreg");
}

impl Config for Test {
    type PalletId = VaultPalletId;
    type Event = TestEvent;
    type RandomnessSource = pallet_randomness_collective_flip::Pallet<Test>;
    type Balance = Balance;
    type SignedFixedPoint = FixedI128;
    type UnsignedFixedPoint = FixedU128;
    type WeightInfo = ();
    type Collateral = CurrencyAdapter<Test, GetCollateralCurrencyId>;
    type Wrapped = CurrencyAdapter<Test, GetWrappedCurrencyId>;
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Test
where
    Call: From<C>,
{
    type OverarchingCall = Call;
    type Extrinsic = Extrinsic;
}

impl security::Config for Test {
    type Event = TestEvent;
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

pub type TestEvent = Event;
pub type TestError = Error<Test>;
pub type SecurityError = security::Error<Test>;
pub type TokensError = orml_tokens::Error<Test>;

pub struct ExtBuilder;

pub const DEFAULT_ID: u64 = 3;
pub const OTHER_ID: u64 = 4;
pub const RICH_ID: u64 = 5;
pub const DEFAULT_COLLATERAL: u128 = 100000;
pub const RICH_COLLATERAL: u128 = DEFAULT_COLLATERAL + 100000;
pub const MULTI_VAULT_TEST_IDS: [u64; 4] = [100, 101, 102, 103];
pub const MULTI_VAULT_TEST_COLLATERAL: u128 = 100000;

impl ExtBuilder {
    pub fn build_with(conf: orml_tokens::GenesisConfig<Test>) -> sp_io::TestExternalities {
        let mut storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

        conf.assimilate_storage(&mut storage).unwrap();

        // Parameters to be set in tests
        vault_registry::GenesisConfig::<Test> {
            minimum_collateral_vault: 0,
            punishment_delay: 0,
            secure_collateral_threshold: FixedU128::one(),
            premium_redeem_threshold: FixedU128::one(),
            liquidation_collateral_threshold: FixedU128::one(),
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        sp_io::TestExternalities::from(storage)
    }
    pub fn build() -> sp_io::TestExternalities {
        ExtBuilder::build_with(orml_tokens::GenesisConfig::<Test> {
            endowed_accounts: vec![
                (DEFAULT_ID, DOT, DEFAULT_COLLATERAL),
                (OTHER_ID, DOT, DEFAULT_COLLATERAL),
                (RICH_ID, DOT, RICH_COLLATERAL),
                (MULTI_VAULT_TEST_IDS[0], DOT, MULTI_VAULT_TEST_COLLATERAL),
                (MULTI_VAULT_TEST_IDS[1], DOT, MULTI_VAULT_TEST_COLLATERAL),
                (MULTI_VAULT_TEST_IDS[2], DOT, MULTI_VAULT_TEST_COLLATERAL),
                (MULTI_VAULT_TEST_IDS[3], DOT, MULTI_VAULT_TEST_COLLATERAL),
            ],
        })
    }
}

pub(crate) fn set_default_thresholds() {
    let secure = FixedU128::checked_from_rational(200, 100).unwrap(); // 200%
    let premium = FixedU128::checked_from_rational(120, 100).unwrap(); // 120%
    let liquidation = FixedU128::checked_from_rational(110, 100).unwrap(); // 110%

    VaultRegistry::set_secure_collateral_threshold(secure);
    VaultRegistry::set_premium_redeem_threshold(premium);
    VaultRegistry::set_liquidation_collateral_threshold(liquidation);
}

pub fn run_test<T>(test: T)
where
    T: FnOnce(),
{
    clear_mocks();
    ext::oracle::collateral_to_wrapped::<Test>.mock_safe(|v| MockResult::Return(Ok(v)));
    ext::oracle::wrapped_to_collateral::<Test>.mock_safe(|v| MockResult::Return(Ok(v)));
    ExtBuilder::build().execute_with(|| {
        System::set_block_number(1);
        Security::set_active_block_number(1);
        set_default_thresholds();
        test()
    })
}
