//! The crate's tests.

use super::*;
use crate as pallet_democracy;
use codec::Encode;
use frame_support::{
    assert_noop, assert_ok, ord_parameter_types, parameter_types,
    traits::{Contains, EqualPrivilegeOnly, GenesisBuild, OnInitialize, SortedMembers},
    weights::Weight,
};
use frame_system::{EnsureRoot, EnsureSignedBy};
use pallet_balances::Error as BalancesError;
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BadOrigin, BlakeTwo256, IdentityLookup},
    Perbill,
};

mod decoders;
mod fast_tracking;
mod preimage;
mod public_proposals;
mod scheduling;
mod voting;

const MAX_PROPOSALS: u32 = 100;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        Scheduler: pallet_scheduler::{Pallet, Call, Storage, Event<T>},
        Democracy: pallet_democracy::{Pallet, Call, Storage, Config<T>, Event<T>},
    }
);

// Test that a fitlered call can be dispatched.
pub struct BaseFilter;
impl Contains<RuntimeCall> for BaseFilter {
    fn contains(call: &RuntimeCall) -> bool {
        !matches!(call, &RuntimeCall::Balances(pallet_balances::Call::set_balance { .. }))
    }
}

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub BlockWeights: frame_system::limits::BlockWeights =
        frame_system::limits::BlockWeights::simple_max(Weight::from_ref_time(1_000_000 as u64));
}
impl frame_system::Config for Test {
    type BaseCallFilter = BaseFilter;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type RuntimeOrigin = RuntimeOrigin;
    type Index = u64;
    type BlockNumber = u64;
    type RuntimeCall = RuntimeCall;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}
parameter_types! {
    pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) * BlockWeights::get().max_block;
}

impl pallet_scheduler::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeOrigin = RuntimeOrigin;
    type PalletsOrigin = OriginCaller;
    type RuntimeCall = RuntimeCall;
    type MaximumWeight = MaximumSchedulerWeight;
    type ScheduleOrigin = EnsureRoot<u64>;
    type MaxScheduledPerBlock = ();
    type OriginPrivilegeCmp = EqualPrivilegeOnly;
    type WeightInfo = ();
    type Preimages = ();
}

parameter_types! {
    pub const ExistentialDeposit: u64 = 1;
    pub const MaxLocks: u32 = 10;
}
impl pallet_balances::Config for Test {
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type MaxLocks = MaxLocks;
    type Balance = u64;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
}
parameter_types! {
    pub const LaunchPeriod: u64 = 2;
    pub const VotingPeriod: u64 = 2;
    pub const FastTrackVotingPeriod: u64 = 2;
    pub const MinimumDeposit: u64 = 1;
    pub const EnactmentPeriod: u64 = 2;
    pub const MaxVotes: u32 = 100;
    pub const MaxProposals: u32 = MAX_PROPOSALS;
    pub static PreimageByteDeposit: u64 = 0;
}
ord_parameter_types! {
    pub const One: u64 = 1;
    pub const Two: u64 = 2;
    pub const Three: u64 = 3;
    pub const Four: u64 = 4;
    pub const Five: u64 = 5;
    pub const Six: u64 = 6;
}
pub struct OneToFive;
impl SortedMembers<u64> for OneToFive {
    fn sorted_members() -> Vec<u64> {
        vec![1, 2, 3, 4, 5]
    }
    #[cfg(feature = "runtime-benchmarks")]
    fn add(_m: &u64) {}
}

impl Config for Test {
    type Proposal = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    type Currency = pallet_balances::Pallet<Self>;
    type EnactmentPeriod = EnactmentPeriod;
    type LaunchPeriod = LaunchPeriod;
    type VotingPeriod = VotingPeriod;
    type FastTrackVotingPeriod = FastTrackVotingPeriod;
    type MinimumDeposit = MinimumDeposit;
    type FastTrackOrigin = EnsureSignedBy<Five, u64>;
    type PreimageByteDeposit = PreimageByteDeposit;
    type Slash = ();
    type Scheduler = Scheduler;
    type MaxVotes = MaxVotes;
    type PalletsOrigin = OriginCaller;
    type WeightInfo = ();
    type MaxProposals = MaxProposals;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
    pallet_balances::GenesisConfig::<Test> {
        balances: vec![(1, 10), (2, 20), (3, 30), (4, 40), (5, 50), (6, 60)],
    }
    .assimilate_storage(&mut t)
    .unwrap();
    pallet_democracy::GenesisConfig::<Test>::default()
        .assimilate_storage(&mut t)
        .unwrap();
    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

#[test]
fn params_should_work() {
    new_test_ext().execute_with(|| {
        assert_eq!(Democracy::referendum_count(), 0);
        assert_eq!(Balances::free_balance(42), 0);
        assert_eq!(Balances::total_issuance(), 210);
    });
}

fn set_balance_proposal(value: u64) -> Vec<u8> {
    RuntimeCall::Balances(pallet_balances::Call::set_balance {
        who: 42,
        new_free: value,
        new_reserved: 0,
    })
    .encode()
}

#[test]
fn set_balance_proposal_is_correctly_filtered_out() {
    for i in 0..10 {
        let call = RuntimeCall::decode(&mut &set_balance_proposal(i)[..]).unwrap();
        assert!(!<Test as frame_system::Config>::BaseCallFilter::contains(&call));
    }
}

fn set_balance_proposal_hash(value: u64) -> H256 {
    BlakeTwo256::hash(&set_balance_proposal(value)[..])
}

fn set_balance_proposal_hash_and_note(value: u64) -> H256 {
    let p = set_balance_proposal(value);
    let h = BlakeTwo256::hash(&p[..]);
    match Democracy::note_preimage(RuntimeOrigin::signed(6), p) {
        Ok(_) => (),
        Err(x) if x == Error::<Test>::DuplicatePreimage.into() => (),
        Err(x) => panic!("{:?}", x),
    }
    h
}

fn propose_set_balance(who: u64, value: u64, delay: u64) -> DispatchResult {
    Democracy::propose(RuntimeOrigin::signed(who), set_balance_proposal_hash(value), delay)
}

fn propose_set_balance_and_note(who: u64, value: u64, delay: u64) -> DispatchResult {
    Democracy::propose(
        RuntimeOrigin::signed(who),
        set_balance_proposal_hash_and_note(value),
        delay,
    )
}

fn next_block() {
    System::set_block_number(System::block_number() + 1);
    Scheduler::on_initialize(System::block_number());
    assert!(Democracy::begin_block(System::block_number()).is_ok());
}

fn fast_forward_to(n: u64) {
    while System::block_number() < n {
        next_block();
    }
}

fn begin_referendum() -> ReferendumIndex {
    System::set_block_number(0);
    assert_ok!(propose_set_balance_and_note(1, 2, 1));
    fast_forward_to(2);
    0
}

fn aye(who: u64) -> Vote<u64> {
    Vote {
        aye: true,
        balance: Balances::free_balance(&who),
    }
}

fn nay(who: u64) -> Vote<u64> {
    Vote {
        aye: false,
        balance: Balances::free_balance(&who),
    }
}

fn tally(r: ReferendumIndex) -> Tally<u64> {
    Democracy::referendum_status(r).unwrap().tally
}
