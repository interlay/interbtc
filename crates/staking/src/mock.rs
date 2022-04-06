use crate as staking;
use crate::{Config, Error};
use frame_support::{parameter_types, traits::Everything};
pub use primitives::{CurrencyId, CurrencyId::Token, TokenSymbol::*};
use primitives::{VaultCurrencyPair, VaultId};
use sp_arithmetic::FixedI128;
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
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
        Staking: staking::{Pallet, Call, Storage, Event<T>},
    }
);

pub type AccountId = u64;
pub type BlockNumber = u64;
pub type Index = u64;
pub type SignedFixedPoint = FixedI128;
pub type SignedInner = i128;

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
}

impl Config for Test {
    type Event = TestEvent;
    type SignedInner = SignedInner;
    type SignedFixedPoint = SignedFixedPoint;
    type CurrencyId = CurrencyId;
    type GetNativeCurrencyId = GetNativeCurrencyId;
}

pub type TestEvent = Event;
pub type TestError = Error<Test>;

pub const VAULT: VaultId<AccountId, CurrencyId> = VaultId {
    account_id: 1,
    currencies: VaultCurrencyPair {
        collateral: Token(DOT),
        wrapped: Token(IBTC),
    },
};
pub const ALICE: VaultId<AccountId, CurrencyId> = VaultId {
    account_id: 2,
    currencies: VaultCurrencyPair {
        collateral: Token(DOT),
        wrapped: Token(IBTC),
    },
};
pub const BOB: VaultId<AccountId, CurrencyId> = VaultId {
    account_id: 3,
    currencies: VaultCurrencyPair {
        collateral: Token(DOT),
        wrapped: Token(IBTC),
    },
};

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
    ExtBuilder::build().execute_with(|| {
        System::set_block_number(1);
        test();
    });
}
