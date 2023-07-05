use crate as vault_registry;
use crate::{Config, Error};
use currency::CurrencyConversion;
use frame_support::{
    parameter_types,
    traits::{ConstU32, Everything, GenesisBuild},
    PalletId,
};
use frame_system::EnsureRoot;
use mocktopus::{macros::mockable, mocking::clear_mocks};
use orml_traits::parameter_type_with_key;
pub use primitives::{CurrencyId, CurrencyId::Token, TokenSymbol::*};
use primitives::{Rate, VaultCurrencyPair, VaultId};
use sp_arithmetic::{FixedI128, FixedPointNumber, FixedU128};
use sp_core::H256;
use sp_runtime::{
    testing::{Header, TestXt},
    traits::{BlakeTwo256, IdentityLookup, One, Zero},
    DispatchError,
};

pub(crate) type Extrinsic = TestXt<RuntimeCall, ()>;
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

        CapacityRewards: reward::<Instance1>::{Pallet, Call, Storage, Event<T>},
        VaultRewards: reward::<Instance2>::{Pallet, Call, Storage, Event<T>},
        VaultStaking: staking::{Pallet, Storage, Event<T>},

        // Operational
        Security: security::{Pallet, Call, Storage, Event<T>},
        VaultRegistry: vault_registry::{Pallet, Call, Config<T>, Storage, Event<T>, ValidateUnsigned},
        Oracle: oracle::{Pallet, Call, Config<T>, Storage, Event<T>},
        Fee: fee::{Pallet, Call, Config<T>, Storage},
        Currency: currency::{Pallet},
        Loans: loans::{Pallet, Storage, Call, Event<T>, Config},
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
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Index = Index;
    type BlockNumber = BlockNumber;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type RuntimeEvent = RuntimeEvent;
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
pub const WORST_CASE_COLLATERAL_CURRENCY: CurrencyId = CurrencyId::LendToken(1);
pub const DEFAULT_NATIVE_CURRENCY: CurrencyId = Token(INTR);
pub const DEFAULT_WRAPPED_CURRENCY: CurrencyId = Token(IBTC);

pub const DEFAULT_CURRENCY_PAIR: VaultCurrencyPair<CurrencyId> = VaultCurrencyPair {
    collateral: DEFAULT_COLLATERAL_CURRENCY,
    wrapped: DEFAULT_WRAPPED_CURRENCY,
};

pub const WORST_CASE_CURRENCY_PAIR: VaultCurrencyPair<CurrencyId> = VaultCurrencyPair {
    collateral: WORST_CASE_COLLATERAL_CURRENCY,
    wrapped: DEFAULT_WRAPPED_CURRENCY,
};

pub const DEFAULT_MAX_EXCHANGE_RATE: u128 = 1_000_000_000_000_000_000; // 1
pub const DEFAULT_MIN_EXCHANGE_RATE: u128 = 20_000_000_000_000_000; // 0.02

pub fn vault_id(account_id: AccountId) -> VaultId<AccountId, CurrencyId> {
    VaultId {
        account_id,
        currencies: VaultCurrencyPair {
            collateral: DEFAULT_COLLATERAL_CURRENCY,
            wrapped: DEFAULT_WRAPPED_CURRENCY,
        },
    }
}

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
    type RuntimeEvent = RuntimeEvent;
    type Balance = Balance;
    type Amount = RawAmount;
    type CurrencyId = CurrencyId;
    type WeightInfo = ();
    type ExistentialDeposits = ExistentialDeposits;
    type CurrencyHooks = ();
    type MaxLocks = MaxLocks;
    type DustRemovalWhitelist = Everything;
    type MaxReserves = ConstU32<0>; // we don't use named reserves
    type ReserveIdentifier = (); // we don't use named reserves
}

pub(crate) type CapacityRewardsInstance = reward::Instance1;

impl reward::Config<CapacityRewardsInstance> for Test {
    type RuntimeEvent = RuntimeEvent;
    type SignedFixedPoint = SignedFixedPoint;
    type PoolId = ();
    type StakeId = CurrencyId;
    type CurrencyId = CurrencyId;
    type MaxRewardCurrencies = ConstU32<10>;
}

pub(crate) type VaultRewardsInstance = reward::Instance2;

impl reward::Config<VaultRewardsInstance> for Test {
    type RuntimeEvent = RuntimeEvent;
    type SignedFixedPoint = SignedFixedPoint;
    type PoolId = CurrencyId;
    type StakeId = VaultId<AccountId, CurrencyId>;
    type CurrencyId = CurrencyId;
    type MaxRewardCurrencies = ConstU32<10>;
}

impl staking::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type SignedFixedPoint = SignedFixedPoint;
    type SignedInner = SignedInner;
    type CurrencyId = CurrencyId;
    type GetNativeCurrencyId = GetNativeCurrencyId;
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
    type RuntimeEvent = RuntimeEvent;
    type OnExchangeRateChange = vault_registry::PoolManager<Test>;
    type WeightInfo = ();
    type MaxNameLength = ConstU32<255>;
}

parameter_types! {
    pub const LoansPalletId: PalletId = PalletId(*b"par/loan");
}

impl loans::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type PalletId = LoansPalletId;
    type ReserveOrigin = EnsureRoot<AccountId>;
    type UpdateOrigin = EnsureRoot<AccountId>;
    type WeightInfo = ();
    type UnixTime = Timestamp;
    type RewardAssetId = GetNativeCurrencyId;
    type ReferenceAssetId = GetWrappedCurrencyId;
    type OnExchangeRateChange = ();
}

#[cfg_attr(test, mockable)]
pub fn convert_to(
    to: CurrencyId,
    amount: currency::Amount<Test>,
) -> Result<currency::Amount<Test>, sp_runtime::DispatchError> {
    currency::CurrencyConvert::<Test, Oracle, Loans>::convert(&amount, to)
}

impl currency::Config for Test {
    type SignedInner = SignedInner;
    type SignedFixedPoint = SignedFixedPoint;
    type UnsignedFixedPoint = UnsignedFixedPoint;
    type Balance = Balance;
    type GetNativeCurrencyId = GetNativeCurrencyId;
    type GetRelayChainCurrencyId = GetCollateralCurrencyId;
    type GetWrappedCurrencyId = GetWrappedCurrencyId;
    type CurrencyConversion = currency::CurrencyConvert<Test, Oracle, Loans>;
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
    type CapacityRewards = CapacityRewards;
    type VaultRewards = VaultRewards;
    type VaultStaking = VaultStaking;
    type OnSweep = ();
    type MaxExpectedValue = MaxExpectedValue;
    type NominationApi = MockDeposit;
}

parameter_types! {
    pub const VaultPalletId: PalletId = PalletId(*b"mod/vreg");
}

pub struct MockDeposit;

impl traits::NominationApi<VaultId<AccountId, CurrencyId>, currency::Amount<Test>> for MockDeposit {
    fn deposit_vault_collateral(
        vault_id: &VaultId<AccountId, CurrencyId>,
        amount: &currency::Amount<Test>,
    ) -> Result<(), DispatchError> {
        // ensure the vault is active
        let _vault = VaultRegistry::get_active_rich_vault_from_id(vault_id)?;

        // Deposit `amount` of stake into the vault staking pool
        <vault_registry::PoolManager<Test>>::deposit_collateral(vault_id, &vault_id.account_id, amount)?;
        amount.lock_on(&vault_id.account_id)?;
        VaultRegistry::try_increase_total_backing_collateral(&vault_id.currencies, &amount)?;

        Ok(())
    }
    fn ensure_opted_in_to_nomination(vault_id: &VaultId<AccountId, CurrencyId>) -> Result<(), DispatchError> {
        Ok(())
    }
    #[cfg(any(feature = "runtime-benchmarks", test))]
    fn opt_in_to_nomination(_vault_id: &VaultId<AccountId, CurrencyId>) {}
}

impl Config for Test {
    type PalletId = VaultPalletId;
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type GetGriefingCollateralCurrencyId = GetNativeCurrencyId;
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Test
where
    RuntimeCall: From<C>,
{
    type OverarchingCall = RuntimeCall;
    type Extrinsic = Extrinsic;
}

impl security::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type MaxErrors = ConstU32<1>;
}

pub type TestEvent = RuntimeEvent;
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

        GenesisBuild::<Test>::assimilate_storage(
            &loans::GenesisConfig {
                max_exchange_rate: Rate::from_inner(DEFAULT_MAX_EXCHANGE_RATE),
                min_exchange_rate: Rate::from_inner(DEFAULT_MIN_EXCHANGE_RATE),
            },
            &mut storage,
        )
        .unwrap();

        // Parameters to be set in tests
        vault_registry::GenesisConfig::<Test> {
            minimum_collateral_vault: vec![(DEFAULT_COLLATERAL_CURRENCY, 0), (WORST_CASE_COLLATERAL_CURRENCY, 0)],
            punishment_delay: 0,
            system_collateral_ceiling: vec![
                (DEFAULT_CURRENCY_PAIR, 1_000_000_000_000),
                (WORST_CASE_CURRENCY_PAIR, 1_000_000_000_000),
            ],
            secure_collateral_threshold: vec![
                (DEFAULT_CURRENCY_PAIR, UnsignedFixedPoint::one()),
                (WORST_CASE_CURRENCY_PAIR, UnsignedFixedPoint::one()),
            ],
            premium_redeem_threshold: vec![
                (DEFAULT_CURRENCY_PAIR, UnsignedFixedPoint::one()),
                (WORST_CASE_CURRENCY_PAIR, UnsignedFixedPoint::one()),
            ],
            liquidation_collateral_threshold: vec![
                (DEFAULT_CURRENCY_PAIR, UnsignedFixedPoint::one()),
                (WORST_CASE_CURRENCY_PAIR, UnsignedFixedPoint::one()),
            ],
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
