use crate::{self as annuity, BlockRewardProvider, Config};
use frame_support::{parameter_types, traits::Everything, PalletId};
pub use primitives::CurrencyId;
use sp_core::H256;
use sp_runtime::{
    generic::Header as GenericHeader,
    traits::{BlakeTwo256, Identity, IdentityLookup},
    DispatchError, DispatchResult,
};

type Header = GenericHeader<BlockNumber, BlakeTwo256>;

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
        Balances: pallet_balances::{Pallet, Call, Storage, Event<T>},
        Annuity: annuity::{Pallet, Call, Storage, Event<T>},
    }
);

pub type AccountId = u64;
pub type Balance = u128;
pub type BlockNumber = u128;
pub type Index = u64;

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
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
    pub const ExistentialDeposit: u64 = 1;
}

impl pallet_balances::Config for Test {
    type Balance = Balance;
    type DustRemoval = ();
    type Event = Event;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = ();
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
}

pub const TOTAL_REWARDS: Balance = 10_000_000;
const VAULT_REWARDS: Balance = TOTAL_REWARDS / 100 * 30;

pub const YEAR_1_REWARDS: Balance = VAULT_REWARDS / 100 * 40;
pub const YEAR_2_REWARDS: Balance = VAULT_REWARDS / 100 * 30;
pub const YEAR_3_REWARDS: Balance = VAULT_REWARDS / 100 * 20;
pub const YEAR_4_REWARDS: Balance = VAULT_REWARDS / 100 * 10;

pub struct MockBlockRewardProvider;

impl BlockRewardProvider<AccountId> for MockBlockRewardProvider {
    type Currency = Balances;
    fn deposit_stake(_: &AccountId, _: Balance) -> DispatchResult {
        Ok(())
    }
    fn distribute_block_reward(_: &AccountId, _: Balance) -> DispatchResult {
        Ok(())
    }
    fn withdraw_reward(_: &AccountId) -> Result<Balance, DispatchError> {
        Ok(0)
    }
}

parameter_types! {
    pub const AnnuityPalletId: PalletId = PalletId(*b"mod/annu");
    pub const EmissionPeriod: BlockNumber = 100;
}

impl Config for Test {
    type AnnuityPalletId = AnnuityPalletId;
    type Event = TestEvent;
    type Currency = Balances;
    type BlockRewardProvider = MockBlockRewardProvider;
    type BlockNumberToBalance = Identity;
    type EmissionPeriod = EmissionPeriod;
    type WeightInfo = ();
}

pub type TestEvent = Event;
// pub type TestError = Error<Test>;

// pub const ALICE: AccountId = 1;
// pub const BOB: AccountId = 2;

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
