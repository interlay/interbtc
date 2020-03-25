/// Tests for BTC-Relay

use crate::{Module, Trait, Event};
use sp_core::{U256, H256};
use frame_support::{impl_outer_origin, impl_outer_event, assert_ok, assert_err, parameter_types, weights::Weight};
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup}, testing::Header, Perbill,
};
use bitcoin::types::*;
use btc_core::Error;

impl_outer_origin! {
	pub enum Origin for Test {}
}

mod test_events {
    pub use crate::Event;
}

impl_outer_event! {
    pub enum TestEvent for Test {
        test_events,
    }
}

// For testing the pallet, we construct most of a mock runtime. This means
// first constructing a configuration type (`Test`) which `impl`s each of the
// configuration traits of modules we want to use.
#[derive(Clone, Eq, PartialEq)]
pub struct Test;
parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: Weight = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
}
impl system::Trait for Test {
	type Origin = Origin;
	type Call = ();
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = TestEvent;
	type BlockHashCount = BlockHashCount;
	type MaximumBlockWeight = MaximumBlockWeight;
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = ();
	type ModuleToIndex = ();
}

impl Trait for Test {
	type Event = TestEvent;
}

pub type System = system::Module<Test>;
pub type BTCRelay = Module<Test>;

pub struct ExtBuilder;

impl ExtBuilder {
    pub fn build() -> sp_io::TestExternalities {
        let mut storage = system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();
        sp_io::TestExternalities::from(storage)
    }
}


// fn ExtBuilder::build() -> sp_io::TestExternalities {
// 	system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
// }


/// Initialize Function
#[test]
fn initialize_once_suceeds() {
    ExtBuilder::build().execute_with(|| {
        let block_height: u32 = 0;
        let block_header = vec![0u8; 80];
        let block_header_hash = H256Le::zero();
        assert_ok!(BTCRelay::initialize(Origin::signed(3), block_header, block_height));
       
        let init_event = TestEvent::test_events(
            Event::Initialized(block_height, block_header_hash),
        );
        assert!(System::events().iter().any(|a| a.event == init_event));
    })
}

#[test]
fn initialize_twice_fails() {
    ExtBuilder::build().execute_with(|| {
        let block_height: u32 = 0;
        let block_header = vec![0u8; 80];
        let block_header_hash = H256Le::zero();
        assert_ok!(BTCRelay::initialize(Origin::signed(3), block_header, block_height));

        let block_height_2: u32 = 0;
        let block_header_2 = vec![1u8; 80];
        assert_err!(BTCRelay::initialize(Origin::signed(3), block_header_2, block_height_2), Error::AlreadyInitialized);
    })
}

/// StoreBlockHeader Function
#[test]
fn store_fork_once_suceeds() {
    ExtBuilder::build().execute_with(|| {
        let block_height: u32 = 1;
        let block_header = vec![1u8; 80];
        let block_header_hash = H256Le::zero();
        let chain_id: u32 = 2;
        assert_ok!(BTCRelay::store_block_header(Origin::signed(3), block_header));
       
        let store_event = TestEvent::test_events(
            Event::StoreForkHeader(chain_id, block_height, block_header_hash),
        );
        assert!(System::events().iter().any(|a| a.event == store_event));
    })
}


fn sample_block_header() -> String {
    "02000000".to_owned() + // ............... Block version: 2
    "b6ff0b1b1680a2862a30ca44d346d9e8" + //
    "910d334beb48ca0c0000000000000000" + // ... Hash of previous block's header
    "9d10aa52ee949386ca9385695f04ede2" + //
    "70dda20810decd12bc9b048aaab31471" + // ... Merkle root
    "24d95a54" + // ........................... Unix time: 1415239972
    "30c31b18" + // ........................... Target: 0x1bc330 * 256**(0x18-3)
    "fe9f0864"
}
fn test_verify_block_header_succeeds() {

}




