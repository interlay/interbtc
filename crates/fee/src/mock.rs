use crate as fee;
use crate::{Config, Error};
use frame_support::{parameter_types, traits::Everything, PalletId};
use mocktopus::mocking::clear_mocks;
use orml_traits::parameter_type_with_key;
use primitives::VaultId;
pub use primitives::{CurrencyId, CurrencyId::Token, TokenSymbol::*};
use sp_arithmetic::{FixedI128, FixedU128};
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup, Zero},
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
        Tokens: orml_tokens::{Pallet, Storage, Config<T>, Event<T>},

        Rewards: reward::{Pallet, Call, Storage, Event<T>},
        Staking: staking::{Pallet, Storage, Event<T>},

        // Operational
        Security: security::{Pallet, Call, Storage, Event<T>},
        Fee: fee::{Pallet, Call, Config<T>, Storage},
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

parameter_types! {
    pub const GetNativeCurrencyId: CurrencyId = Token(INTR);
    pub const GetRelayChainCurrencyId: CurrencyId = Token(DOT);
    pub const GetWrappedCurrencyId: CurrencyId = Token(IBTC);
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

impl staking::Config for Test {
    type Event = TestEvent;
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

impl security::Config for Test {
    type Event = TestEvent;
}

pub struct CurrencyConvert;
impl currency::CurrencyConversion<currency::Amount<Test>, CurrencyId> for CurrencyConvert {
    fn convert(
        _amount: &currency::Amount<Test>,
        _to: CurrencyId,
    ) -> Result<currency::Amount<Test>, sp_runtime::DispatchError> {
        unimplemented!()
    }
}

impl currency::Config for Test {
    type SignedInner = SignedInner;
    type SignedFixedPoint = SignedFixedPoint;
    type UnsignedFixedPoint = UnsignedFixedPoint;
    type Balance = Balance;
    type GetNativeCurrencyId = GetNativeCurrencyId;
    type GetRelayChainCurrencyId = GetRelayChainCurrencyId;
    type GetWrappedCurrencyId = GetWrappedCurrencyId;
    type CurrencyConversion = CurrencyConvert;
}

parameter_types! {
    pub const FeePalletId: PalletId = PalletId(*b"mod/fees");
}

impl Config for Test {
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
