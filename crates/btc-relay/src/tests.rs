/// Tests for BTC-Relay
use crate::{Event, Module, Trait};
use sp_std::collections::btree_map::BTreeMap;
use sp_std::collections::btree_set::BTreeSet;
use bitcoin::parser::*;
use bitcoin::types::*;
use frame_support::{
    assert_err, assert_ok, impl_outer_event, impl_outer_origin, parameter_types, weights::Weight,
};
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    Perbill,
};

use mocktopus::mocking::*;

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

type Error = crate::Error;

pub type System = system::Module<Test>;
pub type BTCRelay = Module<Test>;

pub struct ExtBuilder;

impl ExtBuilder {
    pub fn build() -> sp_io::TestExternalities {
        let storage = system::GenesisConfig::default()
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
fn initialize_once_succeeds() {
    ExtBuilder::build().execute_with(|| {
        let block_height: u32 = 1;
        let block_header = vec![0u8; 80];
        let block_header_hash = BlockHeader::block_hash_le(&block_header);
        assert_ok!(BTCRelay::initialize(
            Origin::signed(3),
            block_header,
            block_height
        ));

        let init_event =
            TestEvent::test_events(Event::Initialized(block_height, block_header_hash));
        assert!(System::events().iter().any(|a| a.event == init_event));
    })
}


#[test]
fn initialize_best_block_already_set_fails() {
    ExtBuilder::build().execute_with(|| {
        let block_height: u32 = 1;
        let block_header = vec![0u8; 80];

        BTCRelay::best_block_exists.mock_safe(|| MockResult::Return(true));

        assert_err!(
            BTCRelay::initialize(Origin::signed(3), block_header, block_height),
            Error::AlreadyInitialized
        );
    })
}

/// StoreBlockHeader Function
#[test]
fn store_block_header_on_mainchain_succeeds() {
    ExtBuilder::build().execute_with(|| {
        BTCRelay::verify_block_header
            .mock_safe(|h| MockResult::Return(Ok(BlockHeader::from_le_bytes(&h))));
        BTCRelay::block_exists.mock_safe(|_| MockResult::Return(true));

        let chain_ref: u32 = 0;
        let block_height: u32 = 100;
        let block_header = hex::decode(sample_block_header()).unwrap();

        let rich_header = RichBlockHeader {
            block_hash: H256Le::zero(),
            block_header: BlockHeader::from_le_bytes(&block_header),
            block_height: block_height,
            chain_ref: chain_ref,
        };
        BTCRelay::get_block_header_from_hash
            .mock_safe(move |_| MockResult::Return(Ok(rich_header)));
       
        let chain = BTreeMap::new();

        let prev_blockchain = BlockChain {
            chain_id: chain_ref,
            chain: chain,
            start_height: 0,
            max_height: block_height,
            no_data: BTreeSet::new(),
            invalid: BTreeSet::new(),
        };

        BTCRelay::get_block_chain_from_id
            .mock_safe(move |_: u32| MockResult::Return(prev_blockchain.clone()));

        let block_header_hash = BlockHeader::block_hash_le(&block_header);
        assert_ok!(BTCRelay::store_block_header(
            Origin::signed(3),
            block_header
        ));

        let store_main_event = TestEvent::test_events(Event::StoreMainChainHeader(
            block_height + 1,
            block_header_hash,
        ));
        assert!(System::events().iter().any(|a| a.event == store_main_event));
    })
}

// verify_block_header  
#[test]
fn test_verify_block_header_no_retarget_succeeds() {
    ExtBuilder::build().execute_with(|| {

        let chain_ref: u32 = 0;
        // no retarget at block 100
        let block_height: u32 = 100;
        let genesis_header = sample_parsed_genesis_header(chain_ref, block_height);
        
        let raw_first_header = header_from_bytes(&(hex::decode(sample_raw_first_header()).unwrap()));
        let rich_first_header = sample_parsed_first_block(chain_ref, block_height + 1);

        // Prev block is genesis
        BTCRelay::get_block_header_from_hash
            .mock_safe(move |_| MockResult::Return(Ok(genesis_header)));
        // submitted block does not yet exist
        BTCRelay::block_exists
            .mock_safe(move |raw_first_header| MockResult::Return(false));

        let verified_header = BTCRelay::verify_block_header(
                raw_first_header
            ).unwrap();
        
        assert_eq!(verified_header, rich_first_header.block_header)
    })
}

#[test]
fn test_verify_block_header_duplicate_fails() {
    ExtBuilder::build().execute_with(|| {

        let chain_ref: u32 = 0;
        // no retarget at block 100
        let block_height: u32 = 100;
        let genesis_header = sample_parsed_genesis_header(chain_ref, block_height);

        let raw_first_header = header_from_bytes(&(hex::decode(sample_raw_first_header()).unwrap()));
        let rich_first_header = sample_parsed_first_block(chain_ref, 101);

        // Prev block is genesis
        BTCRelay::get_block_header_from_hash
            .mock_safe(move |_| MockResult::Return(Ok(genesis_header)));
        // submitted block ALREADY EXISTS
        BTCRelay::block_exists
            .mock_safe(move |raw_first_header| MockResult::Return(true));
        
        
        let raw_first_header = header_from_bytes(&(hex::decode(sample_raw_first_header()).unwrap()));

        assert_err!(
            BTCRelay::verify_block_header(raw_first_header),
            Error::DuplicateBlock
        );
    })
}


#[test]
fn test_verify_block_header_no_prev_block_fails() {
    ExtBuilder::build().execute_with(|| {

        let chain_ref: u32 = 0;
        // Prev block is MISSING
        BTCRelay::get_block_header_from_hash
            .mock_safe(move |_| MockResult::Return(Err(Error::PrevBlock)));
        // submitted block does not yet exist
        BTCRelay::block_exists
            .mock_safe(move |_| MockResult::Return(false));
        
        
        let raw_first_header = header_from_bytes(&(hex::decode(sample_raw_first_header()).unwrap()));
        let rich_first_header = sample_parsed_first_block(chain_ref, 101);

        assert_err!(
            BTCRelay::verify_block_header(raw_first_header),
            Error::PrevBlock
        );    
    })
}


fn sample_raw_genesis_header() -> String {
    "01000000".to_owned() + "a7c3299ed2475e1d6ea5ed18d5bfe243224add249cce99c5c67cc9fb00000000601c73862a0a7238e376f497783c8ecca2cf61a4f002ec8898024230787f399cb575d949ffff001d3a5de07f"
}

fn sample_parsed_genesis_header(chain_ref: u32, block_height: u32) -> RichBlockHeader {
    let genesis_header = hex::decode(sample_raw_genesis_header()).unwrap();
    
    RichBlockHeader {
        block_hash: BlockHeader::block_hash_le(&genesis_header),
        block_header: BlockHeader::from_le_bytes(&genesis_header),
        block_height: block_height,
        chain_ref: chain_ref,
    }
}

fn sample_genesis_height() -> u32 {
    100
}

fn sample_raw_first_header() -> String {
    "01000000".to_owned() + "cb60e68ead74025dcfd4bf4673f3f71b1e678be9c6e6585f4544c79900000000c7f42be7f83eddf2005272412b01204352a5fddbca81942c115468c3c4ec2fff827ad949ffff001d21e05e45"
}

fn sample_parsed_first_block(chain_ref: u32, block_height: u32) -> RichBlockHeader {
    let block_header = hex::decode(sample_raw_first_header()).unwrap();
    
    RichBlockHeader {
        block_hash: BlockHeader::block_hash_le(&block_header),
        block_header: BlockHeader::from_le_bytes(&block_header),
        block_height: block_height,
        chain_ref: chain_ref,
    }
}

fn sample_main_blockchain(chain_ref: u32, max_height: u32) -> BlockChain {
    BlockChain {
        chain_id: chain_ref,
        chain: BTreeMap::new(),
        start_height: 0,
        max_height: max_height,
        no_data: BTreeSet::new(),
        invalid: BTreeSet::new(),
    }
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
