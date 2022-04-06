use crate as replace;
use crate::{Config, Error};
use currency::Amount;
use frame_support::{
    assert_ok, parameter_types,
    traits::{Everything, GenesisBuild},
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
        Tokens: orml_tokens::{Pallet, Storage, Config<T>, Event<T>},

        Rewards: reward::{Pallet, Call, Storage, Event<T>},

        // Operational
        BTCRelay: btc_relay::{Pallet, Call, Config<T>, Storage, Event<T>},
        Security: security::{Pallet, Call, Storage, Event<T>},
        VaultRegistry: vault_registry::{Pallet, Call, Config<T>, Storage, Event<T>},
        Oracle: oracle::{Pallet, Call, Config<T>, Storage, Event<T>},
        Replace: replace::{Pallet, Call, Config<T>, Storage, Event<T>},
        Fee: fee::{Pallet, Call, Config<T>, Storage},
        Nomination: nomination::{Pallet, Call, Config, Storage, Event<T>},
        Staking: staking::{Pallet, Storage, Event<T>},
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
pub type UnsignedInner = u128;

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
    pub const VaultPalletId: PalletId = PalletId(*b"mod/vreg");
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Test
where
    Call: From<C>,
{
    type OverarchingCall = Call;
    type Extrinsic = TestExtrinsic;
}
pub struct CurrencyConvert;
impl currency::CurrencyConversion<currency::Amount<Test>, CurrencyId> for CurrencyConvert {
    fn convert(
        amount: &currency::Amount<Test>,
        to: CurrencyId,
    ) -> Result<currency::Amount<Test>, sp_runtime::DispatchError> {
        let amount = convert_to(to, amount.amount())?;
        Ok(Amount::new(amount, to))
    }
}

#[cfg_attr(test, mockable)]
pub fn convert_to(to: CurrencyId, amount: Balance) -> Result<Balance, sp_runtime::DispatchError> {
    Ok(amount) // default conversion 1:1 - overwritable with mocktopus
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

impl vault_registry::Config for Test {
    type PalletId = VaultPalletId;
    type Event = TestEvent;
    type Balance = Balance;
    type WeightInfo = ();
    type GetGriefingCollateralCurrencyId = GetNativeCurrencyId;
}

impl staking::Config for Test {
    type Event = TestEvent;
    type SignedFixedPoint = SignedFixedPoint;
    type SignedInner = SignedInner;
    type CurrencyId = CurrencyId;
    type GetNativeCurrencyId = GetNativeCurrencyId;
}

parameter_types! {
    pub const ParachainBlocksPerBitcoinBlock: BlockNumber = 100;
}

impl btc_relay::Config for Test {
    type Event = TestEvent;
    type ParachainBlocksPerBitcoinBlock = ParachainBlocksPerBitcoinBlock;
    type WeightInfo = ();
}

impl security::Config for Test {
    type Event = TestEvent;
}

impl nomination::Config for Test {
    type Event = TestEvent;
    type WeightInfo = ();
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

parameter_types! {
    pub const FeePalletId: PalletId = PalletId(*b"mod/fees");
}

impl fee::Config for Test {
    type FeePalletId = FeePalletId;
    type WeightInfo = ();
    type SignedFixedPoint = SignedFixedPoint;
    type SignedInner = SignedInner;
    type UnsignedFixedPoint = UnsignedFixedPoint;
    type UnsignedInner = UnsignedInner;
    type VaultRewards = Rewards;
    type VaultStaking = Staking;
    type OnSweep = ();
}

impl Config for Test {
    type Event = TestEvent;
    type WeightInfo = ();
}

pub type TestEvent = Event;
pub type TestError = Error<Test>;

pub const OLD_VAULT: VaultId<AccountId, CurrencyId> = VaultId {
    account_id: 1,
    currencies: VaultCurrencyPair {
        collateral: DEFAULT_COLLATERAL_CURRENCY,
        wrapped: DEFAULT_WRAPPED_CURRENCY,
    },
};
pub const NEW_VAULT: VaultId<AccountId, CurrencyId> = VaultId {
    account_id: 2,
    currencies: VaultCurrencyPair {
        collateral: DEFAULT_COLLATERAL_CURRENCY,
        wrapped: DEFAULT_WRAPPED_CURRENCY,
    },
};

pub const OLD_VAULT_BALANCE: u128 = 1_000_000;
pub const NEW_VAULT_BALANCE: u128 = 1_000_000;

pub struct ExtBuilder;

impl ExtBuilder {
    pub fn build_with(balances: orml_tokens::GenesisConfig<Test>) -> sp_io::TestExternalities {
        let mut storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

        balances.assimilate_storage(&mut storage).unwrap();

        fee::GenesisConfig::<Test> {
            issue_fee: UnsignedFixedPoint::checked_from_rational(5, 1000).unwrap(), // 0.5%
            issue_griefing_collateral: UnsignedFixedPoint::checked_from_rational(5, 100000).unwrap(), // 0.005%
            refund_fee: UnsignedFixedPoint::checked_from_rational(5, 1000).unwrap(), // 0.5%
            redeem_fee: UnsignedFixedPoint::checked_from_rational(5, 1000).unwrap(), // 0.5%
            premium_redeem_fee: UnsignedFixedPoint::checked_from_rational(5, 100).unwrap(), // 5%
            punishment_fee: UnsignedFixedPoint::checked_from_rational(1, 10).unwrap(), // 10%
            replace_griefing_collateral: UnsignedFixedPoint::checked_from_rational(1, 10).unwrap(), // 10%
            theft_fee: UnsignedFixedPoint::checked_from_rational(5, 100).unwrap(),  // 5%
            theft_fee_max: 10000000,                                                // 0.1 BTC
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
        ExtBuilder::build_with(orml_tokens::GenesisConfig::<Test> {
            balances: vec![
                (OLD_VAULT.account_id, Token(DOT), OLD_VAULT_BALANCE),
                (NEW_VAULT.account_id, Token(DOT), NEW_VAULT_BALANCE),
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
        assert_ok!(<oracle::Pallet<Test>>::_set_exchange_rate(
            DEFAULT_COLLATERAL_CURRENCY,
            UnsignedFixedPoint::one()
        ));
        System::set_block_number(1);
        Security::set_active_block_number(1);
        test();
    });
}
