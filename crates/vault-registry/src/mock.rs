use crate as vault_registry;
use crate::{Config, Error};
use frame_support::{
    parameter_types,
    traits::{ConstU32, Everything, GenesisBuild},
    PalletId,
};
use mocktopus::{macros::mockable, mocking::clear_mocks};
use orml_traits::parameter_type_with_key;
pub use primitives::{CurrencyId, CurrencyId::Token, TokenSymbol::*};
use primitives::{VaultCurrencyPair, VaultId};
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

        Rewards: reward::{Pallet, Call, Storage, Event<T>},

        // Operational
        Security: security::{Pallet, Call, Storage, Event<T>},
        VaultRegistry: vault_registry::{Pallet, Call, Config<T>, Storage, Event<T>, ValidateUnsigned},
        Oracle: oracle::{Pallet, Call, Config<T>, Storage, Event<T>},
        Staking: staking::{Pallet, Storage, Event<T>},
        Fee: fee::{Pallet, Call, Config<T>, Storage},
        Currency: currency::{Pallet},
    }
);

pub type AccountId = u64;
pub type Balance = u128;
pub type RawAmount = i128;
pub type BlockNumber = u64;
pub type Moment = u64;
pub type Index = u64;
pub type SignedFixedPoint = FixedI128;
pub type SignedInner = i128;
pub type UnsignedFixedPoint = FixedU128;

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

impl frame_system::Config for Test {
    type BaseCallFilter = Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type Origin = Origin;
    type Call = Call;
    type Index = Index;
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
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

pub const DEFAULT_COLLATERAL_CURRENCY: CurrencyId = Token(DOT);
pub const DEFAULT_NATIVE_CURRENCY: CurrencyId = Token(INTR);
pub const DEFAULT_WRAPPED_CURRENCY: CurrencyId = Token(IBTC);

pub const DEFAULT_CURRENCY_PAIR: VaultCurrencyPair<CurrencyId> = VaultCurrencyPair {
    collateral: DEFAULT_COLLATERAL_CURRENCY,
    wrapped: DEFAULT_WRAPPED_CURRENCY,
};

parameter_types! {
    pub const GetCollateralCurrencyId: CurrencyId = DEFAULT_COLLATERAL_CURRENCY;
    pub const GetNativeCurrencyId: CurrencyId = DEFAULT_NATIVE_CURRENCY;
    pub const GetWrappedCurrencyId: CurrencyId = DEFAULT_WRAPPED_CURRENCY;
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
    type Amount = RawAmount;
    type CurrencyId = CurrencyId;
    type WeightInfo = ();
    type ExistentialDeposits = ExistentialDeposits;
    type OnDust = ();
    type MaxLocks = MaxLocks;
    type DustRemovalWhitelist = Everything;
    type MaxReserves = ConstU32<0>; // we don't use named reserves
    type ReserveIdentifier = (); // we don't use named reserves
    type OnNewTokenAccount = ();
    type OnKilledTokenAccount = ();
}

impl reward::Config for Test {
    type Event = TestEvent;
    type SignedFixedPoint = SignedFixedPoint;
    type RewardId = VaultId<AccountId, CurrencyId>;
    type CurrencyId = CurrencyId;
    type GetNativeCurrencyId = GetNativeCurrencyId;
    type GetWrappedCurrencyId = GetWrappedCurrencyId;
}

parameter_types! {
    pub const MinimumPeriod: Moment = 5;
}

impl pallet_timestamp::Config for Test {
    type Moment = Moment;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

impl oracle::Config for Test {
    type Event = TestEvent;
    type WeightInfo = ();
}

pub struct CurrencyConvert;
impl currency::CurrencyConversion<currency::Amount<Test>, CurrencyId> for CurrencyConvert {
    fn convert(
        amount: &currency::Amount<Test>,
        to: CurrencyId,
    ) -> Result<currency::Amount<Test>, sp_runtime::DispatchError> {
        convert_to(to, amount.clone())
    }
}

#[cfg_attr(test, mockable)]
pub fn convert_to(
    to: CurrencyId,
    amount: currency::Amount<Test>,
) -> Result<currency::Amount<Test>, sp_runtime::DispatchError> {
    <oracle::Pallet<Test>>::convert(&amount, to)
}

impl currency::Config for Test {
    type SignedInner = SignedInner;
    type SignedFixedPoint = SignedFixedPoint;
    type UnsignedFixedPoint = UnsignedFixedPoint;
    type Balance = Balance;
    type GetNativeCurrencyId = GetNativeCurrencyId;
    type GetRelayChainCurrencyId = GetCollateralCurrencyId;
    type GetWrappedCurrencyId = GetWrappedCurrencyId;
    type CurrencyConversion = CurrencyConvert;
}

parameter_types! {
    pub const FeePalletId: PalletId = PalletId(*b"mod/fees");
    pub const MaxExpectedValue: UnsignedFixedPoint = UnsignedFixedPoint::from_inner(<UnsignedFixedPoint as FixedPointNumber>::DIV);
}

impl fee::Config for Test {
    type FeePalletId = FeePalletId;
    type WeightInfo = ();
    type SignedFixedPoint = SignedFixedPoint;
    type SignedInner = SignedInner;
    type UnsignedFixedPoint = UnsignedFixedPoint;
    type UnsignedInner = Balance;
    type VaultRewards = Rewards;
    type VaultStaking = Staking;
    type OnSweep = ();
    type MaxExpectedValue = MaxExpectedValue;
}

parameter_types! {
    pub const VaultPalletId: PalletId = PalletId(*b"mod/vreg");
}

impl Config for Test {
    type PalletId = VaultPalletId;
    type Event = TestEvent;
    type Balance = Balance;
    type WeightInfo = ();
    type GetGriefingCollateralCurrencyId = GetNativeCurrencyId;
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

impl staking::Config for Test {
    type Event = TestEvent;
    type SignedFixedPoint = SignedFixedPoint;
    type SignedInner = SignedInner;
    type CurrencyId = CurrencyId;
    type GetNativeCurrencyId = GetNativeCurrencyId;
}

pub type TestEvent = Event;
pub type TestError = Error<Test>;
pub type TokensError = orml_tokens::Error<Test>;

pub struct ExtBuilder;

pub const DEFAULT_ID: VaultId<AccountId, CurrencyId> = VaultId {
    account_id: 3,
    currencies: VaultCurrencyPair {
        collateral: DEFAULT_COLLATERAL_CURRENCY,
        wrapped: DEFAULT_WRAPPED_CURRENCY,
    },
};
pub const OTHER_ID: VaultId<AccountId, CurrencyId> = VaultId {
    account_id: 4,
    currencies: VaultCurrencyPair {
        collateral: DEFAULT_COLLATERAL_CURRENCY,
        wrapped: DEFAULT_WRAPPED_CURRENCY,
    },
};
pub const RICH_ID: VaultId<AccountId, CurrencyId> = VaultId {
    account_id: 5,
    currencies: VaultCurrencyPair {
        collateral: DEFAULT_COLLATERAL_CURRENCY,
        wrapped: DEFAULT_WRAPPED_CURRENCY,
    },
};
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
            minimum_collateral_vault: vec![(DEFAULT_COLLATERAL_CURRENCY, 0)],
            punishment_delay: 0,
            system_collateral_ceiling: vec![(DEFAULT_CURRENCY_PAIR, 1_000_000_000_000)],
            secure_collateral_threshold: vec![(DEFAULT_CURRENCY_PAIR, UnsignedFixedPoint::one())],
            premium_redeem_threshold: vec![(DEFAULT_CURRENCY_PAIR, UnsignedFixedPoint::one())],
            liquidation_collateral_threshold: vec![(DEFAULT_CURRENCY_PAIR, UnsignedFixedPoint::one())],
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        sp_io::TestExternalities::from(storage)
    }
    pub fn build() -> sp_io::TestExternalities {
        ExtBuilder::build_with(orml_tokens::GenesisConfig::<Test> {
            balances: vec![
                (DEFAULT_ID.account_id, DEFAULT_COLLATERAL_CURRENCY, DEFAULT_COLLATERAL),
                (OTHER_ID.account_id, DEFAULT_COLLATERAL_CURRENCY, DEFAULT_COLLATERAL),
                (RICH_ID.account_id, DEFAULT_COLLATERAL_CURRENCY, RICH_COLLATERAL),
                (
                    MULTI_VAULT_TEST_IDS[0],
                    DEFAULT_COLLATERAL_CURRENCY,
                    MULTI_VAULT_TEST_COLLATERAL,
                ),
                (
                    MULTI_VAULT_TEST_IDS[1],
                    DEFAULT_COLLATERAL_CURRENCY,
                    MULTI_VAULT_TEST_COLLATERAL,
                ),
                (
                    MULTI_VAULT_TEST_IDS[2],
                    DEFAULT_COLLATERAL_CURRENCY,
                    MULTI_VAULT_TEST_COLLATERAL,
                ),
                (
                    MULTI_VAULT_TEST_IDS[3],
                    DEFAULT_COLLATERAL_CURRENCY,
                    MULTI_VAULT_TEST_COLLATERAL,
                ),
            ],
        })
    }
}

pub(crate) fn set_default_thresholds() {
    let secure = UnsignedFixedPoint::checked_from_rational(200, 100).unwrap(); // 200%
    let premium = UnsignedFixedPoint::checked_from_rational(120, 100).unwrap(); // 120%
    let liquidation = UnsignedFixedPoint::checked_from_rational(110, 100).unwrap(); // 110%

    VaultRegistry::_set_secure_collateral_threshold(DEFAULT_CURRENCY_PAIR, secure);
    VaultRegistry::_set_premium_redeem_threshold(DEFAULT_CURRENCY_PAIR, premium);
    VaultRegistry::_set_liquidation_collateral_threshold(DEFAULT_CURRENCY_PAIR, liquidation);
}

pub fn run_test<T>(test: T)
where
    T: FnOnce(),
{
    clear_mocks();
    ExtBuilder::build().execute_with(|| {
        System::set_block_number(1);
        Security::set_active_block_number(1);
        set_default_thresholds();
        <oracle::Pallet<Test>>::_set_exchange_rate(DEFAULT_COLLATERAL_CURRENCY, UnsignedFixedPoint::one()).unwrap();
        test()
    })
}
