/// Tests for BTC-Relay
use crate::{Event};
use crate::mock::{BTCRelay, Error, ExtBuilder, Origin, System, TestEvent};
use sp_std::collections::btree_map::BTreeMap;
use sp_std::collections::btree_set::BTreeSet;
use bitcoin::parser::*;
use bitcoin::types::*;
use security::ErrorCode;
use frame_support::{assert_err, assert_ok};
use mocktopus::mocking::*;

/// # Getters and setters
///
/// get_chain_position_from_chain_id 
/// set_chain_from_position_and_id
#[test]
fn get_chain_position_from_chain_id_succeeds() {
    ExtBuilder::build().execute_with(|| {
        // position and id of chains
        let mut chains_pos_id: Vec<(u32,u32)> = Vec::new();
        chains_pos_id.append(&mut vec![(0,0),(1,1),(2,3),(3,6)]);

        for (pos, id) in chains_pos_id.iter() {
            // insert chain
            BTCRelay::set_chain_from_position_and_id(*pos, *id);
        
            // check that chain is in right position
            let curr_pos = BTCRelay::get_chain_position_from_chain_id(*id)
                .unwrap();

            assert_eq!(curr_pos, *pos);
        }
        
    })
}

/// get_block_header_from_hash
/// set_block_header_from_hash
#[test]
fn get_block_header_from_hash_succeeds() {
    ExtBuilder::build().execute_with(|| {
        let chain_ref: u32 = 2;
        let block_height: u32 = 100;
        let block_header = hex::decode(sample_block_header()).unwrap();

        let rich_header = RichBlockHeader {
            block_hash: H256Le::zero(),
            block_header: BlockHeader::from_le_bytes(&block_header),
            block_height: block_height,
            chain_ref: chain_ref,
        };

        BTCRelay::set_block_header_from_hash(rich_header.block_hash, &rich_header);

        let curr_header = BTCRelay::get_block_header_from_hash(rich_header.block_hash).unwrap();
        assert_eq!(rich_header, curr_header); 
    })
}

#[test]
fn get_block_header_from_hash_fails() {
    ExtBuilder::build().execute_with(|| {
        let block_hash = H256Le::zero();

        assert_err!(BTCRelay::get_block_header_from_hash(block_hash),
            Error::BlockNotFound);
    })
}

/// get_block_chain_from_id
/// set_block_chain_from_id
#[test]
fn get_block_chain_from_id_succeeds() {
    ExtBuilder::build().execute_with(|| {
        let chain_ref: u32 = 1;
        let block_height: u32 = 100;

        let blockchain = get_empty_block_chain_from_chain_id_and_height(
            chain_ref, block_height
        );

        BTCRelay::set_block_chain_from_id(chain_ref, &blockchain);

        let curr_blockchain = BTCRelay::get_block_chain_from_id(chain_ref);

        assert_eq!(curr_blockchain, blockchain);
    })
}

/// # Main functions
///
/// initialize 
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

/// store_block_header function
#[test]
fn store_block_header_on_mainchain_succeeds() {
    ExtBuilder::build().execute_with(|| {
        BTCRelay::verify_block_header
            .mock_safe(|h| MockResult::Return(Ok(BlockHeader::from_le_bytes(&h))));
        BTCRelay::block_header_exists.mock_safe(|_| MockResult::Return(true));

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

        let prev_blockchain = get_empty_block_chain_from_chain_id_and_height(
            chain_ref, block_height
        );

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

#[test]
fn store_block_header_on_fork_succeeds() {
    ExtBuilder::build().execute_with(|| {
        BTCRelay::verify_block_header
            .mock_safe(|h| MockResult::Return(Ok(BlockHeader::from_le_bytes(&h))));
        BTCRelay::block_header_exists.mock_safe(|_| MockResult::Return(true));

        let chain_ref: u32 = 1;
        let block_height: u32 = 100;
        let block_header = hex::decode(sample_block_header()).unwrap();

        let rich_header = RichBlockHeader {
            block_hash: H256Le::zero(),
            block_header: BlockHeader::from_le_bytes(&block_header),
            block_height: block_height - 1,
            chain_ref: chain_ref,
        };
        BTCRelay::get_block_header_from_hash
            .mock_safe(move |_| MockResult::Return(Ok(rich_header)));
       
        let prev_blockchain = get_empty_block_chain_from_chain_id_and_height(
            chain_ref, block_height
        );

        BTCRelay::get_block_chain_from_id
            .mock_safe(move |_: u32| MockResult::Return(prev_blockchain.clone()));

        let block_header_hash = BlockHeader::block_hash_le(&block_header);
        assert_ok!(BTCRelay::store_block_header(
            Origin::signed(3),
            block_header
        ));

        let store_fork_event = TestEvent::test_events(Event::StoreForkHeader(
            chain_ref,
            block_height,
            block_header_hash,
        ));
        assert!(System::events().iter().any(|a| a.event == store_fork_event));
    })
}

/// check_and_do_reorg function
#[test]
fn check_and_do_reorg_is_main_chain_succeeds() {
    ExtBuilder::build().execute_with(|| {
        let chain_ref: u32 = 0;
        let block_height: u32 = 10;

        let blockchain = get_empty_block_chain_from_chain_id_and_height(
            chain_ref, block_height
        );

        assert_ok!(BTCRelay::check_and_do_reorg(&blockchain));
    })
}

#[test]
fn check_and_do_reorg_fork_id_not_found() {
    ExtBuilder::build().execute_with(|| {
        let chain_ref: u32 = 99;
        let block_height: u32 = 10;

        let blockchain = get_empty_block_chain_from_chain_id_and_height(
            chain_ref, block_height
        );

        assert_err!(
            BTCRelay::check_and_do_reorg(&blockchain), 
            Error::ForkIdNotFound
        );
    })
}

#[test]
fn check_and_do_reorg_swap_fork_position() {
    ExtBuilder::build().execute_with(|| {
        let fork_chain_ref: u32 = 4;
        let fork_block_height: u32 = 100;
        let fork_position: u32 = 2;

        let swap_chain_ref: u32 = 3;
        let swap_block_height: u32 = 99;
        let swap_position: u32 = 1;

        let fork = get_empty_block_chain_from_chain_id_and_height(
            fork_chain_ref, fork_block_height
        );
        let swap = get_empty_block_chain_from_chain_id_and_height(
            swap_chain_ref, swap_block_height
        );

        // make sure the main chain is set
        BTCRelay::set_chain_from_position_and_id(0, 0);  
        // insert the swap chain in Chains
        BTCRelay::set_chain_from_position_and_id(swap_position, swap_chain_ref);
        // insert the fork chain in Chains
        BTCRelay::set_chain_from_position_and_id(fork_position, fork_chain_ref);
        
        // check that fork is at its initial position
        let current_position = BTCRelay::get_chain_position_from_chain_id(
            fork_chain_ref).unwrap();

        assert_eq!(current_position, fork_position);

        BTCRelay::get_chain_position_from_chain_id
            .mock_safe(move |_| MockResult::Return(Ok(fork_position))); 

        BTCRelay::get_chain_id_from_position
            .mock_safe(move |_| MockResult::Return(swap_position.clone()));

        BTCRelay::get_block_chain_from_id
            .mock_safe(move |_| MockResult::Return(swap.clone()));

        assert_ok!(BTCRelay::check_and_do_reorg(&fork));
        
        // assert that positions have been swapped
        let new_position = BTCRelay::get_chain_position_from_chain_id(
            fork_chain_ref
            ).unwrap();
        assert_eq!(new_position, swap_position);
    })
}

/// verify_block_header  
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
        // Not duplicate block
        BTCRelay::block_header_exists
            .mock_safe(move |_| MockResult::Return(false));

        let verified_header = BTCRelay::verify_block_header(
                raw_first_header
            ).unwrap();
        
        assert_eq!(verified_header, rich_first_header.block_header)
    })
}

#[test]
fn test_verify_block_header_correct_retarget_increase_succeeds() {
    ExtBuilder::build().execute_with(|| {

        let chain_ref: u32 = 0;
        // Next block requires retarget
        let block_height: u32 = 2015;
        // Sample interval with INCREASING target
        let retarget_headers = sample_retarget_interval_increase();

        let prev_block_header_rich = RichBlockHeader::construct_rich_block_header(
            retarget_headers[1], 
            chain_ref, 
            block_height);
        
        let curr_block_header = BlockHeader::from_le_bytes(&retarget_headers[2]); 

        // Prev block exists
        BTCRelay::get_block_header_from_hash
             .mock_safe(move |_| MockResult::Return(Ok(prev_block_header_rich)));
        // Not duplicate block
        BTCRelay::block_header_exists
             .mock_safe(move |_| MockResult::Return(false));
        // Compute new target returns target of submitted header (i.e., correct)    
        BTCRelay::compute_new_target.mock_safe(move |_,_| MockResult::Return(Ok(curr_block_header.target)));

        let verified_header = BTCRelay::verify_block_header(
            retarget_headers[2]
        ).unwrap();
    
        assert_eq!(verified_header, curr_block_header)
    })
}

#[test]
fn test_verify_block_header_correct_retarget_decrease_succeeds() {
    ExtBuilder::build().execute_with(|| {

        let chain_ref: u32 = 0;
        // Next block requires retarget
        let block_height: u32 = 2015;
        // Sample interval with DECREASING target
        let retarget_headers = sample_retarget_interval_decrease();

        let prev_block_header_rich = RichBlockHeader::construct_rich_block_header(
            retarget_headers[1], 
            chain_ref, 
            block_height);
        
        let curr_block_header = BlockHeader::from_le_bytes(&retarget_headers[2]); 

        // Prev block exists
        BTCRelay::get_block_header_from_hash
             .mock_safe(move |_| MockResult::Return(Ok(prev_block_header_rich)));
        // Not duplicate block
        BTCRelay::block_header_exists
             .mock_safe(move |_| MockResult::Return(false));
        // Compute new target returns target of submitted header (i.e., correct)    
        BTCRelay::compute_new_target.mock_safe(move |_,_| MockResult::Return(Ok(curr_block_header.target)));

        let verified_header = BTCRelay::verify_block_header(
            retarget_headers[2]
        ).unwrap();
    
        assert_eq!(verified_header, curr_block_header)
    })
}



#[test]
fn test_verify_block_header_missing_retarget_succeeds() {
    ExtBuilder::build().execute_with(|| {

        let chain_ref: u32 = 0;
        // Next block requires retarget
        let block_height: u32 = 2015;
        let retarget_headers = sample_retarget_interval_increase();

        let prev_block_header_rich = RichBlockHeader::construct_rich_block_header(
            retarget_headers[1], 
            chain_ref, 
            block_height);
        
        let curr_block_header = BlockHeader::from_le_bytes(&retarget_headers[2]); 

        // Prev block exists
        BTCRelay::get_block_header_from_hash
             .mock_safe(move |_| MockResult::Return(Ok(prev_block_header_rich)));
        // Not duplicate block
        BTCRelay::block_header_exists
             .mock_safe(move |_| MockResult::Return(false));
        // Compute new target returns HIGHER target    
        BTCRelay::compute_new_target.mock_safe(move |_,_| MockResult::Return(Ok(curr_block_header.target+1)));

        assert_err!(
            BTCRelay::verify_block_header(retarget_headers[2]), Error::DiffTargetHeader
        );
    })
}

#[test]
fn test_compute_new_target() {
    let chain_ref: u32 = 0;
    // no retarget at block 100
    let block_height: u32 = 2016;
    let retarget_headers = sample_retarget_interval_increase();

    let last_retarget_time = BlockHeader::from_le_bytes(&retarget_headers[0]).timestamp;
    let prev_block_header = RichBlockHeader::construct_rich_block_header(
        retarget_headers[1], 
        chain_ref, 
        block_height);
    
    let curr_block_header = BlockHeader::from_le_bytes(&retarget_headers[2]); 

    BTCRelay::get_last_retarget_time.mock_safe(move |_,_| MockResult::Return(Ok(last_retarget_time)));

    let new_target = BTCRelay::compute_new_target(
        &prev_block_header,
        block_height).unwrap();
    
    assert_eq!(new_target,curr_block_header.target);
}

#[test]
fn test_verify_block_header_duplicate_fails() {
    ExtBuilder::build().execute_with(|| {

        let chain_ref: u32 = 0;
        // no retarget at block 100
        let block_height: u32 = 100;
        let genesis_header = sample_parsed_genesis_header(chain_ref, block_height);

        let rich_first_header = sample_parsed_first_block(chain_ref, 101);

        // Prev block is genesis
        BTCRelay::get_block_header_from_hash
            .mock_safe(move |_| MockResult::Return(Ok(genesis_header)));
        // submitted block ALREADY EXISTS
        BTCRelay::block_header_exists
            .mock_safe(move |block_hash| {
                assert_eq!(&block_hash, &rich_first_header.block_hash);
                MockResult::Return(true)
            });
        
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
        BTCRelay::block_header_exists
            .mock_safe(move |_| MockResult::Return(false));
                
        let raw_first_header = header_from_bytes(&(hex::decode(sample_raw_first_header()).unwrap()));

        assert_err!(
            BTCRelay::verify_block_header(raw_first_header),
            Error::PrevBlock
        );    
    })
}

#[test]
fn test_verify_block_header_low_diff_fails() {
    ExtBuilder::build().execute_with(|| {  

    let chain_ref: u32 = 0;
    // no retarget at block 100
    let block_height: u32 = 100;
    let genesis_header = sample_parsed_genesis_header(chain_ref, block_height);
    
    // block header with high target but weak hash
    let raw_first_header_weak = header_from_bytes(&(hex::decode(sample_raw_first_header_low_diff()).unwrap()));

    // Prev block is genesis
    BTCRelay::get_block_header_from_hash
        .mock_safe(move |_| MockResult::Return(Ok(genesis_header)));
    // submitted block does not yet exist
    BTCRelay::block_header_exists
        .mock_safe(move |_| MockResult::Return(false));


    assert_err!(
        BTCRelay::verify_block_header(raw_first_header_weak), 
        Error::LowDiff
    );

    });
}



// TODO: this currently fails with TX_FORMAT error in parser
/*
#[test]
fn test_validate_transaction_succeeds() {
    ExtBuilder::build().execute_with(|| {  

        //let raw_tx = bitcoin_spv::utils::reverse_endianness(&hex::decode(sample_accepted_transaction()).unwrap());
        let raw_tx = hex::decode(sample_accepted_transaction()).unwrap();
        let payment_value: u64 =  1;//2500200000;
        let recipient_btc_address = hex::decode("a91466c7060feb882664ae62ffad0051fe843e318e8587".to_owned()).unwrap();
        let op_return_id = hex::decode("aa21a9ede5c17d15b8b1fa2811b7e6da66ffa5e1aaa05922c69068bf90cd585b95bb4675".to_owned()).unwrap();

        assert_ok!(BTCRelay::validate_transaction(
            Origin::signed(3),
            raw_tx, 
            payment_value, 
            recipient_btc_address, 
            op_return_id
        ))

    });
}
*/

/// flag_block_error
#[test]
fn flag_block_error_succeeds() {
    ExtBuilder::build().execute_with(|| {
        let chain_ref: u32 = 1;
        let block_height: u32 = 100;
        let block_header = hex::decode(sample_block_header()).unwrap();

        let rich_header = RichBlockHeader {
            block_hash: H256Le::zero(),
            block_header: BlockHeader::from_le_bytes(&block_header),
            block_height: block_height,
            chain_ref: chain_ref,
        };

        BTCRelay::set_block_header_from_hash(rich_header.block_hash, &rich_header);
       
        let blockchain = get_empty_block_chain_from_chain_id_and_height(
            chain_ref, block_height
        );

        BTCRelay::set_block_chain_from_id(chain_ref, &blockchain);

        let error_codes = vec![ErrorCode::NoDataBTCRelay, ErrorCode::InvalidBTCRelay];

        for error in error_codes.iter() {
            assert_ok!(BTCRelay::flag_block_error(rich_header.block_hash, error.clone()));
            
            let curr_chain = BTCRelay::get_block_chain_from_id(chain_ref);

            if *error == ErrorCode::NoDataBTCRelay {
                assert!(curr_chain.no_data.contains(&block_height));
            } else if *error == ErrorCode::InvalidBTCRelay {
                assert!(curr_chain.invalid.contains(&block_height));
            };
        
            let error_event = TestEvent::test_events(Event::FlagBlockError(
                rich_header.block_hash,
                chain_ref,
                error.clone(),
            ));
            assert!(System::events().iter().any(|a| a.event == error_event));
        }
    })
}

#[test]
fn flag_block_error_fails() {
    ExtBuilder::build().execute_with(|| {
        let chain_ref: u32 = 1;
        let block_height: u32 = 100;
        let block_header = hex::decode(sample_block_header()).unwrap();

        let rich_header = RichBlockHeader {
            block_hash: H256Le::zero(),
            block_header: BlockHeader::from_le_bytes(&block_header),
            block_height: block_height,
            chain_ref: chain_ref,
        };

        BTCRelay::set_block_header_from_hash(rich_header.block_hash, &rich_header);
       
        let blockchain = get_empty_block_chain_from_chain_id_and_height(
            chain_ref, block_height
        );

        BTCRelay::set_block_chain_from_id(chain_ref, &blockchain);

        // not a valid error code
        let error = ErrorCode::Liquidation;

        assert_err!(BTCRelay::flag_block_error(rich_header.block_hash, error),
            Error::UnknownErrorcode);
    })
}
/// # Util functions
///
fn get_empty_block_chain_from_chain_id_and_height(
    chain_id: u32,
    block_height: u32,
) -> BlockChain {
    let chain = BTreeMap::new();

    let blockchain = BlockChain {
        chain_id: chain_id,
        chain: chain,
        start_height: 0,
        max_height: block_height,
        no_data: BTreeSet::new(),
        invalid: BTreeSet::new(),
    };

    blockchain
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

fn sample_raw_first_header_low_diff() -> String {
    "01000000".to_owned() + 
    "cb60e68ead74025dcfd4bf4673f3f71b1e678be9c6e6585f4544c79900000000" +
    "c7f42be7f83eddf2005272412b01204352a5fddbca81942c115468c3c4ec2fff" + 
    "827ad949" + 
    "413b1417" +  // high target 
    "21e05e45"
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



fn sample_retarget_interval_increase() -> [RawBlockHeader; 3] {
    // block height 66528
    let last_retarget_header = header_from_bytes(&hex::decode("01000000".to_owned() + "4e8e5cf3c4e4b8f63a9cf88beb2dbaba1949182101ae4e5cf54ad100000000009f2a2344e8112b0d7bd8089414106ee5f17bb6cd64078883e1b661fa251aac6bed1d3c4cf4a3051c4dcd2b02").unwrap());
    // block height 66543
    let prev_block_header = header_from_bytes(&hex::decode("01000000".to_owned()  + "1e321d88cb25946c4ca521eece3752803c021f9403fc4e0171203a0500000000317057f8b50414848a5a3a26d9eb8ace3d6f5495df456d0104dd1421159faf5029293c4cf4a3051c73199005").unwrap());
    // block height 68544
    let curr_header =  header_from_bytes(&hex::decode("01000000".to_owned() + "fb57c71ccd211b3de4ccc2e23b50a7cdb72aab91e60737b3a2bfdf030000000088a88ad9df68925e880e5d52b7e50cef225871c68b40a2cd0bca1084cd436037f388404cfd68011caeb1f801").unwrap());

    [last_retarget_header, prev_block_header, curr_header]
}


fn sample_retarget_interval_decrease() -> [RawBlockHeader; 3] {
    // block height 558432
    let last_retarget_header = header_from_bytes(&hex::decode("00c0ff2f".to_owned() + "6550b5dae76559589e3e3e135237072b6bc498949da6280000000000000000005988783435f506d2ccfbadb484e56d6f1d5dfdd480650acae1e3b43d3464ea73caf13b5c33d62f171d508fdb").unwrap());
    // block height 560447
    let prev_block_header = header_from_bytes(&hex::decode("00000020".to_owned()  + "d8e8e54ca5e33522b94fbba5de736efc55ff75e832cf2300000000000000000007b395f80858ee022c9c3c2f0f5cee4bd807039f0729b0559ae4326c3ba77d6b209f4e5c33d62f1746ee356d").unwrap());
    // block height 560448
    let curr_header =  header_from_bytes(&hex::decode("00000020".to_owned() + "6b05bd2c4a06b3d8503a033c2593396a25a79e1dcadb140000000000000000001b08df3d42cd9a38d8b66adf9dc5eb464f503633bd861085ffff723634531596a1a24e5c35683017bf67b72a").unwrap());

    [last_retarget_header, prev_block_header, curr_header]
}


fn sample_accepted_transaction() -> String {
    "020000000001010000000000000000000000000000000000000000000000000000000000000000ffffffff0502cb000101ffffffff02400606950000000017a91466c7060feb882664ae62ffad0051fe843e318e85870000000000000000266a24aa21a9ede5c17d15b8b1fa2811b7e6da66ffa5e1aaa05922c69068bf90cd585b95bb46750120000000000000000000000000000000000000000000000000000000000000000000000000".to_owned()
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

