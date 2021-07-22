use crate as relay;
use crate::{Config, Error};
use codec::{Decode, Encode};
use frame_support::{parameter_types, traits::GenesisBuild, PalletId};
use mocktopus::mocking::clear_mocks;
use orml_tokens::CurrencyAdapter;
use orml_traits::parameter_type_with_key;
use sp_arithmetic::{FixedI128, FixedPointNumber, FixedU128};
use sp_core::H256;
use sp_runtime::{
    testing::{Header, TestXt},
    traits::{BlakeTwo256, IdentityLookup, Zero},
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
        RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Pallet, Storage},

        // Tokens & Balances
        Tokens: orml_tokens::{Pallet, Storage, Config<T>, Event<T>},

        Rewards: reward::{Pallet, Call, Storage, Event<T>},

        // Operational
        BTCRelay: btc_relay::{Pallet, Call, Config<T>, Storage, Event<T>},
        Security: security::{Pallet, Call, Storage, Event<T>},
        Relay: relay::{Pallet, Call, Storage, Event<T>},
        VaultRegistry: vault_registry::{Pallet, Call, Config<T>, Storage, Event<T>},
        ExchangeRateOracle: exchange_rate_oracle::{Pallet, Call, Config<T>, Storage, Event<T>},
        Redeem: redeem::{Pallet, Call, Config<T>, Storage, Event<T>},
        Replace: replace::{Pallet, Call, Config<T>, Storage, Event<T>},
        Fee: fee::{Pallet, Call, Config<T>, Storage},
        Refund: refund::{Pallet, Call, Config<T>, Storage, Event<T>},
        Nomination: nomination::{Pallet, Call, Config, Storage, Event<T>},
        Staking: staking::{Pallet, Storage, Event<T>},
    }
);

pub type AccountId = u64;
pub type Balance = u128;
pub type Amount = i128;
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
    type BaseCallFilter = ();
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
}

impl pallet_randomness_collective_flip::Config for Test {}

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

impl reward::Config for Test {
    type Event = TestEvent;
    type SignedFixedPoint = SignedFixedPoint;
    type CurrencyId = CurrencyId;
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
    type SignedInner = SignedInner;
    type Balance = Balance;
    type SignedFixedPoint = SignedFixedPoint;
    type UnsignedFixedPoint = UnsignedFixedPoint;
    type WeightInfo = ();
    type VaultRewards = reward::RewardsCurrencyAdapter<Test, (), GetWrappedCurrencyId>;
    type Collateral = CurrencyAdapter<Test, GetCollateralCurrencyId>;
    type Wrapped = CurrencyAdapter<Test, GetWrappedCurrencyId>;
    type GetRewardsCurrencyId = GetWrappedCurrencyId;
}

impl staking::Config for Test {
    type Event = TestEvent;
    type SignedFixedPoint = SignedFixedPoint;
    type SignedInner = SignedInner;
    type CurrencyId = CurrencyId;
}

impl exchange_rate_oracle::Config for Test {
    type Event = TestEvent;
    type Balance = Balance;
    type UnsignedFixedPoint = UnsignedFixedPoint;
    type WeightInfo = ();
}

parameter_types! {
    pub const FeePalletId: PalletId = PalletId(*b"mod/fees");
}

impl fee::Config for Test {
    type PalletId = FeePalletId;
    type WeightInfo = ();
    type SignedFixedPoint = SignedFixedPoint;
    type SignedInner = SignedInner;
    type UnsignedFixedPoint = UnsignedFixedPoint;
    type UnsignedInner = UnsignedInner;
    type VaultRewards = reward::RewardsCurrencyAdapter<Test, (), GetWrappedCurrencyId>;
    type VaultStaking = staking::StakingCurrencyAdapter<Test, GetWrappedCurrencyId>;
    type Wrapped = CurrencyAdapter<Test, GetWrappedCurrencyId>;
    type OnSweep = ();
}

impl refund::Config for Test {
    type Event = TestEvent;
    type WeightInfo = ();
}

parameter_types! {
    pub const ParachainBlocksPerBitcoinBlock: BlockNumber = 100;
}

impl btc_relay::Config for Test {
    type Event = TestEvent;
    type ParachainBlocksPerBitcoinBlock = ParachainBlocksPerBitcoinBlock;
    type WeightInfo = ();
}

impl redeem::Config for Test {
    type Event = TestEvent;
    type WeightInfo = ();
}

impl replace::Config for Test {
    type Event = TestEvent;
    type WeightInfo = ();
}

impl nomination::Config for Test {
    type Event = TestEvent;
    type WeightInfo = ();
    type VaultRewards = reward::RewardsCurrencyAdapter<Test, (), GetWrappedCurrencyId>;
}

impl Config for Test {
    type Event = TestEvent;
    type WeightInfo = ();
}

pub type TestEvent = Event;
pub type TestError = Error<Test>;
pub type RedeemError = redeem::Error<Test>;

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
pub const CAROL: AccountId = 3;
pub const DAVE: AccountId = 4;
pub const EVE: AccountId = 5;

pub const ALICE_BALANCE: u128 = 1_000_000;
pub const BOB_BALANCE: u128 = 1_000_000;
pub const CAROL_BALANCE: u128 = 1_000_000;
pub const DAVE_BALANCE: u128 = 1_000_000;
pub const EVE_BALANCE: u128 = 1_000_000;

pub struct ExtBuilder;

impl ExtBuilder {
    pub fn build_with<F>(conf: F) -> sp_io::TestExternalities
    where
        F: FnOnce(&mut sp_core::storage::Storage),
    {
        let mut storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

        fee::GenesisConfig::<Test> {
            issue_fee: UnsignedFixedPoint::checked_from_rational(5, 1000).unwrap(), // 0.5%
            issue_griefing_collateral: UnsignedFixedPoint::checked_from_rational(5, 100000).unwrap(), // 0.005%
            refund_fee: UnsignedFixedPoint::checked_from_rational(5, 1000).unwrap(), // 0.5%
            redeem_fee: UnsignedFixedPoint::checked_from_rational(5, 1000).unwrap(), // 0.5%
            premium_redeem_fee: UnsignedFixedPoint::checked_from_rational(5, 100).unwrap(), // 5%
            punishment_fee: UnsignedFixedPoint::checked_from_rational(1, 10).unwrap(), // 10%
            replace_griefing_collateral: UnsignedFixedPoint::checked_from_rational(1, 10).unwrap(), // 10%
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        conf(&mut storage);

        storage.into()
    }

    pub fn build() -> sp_io::TestExternalities {
        ExtBuilder::build_with(|storage| {
            orml_tokens::GenesisConfig::<Test> {
                balances: vec![
                    (ALICE, DOT, ALICE_BALANCE),
                    (BOB, DOT, BOB_BALANCE),
                    (CAROL, DOT, CAROL_BALANCE),
                    (DAVE, DOT, DAVE_BALANCE),
                    (EVE, DOT, EVE_BALANCE),
                ],
            }
            .assimilate_storage(storage)
            .unwrap();
        })
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
