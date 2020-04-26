use crate::mock::{run_test, BTCRelay, Error, Origin, System, TestEvent};
/// Tests for BTC-Relay
use crate::Event;
use bitcoin::merkle::*;
use bitcoin::parser::*;
use bitcoin::types::*;
use frame_support::{assert_err, assert_ok};
use security::ErrorCode;
use sp_std::collections::btree_set::BTreeSet;

use mocktopus::mocking::*;

/// # Getters and setters
///
/// get_chain_position_from_chain_id
/// set_chain_from_position_and_id
#[test]
fn get_chain_position_from_chain_id_succeeds() {
    run_test(|| {
        // position and id of chains
        let mut chains_pos_id: Vec<(u32, u32)> = Vec::new();
        chains_pos_id.append(&mut vec![(0, 0), (1, 1), (2, 3), (3, 6)]);

        for (pos, id) in chains_pos_id.iter() {
            // insert chain
            BTCRelay::set_chain_from_position_and_id(*pos, *id);
            // check that chain is in right position
            let curr_pos = BTCRelay::get_chain_position_from_chain_id(*id).unwrap();

            assert_eq!(curr_pos, *pos);
        }
    })
}

/// get_block_header_from_hash
/// set_block_header_from_hash
#[test]
fn get_block_header_from_hash_succeeds() {
    run_test(|| {
        let chain_ref: u32 = 2;
        let block_height: u32 = 100;
        let block_header = hex::decode(sample_block_header_hex()).unwrap();

        let rich_header = RichBlockHeader {
            block_hash: H256Le::zero(),
            block_header: BlockHeader::from_le_bytes(&block_header).unwrap(),
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
    run_test(|| {
        let block_hash = H256Le::zero();

        assert_err!(
            BTCRelay::get_block_header_from_hash(block_hash),
            Error::BlockNotFound
        );
    })
}

/// get_block_chain_from_id
/// set_block_chain_from_id
#[test]
fn get_block_chain_from_id_succeeds() {
    run_test(|| {
        let chain_ref: u32 = 1;
        let start_height: u32 = 10;
        let block_height: u32 = 100;

        let blockchain =
            get_empty_block_chain_from_chain_id_and_height(chain_ref, start_height, block_height);

        BTCRelay::set_block_chain_from_id(chain_ref, &blockchain);

        let curr_blockchain = BTCRelay::get_block_chain_from_id(chain_ref).unwrap();

        assert_eq!(curr_blockchain, blockchain);
    })
}

/// # Main functions
///
/// initialize
#[test]
fn initialize_once_succeeds() {
    run_test(|| {
        let block_height: u32 = 1;
        let block_header = RawBlockHeader::from_hex(sample_block_header_hex()).unwrap();
        let block_header_hash = block_header.hash();
        BTCRelay::best_block_exists.mock_safe(|| MockResult::Return(false));

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
    run_test(|| {
        let block_height: u32 = 1;
        let raw_block_header = RawBlockHeader::from_hex(sample_block_header_hex()).unwrap();

        BTCRelay::best_block_exists.mock_safe(|| MockResult::Return(true));

        assert_err!(
            BTCRelay::initialize(Origin::signed(3), raw_block_header, block_height),
            Error::AlreadyInitialized
        );
    })
}

/// store_block_header function
#[test]
fn store_block_header_on_mainchain_succeeds() {
    run_test(|| {
        BTCRelay::verify_block_header.mock_safe(|h| {
            MockResult::Return(Ok(BlockHeader::from_le_bytes(h.as_slice()).unwrap()))
        });
        BTCRelay::block_header_exists.mock_safe(|_| MockResult::Return(true));

        let chain_ref: u32 = 0;
        let start_height: u32 = 0;
        let block_height: u32 = 100;
        let block_header = RawBlockHeader::from_hex(sample_block_header_hex()).unwrap();

        let rich_header = RichBlockHeader {
            block_hash: H256Le::zero(),
            block_header: parse_block_header(&block_header).unwrap(),
            block_height: block_height,
            chain_ref: chain_ref,
        };
        BTCRelay::get_block_header_from_hash
            .mock_safe(move |_| MockResult::Return(Ok(rich_header)));

        let prev_blockchain =
            get_empty_block_chain_from_chain_id_and_height(chain_ref, start_height, block_height);
        BTCRelay::get_block_chain_from_id
            .mock_safe(move |_: u32| MockResult::Return(Ok(prev_blockchain.clone())));

        let block_header_hash = block_header.hash();
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
    run_test(|| {
        BTCRelay::verify_block_header.mock_safe(|h| MockResult::Return(parse_block_header(&h)));
        BTCRelay::block_header_exists.mock_safe(|_| MockResult::Return(true));

        let chain_ref: u32 = 1;
        let start_height: u32 = 20;
        let block_height: u32 = 100;
        let block_header = RawBlockHeader::from_hex(sample_block_header_hex()).unwrap();

        let rich_header = RichBlockHeader {
            block_hash: H256Le::zero(),
            block_header: parse_block_header(&block_header).unwrap(),
            block_height: block_height - 1,
            chain_ref: chain_ref,
        };
        BTCRelay::get_block_header_from_hash
            .mock_safe(move |_| MockResult::Return(Ok(rich_header)));

        let prev_blockchain =
            get_empty_block_chain_from_chain_id_and_height(chain_ref, start_height, block_height);
        BTCRelay::get_block_chain_from_id
            .mock_safe(move |_: u32| MockResult::Return(Ok(prev_blockchain.clone())));

        let block_header_hash = block_header.hash();
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
    run_test(|| {
        let chain_ref: u32 = 0;
        let start_height: u32 = 3;
        let block_height: u32 = 10;

        let blockchain =
            get_empty_block_chain_from_chain_id_and_height(chain_ref, start_height, block_height);

        assert_ok!(BTCRelay::check_and_do_reorg(&blockchain));
    })
}

#[test]
fn check_and_do_reorg_fork_id_not_found() {
    run_test(|| {
        let chain_ref: u32 = 99;
        let start_height: u32 = 3;
        let block_height: u32 = 10;

        let blockchain =
            get_empty_block_chain_from_chain_id_and_height(chain_ref, start_height, block_height);

        assert_err!(
            BTCRelay::check_and_do_reorg(&blockchain),
            Error::ForkIdNotFound
        );
    })
}

#[test]
fn check_and_do_reorg_swap_fork_position() {
    run_test(|| {
        // insert the main chain in Chains and ChainsIndex
        let main_chain_ref: u32 = 0;
        let main_start_height: u32 = 3;
        let main_block_height: u32 = 110;
        let main_position: u32 = 0;
        let main = get_empty_block_chain_from_chain_id_and_height(
            main_chain_ref,
            main_start_height,
            main_block_height,
        );
        BTCRelay::set_chain_from_position_and_id(main_position, main_chain_ref);
        BTCRelay::set_block_chain_from_id(main_chain_ref, &main);

        // insert the fork chain in Chains and ChainsIndex
        let fork_chain_ref: u32 = 4;
        let fork_start_height: u32 = 20;
        let fork_block_height: u32 = 100;
        let fork_position: u32 = 2;
        let fork = get_empty_block_chain_from_chain_id_and_height(
            fork_chain_ref,
            fork_start_height,
            fork_block_height,
        );
        BTCRelay::set_chain_from_position_and_id(fork_position, fork_chain_ref);
        BTCRelay::set_block_chain_from_id(fork_chain_ref, &fork);

        // insert the swap chain in Chains and ChainsIndex
        let swap_chain_ref: u32 = 3;
        let swap_start_height: u32 = 43;
        let swap_block_height: u32 = 99;
        let swap_position: u32 = 1;
        let swap = get_empty_block_chain_from_chain_id_and_height(
            swap_chain_ref,
            swap_start_height,
            swap_block_height,
        );
        BTCRelay::set_chain_from_position_and_id(swap_position, swap_chain_ref);
        BTCRelay::set_block_chain_from_id(swap_chain_ref, &swap);

        // check that fork is at its initial position
        let current_position = BTCRelay::get_chain_position_from_chain_id(fork_chain_ref).unwrap();

        assert_eq!(current_position, fork_position);

        assert_ok!(BTCRelay::check_and_do_reorg(&fork));
        // assert that positions have been swapped
        let new_position = BTCRelay::get_chain_position_from_chain_id(fork_chain_ref).unwrap();
        assert_eq!(new_position, swap_position);

        // assert the main chain has not changed
        let curr_main_chain = BTCRelay::get_block_chain_from_id(main_chain_ref);
        assert_eq!(curr_main_chain, Ok(main));
    })
}

#[test]
fn check_and_do_reorg_new_fork_is_main_chain() {
    run_test(|| {
        // insert the main chain in Chains and ChainsIndex
        let main_chain_ref: u32 = 0;
        let main_start_height: u32 = 4;
        let main_block_height: u32 = 110;
        let main_position: u32 = 0;
        let main = get_empty_block_chain_from_chain_id_and_height(
            main_chain_ref,
            main_start_height,
            main_block_height,
        );
        BTCRelay::set_chain_from_position_and_id(main_position, main_chain_ref);
        BTCRelay::set_block_chain_from_id(main_chain_ref, &main);

        // insert the fork chain in Chains and ChainsIndex
        let fork_chain_ref: u32 = 4;
        let fork_block_height: u32 = 117;
        let fork_position: u32 = 1;
        let fork = get_empty_block_chain_from_chain_id_and_height(
            fork_chain_ref,
            main_start_height,
            fork_block_height,
        );
        BTCRelay::set_chain_from_position_and_id(fork_position, fork_chain_ref);
        BTCRelay::set_block_chain_from_id(fork_chain_ref, &fork);

        // set the best block
        let best_block_hash = H256Le::zero();
        BTCRelay::set_best_block(best_block_hash);
        BTCRelay::set_best_block_height(fork_block_height);

        // check that fork is at its initial position
        let current_position = BTCRelay::get_chain_position_from_chain_id(fork_chain_ref).unwrap();

        assert_eq!(current_position, fork_position);

        BTCRelay::swap_main_blockchain.mock_safe(|_| MockResult::Return(Ok(())));

        assert_ok!(BTCRelay::check_and_do_reorg(&fork));
        // assert that the new main chain is set
        let reorg_event = TestEvent::test_events(Event::ChainReorg(
            best_block_hash,
            fork_block_height,
            fork.max_height - fork.start_height,
        ));
        assert!(System::events().iter().any(|a| a.event == reorg_event));
    })
}
#[test]
fn check_and_do_reorg_new_fork_below_stable_transaction_confirmations() {
    run_test(|| {
        // insert the main chain in Chains and ChainsIndex
        let main_chain_ref: u32 = 0;
        let main_start_height: u32 = 4;
        let main_block_height: u32 = 110;
        let main_position: u32 = 0;
        let main = get_empty_block_chain_from_chain_id_and_height(
            main_chain_ref,
            main_start_height,
            main_block_height,
        );
        BTCRelay::set_chain_from_position_and_id(main_position, main_chain_ref);
        BTCRelay::set_block_chain_from_id(main_chain_ref, &main);

        // insert the fork chain in Chains and ChainsIndex
        let fork_chain_ref: u32 = 4;
        let fork_block_height: u32 = 113;
        let fork_position: u32 = 1;
        let fork = get_empty_block_chain_from_chain_id_and_height(
            fork_chain_ref,
            main_start_height,
            fork_block_height,
        );
        BTCRelay::set_chain_from_position_and_id(fork_position, fork_chain_ref);
        BTCRelay::set_block_chain_from_id(fork_chain_ref, &fork);

        // set the best block
        let best_block_hash = H256Le::zero();
        BTCRelay::set_best_block(best_block_hash);
        BTCRelay::set_best_block_height(fork_block_height);

        // check that fork is at its initial position
        let current_position = BTCRelay::get_chain_position_from_chain_id(fork_chain_ref).unwrap();

        assert_eq!(current_position, fork_position);

        BTCRelay::swap_main_blockchain.mock_safe(|_| MockResult::Return(Ok(())));

        assert_ok!(BTCRelay::check_and_do_reorg(&fork));
        // assert that the fork has not overtaken the main chain
        let ahead_event = TestEvent::test_events(Event::ForkAheadOfMainChain(
            main_block_height,
            fork_block_height,
            fork_chain_ref,
        ));
        assert!(System::events().iter().any(|a| a.event == ahead_event));
    })
}

/// insert_sorted
#[test]
fn insert_sorted_succeeds() {
    run_test(|| {
        // insert the main chain in Chains and ChainsIndex
        let main_chain_ref: u32 = 0;
        let main_start_height: u32 = 60;
        let main_block_height: u32 = 110;
        let main_position: u32 = 0;
        let main = get_empty_block_chain_from_chain_id_and_height(
            main_chain_ref,
            main_start_height,
            main_block_height,
        );
        BTCRelay::set_block_chain_from_id(main_chain_ref, &main);
        assert_eq!(Ok(()), BTCRelay::insert_sorted(&main));

        let curr_main_pos = BTCRelay::get_chain_position_from_chain_id(main_chain_ref).unwrap();
        assert_eq!(curr_main_pos, main_position);
        // insert the swap chain in Chains and ChainsIndex
        let swap_chain_ref: u32 = 3;
        let swap_start_height: u32 = 70;
        let swap_block_height: u32 = 99;
        let swap_position: u32 = 1;
        let swap = get_empty_block_chain_from_chain_id_and_height(
            swap_chain_ref,
            swap_start_height,
            swap_block_height,
        );
        BTCRelay::set_block_chain_from_id(swap_chain_ref, &swap);
        assert_eq!(Ok(()), BTCRelay::insert_sorted(&swap));

        let curr_swap_pos = BTCRelay::get_chain_position_from_chain_id(swap_chain_ref).unwrap();
        assert_eq!(curr_swap_pos, swap_position);

        // insert the fork chain in Chains and ChainsIndex
        let fork_chain_ref: u32 = 4;
        let fork_start_height: u32 = 77;
        let fork_block_height: u32 = 100;
        let fork_position: u32 = 1;
        let new_swap_pos: u32 = 2;
        let fork = get_empty_block_chain_from_chain_id_and_height(
            fork_chain_ref,
            fork_start_height,
            fork_block_height,
        );
        BTCRelay::set_block_chain_from_id(fork_chain_ref, &fork);
        assert_eq!(Ok(()), BTCRelay::insert_sorted(&fork));

        let curr_fork_pos = BTCRelay::get_chain_position_from_chain_id(fork_chain_ref).unwrap();
        assert_eq!(curr_fork_pos, fork_position);
        let curr_swap_pos = BTCRelay::get_chain_position_from_chain_id(swap_chain_ref).unwrap();
        assert_eq!(curr_swap_pos, new_swap_pos);
    })
}

/// swap_main_blockchain
#[test]
fn swap_main_blockchain_succeeds() {
    run_test(|| {
        // insert main chain and headers
        let main_chain_ref: u32 = 0;
        let main_start: u32 = 0;
        let main_height: u32 = 10;
        let main_position: u32 = 0;
        let main = store_blockchain_and_random_headers(
            main_chain_ref,
            main_start,
            main_height,
            main_position,
        );

        // insert the fork chain and headers
        let fork_chain_ref: u32 = 4;
        let fork_start: u32 = 5;
        let fork_height: u32 = 17;
        let fork_position: u32 = 1;

        let fork = store_blockchain_and_random_headers(
            fork_chain_ref,
            fork_start,
            fork_height,
            fork_position,
        );

        let old_main_ref = fork_chain_ref + 1;
        // mock the chain counter
        BTCRelay::increment_chain_counter.mock_safe(move || MockResult::Return(old_main_ref));

        // swap the main and fork
        assert_ok!(BTCRelay::swap_main_blockchain(&fork));

        // check that the new main chain is correct
        let new_main = BTCRelay::get_block_chain_from_id(main_chain_ref).unwrap();
        assert_eq!(fork_height, new_main.max_height);
        assert_eq!(main_start, new_main.start_height);
        assert_eq!(main_chain_ref, new_main.chain_id);
        assert_eq!(
            fork_height + 1,
            BTCRelay::_blocks_count(main_chain_ref) as u32
        );

        assert_eq!(main.no_data, new_main.no_data);
        assert_eq!(main.invalid, new_main.invalid);

        // check that the fork is deleted
        assert_eq!(
            Err(Error::InvalidChainID),
            BTCRelay::get_block_chain_from_id(fork_chain_ref)
        );

        // check that the old main chain is stored in a old fork
        let old_main = BTCRelay::get_block_chain_from_id(old_main_ref).unwrap();
        assert_eq!(main_height, old_main.max_height);
        assert_eq!(fork_start, old_main.start_height);
        assert_eq!(old_main_ref, old_main.chain_id);
        let old_main_length = BTCRelay::_blocks_count(old_main.chain_id);
        assert_eq!(main_height - fork_start + 1, old_main_length as u32);

        assert_eq!(main.no_data, old_main.no_data);
        assert_eq!(main.invalid, old_main.invalid);

        // check that the best block is set
        assert_eq!(
            BTCRelay::get_block_hash(new_main.chain_id, fork_height).unwrap(),
            BTCRelay::get_best_block()
        );

        // check that the best block height is correct
        assert_eq!(fork_height, BTCRelay::get_best_block_height());
        // check that all fork headers are updated
        for height in fork_start..=fork_height {
            let block_hash = BTCRelay::get_block_hash(main_chain_ref, height).unwrap();
            let header = BTCRelay::get_block_header_from_hash(block_hash).unwrap();
            assert_eq!(header.chain_ref, main_chain_ref);
        }

        // check that all main headers are updated
        for height in fork_start..=main_height {
            let block_hash = BTCRelay::get_block_hash(old_main_ref, height).unwrap();
            let header = BTCRelay::get_block_header_from_hash(block_hash).unwrap();
            assert_eq!(header.chain_ref, old_main_ref);
        }
    })
}

/// verify_block_header
#[test]
fn test_verify_block_header_no_retarget_succeeds() {
    run_test(|| {
        let chain_ref: u32 = 0;
        // no retarget at block 100
        let block_height: u32 = 100;
        let genesis_header = sample_parsed_genesis_header(chain_ref, block_height);

        let raw_first_header = RawBlockHeader::from_hex(sample_raw_first_header()).unwrap();
        let rich_first_header = sample_parsed_first_block(chain_ref, block_height + 1);

        // Prev block is genesis
        BTCRelay::get_block_header_from_hash
            .mock_safe(move |_| MockResult::Return(Ok(genesis_header)));
        // Not duplicate block
        BTCRelay::block_header_exists.mock_safe(move |_| MockResult::Return(false));

        let verified_header = BTCRelay::verify_block_header(&raw_first_header).unwrap();
        assert_eq!(verified_header, rich_first_header.block_header)
    })
}

#[test]
fn test_verify_block_header_correct_retarget_increase_succeeds() {
    run_test(|| {
        let chain_ref: u32 = 0;
        // Next block requires retarget
        let block_height: u32 = 2015;
        // Sample interval with INCREASING target
        let retarget_headers = sample_retarget_interval_increase();

        let prev_block_header_rich =
            RichBlockHeader::construct(retarget_headers[1], chain_ref, block_height).unwrap();

        let curr_block_header = parse_block_header(&retarget_headers[2]).unwrap();
        // Prev block exists
        BTCRelay::get_block_header_from_hash
            .mock_safe(move |_| MockResult::Return(Ok(prev_block_header_rich)));
        // Not duplicate block
        BTCRelay::block_header_exists.mock_safe(move |_| MockResult::Return(false));
        // Compute new target returns target of submitted header (i.e., correct)
        BTCRelay::compute_new_target
            .mock_safe(move |_, _| MockResult::Return(Ok(curr_block_header.target)));

        let verified_header = BTCRelay::verify_block_header(&retarget_headers[2]).unwrap();
        assert_eq!(verified_header, curr_block_header)
    })
}

#[test]
fn test_verify_block_header_correct_retarget_decrease_succeeds() {
    run_test(|| {
        let chain_ref: u32 = 0;
        // Next block requires retarget
        let block_height: u32 = 2015;
        // Sample interval with DECREASING target
        let retarget_headers = sample_retarget_interval_decrease();

        let prev_block_header_rich =
            RichBlockHeader::construct(retarget_headers[1], chain_ref, block_height).unwrap();

        let curr_block_header = parse_block_header(&retarget_headers[2]).unwrap();
        // Prev block exists
        BTCRelay::get_block_header_from_hash
            .mock_safe(move |_| MockResult::Return(Ok(prev_block_header_rich)));
        // Not duplicate block
        BTCRelay::block_header_exists.mock_safe(move |_| MockResult::Return(false));
        // Compute new target returns target of submitted header (i.e., correct)
        BTCRelay::compute_new_target
            .mock_safe(move |_, _| MockResult::Return(Ok(curr_block_header.target)));

        let verified_header = BTCRelay::verify_block_header(&retarget_headers[2]).unwrap();
        assert_eq!(verified_header, curr_block_header)
    })
}

#[test]
fn test_verify_block_header_missing_retarget_succeeds() {
    run_test(|| {
        let chain_ref: u32 = 0;
        // Next block requires retarget
        let block_height: u32 = 2015;
        let retarget_headers = sample_retarget_interval_increase();

        let prev_block_header_rich =
            RichBlockHeader::construct(retarget_headers[1], chain_ref, block_height).unwrap();

        let curr_block_header = parse_block_header(&retarget_headers[2]).unwrap();
        // Prev block exists
        BTCRelay::get_block_header_from_hash
            .mock_safe(move |_| MockResult::Return(Ok(prev_block_header_rich)));
        // Not duplicate block
        BTCRelay::block_header_exists.mock_safe(move |_| MockResult::Return(false));
        // Compute new target returns HIGHER target
        BTCRelay::compute_new_target
            .mock_safe(move |_, _| MockResult::Return(Ok(curr_block_header.target + 1)));

        assert_err!(
            BTCRelay::verify_block_header(&retarget_headers[2]),
            Error::DiffTargetHeader
        );
    })
}

#[test]
fn test_compute_new_target() {
    let chain_ref: u32 = 0;
    // no retarget at block 100
    let block_height: u32 = 2016;
    let retarget_headers = sample_retarget_interval_increase();

    let last_retarget_time = parse_block_header(&retarget_headers[0]).unwrap().timestamp;
    let prev_block_header =
        RichBlockHeader::construct(retarget_headers[1], chain_ref, block_height).unwrap();

    let curr_block_header = parse_block_header(&retarget_headers[2]).unwrap();

    BTCRelay::get_last_retarget_time
        .mock_safe(move |_, _| MockResult::Return(Ok(last_retarget_time)));

    let new_target = BTCRelay::compute_new_target(&prev_block_header, block_height).unwrap();

    assert_eq!(new_target, curr_block_header.target);
}

#[test]
fn test_verify_block_header_duplicate_fails() {
    run_test(|| {
        let chain_ref: u32 = 0;
        // no retarget at block 100
        let block_height: u32 = 100;
        let genesis_header = sample_parsed_genesis_header(chain_ref, block_height);

        let rich_first_header = sample_parsed_first_block(chain_ref, 101);

        // Prev block is genesis
        BTCRelay::get_block_header_from_hash
            .mock_safe(move |_| MockResult::Return(Ok(genesis_header)));
        // submitted block ALREADY EXISTS
        BTCRelay::block_header_exists.mock_safe(move |block_hash| {
            assert_eq!(&block_hash, &rich_first_header.block_hash);
            MockResult::Return(true)
        });

        let raw_first_header = RawBlockHeader::from_hex(sample_raw_first_header()).unwrap();
        assert_err!(
            BTCRelay::verify_block_header(&raw_first_header),
            Error::DuplicateBlock
        );
    })
}

#[test]
fn test_verify_block_header_no_prev_block_fails() {
    run_test(|| {
        // Prev block is MISSING
        BTCRelay::get_block_header_from_hash
            .mock_safe(move |_| MockResult::Return(Err(Error::PrevBlock)));
        // submitted block does not yet exist
        BTCRelay::block_header_exists.mock_safe(move |_| MockResult::Return(false));

        let raw_first_header = RawBlockHeader::from_hex(sample_raw_first_header()).unwrap();
        assert_err!(
            BTCRelay::verify_block_header(&raw_first_header),
            Error::PrevBlock
        );
    })
}

#[test]
fn test_verify_block_header_low_diff_fails() {
    run_test(|| {
        let chain_ref: u32 = 0;
        // no retarget at block 100
        let block_height: u32 = 100;
        let genesis_header = sample_parsed_genesis_header(chain_ref, block_height);

        // block header with high target but weak hash
        let raw_first_header_weak =
            RawBlockHeader::from_hex(sample_raw_first_header_low_diff()).unwrap();

        // Prev block is genesis
        BTCRelay::get_block_header_from_hash
            .mock_safe(move |_| MockResult::Return(Ok(genesis_header)));
        // submitted block does not yet exist
        BTCRelay::block_header_exists.mock_safe(move |_| MockResult::Return(false));

        assert_err!(
            BTCRelay::verify_block_header(&raw_first_header_weak),
            Error::LowDiff
        );
    });
}

// TODO: this currently fails with TX_FORMAT error in parser

#[test]
fn test_validate_transaction_succeeds() {
    run_test(|| {
        let raw_tx = hex::decode(sample_accepted_transaction()).unwrap();
        let payment_value: i64 = 2500200000;
        let recipient_btc_address =
            hex::decode("66c7060feb882664ae62ffad0051fe843e318e85".to_owned()).unwrap();
        let op_return_id = hex::decode(
            "aa21a9ede5c17d15b8b1fa2811b7e6da66ffa5e1aaa05922c69068bf90cd585b95bb4675".to_owned(),
        )
        .unwrap();

        let outputs = vec![sample_valid_payment_output(), sample_valid_data_output()];

        BTCRelay::parse_transaction
            .mock_safe(move |_| MockResult::Return(Ok(sample_transaction_parsed(&outputs))));

        assert_ok!(BTCRelay::validate_transaction(
            Origin::signed(3),
            raw_tx,
            payment_value,
            recipient_btc_address,
            op_return_id
        ))
    });
}

#[test]
fn test_validate_transaction_invalid_no_outputs_fails() {
    run_test(|| {
        // Simulate input (we mock the parsed transaction)
        let raw_tx = hex::decode(sample_accepted_transaction()).unwrap();

        let payment_value: i64 = 2500200000;
        let recipient_btc_address =
            hex::decode("66c7060feb882664ae62ffad0051fe843e318e85".to_owned()).unwrap();
        let op_return_id = hex::decode(
            "aa21a9ede5c17d15b8b1fa2811b7e6da66ffa5e1aaa05922c69068bf90cd585b95bb4675".to_owned(),
        )
        .unwrap();
        // missing required data output
        let outputs = vec![sample_valid_payment_output()];

        BTCRelay::parse_transaction
            .mock_safe(move |_| MockResult::Return(Ok(sample_transaction_parsed(&outputs))));

        assert_err!(
            BTCRelay::validate_transaction(
                Origin::signed(3),
                raw_tx,
                payment_value,
                recipient_btc_address,
                op_return_id
            ),
            Error::MalformedTransaction
        )
    });
}

#[test]
fn test_validate_transaction_insufficient_payment_value_fails() {
    run_test(|| {
        // Simulate input (we mock the parsed transaction)
        let raw_tx = vec![0u8; 342];

        let payment_value: i64 = 2500200000;
        let recipient_btc_address =
            hex::decode("66c7060feb882664ae62ffad0051fe843e318e85".to_owned()).unwrap();
        let op_return_id = hex::decode(
            "aa21a9ede5c17d15b8b1fa2811b7e6da66ffa5e1aaa05922c69068bf90cd585b95bb4675".to_owned(),
        )
        .unwrap();

        let outputs = vec![
            sample_insufficient_value_payment_output(),
            sample_valid_data_output(),
        ];

        BTCRelay::parse_transaction
            .mock_safe(move |_| MockResult::Return(Ok(sample_transaction_parsed(&outputs))));

        assert_err!(
            BTCRelay::validate_transaction(
                Origin::signed(3),
                raw_tx,
                payment_value,
                recipient_btc_address,
                op_return_id
            ),
            Error::InsufficientValue
        )
    });
}

#[test]
fn test_validate_transaction_wrong_recipient_fails() {
    run_test(|| {
        // Simulate input (we mock the parsed transaction)
        let raw_tx = vec![0u8; 342];

        let payment_value: i64 = 2500200000;
        let recipient_btc_address =
            hex::decode("66c7060feb882664ae62ffad0051fe843e318e85".to_owned()).unwrap();
        let op_return_id = hex::decode(
            "aa21a9ede5c17d15b8b1fa2811b7e6da66ffa5e1aaa05922c69068bf90cd585b95bb4675".to_owned(),
        )
        .unwrap();

        let outputs = vec![
            sample_wrong_recipient_payment_output(),
            sample_valid_data_output(),
        ];

        BTCRelay::parse_transaction
            .mock_safe(move |_| MockResult::Return(Ok(sample_transaction_parsed(&outputs))));

        assert_err!(
            BTCRelay::validate_transaction(
                Origin::signed(3),
                raw_tx,
                payment_value,
                recipient_btc_address,
                op_return_id
            ),
            Error::WrongRecipient
        )
    });
}

#[test]
fn test_validate_transaction_incorrect_opreturn_fails() {
    run_test(|| {
        // Simulate input (we mock the parsed transaction)
        let raw_tx = vec![0u8; 342];

        let payment_value: i64 = 2500200000;
        let recipient_btc_address =
            hex::decode("66c7060feb882664ae62ffad0051fe843e318e85".to_owned()).unwrap();
        let op_return_id = hex::decode(
            "6a24aa21a9ede5c17d15b8b1fa2811b7e6da66ffa5e1aaa05922c69068bf90cd585b95bb4675"
                .to_owned(),
        )
        .unwrap();

        let outputs = vec![
            sample_valid_payment_output(),
            sample_incorrect_data_output(),
        ];

        BTCRelay::parse_transaction
            .mock_safe(move |_| MockResult::Return(Ok(sample_transaction_parsed(&outputs))));

        assert_err!(
            BTCRelay::validate_transaction(
                Origin::signed(3),
                raw_tx,
                payment_value,
                recipient_btc_address,
                op_return_id
            ),
            Error::InvalidOpreturn
        )
    });
}

/// flag_block_error
#[test]
fn test_flag_block_error_succeeds() {
    run_test(|| {
        let chain_ref: u32 = 1;
        let start_height: u32 = 10;
        let block_height: u32 = 100;
        let block_header = hex::decode(sample_block_header_hex()).unwrap();

        let rich_header = RichBlockHeader {
            block_hash: H256Le::zero(),
            block_header: BlockHeader::from_le_bytes(&block_header).unwrap(),
            block_height: block_height,
            chain_ref: chain_ref,
        };

        BTCRelay::set_block_header_from_hash(rich_header.block_hash, &rich_header);

        let blockchain =
            get_empty_block_chain_from_chain_id_and_height(chain_ref, start_height, block_height);
        BTCRelay::set_block_chain_from_id(chain_ref, &blockchain);

        let error_codes = vec![ErrorCode::NoDataBTCRelay, ErrorCode::InvalidBTCRelay];

        for error in error_codes.iter() {
            assert_ok!(BTCRelay::flag_block_error(
                rich_header.block_hash,
                error.clone()
            ));
            let curr_chain = BTCRelay::get_block_chain_from_id(chain_ref).unwrap();

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
fn test_flag_block_error_fails() {
    run_test(|| {
        let chain_ref: u32 = 1;
        let start_height: u32 = 20;
        let block_height: u32 = 100;
        let block_header = hex::decode(sample_block_header_hex()).unwrap();

        let rich_header = RichBlockHeader {
            block_hash: H256Le::zero(),
            block_header: BlockHeader::from_le_bytes(&block_header).unwrap(),
            block_height: block_height,
            chain_ref: chain_ref,
        };

        BTCRelay::set_block_header_from_hash(rich_header.block_hash, &rich_header);

        let blockchain =
            get_empty_block_chain_from_chain_id_and_height(chain_ref, start_height, block_height);
        BTCRelay::set_block_chain_from_id(chain_ref, &blockchain);

        // not a valid error code
        let error = ErrorCode::Liquidation;

        assert_err!(
            BTCRelay::flag_block_error(rich_header.block_hash, error),
            Error::UnknownErrorcode
        );
    })
}

/// clear_block_error
#[test]
fn test_clear_block_error_succeeds() {
    run_test(|| {
        let chain_ref: u32 = 1;
        let start_height: u32 = 15;
        let block_height: u32 = 100;
        let block_header = hex::decode(sample_block_header_hex()).unwrap();

        let rich_header = RichBlockHeader {
            block_hash: H256Le::zero(),
            block_header: BlockHeader::from_le_bytes(&block_header).unwrap(),
            block_height: block_height,
            chain_ref: chain_ref,
        };

        BTCRelay::set_block_header_from_hash(rich_header.block_hash, &rich_header);

        let mut blockchain =
            get_empty_block_chain_from_chain_id_and_height(chain_ref, start_height, block_height);
        blockchain.no_data.insert(block_height);
        blockchain.invalid.insert(block_height);

        BTCRelay::set_block_chain_from_id(chain_ref, &blockchain);

        let error_codes = vec![ErrorCode::NoDataBTCRelay, ErrorCode::InvalidBTCRelay];

        for error in error_codes.iter() {
            assert_ok!(BTCRelay::clear_block_error(
                rich_header.block_hash,
                error.clone()
            ));
            let curr_chain = BTCRelay::get_block_chain_from_id(chain_ref).unwrap();

            if *error == ErrorCode::NoDataBTCRelay {
                assert!(!curr_chain.no_data.contains(&block_height));
            } else if *error == ErrorCode::InvalidBTCRelay {
                assert!(!curr_chain.invalid.contains(&block_height));
            };
            let error_event = TestEvent::test_events(Event::ClearBlockError(
                rich_header.block_hash,
                chain_ref,
                error.clone(),
            ));
            assert!(System::events().iter().any(|a| a.event == error_event));
        }
    })
}

#[test]
fn test_clear_block_error_fails() {
    run_test(|| {
        let chain_ref: u32 = 1;
        let start_height: u32 = 20;
        let block_height: u32 = 100;
        let block_header = hex::decode(sample_block_header_hex()).unwrap();

        let rich_header = RichBlockHeader {
            block_hash: H256Le::zero(),
            block_header: BlockHeader::from_le_bytes(&block_header).unwrap(),
            block_height: block_height,
            chain_ref: chain_ref,
        };

        BTCRelay::set_block_header_from_hash(rich_header.block_hash, &rich_header);

        let blockchain =
            get_empty_block_chain_from_chain_id_and_height(chain_ref, start_height, block_height);
        BTCRelay::set_block_chain_from_id(chain_ref, &blockchain);

        // not a valid error code
        let error = ErrorCode::Liquidation;

        assert_err!(
            BTCRelay::clear_block_error(rich_header.block_hash, error),
            Error::UnknownErrorcode
        );
    })
}

#[test]
fn test_verify_transaction_inclusion_succeeds() {
    run_test(|| {
        let chain_ref = 0;
        let fork_ref = 1;
        let block_height = 203;
        let start = 10;
        let main_chain_height = 300;
        let fork_chain_height = 280;
        // Random init since we mock this
        let raw_merkle_proof = vec![0u8; 100];
        let confirmations = 0;
        let insecure = false;
        let rich_block_header = sample_rich_tx_block_header(chain_ref, main_chain_height);

        let proof_result = sample_valid_proof_result();

        let main =
            get_empty_block_chain_from_chain_id_and_height(chain_ref, start, main_chain_height);

        let fork =
            get_empty_block_chain_from_chain_id_and_height(fork_ref, start, fork_chain_height);

        BTCRelay::get_chain_id_from_position
            .mock_safe(move |_| MockResult::Return(fork_ref.clone()));
        BTCRelay::get_block_chain_from_id.mock_safe(move |id| {
            if id == chain_ref.clone() {
                return MockResult::Return(Ok(main.clone()));
            } else {
                return MockResult::Return(Ok(fork.clone()));
            }
        });

        BTCRelay::get_best_block_height.mock_safe(move || MockResult::Return(main_chain_height));

        BTCRelay::verify_merkle_proof.mock_safe(move |_| MockResult::Return(Ok(proof_result)));

        BTCRelay::get_block_header_from_height
            .mock_safe(move |_, _| MockResult::Return(Ok(rich_block_header)));

        BTCRelay::check_confirmations.mock_safe(|_, _, _, _| MockResult::Return(Ok(())));

        assert_ok!(BTCRelay::verify_transaction_inclusion(
            Origin::signed(3),
            proof_result.transaction_hash,
            block_height,
            raw_merkle_proof,
            confirmations,
            insecure
        ));
    });
}

#[test]
fn test_verify_transaction_inclusion_invalid_tx_id_fails() {
    run_test(|| {
        let chain_ref = 0;
        let fork_ref = 1;
        let block_height = 203;
        let start = 10;
        let main_chain_height = 300;
        let fork_chain_height = 280;
        // Random init since we mock this
        let raw_merkle_proof = vec![0u8; 100];
        let confirmations = 0;
        let insecure = false;
        let rich_block_header = sample_rich_tx_block_header(chain_ref, main_chain_height);

        // Mismatching TXID
        let invalid_tx_id = H256Le::from_bytes_le(
            &hex::decode(
                "0000000000000000000000000000000000000000000000000000000000000000".to_owned(),
            )
            .unwrap(),
        );

        let proof_result = sample_valid_proof_result();

        let main =
            get_empty_block_chain_from_chain_id_and_height(chain_ref, start, main_chain_height);

        let fork =
            get_empty_block_chain_from_chain_id_and_height(fork_ref, start, fork_chain_height);

        BTCRelay::get_chain_id_from_position
            .mock_safe(move |_| MockResult::Return(fork_ref.clone()));
        BTCRelay::get_block_chain_from_id.mock_safe(move |id| {
            if id == chain_ref.clone() {
                return MockResult::Return(Ok(main.clone()));
            } else {
                return MockResult::Return(Ok(fork.clone()));
            }
        });

        BTCRelay::get_best_block_height.mock_safe(move || MockResult::Return(main_chain_height));

        BTCRelay::verify_merkle_proof.mock_safe(move |_| MockResult::Return(Ok(proof_result)));

        BTCRelay::get_block_header_from_height
            .mock_safe(move |_, _| MockResult::Return(Ok(rich_block_header)));

        BTCRelay::check_confirmations.mock_safe(|_, _, _, _| MockResult::Return(Ok(())));

        assert_err!(
            BTCRelay::verify_transaction_inclusion(
                Origin::signed(3),
                invalid_tx_id,
                block_height,
                raw_merkle_proof,
                confirmations,
                insecure
            ),
            Error::InvalidTxid
        );
    });
}

#[test]
fn test_verify_transaction_inclusion_invalid_merkle_root_fails() {
    run_test(|| {
        let chain_ref = 0;
        let fork_ref = 1;
        let block_height = 203;
        let start = 10;
        let main_chain_height = 300;
        let fork_chain_height = 280;
        // Random init since we mock this
        let raw_merkle_proof = vec![0u8; 100];
        let confirmations = 0;
        let insecure = false;
        let mut rich_block_header = sample_rich_tx_block_header(chain_ref, main_chain_height);

        // Mismatching merkle root
        let invalid_merkle_root = H256Le::from_bytes_le(
            &hex::decode(
                "0000000000000000000000000000000000000000000000000000000000000000".to_owned(),
            )
            .unwrap(),
        );
        rich_block_header.block_header.merkle_root = invalid_merkle_root;

        let proof_result = sample_valid_proof_result();

        let main =
            get_empty_block_chain_from_chain_id_and_height(chain_ref, start, main_chain_height);

        let fork =
            get_empty_block_chain_from_chain_id_and_height(fork_ref, start, fork_chain_height);

        BTCRelay::get_chain_id_from_position
            .mock_safe(move |_| MockResult::Return(fork_ref.clone()));
        BTCRelay::get_block_chain_from_id.mock_safe(move |id| {
            if id == chain_ref.clone() {
                return MockResult::Return(Ok(main.clone()));
            } else {
                return MockResult::Return(Ok(fork.clone()));
            }
        });

        BTCRelay::get_best_block_height.mock_safe(move || MockResult::Return(main_chain_height));

        BTCRelay::verify_merkle_proof.mock_safe(move |_| MockResult::Return(Ok(proof_result)));

        BTCRelay::get_block_header_from_height
            .mock_safe(move |_, _| MockResult::Return(Ok(rich_block_header)));

        BTCRelay::check_confirmations.mock_safe(|_, _, _, _| MockResult::Return(Ok(())));

        assert_err!(
            BTCRelay::verify_transaction_inclusion(
                Origin::signed(3),
                proof_result.transaction_hash,
                block_height,
                raw_merkle_proof,
                confirmations,
                insecure
            ),
            Error::InvalidMerkleProof
        );
    });
}

#[test]
fn test_verify_transaction_inclusion_fails_with_ongoing_fork() {
    run_test(|| {
        BTCRelay::get_block_chain_from_id
            .mock_safe(|_| MockResult::Return(Ok(BlockChain::default())));

        let tx_id = sample_valid_proof_result().transaction_hash;
        let block_height = 203;
        let raw_merkle_proof = vec![0u8; 100];
        let confirmations = 0;
        let insecure = false;

        assert_err!(
            BTCRelay::verify_transaction_inclusion(
                Origin::signed(3),
                tx_id,
                block_height,
                raw_merkle_proof,
                confirmations,
                insecure
            ),
            Error::OngoingFork
        );
    });
}

#[test]
fn test_check_confirmations_insecure_succeeds() {
    run_test(|| {
        let main_chain_height = 100;
        let tx_block_height = 90;
        let insecure = true;

        let req_confs = 5;
        assert_ok!(BTCRelay::check_confirmations(
            main_chain_height,
            req_confs,
            tx_block_height,
            insecure
        ))
    });
}

#[test]
fn test_check_confirmations_insecure_insufficient_confs_fails() {
    run_test(|| {
        let main_chain_height = 100;
        let tx_block_height = 99;
        let insecure = true;

        let req_confs = 5;

        assert_err!(
            BTCRelay::check_confirmations(main_chain_height, req_confs, tx_block_height, insecure),
            Error::Confirmations
        )
    });
}

#[test]
fn test_check_confirmations_secure_stable_confs_succeeds() {
    run_test(|| {
        let main_chain_height = 100;
        let tx_block_height = 90;
        let insecure = false;

        let req_confs = 5;
        // relevant check: ok
        let stable_confs = 10;

        BTCRelay::get_stable_transaction_confirmations
            .mock_safe(move || MockResult::Return(stable_confs));
        assert_ok!(BTCRelay::check_confirmations(
            main_chain_height,
            req_confs,
            tx_block_height,
            insecure
        ))
    });
}

#[test]
fn test_check_confirmations_secure_user_confs_succeeds() {
    run_test(|| {
        let main_chain_height = 100;
        let tx_block_height = 85;
        let insecure = false;

        // relevant check: ok
        let req_confs = 15;
        let stable_confs = 10;

        BTCRelay::get_stable_transaction_confirmations
            .mock_safe(move || MockResult::Return(stable_confs));
        assert_ok!(BTCRelay::check_confirmations(
            main_chain_height,
            req_confs,
            tx_block_height,
            insecure
        ))
    });
}

#[test]
fn test_check_confirmations_secure_insufficient_stable_confs_succeeds() {
    run_test(|| {
        let main_chain_height = 100;
        let tx_block_height = 91;
        let insecure = false;

        let req_confs = 5;
        // relevant check: fails
        let stable_confs = 10;

        BTCRelay::get_stable_transaction_confirmations
            .mock_safe(move || MockResult::Return(stable_confs));

        assert_err!(
            BTCRelay::check_confirmations(main_chain_height, req_confs, tx_block_height, insecure),
            Error::InsufficientStableConfirmations
        )
    });
}

#[test]
fn test_check_confirmations_secure_insufficient_user_confs_succeeds() {
    run_test(|| {
        let main_chain_height = 100;
        let tx_block_height = 90;
        let insecure = false;

        // relevant check: fails
        let req_confs = 15;
        let stable_confs = 10;

        BTCRelay::get_stable_transaction_confirmations
            .mock_safe(move || MockResult::Return(stable_confs));

        assert_err!(
            BTCRelay::check_confirmations(main_chain_height, req_confs, tx_block_height, insecure),
            Error::Confirmations
        )
    });
}
/// # Util functions

fn sample_valid_proof_result() -> ProofResult {
    let tx_id = H256Le::from_bytes_le(
        &hex::decode("c8589f304d3b9df1d4d8b3d15eb6edaaa2af9d796e9d9ace12b31f293705c5e9".to_owned())
            .unwrap(),
    );
    let merkle_root = H256Le::from_bytes_le(
        &hex::decode("1EE1FB90996CA1D5DCD12866BA9066458BF768641215933D7D8B3A10EF79D090".to_owned())
            .unwrap(),
    );

    ProofResult {
        extracted_root: merkle_root,
        transaction_hash: tx_id,
        transaction_position: 0,
    }
}

fn get_empty_block_chain_from_chain_id_and_height(
    chain_id: u32,
    start_height: u32,
    block_height: u32,
) -> BlockChain {
    let blockchain = BlockChain {
        chain_id: chain_id,
        start_height: start_height,
        max_height: block_height,
        no_data: BTreeSet::new(),
        invalid: BTreeSet::new(),
    };

    blockchain
}

fn store_blockchain_and_random_headers(
    id: u32,
    start_height: u32,
    max_height: u32,
    position: u32,
) -> BlockChain {
    let mut chain = get_empty_block_chain_from_chain_id_and_height(id, start_height, max_height);

    // create and insert main chain headers
    for height in chain.start_height..chain.max_height + 1 {
        let block_header = hex::decode(sample_block_header_hex()).unwrap();
        let mut fake_block = height.to_be_bytes().repeat(7);
        fake_block.append(&mut id.to_be_bytes().to_vec());
        let block_hash = H256Le::from_bytes_be(fake_block.as_slice());

        let rich_header = RichBlockHeader {
            block_hash: block_hash,
            block_header: BlockHeader::from_le_bytes(&block_header).unwrap(),
            block_height: height,
            chain_ref: id,
        };

        BTCRelay::set_block_header_from_hash(block_hash, &rich_header);
        chain = BTCRelay::extend_blockchain(height, &block_hash, chain).unwrap();
    }
    // insert the main chain in Chains and ChainsIndex
    BTCRelay::set_chain_from_position_and_id(position, id);
    BTCRelay::set_block_chain_from_id(id, &chain);

    chain
}

fn sample_raw_genesis_header() -> String {
    "01000000".to_owned() + "a7c3299ed2475e1d6ea5ed18d5bfe243224add249cce99c5c67cc9fb00000000601c73862a0a7238e376f497783c8ecca2cf61a4f002ec8898024230787f399cb575d949ffff001d3a5de07f"
}

fn sample_parsed_genesis_header(chain_ref: u32, block_height: u32) -> RichBlockHeader {
    let genesis_header = RawBlockHeader::from_hex(sample_raw_genesis_header()).unwrap();
    RichBlockHeader {
        block_hash: genesis_header.hash(),
        block_header: parse_block_header(&genesis_header).unwrap(),
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
    let block_header = RawBlockHeader::from_hex(sample_raw_first_header()).unwrap();
    RichBlockHeader {
        block_hash: block_header.hash(),
        block_header: parse_block_header(&block_header).unwrap(),
        block_height: block_height,
        chain_ref: chain_ref,
    }
}

fn sample_retarget_interval_increase() -> [RawBlockHeader; 3] {
    // block height 66528
    let last_retarget_header = RawBlockHeader::from_hex("01000000".to_owned() + "4e8e5cf3c4e4b8f63a9cf88beb2dbaba1949182101ae4e5cf54ad100000000009f2a2344e8112b0d7bd8089414106ee5f17bb6cd64078883e1b661fa251aac6bed1d3c4cf4a3051c4dcd2b02").unwrap();
    // block height 66543
    let prev_block_header = RawBlockHeader::from_hex("01000000".to_owned()  + "1e321d88cb25946c4ca521eece3752803c021f9403fc4e0171203a0500000000317057f8b50414848a5a3a26d9eb8ace3d6f5495df456d0104dd1421159faf5029293c4cf4a3051c73199005").unwrap();
    // block height 68544
    let curr_header = RawBlockHeader::from_hex("01000000".to_owned() + "fb57c71ccd211b3de4ccc2e23b50a7cdb72aab91e60737b3a2bfdf030000000088a88ad9df68925e880e5d52b7e50cef225871c68b40a2cd0bca1084cd436037f388404cfd68011caeb1f801").unwrap();

    [last_retarget_header, prev_block_header, curr_header]
}

fn sample_retarget_interval_decrease() -> [RawBlockHeader; 3] {
    // block height 558432
    let last_retarget_header = RawBlockHeader::from_hex("00c0ff2f".to_owned() + "6550b5dae76559589e3e3e135237072b6bc498949da6280000000000000000005988783435f506d2ccfbadb484e56d6f1d5dfdd480650acae1e3b43d3464ea73caf13b5c33d62f171d508fdb").unwrap();
    // block height 560447
    let prev_block_header = RawBlockHeader::from_hex("00000020".to_owned()  + "d8e8e54ca5e33522b94fbba5de736efc55ff75e832cf2300000000000000000007b395f80858ee022c9c3c2f0f5cee4bd807039f0729b0559ae4326c3ba77d6b209f4e5c33d62f1746ee356d").unwrap();
    // block height 560448
    let curr_header = RawBlockHeader::from_hex("00000020".to_owned() + "6b05bd2c4a06b3d8503a033c2593396a25a79e1dcadb140000000000000000001b08df3d42cd9a38d8b66adf9dc5eb464f503633bd861085ffff723634531596a1a24e5c35683017bf67b72a").unwrap();

    [last_retarget_header, prev_block_header, curr_header]
}

fn sample_accepted_transaction() -> String {
    "020000000001010000000000000000000000000000000000000000000000000000000000000000ffffffff0502cb000101ffffffff02400606950000000017a91466c7060feb882664ae62ffad0051fe843e318e85870000000000000000266a24aa21a9ede5c17d15b8b1fa2811b7e6da66ffa5e1aaa05922c69068bf90cd585b95bb46750120000000000000000000000000000000000000000000000000000000000000000000000000".to_owned()
}

fn sample_block_header_hex() -> String {
    "02000000".to_owned() + // ............... Block version: 2
    "b6ff0b1b1680a2862a30ca44d346d9e8" + //
    "910d334beb48ca0c0000000000000000" + // ... Hash of previous block's header
    "9d10aa52ee949386ca9385695f04ede2" + //
    "70dda20810decd12bc9b048aaab31471" + // ... Merkle root
    "24d95a54" + // ........................... Unix time: 1415239972
    "30c31b18" + // ........................... Target: 0x1bc330 * 256**(0x18-3)
    "fe9f0864"
}

fn sample_rich_tx_block_header(chain_ref: u32, block_height: u32) -> RichBlockHeader {
    let raw_header = RawBlockHeader::from_hex("0000003096cb3d93696c4f56c10da153963d35abf4692c07b2b3bf0702fb4cb32a8682211ee1fb90996ca1d5dcd12866ba9066458bf768641215933d7d8b3a10ef79d090e8a13a5effff7f2005000000".to_owned()).unwrap();
    RichBlockHeader {
        block_hash: raw_header.hash(),
        block_header: parse_block_header(&raw_header).unwrap(),
        block_height: block_height,
        chain_ref: chain_ref,
    }
}

fn sample_valid_payment_output() -> TransactionOutput {
    TransactionOutput {
        value: 2500200000,
        script: hex::decode("a91466c7060feb882664ae62ffad0051fe843e318e8587".to_owned()).unwrap(),
    }
}

fn sample_insufficient_value_payment_output() -> TransactionOutput {
    TransactionOutput {
        value: 100,
        script: hex::decode("a91466c7060feb882664ae62ffad0051fe843e318e8587".to_owned()).unwrap(),
    }
}

fn sample_wrong_recipient_payment_output() -> TransactionOutput {
    TransactionOutput {
        value: 2500200000,
        script: hex::decode("a914000000000000000000000000000000000000000087".to_owned()).unwrap(),
    }
}

fn sample_valid_data_output() -> TransactionOutput {
    TransactionOutput {
        value: 0,
        script: hex::decode(
            "6a24aa21a9ede5c17d15b8b1fa2811b7e6da66ffa5e1aaa05922c69068bf90cd585b95bb4675"
                .to_owned(),
        )
        .unwrap(),
    }
}

fn sample_incorrect_data_output() -> TransactionOutput {
    TransactionOutput {
        value: 0,
        script: hex::decode(
            "6a24000000000000000000000000000000000000000000000000000000000000000000000000"
                .to_owned(),
        )
        .unwrap(),
    }
}

fn sample_transaction_parsed(outputs: &Vec<TransactionOutput>) -> Transaction {
    let mut inputs: Vec<TransactionInput> = Vec::new();

    let spent_output_txid =
        hex::decode("b28f1e58af1d4db02d1b9f0cf8d51ece3dd5f5013fd108647821ea255ae5daff".to_owned())
            .unwrap();
    let input = TransactionInput {
        previous_hash: H256Le::from_bytes_le(&spent_output_txid),
        previous_index: 0,
        coinbase: false,
        height: None,
        script: hex::decode("16001443feac9ca9d20883126e30e962ca11fda07f808b".to_owned()).unwrap(),
        sequence: 4294967295,
        witness: None,
    };

    inputs.push(input);

    Transaction {
        version: 2,
        inputs: inputs,
        outputs: outputs.to_vec(),
        block_height: Some(203),
        locktime: Some(0),
    }
}
