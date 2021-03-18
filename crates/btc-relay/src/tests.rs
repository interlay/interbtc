/// Tests for BTC-Relay
use primitive_types::U256;

use crate::{ext, mock::*, types::*, BtcAddress};

type Event = crate::Event<Test>;

use bitcoin::{formatter::TryFormattable, merkle::*, parser::*, types::*};
use frame_support::{assert_err, assert_ok};
use mocktopus::mocking::*;
use security::{ErrorCode, StatusCode};
use sp_std::{collections::btree_set::BTreeSet, convert::TryInto, str::FromStr};

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

        let rich_header = RichBlockHeader::<AccountId> {
            block_hash: H256Le::zero(),
            block_header: BlockHeader::from_le_bytes(&block_header).unwrap(),
            block_height: block_height,
            chain_ref: chain_ref,
            account_id: Default::default(),
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
            TestError::BlockNotFound
        );
    })
}

/// next_best_fork_chain
/// set_block_chain_from_id
#[test]
fn next_best_fork_chain_succeeds() {
    run_test(|| {
        let chain_ref: u32 = 1;
        let start_height: u32 = 10;
        let block_height: u32 = 100;

        let blockchain = get_empty_block_chain_from_chain_id_and_height(chain_ref, start_height, block_height);

        BTCRelay::set_block_chain_from_id(chain_ref, &blockchain);

        let curr_blockchain = BTCRelay::get_block_chain_from_id(chain_ref).unwrap();

        assert_eq!(curr_blockchain, blockchain);
    })
}

#[test]
fn test_get_block_chain_from_id_empty_chain_fails() {
    run_test(|| {
        assert_err!(BTCRelay::get_block_chain_from_id(1), TestError::InvalidChainID);
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

        assert_ok!(BTCRelay::initialize(Origin::signed(3), block_header, block_height));

        let init_event = TestEvent::btc_relay(Event::Initialized(block_height, block_header_hash, 3));
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
            TestError::AlreadyInitialized
        );
    })
}

/// store_block_header function
#[test]
fn store_block_header_on_mainchain_succeeds() {
    run_test(|| {
        BTCRelay::verify_block_header
            .mock_safe(|h| MockResult::Return(Ok(BlockHeader::from_le_bytes(h.as_bytes()).unwrap())));
        BTCRelay::block_header_exists.mock_safe(|_| MockResult::Return(true));

        let chain_ref: u32 = 0;
        let start_height: u32 = 0;
        let block_height: u32 = 100;
        let block_header = RawBlockHeader::from_hex(sample_block_header_hex()).unwrap();

        let rich_header = RichBlockHeader::<AccountId> {
            block_hash: H256Le::zero(),
            block_header: parse_block_header(&block_header).unwrap(),
            block_height: block_height,
            chain_ref: chain_ref,
            account_id: Default::default(),
        };
        BTCRelay::get_block_header_from_hash.mock_safe(move |_| MockResult::Return(Ok(rich_header)));

        let prev_blockchain = get_empty_block_chain_from_chain_id_and_height(chain_ref, start_height, block_height);
        BTCRelay::get_block_chain_from_id.mock_safe(move |_: u32| MockResult::Return(Ok(prev_blockchain.clone())));

        let block_header_hash = block_header.hash();
        assert_ok!(BTCRelay::store_block_header(Origin::signed(3), block_header));

        let store_main_event =
            TestEvent::btc_relay(Event::StoreMainChainHeader(block_height + 1, block_header_hash, 3));
        assert!(System::events().iter().any(|a| a.event == store_main_event));
    })
}

#[test]
fn store_block_header_on_fork_succeeds() {
    run_test(|| {
        BTCRelay::verify_block_header.mock_safe(|h| {
            MockResult::Return(match parse_block_header(&h) {
                Ok(h) => Ok(h),
                Err(e) => Err(TestError::from(e).into()),
            })
        });
        BTCRelay::block_header_exists.mock_safe(|_| MockResult::Return(true));

        let chain_ref: u32 = 1;
        let start_height: u32 = 20;
        let block_height: u32 = 100;
        let block_header = RawBlockHeader::from_hex(sample_block_header_hex()).unwrap();

        let rich_header = RichBlockHeader::<AccountId> {
            block_hash: H256Le::zero(),
            block_header: parse_block_header(&block_header).unwrap(),
            block_height: block_height - 1,
            chain_ref: chain_ref,
            account_id: Default::default(),
        };
        BTCRelay::get_block_header_from_hash.mock_safe(move |_| MockResult::Return(Ok(rich_header)));

        let prev_blockchain = get_empty_block_chain_from_chain_id_and_height(chain_ref, start_height, block_height);
        BTCRelay::get_block_chain_from_id.mock_safe(move |_: u32| MockResult::Return(Ok(prev_blockchain.clone())));

        let block_header_hash = block_header.hash();
        assert_ok!(BTCRelay::store_block_header(Origin::signed(3), block_header));

        let store_fork_event =
            TestEvent::btc_relay(Event::StoreForkHeader(chain_ref, block_height, block_header_hash, 3));
        assert!(System::events().iter().any(|a| a.event == store_fork_event));
    })
}

#[test]
fn store_block_header_parachain_shutdown_fails() {
    run_test(|| {
        let block_header = RawBlockHeader::from_hex(sample_block_header_hex()).unwrap();

        ext::security::ensure_parachain_status_not_shutdown::<Test>
            .mock_safe(|| MockResult::Return(Err(SecurityError::ParachainShutdown.into())));

        assert_err!(
            BTCRelay::store_block_header(Origin::signed(3), block_header),
            SecurityError::ParachainShutdown,
        );
    })
}
/// check_and_do_reorg function
#[test]
fn check_and_do_reorg_is_main_chain_succeeds() {
    run_test(|| {
        let chain_ref: u32 = 0;
        let start_height: u32 = 3;
        let block_height: u32 = 10;

        let blockchain = get_empty_block_chain_from_chain_id_and_height(chain_ref, start_height, block_height);

        assert_ok!(BTCRelay::check_and_do_reorg(&blockchain));
    })
}

#[test]
fn check_and_do_reorg_fork_id_not_found() {
    run_test(|| {
        let chain_ref: u32 = 99;
        let start_height: u32 = 3;
        let block_height: u32 = 10;

        let blockchain = get_empty_block_chain_from_chain_id_and_height(chain_ref, start_height, block_height);

        assert_err!(BTCRelay::check_and_do_reorg(&blockchain), TestError::ForkIdNotFound);
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
        let main = get_empty_block_chain_from_chain_id_and_height(main_chain_ref, main_start_height, main_block_height);
        BTCRelay::set_chain_from_position_and_id(main_position, main_chain_ref);
        BTCRelay::set_block_chain_from_id(main_chain_ref, &main);

        // insert the fork chain in Chains and ChainsIndex
        let fork_chain_ref: u32 = 4;
        let fork_start_height: u32 = 20;
        let fork_block_height: u32 = 100;
        let fork_position: u32 = 2;
        let fork = get_empty_block_chain_from_chain_id_and_height(fork_chain_ref, fork_start_height, fork_block_height);
        BTCRelay::set_chain_from_position_and_id(fork_position, fork_chain_ref);
        BTCRelay::set_block_chain_from_id(fork_chain_ref, &fork);

        // insert the swap chain in Chains and ChainsIndex
        let swap_chain_ref: u32 = 3;
        let swap_start_height: u32 = 43;
        let swap_block_height: u32 = 99;
        let swap_position: u32 = 1;
        let swap = get_empty_block_chain_from_chain_id_and_height(swap_chain_ref, swap_start_height, swap_block_height);
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
        let main = get_empty_block_chain_from_chain_id_and_height(main_chain_ref, main_start_height, main_block_height);
        BTCRelay::set_chain_from_position_and_id(main_position, main_chain_ref);
        BTCRelay::set_block_chain_from_id(main_chain_ref, &main);

        // insert the fork chain in Chains and ChainsIndex
        let fork_chain_ref: u32 = 4;
        let fork_block_height: u32 = 117;
        let fork_position: u32 = 1;
        let fork = get_empty_block_chain_from_chain_id_and_height(fork_chain_ref, main_start_height, fork_block_height);
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
        let reorg_event = TestEvent::btc_relay(Event::ChainReorg(
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
        let main = get_empty_block_chain_from_chain_id_and_height(main_chain_ref, main_start_height, main_block_height);
        BTCRelay::set_chain_from_position_and_id(main_position, main_chain_ref);
        BTCRelay::set_block_chain_from_id(main_chain_ref, &main);

        // insert the fork chain in Chains and ChainsIndex
        let fork_chain_ref: u32 = 4;
        let fork_block_height: u32 = 113;
        let fork_position: u32 = 1;
        let fork = get_empty_block_chain_from_chain_id_and_height(fork_chain_ref, main_start_height, fork_block_height);
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
        let ahead_event = TestEvent::btc_relay(Event::ForkAheadOfMainChain(
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
        let main = get_empty_block_chain_from_chain_id_and_height(main_chain_ref, main_start_height, main_block_height);
        BTCRelay::set_block_chain_from_id(main_chain_ref, &main);
        assert_eq!(Ok(()), BTCRelay::insert_sorted(&main));

        let curr_main_pos = BTCRelay::get_chain_position_from_chain_id(main_chain_ref).unwrap();
        assert_eq!(curr_main_pos, main_position);
        // insert the swap chain in Chains and ChainsIndex
        let swap_chain_ref: u32 = 3;
        let swap_start_height: u32 = 70;
        let swap_block_height: u32 = 99;
        let swap_position: u32 = 1;
        let swap = get_empty_block_chain_from_chain_id_and_height(swap_chain_ref, swap_start_height, swap_block_height);
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
        let fork = get_empty_block_chain_from_chain_id_and_height(fork_chain_ref, fork_start_height, fork_block_height);
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
        let main = store_blockchain_and_random_headers(main_chain_ref, main_start, main_height, main_position);

        // simulate error
        let header = BTCRelay::get_block_header_from_height(&main, 2).unwrap();
        BTCRelay::flag_block_error(header.block_hash, ErrorCode::NoDataBTCRelay).unwrap();
        set_parachain_nodata_error();

        // insert the fork chain and headers
        let fork_chain_ref: u32 = 4;
        let fork_start: u32 = 5;
        let fork_height: u32 = 17;
        let fork_position: u32 = 1;

        let fork = store_blockchain_and_random_headers(fork_chain_ref, fork_start, fork_height, fork_position);

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
        assert_eq!(fork_height + 1, BTCRelay::_blocks_count(main_chain_ref) as u32);

        assert_eq!(main.no_data, BTreeSet::new());
        assert_eq!(main.invalid, new_main.invalid);

        // check that the fork is deleted
        assert_err!(
            BTCRelay::get_block_chain_from_id(fork_chain_ref),
            TestError::InvalidChainID,
        );

        // check that the parachain has recovered
        assert_ok!(ext::security::ensure_parachain_status_running::<Test>());
        assert!(!ext::security::is_parachain_error_no_data_btcrelay::<Test>());

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
        BTCRelay::get_block_header_from_hash.mock_safe(move |_| MockResult::Return(Ok(genesis_header)));
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
            RichBlockHeader::<AccountId>::new(retarget_headers[1], chain_ref, block_height, Default::default())
                .unwrap();

        let curr_block_header = parse_block_header(&retarget_headers[2]).unwrap();
        // Prev block exists
        BTCRelay::get_block_header_from_hash.mock_safe(move |_| MockResult::Return(Ok(prev_block_header_rich)));
        // Not duplicate block
        BTCRelay::block_header_exists.mock_safe(move |_| MockResult::Return(false));
        // Compute new target returns target of submitted header (i.e., correct)
        BTCRelay::compute_new_target.mock_safe(move |_, _| MockResult::Return(Ok(curr_block_header.target)));

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
            RichBlockHeader::<AccountId>::new(retarget_headers[1], chain_ref, block_height, Default::default())
                .unwrap();

        let curr_block_header = parse_block_header(&retarget_headers[2]).unwrap();
        // Prev block exists
        BTCRelay::get_block_header_from_hash.mock_safe(move |_| MockResult::Return(Ok(prev_block_header_rich)));
        // Not duplicate block
        BTCRelay::block_header_exists.mock_safe(move |_| MockResult::Return(false));
        // Compute new target returns target of submitted header (i.e., correct)
        BTCRelay::compute_new_target.mock_safe(move |_, _| MockResult::Return(Ok(curr_block_header.target)));

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
            RichBlockHeader::<AccountId>::new(retarget_headers[1], chain_ref, block_height, Default::default())
                .unwrap();

        let curr_block_header = parse_block_header(&retarget_headers[2]).unwrap();
        // Prev block exists
        BTCRelay::get_block_header_from_hash.mock_safe(move |_| MockResult::Return(Ok(prev_block_header_rich)));
        // Not duplicate block
        BTCRelay::block_header_exists.mock_safe(move |_| MockResult::Return(false));
        // Compute new target returns HIGHER target
        BTCRelay::compute_new_target.mock_safe(move |_, _| MockResult::Return(Ok(curr_block_header.target + 1)));

        assert_err!(
            BTCRelay::verify_block_header(&retarget_headers[2]),
            TestError::DiffTargetHeader
        );
    })
}

#[test]
fn test_compute_new_target() {
    let chain_ref: u32 = 0;
    // no retarget at block 100
    let block_height: u32 = 2016;
    let retarget_headers = sample_retarget_interval_increase();

    let last_retarget_time = parse_block_header(&retarget_headers[0]).unwrap().timestamp as u64;
    let prev_block_header =
        RichBlockHeader::<AccountId>::new(retarget_headers[1], chain_ref, block_height, Default::default()).unwrap();

    let curr_block_header = parse_block_header(&retarget_headers[2]).unwrap();

    BTCRelay::get_last_retarget_time.mock_safe(move |_, _| MockResult::Return(Ok(last_retarget_time)));

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
        BTCRelay::get_block_header_from_hash.mock_safe(move |_| MockResult::Return(Ok(genesis_header)));
        // submitted block ALREADY EXISTS
        BTCRelay::block_header_exists.mock_safe(move |block_hash| {
            assert_eq!(&block_hash, &rich_first_header.block_hash);
            MockResult::Return(true)
        });

        let raw_first_header = RawBlockHeader::from_hex(sample_raw_first_header()).unwrap();
        assert_err!(
            BTCRelay::verify_block_header(&raw_first_header),
            TestError::DuplicateBlock
        );
    })
}

#[test]
fn test_verify_block_header_no_prev_block_fails() {
    run_test(|| {
        // Prev block is MISSING
        BTCRelay::get_block_header_from_hash.mock_safe(move |_| MockResult::Return(Err(TestError::PrevBlock.into())));
        // submitted block does not yet exist
        BTCRelay::block_header_exists.mock_safe(move |_| MockResult::Return(false));

        let raw_first_header = RawBlockHeader::from_hex(sample_raw_first_header()).unwrap();
        assert_err!(BTCRelay::verify_block_header(&raw_first_header), TestError::PrevBlock);
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
        let raw_first_header_weak = RawBlockHeader::from_hex(sample_raw_first_header_low_diff()).unwrap();

        // Prev block is genesis
        BTCRelay::get_block_header_from_hash.mock_safe(move |_| MockResult::Return(Ok(genesis_header)));
        // submitted block does not yet exist
        BTCRelay::block_header_exists.mock_safe(move |_| MockResult::Return(false));

        assert_err!(
            BTCRelay::verify_block_header(&raw_first_header_weak),
            TestError::LowDiff
        );
    });
}

#[test]
fn test_validate_transaction_succeeds_with_payment() {
    run_test(|| {
        let raw_tx = hex::decode(sample_accepted_transaction()).unwrap();
        let minimum_btc: i64 = 2500200000;
        let recipient_btc_address =
            BtcAddress::P2SH(H160::from_str(&"66c7060feb882664ae62ffad0051fe843e318e85").unwrap());

        let outputs = vec![sample_valid_payment_output()];

        BTCRelay::parse_transaction.mock_safe(move |_| MockResult::Return(Ok(sample_transaction_parsed(&outputs))));

        BTCRelay::is_op_return_disabled.mock_safe(move || MockResult::Return(true));

        assert_ok!(BTCRelay::validate_transaction(
            Origin::signed(3),
            raw_tx,
            minimum_btc,
            recipient_btc_address,
            Some(vec![])
        ));
    });
}

#[test]
fn test_validate_transaction_succeeds_with_payment_and_op_return() {
    run_test(|| {
        let raw_tx = hex::decode(sample_accepted_transaction()).unwrap();
        let minimum_btc: i64 = 2500200000;
        let recipient_btc_address =
            BtcAddress::P2SH(H160::from_str(&"66c7060feb882664ae62ffad0051fe843e318e85").unwrap());
        let op_return_id =
            hex::decode("aa21a9ede5c17d15b8b1fa2811b7e6da66ffa5e1aaa05922c69068bf90cd585b95bb4675".to_owned()).unwrap();

        let outputs = vec![sample_valid_payment_output(), sample_valid_data_output()];

        BTCRelay::parse_transaction.mock_safe(move |_| MockResult::Return(Ok(sample_transaction_parsed(&outputs))));

        assert_ok!(BTCRelay::validate_transaction(
            Origin::signed(3),
            raw_tx,
            minimum_btc,
            recipient_btc_address,
            Some(op_return_id)
        ));
    });
}

#[test]
fn test_validate_transaction_succeeds_with_op_return_and_payment() {
    run_test(|| {
        let raw_tx = hex::decode(sample_accepted_transaction()).unwrap();
        let minimum_btc: i64 = 2500200000;
        let recipient_btc_address =
            BtcAddress::P2SH(H160::from_str(&"66c7060feb882664ae62ffad0051fe843e318e85").unwrap());
        let op_return_id =
            hex::decode("aa21a9ede5c17d15b8b1fa2811b7e6da66ffa5e1aaa05922c69068bf90cd585b95bb4675".to_owned()).unwrap();

        let outputs = vec![sample_valid_data_output(), sample_valid_payment_output()];

        BTCRelay::parse_transaction.mock_safe(move |_| MockResult::Return(Ok(sample_transaction_parsed(&outputs))));

        assert_ok!(BTCRelay::validate_transaction(
            Origin::signed(3),
            raw_tx,
            minimum_btc,
            recipient_btc_address,
            Some(op_return_id)
        ));
    });
}

#[test]
fn test_validate_transaction_succeeds_with_payment_and_refund_and_op_return() {
    run_test(|| {
        let raw_tx = hex::decode(sample_accepted_transaction()).unwrap();
        let minimum_btc: i64 = 2500200000;
        let recipient_btc_address =
            BtcAddress::P2SH(H160::from_str(&"66c7060feb882664ae62ffad0051fe843e318e85").unwrap());
        let op_return_id =
            hex::decode("aa21a9ede5c17d15b8b1fa2811b7e6da66ffa5e1aaa05922c69068bf90cd585b95bb4675".to_owned()).unwrap();

        let outputs = vec![
            sample_valid_payment_output(),
            sample_wrong_recipient_payment_output(),
            sample_valid_data_output(),
        ];

        BTCRelay::parse_transaction.mock_safe(move |_| MockResult::Return(Ok(sample_transaction_parsed(&outputs))));

        assert_ok!(BTCRelay::validate_transaction(
            Origin::signed(3),
            raw_tx,
            minimum_btc,
            recipient_btc_address,
            Some(op_return_id)
        ));
    });
}

#[test]
fn test_validate_transaction_invalid_no_outputs_fails() {
    run_test(|| {
        // Simulate input (we mock the parsed transaction)
        let raw_tx = hex::decode(sample_accepted_transaction()).unwrap();

        let minimum_btc: i64 = 2500200000;
        let recipient_btc_address =
            BtcAddress::P2SH(H160::from_str(&"66c7060feb882664ae62ffad0051fe843e318e85").unwrap());
        let op_return_id =
            hex::decode("aa21a9ede5c17d15b8b1fa2811b7e6da66ffa5e1aaa05922c69068bf90cd585b95bb4675".to_owned()).unwrap();
        // missing required data output
        let outputs = vec![sample_valid_payment_output()];

        BTCRelay::parse_transaction.mock_safe(move |_| MockResult::Return(Ok(sample_transaction_parsed(&outputs))));

        assert_err!(
            BTCRelay::validate_transaction(
                Origin::signed(3),
                raw_tx,
                minimum_btc,
                recipient_btc_address,
                Some(op_return_id)
            ),
            TestError::MalformedTransaction
        )
    });
}

#[test]
fn test_validate_transaction_insufficient_payment_value_fails() {
    run_test(|| {
        // Simulate input (we mock the parsed transaction)
        let raw_tx = vec![0u8; 342];

        let minimum_btc: i64 = 2500200000;
        let recipient_btc_address =
            BtcAddress::P2SH(H160::from_str(&"66c7060feb882664ae62ffad0051fe843e318e85").unwrap());
        let op_return_id =
            hex::decode("aa21a9ede5c17d15b8b1fa2811b7e6da66ffa5e1aaa05922c69068bf90cd585b95bb4675".to_owned()).unwrap();

        let outputs = vec![sample_insufficient_value_payment_output(), sample_valid_data_output()];

        BTCRelay::parse_transaction.mock_safe(move |_| MockResult::Return(Ok(sample_transaction_parsed(&outputs))));

        assert_err!(
            BTCRelay::validate_transaction(
                Origin::signed(3),
                raw_tx,
                minimum_btc,
                recipient_btc_address,
                Some(op_return_id)
            ),
            TestError::InsufficientValue
        )
    });
}

#[test]
fn test_validate_transaction_wrong_recipient_fails() {
    run_test(|| {
        // Simulate input (we mock the parsed transaction)
        let raw_tx = vec![0u8; 342];

        let minimum_btc: i64 = 2500200000;
        let recipient_btc_address =
            BtcAddress::P2SH(H160::from_str(&"66c7060feb882664ae62ffad0051fe843e318e85").unwrap());
        let op_return_id =
            hex::decode("aa21a9ede5c17d15b8b1fa2811b7e6da66ffa5e1aaa05922c69068bf90cd585b95bb4675".to_owned()).unwrap();

        let outputs = vec![
            sample_wrong_recipient_payment_output(),
            sample_wrong_recipient_payment_output(),
            sample_valid_data_output(),
        ];

        BTCRelay::parse_transaction.mock_safe(move |_| MockResult::Return(Ok(sample_transaction_parsed(&outputs))));

        assert_err!(
            BTCRelay::validate_transaction(
                Origin::signed(3),
                raw_tx,
                minimum_btc,
                recipient_btc_address,
                Some(op_return_id)
            ),
            TestError::WrongRecipient
        )
    });
}

#[test]
fn test_validate_transaction_incorrect_opreturn_fails() {
    run_test(|| {
        // Simulate input (we mock the parsed transaction)
        let raw_tx = vec![0u8; 342];

        let minimum_btc: i64 = 2500200000;
        let recipient_btc_address =
            BtcAddress::P2SH(H160::from_str(&"66c7060feb882664ae62ffad0051fe843e318e85").unwrap());
        let op_return_id =
            hex::decode("6a24aa21a9ede5c17d15b8b1fa2811b7e6da66ffa5e1aaa05922c69068bf90cd585b95bb4675".to_owned())
                .unwrap();

        let outputs = vec![sample_valid_payment_output(), sample_incorrect_data_output()];

        BTCRelay::parse_transaction.mock_safe(move |_| MockResult::Return(Ok(sample_transaction_parsed(&outputs))));

        assert_err!(
            BTCRelay::validate_transaction(
                Origin::signed(3),
                raw_tx,
                minimum_btc,
                recipient_btc_address,
                Some(op_return_id)
            ),
            TestError::InvalidOpReturn
        )
    });
}

#[test]
fn test_verify_and_validate_transaction_succeeds() {
    run_test(|| {
        BTCRelay::get_block_chain_from_id.mock_safe(|_| MockResult::Return(Ok(BlockChain::default())));

        let raw_tx = hex::decode(sample_example_real_rawtx()).unwrap();
        let transaction = parse_transaction(&raw_tx).unwrap();
        // txid are returned by Bitcoin-core
        let real_txid = H256Le::from_hex_be(&sample_example_real_txid());
        let real_tx_hash = H256Le::from_hex_be(&sample_example_real_transaction_hash());

        // see https://learnmeabitcoin.com/explorer/transaction/json.php?txid=c586389e5e4b3acb9d6c8be1c19ae8ab2795397633176f5a6442a261bbdefc3a
        assert_eq!(transaction.hash(), real_tx_hash);
        assert_eq!(transaction.tx_id(), real_txid);

        // rest are example values -- not checked in this test.
        // let block_height = 0;
        let raw_merkle_proof = vec![0u8; 100];
        let confirmations = None;
        let minimum_btc: i64 = 0;
        let recipient_btc_address =
            BtcAddress::P2SH(H160::from_str(&"66c7060feb882664ae62ffad0051fe843e318e85").unwrap());
        let op_return_id =
            hex::decode("aa21a9ede5c17d15b8b1fa2811b7e6da66ffa5e1aaa05922c69068bf90cd585b95bb4675".to_owned()).unwrap();
        BTCRelay::_validate_transaction
            .mock_safe(move |_, _, _, _| MockResult::Return(Ok((recipient_btc_address.clone(), 0))));
        BTCRelay::_verify_transaction_inclusion.mock_safe(move |_, _, _| MockResult::Return(Ok(())));

        assert_ok!(BTCRelay::verify_and_validate_transaction(
            Origin::signed(3),
            real_txid,
            raw_merkle_proof,
            confirmations,
            raw_tx,
            minimum_btc,
            recipient_btc_address,
            Some(op_return_id)
        ));
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

        let rich_header = RichBlockHeader::<AccountId> {
            block_hash: H256Le::zero(),
            block_header: BlockHeader::from_le_bytes(&block_header).unwrap(),
            block_height: block_height,
            chain_ref: chain_ref,
            account_id: Default::default(),
        };

        BTCRelay::set_block_header_from_hash(rich_header.block_hash, &rich_header);

        let blockchain = get_empty_block_chain_from_chain_id_and_height(chain_ref, start_height, block_height);
        BTCRelay::set_block_chain_from_id(chain_ref, &blockchain);

        let error_codes = vec![ErrorCode::NoDataBTCRelay, ErrorCode::InvalidBTCRelay];

        for error in error_codes.iter() {
            assert_ok!(BTCRelay::flag_block_error(rich_header.block_hash, error.clone()));
            let curr_chain = BTCRelay::get_block_chain_from_id(chain_ref).unwrap();

            if *error == ErrorCode::NoDataBTCRelay {
                assert!(curr_chain.no_data.contains(&block_height));
            } else if *error == ErrorCode::InvalidBTCRelay {
                assert!(curr_chain.invalid.contains(&block_height));
            };
            let error_event =
                TestEvent::btc_relay(Event::FlagBlockError(rich_header.block_hash, chain_ref, error.clone()));
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

        let rich_header = RichBlockHeader::<AccountId> {
            block_hash: H256Le::zero(),
            block_header: BlockHeader::from_le_bytes(&block_header).unwrap(),
            block_height: block_height,
            chain_ref: chain_ref,
            account_id: Default::default(),
        };

        BTCRelay::set_block_header_from_hash(rich_header.block_hash, &rich_header);

        let blockchain = get_empty_block_chain_from_chain_id_and_height(chain_ref, start_height, block_height);
        BTCRelay::set_block_chain_from_id(chain_ref, &blockchain);

        // not a valid error code
        let error = ErrorCode::Liquidation;

        assert_err!(
            BTCRelay::flag_block_error(rich_header.block_hash, error),
            TestError::UnknownErrorcode
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

        let rich_header = RichBlockHeader::<AccountId> {
            block_hash: H256Le::zero(),
            block_header: BlockHeader::from_le_bytes(&block_header).unwrap(),
            block_height: block_height,
            chain_ref: chain_ref,
            account_id: Default::default(),
        };

        BTCRelay::set_block_header_from_hash(rich_header.block_hash, &rich_header);

        let mut blockchain = get_empty_block_chain_from_chain_id_and_height(chain_ref, start_height, block_height);
        blockchain.no_data.insert(block_height);
        blockchain.invalid.insert(block_height);
        set_parachain_nodata_error();
        ext::security::insert_error::<Test>(ErrorCode::InvalidBTCRelay);

        BTCRelay::set_block_chain_from_id(chain_ref, &blockchain);

        let clear_error = move |error: ErrorCode| {
            assert_ok!(BTCRelay::clear_block_error(rich_header.block_hash, error.clone()));
            let curr_chain = BTCRelay::get_block_chain_from_id(chain_ref).unwrap();

            if error == ErrorCode::NoDataBTCRelay {
                assert!(!curr_chain.no_data.contains(&block_height));
            } else if error == ErrorCode::InvalidBTCRelay {
                assert!(!curr_chain.invalid.contains(&block_height));
            };
            let error_event =
                TestEvent::btc_relay(Event::ClearBlockError(rich_header.block_hash, chain_ref, error.clone()));
            assert!(System::events().iter().any(|a| a.event == error_event));
        };

        clear_error(ErrorCode::NoDataBTCRelay);
        // ensure not recovered while there are still invalid blocks
        assert_err!(
            ext::security::ensure_parachain_status_running::<Test>(),
            SecurityError::ParachainNotRunning
        );
        assert!(ext::security::is_parachain_error_invalid_btcrelay::<Test>());
        clear_error(ErrorCode::InvalidBTCRelay);

        assert_ok!(ext::security::ensure_parachain_status_running::<Test>());
        assert!(!ext::security::is_parachain_error_invalid_btcrelay::<Test>());
        assert!(!ext::security::is_parachain_error_no_data_btcrelay::<Test>());
    })
}

#[test]
fn test_clear_block_error_fails() {
    run_test(|| {
        let chain_ref: u32 = 1;
        let start_height: u32 = 20;
        let block_height: u32 = 100;
        let block_header = hex::decode(sample_block_header_hex()).unwrap();

        let rich_header = RichBlockHeader::<AccountId> {
            block_hash: H256Le::zero(),
            block_header: BlockHeader::from_le_bytes(&block_header).unwrap(),
            block_height: block_height,
            chain_ref: chain_ref,
            account_id: Default::default(),
        };

        BTCRelay::set_block_header_from_hash(rich_header.block_hash, &rich_header);

        let blockchain = get_empty_block_chain_from_chain_id_and_height(chain_ref, start_height, block_height);
        BTCRelay::set_block_chain_from_id(chain_ref, &blockchain);

        // not a valid error code
        let error = ErrorCode::Liquidation;

        assert_err!(
            BTCRelay::clear_block_error(rich_header.block_hash, error),
            TestError::UnknownErrorcode
        );
    })
}

#[test]
fn test_transaction_verification_allowed_succeeds() {
    run_test(|| {
        let main_start: u32 = 0;
        let main_height: u32 = 10;
        BTCRelay::get_block_chain_from_id.mock_safe(move |_| {
            MockResult::Return(Ok(get_empty_block_chain_from_chain_id_and_height(
                1,
                main_start,
                main_height,
            )))
        });
        assert_ok!(BTCRelay::transaction_verification_allowed(main_start + 1));
    })
}

#[test]
fn test_transaction_verification_allowed_invalid_fails() {
    run_test(|| {
        let main_start: u32 = 0;
        let main_height: u32 = 10;
        BTCRelay::get_block_chain_from_id.mock_safe(move |_| {
            MockResult::Return(Ok(get_invalid_empty_block_chain_from_chain_id_and_height(
                1,
                main_start,
                main_height,
            )))
        });
        assert_err!(
            BTCRelay::transaction_verification_allowed(main_start + 1),
            TestError::Invalid
        );
    })
}

#[test]
fn test_transaction_verification_allowed_no_data_fails() {
    run_test(|| {
        let main_start: u32 = 0;
        let main_height: u32 = 10;
        BTCRelay::get_block_chain_from_id.mock_safe(move |_| {
            MockResult::Return(Ok(get_nodata_empty_block_chain_from_chain_id_and_height(
                1,
                main_start,
                main_height,
            )))
        });
        // NO_DATA height is main_height - 1
        assert_err!(
            BTCRelay::transaction_verification_allowed(main_height),
            TestError::NoData
        );
    })
}

#[test]
fn test_transaction_verification_allowed_no_data_succeeds() {
    run_test(|| {
        let main_start: u32 = 0;
        let main_height: u32 = 10;
        BTCRelay::get_block_chain_from_id.mock_safe(move |_| {
            MockResult::Return(Ok(get_nodata_empty_block_chain_from_chain_id_and_height(
                1,
                main_start,
                main_height,
            )))
        });
        // NO_DATA height is main_height - 1
        assert_ok!(BTCRelay::transaction_verification_allowed(main_start + 1));
    })
}

#[test]
fn test_verify_transaction_inclusion_succeeds() {
    run_test(|| {
        let chain_ref = 0;
        let fork_ref = 1;
        let start = 10;
        let main_chain_height = 300;
        let fork_chain_height = 280;
        // Random init since we mock this
        let raw_merkle_proof = vec![0u8; 100];
        let confirmations = None;
        let rich_block_header = sample_rich_tx_block_header(chain_ref, main_chain_height);

        let proof = sample_merkle_proof();
        let proof_result = sample_valid_proof_result();

        let main = get_empty_block_chain_from_chain_id_and_height(chain_ref, start, main_chain_height);

        let fork = get_empty_block_chain_from_chain_id_and_height(fork_ref, start, fork_chain_height);

        BTCRelay::get_chain_id_from_position.mock_safe(move |_| MockResult::Return(Ok(fork_ref.clone())));
        BTCRelay::get_block_chain_from_id.mock_safe(move |id| {
            if id == chain_ref.clone() {
                return MockResult::Return(Ok(main.clone()));
            } else {
                return MockResult::Return(Ok(fork.clone()));
            }
        });

        BTCRelay::get_best_block_height.mock_safe(move || MockResult::Return(main_chain_height));

        BTCRelay::parse_merkle_proof.mock_safe(move |_| MockResult::Return(Ok(proof.clone())));
        BTCRelay::verify_merkle_proof.mock_safe(move |_| MockResult::Return(Ok(proof_result)));

        BTCRelay::get_block_header_from_hash.mock_safe(move |_| MockResult::Return(Ok(rich_block_header)));

        BTCRelay::check_bitcoin_confirmations.mock_safe(|_, _, _| MockResult::Return(Ok(())));

        BTCRelay::check_parachain_confirmations.mock_safe(|_| MockResult::Return(Ok(())));

        assert_ok!(BTCRelay::verify_transaction_inclusion(
            Origin::signed(3),
            proof_result.transaction_hash,
            raw_merkle_proof,
            confirmations
        ));
    });
}

#[test]
fn test_verify_transaction_inclusion_empty_fork_succeeds() {
    run_test(|| {
        let chain_ref = 0;
        let start = 10;
        let main_chain_height = 300;
        // Random init since we mock this
        let raw_merkle_proof = vec![0u8; 100];
        let confirmations = None;
        let rich_block_header = sample_rich_tx_block_header(chain_ref, main_chain_height);

        let proof = sample_merkle_proof();
        let proof_result = sample_valid_proof_result();

        let main = get_empty_block_chain_from_chain_id_and_height(chain_ref, start, main_chain_height);

        BTCRelay::get_block_chain_from_id.mock_safe(move |id| {
            if id == chain_ref.clone() {
                return MockResult::Return(Ok(main.clone()));
            } else {
                return MockResult::Return(Err(TestError::InvalidChainID.into()));
            }
        });

        BTCRelay::get_best_block_height.mock_safe(move || MockResult::Return(main_chain_height));

        BTCRelay::parse_merkle_proof.mock_safe(move |_| MockResult::Return(Ok(proof.clone())));
        BTCRelay::verify_merkle_proof.mock_safe(move |_| MockResult::Return(Ok(proof_result)));

        BTCRelay::get_block_header_from_hash.mock_safe(move |_| MockResult::Return(Ok(rich_block_header)));

        BTCRelay::check_bitcoin_confirmations.mock_safe(|_, _, _| MockResult::Return(Ok(())));

        BTCRelay::check_parachain_confirmations.mock_safe(|_| MockResult::Return(Ok(())));

        assert_ok!(BTCRelay::verify_transaction_inclusion(
            Origin::signed(3),
            proof_result.transaction_hash,
            raw_merkle_proof,
            confirmations,
        ));
    });
}

#[test]
fn test_verify_transaction_inclusion_invalid_tx_id_fails() {
    run_test(|| {
        let chain_ref = 0;
        let fork_ref = 1;
        let start = 10;
        let main_chain_height = 300;
        let fork_chain_height = 280;
        // Random init since we mock this
        let raw_merkle_proof = vec![0u8; 100];
        let confirmations = None;
        let rich_block_header = sample_rich_tx_block_header(chain_ref, main_chain_height);

        // Mismatching TXID
        let invalid_tx_id = H256Le::from_bytes_le(
            &hex::decode("0000000000000000000000000000000000000000000000000000000000000000".to_owned()).unwrap(),
        );

        let proof = sample_merkle_proof();
        let proof_result = sample_valid_proof_result();

        let main = get_empty_block_chain_from_chain_id_and_height(chain_ref, start, main_chain_height);

        let fork = get_empty_block_chain_from_chain_id_and_height(fork_ref, start, fork_chain_height);

        BTCRelay::get_chain_id_from_position.mock_safe(move |_| MockResult::Return(Ok(fork_ref.clone())));
        BTCRelay::get_block_chain_from_id.mock_safe(move |id| {
            if id == chain_ref.clone() {
                return MockResult::Return(Ok(main.clone()));
            } else {
                return MockResult::Return(Ok(fork.clone()));
            }
        });

        BTCRelay::get_best_block_height.mock_safe(move || MockResult::Return(main_chain_height));

        BTCRelay::parse_merkle_proof.mock_safe(move |_| MockResult::Return(Ok(proof.clone())));
        BTCRelay::verify_merkle_proof.mock_safe(move |_| MockResult::Return(Ok(proof_result)));

        BTCRelay::get_block_header_from_hash.mock_safe(move |_| MockResult::Return(Ok(rich_block_header)));

        BTCRelay::check_bitcoin_confirmations.mock_safe(|_, _, _| MockResult::Return(Ok(())));

        BTCRelay::check_parachain_confirmations.mock_safe(|_| MockResult::Return(Ok(())));

        assert_err!(
            BTCRelay::verify_transaction_inclusion(Origin::signed(3), invalid_tx_id, raw_merkle_proof, confirmations,),
            TestError::InvalidTxid
        );
    });
}

#[test]
fn test_verify_transaction_inclusion_invalid_merkle_root_fails() {
    run_test(|| {
        let chain_ref = 0;
        let fork_ref = 1;
        let start = 10;
        let main_chain_height = 300;
        let fork_chain_height = 280;
        // Random init since we mock this
        let raw_merkle_proof = vec![0u8; 100];
        let confirmations = None;
        let mut rich_block_header = sample_rich_tx_block_header(chain_ref, main_chain_height);

        // Mismatching merkle root
        let invalid_merkle_root = H256Le::from_bytes_le(
            &hex::decode("0000000000000000000000000000000000000000000000000000000000000000".to_owned()).unwrap(),
        );
        rich_block_header.block_header.merkle_root = invalid_merkle_root;

        let proof = sample_merkle_proof();
        let proof_result = sample_valid_proof_result();

        let main = get_empty_block_chain_from_chain_id_and_height(chain_ref, start, main_chain_height);

        let fork = get_empty_block_chain_from_chain_id_and_height(fork_ref, start, fork_chain_height);

        BTCRelay::get_chain_id_from_position.mock_safe(move |_| MockResult::Return(Ok(fork_ref.clone())));
        BTCRelay::get_block_chain_from_id.mock_safe(move |id| {
            if id == chain_ref.clone() {
                return MockResult::Return(Ok(main.clone()));
            } else {
                return MockResult::Return(Ok(fork.clone()));
            }
        });

        BTCRelay::get_best_block_height.mock_safe(move || MockResult::Return(main_chain_height));

        BTCRelay::parse_merkle_proof.mock_safe(move |_| MockResult::Return(Ok(proof.clone())));

        BTCRelay::get_block_header_from_hash.mock_safe(move |_| MockResult::Return(Ok(rich_block_header)));

        BTCRelay::check_bitcoin_confirmations.mock_safe(|_, _, _| MockResult::Return(Ok(())));

        BTCRelay::check_parachain_confirmations.mock_safe(|_| MockResult::Return(Ok(())));

        assert_err!(
            BTCRelay::verify_transaction_inclusion(
                Origin::signed(3),
                proof_result.transaction_hash,
                raw_merkle_proof,
                confirmations,
            ),
            TestError::InvalidMerkleProof
        );
    });
}

#[test]
fn test_verify_transaction_inclusion_fails_with_ongoing_fork() {
    run_test(|| {
        BTCRelay::get_chain_id_from_position.mock_safe(|_| MockResult::Return(Ok(1)));
        BTCRelay::get_block_chain_from_id.mock_safe(|_| MockResult::Return(Ok(BlockChain::default())));

        let tx_id = sample_valid_proof_result().transaction_hash;
        let raw_merkle_proof = vec![0u8; 100];
        let confirmations = None;

        assert_err!(
            BTCRelay::verify_transaction_inclusion(Origin::signed(3), tx_id, raw_merkle_proof, confirmations,),
            TestError::OngoingFork
        );
    });
}

#[test]
fn test_check_bitcoin_confirmations_insecure_succeeds() {
    run_test(|| {
        let main_chain_height = 100;
        let tx_block_height = 90;

        let req_confs = Some(5);
        assert_ok!(BTCRelay::check_bitcoin_confirmations(
            main_chain_height,
            req_confs,
            tx_block_height,
        ));
    });
}

#[test]
fn test_check_bitcoin_confirmations_insecure_insufficient_confs_fails() {
    run_test(|| {
        let main_chain_height = 100;
        let tx_block_height = 99;

        let req_confs = Some(5);

        assert_err!(
            BTCRelay::check_bitcoin_confirmations(main_chain_height, req_confs, tx_block_height,),
            TestError::BitcoinConfirmations
        )
    });
}

#[test]
fn test_check_bitcoin_confirmations_secure_stable_confs_succeeds() {
    run_test(|| {
        let main_chain_height = 100;
        let tx_block_height = 90;

        let req_confs = None;
        // relevant check: ok
        let stable_confs = 10;

        BTCRelay::get_stable_transaction_confirmations.mock_safe(move || MockResult::Return(stable_confs));
        assert_ok!(BTCRelay::check_bitcoin_confirmations(
            main_chain_height,
            req_confs,
            tx_block_height,
        ));
    });
}

#[test]
fn test_check_bitcoin_confirmations_secure_user_confs_succeeds() {
    run_test(|| {
        let main_chain_height = 100;
        let tx_block_height = 91;
        // relevant check: ok
        let req_confs = None;
        let stable_confs = 10;

        BTCRelay::get_stable_transaction_confirmations.mock_safe(move || MockResult::Return(stable_confs));
        assert_ok!(BTCRelay::check_bitcoin_confirmations(
            main_chain_height,
            req_confs,
            tx_block_height,
        ));
    });
}

#[test]
fn test_check_bitcoin_confirmations_secure_insufficient_stable_confs_succeeds() {
    run_test(|| {
        let main_chain_height = 100;
        let tx_block_height = 92;

        let req_confs = None;
        // relevant check: fails
        let stable_confs = 10;

        BTCRelay::get_stable_transaction_confirmations.mock_safe(move || MockResult::Return(stable_confs));

        assert_err!(
            BTCRelay::check_bitcoin_confirmations(main_chain_height, req_confs, tx_block_height,),
            TestError::BitcoinConfirmations
        )
    });
}

#[test]
fn test_check_parachain_confirmations_succeeds() {
    run_test(|| {
        let chain_ref = 0;
        let block_height = 245;
        let block_hash = sample_parsed_first_block(chain_ref, block_height).block_hash;
        BTCRelay::set_parachain_height_from_hash(block_hash);
        System::set_block_number(5 + PARACHAIN_CONFIRMATIONS);

        assert_ok!(BTCRelay::check_parachain_confirmations(block_hash));
    });
}

#[test]
fn test_check_parachain_confirmations_insufficient_confs_fails() {
    run_test(|| {
        let chain_ref = 0;
        let block_height = 245;
        let block_hash = sample_parsed_first_block(chain_ref, block_height).block_hash;
        BTCRelay::set_parachain_height_from_hash(block_hash);

        assert_err!(
            BTCRelay::check_parachain_confirmations(block_hash),
            TestError::ParachainConfirmations
        );
    });
}

#[test]
fn get_chain_from_id_err() {
    run_test(|| {
        assert_err!(BTCRelay::get_block_chain_from_id(0), TestError::InvalidChainID);
    });
}

#[test]
fn get_chain_from_id_ok() {
    run_test(|| {
        // insert the main chain in Chains and ChainsIndex
        let main_chain_ref: u32 = 0;
        let main_start_height: u32 = 3;
        let main_block_height: u32 = 110;
        let main_position: u32 = 0;
        let main = get_empty_block_chain_from_chain_id_and_height(main_chain_ref, main_start_height, main_block_height);
        BTCRelay::set_chain_from_position_and_id(main_position, main_chain_ref);
        BTCRelay::set_block_chain_from_id(main_chain_ref, &main);

        assert_eq!(Ok(main), BTCRelay::get_block_chain_from_id(main_chain_ref));
    });
}

#[test]
fn store_generated_block_headers() {
    let target = U256::from(2).pow(254.into());
    let miner = BtcAddress::P2PKH(H160::from_str(&"66c7060feb882664ae62ffad0051fe843e318e85").unwrap());
    let get_header = |block: &Block| RawBlockHeader::from_bytes(&block.header.try_format().unwrap()).unwrap();

    run_test(|| {
        let mut last_block = BlockBuilder::new().with_coinbase(&miner, 50, 0).mine(target).unwrap();
        assert_ok!(BTCRelay::initialize(Origin::signed(3), get_header(&last_block), 0));
        for i in 1..20 {
            last_block = BlockBuilder::new()
                .with_coinbase(&miner, 50, i)
                .with_previous_hash(last_block.header.hash().unwrap())
                .mine(target)
                .unwrap();
            assert_ok!(BTCRelay::store_block_header(Origin::signed(3), get_header(&last_block)));
        }
        let main_chain: BlockChain = BTCRelay::get_block_chain_from_id(crate::MAIN_CHAIN_ID).unwrap();
        assert_eq!(main_chain.start_height, 0);
        assert_eq!(main_chain.max_height, 19);
    })
}

#[test]
fn test_extract_value_fails_with_wrong_recipient() {
    run_test(|| {
        let recipient_btc_address_0 = BtcAddress::P2SH(H160([0; 20]));
        let recipient_btc_address_1 = BtcAddress::P2SH(H160([1; 20]));

        let transaction = TransactionBuilder::new()
            .with_version(2)
            .add_output(TransactionOutput::payment(32, &recipient_btc_address_0))
            .build();

        assert_err!(
            BTCRelay::extract_payment_value(transaction, recipient_btc_address_1),
            TestError::WrongRecipient
        );
    })
}

#[test]
fn test_extract_value_succeeds() {
    run_test(|| {
        let recipient_btc_address = BtcAddress::P2SH(H160([0; 20]));
        let recipient_value = 64;

        let transaction = TransactionBuilder::new()
            .with_version(2)
            .add_output(TransactionOutput::payment(recipient_value, &recipient_btc_address))
            .build();

        assert_eq!(
            BTCRelay::extract_payment_value(transaction, recipient_btc_address).unwrap(),
            recipient_value
        );
    })
}

#[test]
fn test_extract_value_and_op_return_fails_with_not_enough_outputs() {
    run_test(|| {
        let recipient_btc_address = BtcAddress::P2SH(H160::zero());

        let transaction = TransactionBuilder::new()
            .with_version(2)
            .add_output(TransactionOutput::payment(100, &recipient_btc_address))
            .build();

        assert_err!(
            BTCRelay::extract_payment_value_and_op_return(transaction, recipient_btc_address),
            TestError::MalformedTransaction
        );
    })
}

#[test]
fn test_extract_value_and_op_return_fails_with_no_op_return() {
    run_test(|| {
        let recipient_btc_address_0 = BtcAddress::P2SH(H160([0; 20]));
        let recipient_btc_address_1 = BtcAddress::P2SH(H160([1; 20]));

        let transaction = TransactionBuilder::new()
            .with_version(2)
            .add_output(TransactionOutput::payment(100, &recipient_btc_address_0))
            .add_output(TransactionOutput::payment(100, &recipient_btc_address_1))
            .build();

        assert_err!(
            BTCRelay::extract_payment_value_and_op_return(transaction, recipient_btc_address_0),
            TestError::NotOpReturn
        );
    })
}

#[test]
fn test_extract_value_and_op_return_fails_with_no_recipient() {
    run_test(|| {
        let recipient_btc_address_0 = BtcAddress::P2SH(H160([0; 20]));
        let recipient_btc_address_1 = BtcAddress::P2SH(H160([1; 20]));
        let recipient_btc_address_2 = BtcAddress::P2SH(H160([2; 20]));

        let transaction = TransactionBuilder::new()
            .with_version(2)
            .add_output(TransactionOutput::payment(100, &recipient_btc_address_1))
            .add_output(TransactionOutput::payment(100, &recipient_btc_address_2))
            .build();

        assert_err!(
            BTCRelay::extract_payment_value_and_op_return(transaction, recipient_btc_address_0),
            TestError::WrongRecipient
        );
    })
}

#[test]
fn test_extract_value_and_op_return_succeeds() {
    run_test(|| {
        let recipient_btc_address = BtcAddress::P2SH(H160::zero());
        let recipient_value = 1234;
        let op_return = vec![1; 32];

        let transaction = TransactionBuilder::new()
            .with_version(2)
            .add_output(TransactionOutput::payment(recipient_value, &recipient_btc_address))
            .add_output(TransactionOutput::op_return(0, &op_return))
            .build();

        let (extr_value, extr_data) =
            BTCRelay::extract_payment_value_and_op_return(transaction, recipient_btc_address).unwrap();

        assert_eq!(extr_value, recipient_value);
        assert_eq!(extr_data, op_return);
    })
}

#[test]
fn test_check_and_do_reorg() {
    use crate::{sp_api_hidden_includes_decl_storage::hidden_include::StorageMap, Chains, ChainsIndex};
    use bitcoin::types::BlockChain;
    use sp_std::collections::btree_set::BTreeSet;

    // data taken from testnet fork
    run_test(|| {
        Chains::insert(0, 0);
        Chains::insert(2, 7);

        ChainsIndex::insert(
            0,
            BlockChain {
                chain_id: 0,
                start_height: 1_892_642,
                max_height: 1_897_317,
                no_data: BTreeSet::new(),
                invalid: BTreeSet::new(),
            },
        );

        ChainsIndex::insert(
            2,
            BlockChain {
                chain_id: 2,
                start_height: 1_893_831,
                max_height: 1_893_831,
                no_data: BTreeSet::new(),
                invalid: BTreeSet::new(),
            },
        );

        ChainsIndex::insert(
            4,
            BlockChain {
                chain_id: 4,
                start_height: 1_895_256,
                max_height: 1_895_256,
                no_data: BTreeSet::new(),
                invalid: BTreeSet::new(),
            },
        );

        ChainsIndex::insert(
            6,
            BlockChain {
                chain_id: 6,
                start_height: 1_896_846,
                max_height: 1_896_846,
                no_data: BTreeSet::new(),
                invalid: BTreeSet::new(),
            },
        );

        ChainsIndex::insert(
            7,
            BlockChain {
                chain_id: 7,
                start_height: 1_897_317,
                max_height: 1_897_910,
                no_data: BTreeSet::new(),
                invalid: BTreeSet::new(),
            },
        );

        BTCRelay::swap_main_blockchain.mock_safe(|_| MockResult::Return(Ok(())));

        // we should skip empty `Chains`, this can occur if the
        // previous index is accidentally deleted
        assert_ok!(BTCRelay::check_and_do_reorg(&BlockChain {
            chain_id: 7,
            start_height: 1_897_317,
            max_height: 1_897_910,
            no_data: BTreeSet::new(),
            invalid: BTreeSet::new(),
        }));
    })
}

#[test]
fn test_remove_blockchain_from_chain() {
    use crate::{
        sp_api_hidden_includes_decl_storage::hidden_include::{IterableStorageMap, StorageMap},
        Chains,
    };

    run_test(|| {
        Chains::insert(0, 0);
        Chains::insert(8, 5);
        Chains::insert(2, 7);

        assert_ok!(BTCRelay::remove_blockchain_from_chain(2));

        let mut chains = <Chains>::iter().collect::<Vec<(u32, u32)>>();
        chains.sort_by_key(|k| k.0);
        assert_eq!(chains, vec![(0, 0), (2, 5)]);
    })
}

#[test]
fn test_ensure_relayer_authorized() {
    use crate::{sp_api_hidden_includes_decl_storage::hidden_include::StorageValue, DisableRelayerAuth};

    run_test(|| {
        DisableRelayerAuth::set(true);
        assert_ok!(BTCRelay::ensure_relayer_authorized(0));

        DisableRelayerAuth::set(false);
        assert_err!(BTCRelay::ensure_relayer_authorized(0), TestError::RelayerNotAuthorized);

        BTCRelay::register_authorized_relayer(0);
        assert_ok!(BTCRelay::ensure_relayer_authorized(0));

        BTCRelay::deregister_authorized_relayer(0);
        assert_err!(BTCRelay::ensure_relayer_authorized(0), TestError::RelayerNotAuthorized);
    })
}

#[test]
fn test_store_block_header_and_update_sla_succeeds() {
    run_test(|| {
        BTCRelay::_store_block_header.mock_safe(|_, _| MockResult::Return(Ok(())));

        ext::sla::event_update_relayer_sla::<Test>.mock_safe(|&relayer_id, event| {
            assert_eq!(relayer_id, 0);
            assert_eq!(event, ext::sla::RelayerEvent::BlockSubmission);
            MockResult::Return(Ok(()))
        });

        assert_ok!(BTCRelay::_store_block_header_and_update_sla(
            0,
            RawBlockHeader::default()
        ));
    })
}

#[test]
fn test_store_block_header_and_update_sla_succeeds_with_duplicate() {
    run_test(|| {
        BTCRelay::_store_block_header.mock_safe(|_, _| MockResult::Return(Err(TestError::DuplicateBlock.into())));

        BTCRelay::get_best_block.mock_safe(|| MockResult::Return(RawBlockHeader::default().hash()));

        ext::sla::event_update_relayer_sla::<Test>.mock_safe(|&relayer_id, event| {
            assert_eq!(relayer_id, 0);
            assert_eq!(event, ext::sla::RelayerEvent::DuplicateBlockSubmission);
            MockResult::Return(Ok(()))
        });

        assert_ok!(BTCRelay::_store_block_header_and_update_sla(
            0,
            RawBlockHeader::default()
        ));
    })
}

#[test]
fn test_store_block_header_and_update_sla_fails_with_invalid() {
    run_test(|| {
        BTCRelay::_store_block_header.mock_safe(|_, _| MockResult::Return(Err(TestError::DiffTargetHeader.into())));

        ext::sla::event_update_relayer_sla::<Test>.mock_safe(|_, _| {
            panic!("Should not call sla update for invalid block");
        });

        assert_err!(
            BTCRelay::_store_block_header_and_update_sla(0, RawBlockHeader::default()),
            TestError::DiffTargetHeader
        );
    })
}

/// # Util functions

const SAMPLE_TX_ID: &'static str = "c8589f304d3b9df1d4d8b3d15eb6edaaa2af9d796e9d9ace12b31f293705c5e9";

const SAMPLE_MERKLE_ROOT: &'static str = "1EE1FB90996CA1D5DCD12866BA9066458BF768641215933D7D8B3A10EF79D090";

fn sample_merkle_proof() -> MerkleProof {
    MerkleProof {
        block_header: sample_block_header(),
        transactions_count: 1,
        hashes: vec![H256Le::from_hex_le(SAMPLE_TX_ID)],
        flag_bits: vec![true],
    }
}

fn sample_block_header() -> BlockHeader {
    BlockHeader {
        merkle_root: H256Le::from_hex_le(SAMPLE_MERKLE_ROOT),
        target: 123.into(),
        timestamp: 1601494682,
        version: 2,
        hash_prev_block: H256Le::from_hex_be("0000000000000000000e84948eaacb9b03382782f16f2d8a354de69f2e5a2a68"),
        nonce: 0,
    }
}

fn sample_valid_proof_result() -> ProofResult {
    let tx_id = H256Le::from_hex_le(SAMPLE_TX_ID);
    let merkle_root = H256Le::from_hex_le(SAMPLE_MERKLE_ROOT);

    ProofResult {
        extracted_root: merkle_root,
        transaction_hash: tx_id,
        transaction_position: 0,
    }
}

fn get_empty_block_chain_from_chain_id_and_height(chain_id: u32, start_height: u32, block_height: u32) -> BlockChain {
    let blockchain = BlockChain {
        chain_id: chain_id,
        start_height: start_height,
        max_height: block_height,
        no_data: BTreeSet::new(),
        invalid: BTreeSet::new(),
    };

    blockchain
}

fn get_invalid_empty_block_chain_from_chain_id_and_height(
    chain_id: u32,
    start_height: u32,
    block_height: u32,
) -> BlockChain {
    let mut blockchain = get_empty_block_chain_from_chain_id_and_height(chain_id, start_height, block_height);
    blockchain.invalid.insert(block_height - 1);

    blockchain
}

fn get_nodata_empty_block_chain_from_chain_id_and_height(
    chain_id: u32,
    start_height: u32,
    block_height: u32,
) -> BlockChain {
    let mut blockchain = get_empty_block_chain_from_chain_id_and_height(chain_id, start_height, block_height);
    blockchain.no_data.insert(block_height - 1);

    blockchain
}

fn store_blockchain_and_random_headers(id: u32, start_height: u32, max_height: u32, position: u32) -> BlockChain {
    let mut chain = get_empty_block_chain_from_chain_id_and_height(id, start_height, max_height);

    // create and insert main chain headers
    for height in chain.start_height..chain.max_height + 1 {
        let block_header = hex::decode(sample_block_header_hex()).unwrap();
        let mut fake_block = height.to_be_bytes().repeat(7);
        fake_block.append(&mut id.to_be_bytes().to_vec());
        let block_hash = H256Le::from_bytes_be(fake_block.as_slice());

        let rich_header = RichBlockHeader::<AccountId> {
            block_hash: block_hash,
            block_header: BlockHeader::from_le_bytes(&block_header).unwrap(),
            block_height: height,
            chain_ref: id,
            account_id: Default::default(),
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

fn sample_parsed_genesis_header(chain_ref: u32, block_height: u32) -> RichBlockHeader<AccountId> {
    let genesis_header = RawBlockHeader::from_hex(sample_raw_genesis_header()).unwrap();
    RichBlockHeader::<AccountId> {
        block_hash: genesis_header.hash(),
        block_header: parse_block_header(&genesis_header).unwrap(),
        block_height: block_height,
        chain_ref: chain_ref,
        account_id: Default::default(),
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

fn sample_parsed_first_block(chain_ref: u32, block_height: u32) -> RichBlockHeader<AccountId> {
    let block_header = RawBlockHeader::from_hex(sample_raw_first_header()).unwrap();
    RichBlockHeader::<AccountId> {
        block_hash: block_header.hash(),
        block_header: parse_block_header(&block_header).unwrap(),
        block_height: block_height,
        chain_ref: chain_ref,
        account_id: Default::default(),
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

fn sample_rich_tx_block_header(chain_ref: u32, block_height: u32) -> RichBlockHeader<AccountId> {
    let raw_header = RawBlockHeader::from_hex("0000003096cb3d93696c4f56c10da153963d35abf4692c07b2b3bf0702fb4cb32a8682211ee1fb90996ca1d5dcd12866ba9066458bf768641215933d7d8b3a10ef79d090e8a13a5effff7f2005000000".to_owned()).unwrap();
    RichBlockHeader::<AccountId> {
        block_hash: raw_header.hash(),
        block_header: parse_block_header(&raw_header).unwrap(),
        block_height: block_height,
        chain_ref: chain_ref,
        account_id: Default::default(),
    }
}

fn sample_valid_payment_output() -> TransactionOutput {
    TransactionOutput {
        value: 2500200000,
        script: "a91466c7060feb882664ae62ffad0051fe843e318e8587".try_into().unwrap(),
    }
}

fn sample_insufficient_value_payment_output() -> TransactionOutput {
    TransactionOutput {
        value: 100,
        script: "a91466c7060feb882664ae62ffad0051fe843e318e8587".try_into().unwrap(),
    }
}

fn sample_wrong_recipient_payment_output() -> TransactionOutput {
    TransactionOutput {
        value: 2500200000,
        script: "a914000000000000000000000000000000000000000087".try_into().unwrap(),
    }
}

fn sample_valid_data_output() -> TransactionOutput {
    TransactionOutput {
        value: 0,
        script: "6a24aa21a9ede5c17d15b8b1fa2811b7e6da66ffa5e1aaa05922c69068bf90cd585b95bb4675"
            .try_into()
            .unwrap(),
    }
}

fn sample_incorrect_data_output() -> TransactionOutput {
    TransactionOutput {
        value: 0,
        script: "6a24000000000000000000000000000000000000000000000000000000000000000000000000"
            .try_into()
            .unwrap(),
    }
}

fn sample_transaction_parsed(outputs: &Vec<TransactionOutput>) -> Transaction {
    let mut inputs: Vec<TransactionInput> = Vec::new();

    let spent_output_txid =
        hex::decode("b28f1e58af1d4db02d1b9f0cf8d51ece3dd5f5013fd108647821ea255ae5daff".to_owned()).unwrap();
    let input = TransactionInput {
        previous_hash: H256Le::from_bytes_le(&spent_output_txid),
        previous_index: 0,
        coinbase: false,
        height: None,
        script: hex::decode("16001443feac9ca9d20883126e30e962ca11fda07f808b".to_owned()).unwrap(),
        sequence: 4294967295,
        flags: 0,
        witness: vec![],
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

fn sample_example_real_rawtx() -> String {
    "0200000000010140d43a99926d43eb0e619bf0b3d83b4a31f60c176beecfb9d35bf45e54d0f7420100000017160014a4b4ca48de0b3fffc15404a1acdc8dbaae226955ffffffff0100e1f5050000000017a9144a1154d50b03292b3024370901711946cb7cccc387024830450221008604ef8f6d8afa892dee0f31259b6ce02dd70c545cfcfed8148179971876c54a022076d771d6e91bed212783c9b06e0de600fab2d518fad6f15a2b191d7fbd262a3e0121039d25ab79f41f75ceaf882411fd41fa670a4c672c23ffaf0e361a969cde0692e800000000".to_owned()
}

fn sample_example_real_txid() -> String {
    "c586389e5e4b3acb9d6c8be1c19ae8ab2795397633176f5a6442a261bbdefc3a".to_owned()
}

fn sample_example_real_transaction_hash() -> String {
    "b759d39a8596b70b3a46700b83e1edb247e17ba58df305421864fe7a9ac142ea".to_owned()
}

fn set_parachain_nodata_error() {
    ext::security::insert_error::<Test>(ErrorCode::NoDataBTCRelay);
    ext::security::set_status::<Test>(StatusCode::Error);
    assert!(ext::security::is_parachain_error_no_data_btcrelay::<Test>());
}
