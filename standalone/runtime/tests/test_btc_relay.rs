#![feature(exclusive_range_pattern)]
mod bitcoin_data;
mod mock;

use bitcoin_data::{get_bitcoin_testdata, get_fork_testdata};
use mock::{assert_eq, *};

type BTCRelayError = btc_relay::Error<Runtime>;

#[test]
#[cfg_attr(feature = "skip-slow-tests", ignore)]
fn integration_test_submit_block_headers_and_verify_transaction_inclusion() {
    ExtBuilder::build().execute_without_relay_init(|| {
        // ensure that difficulty check is enabled
        BTCRelayPallet::set_disable_difficulty_check(false);
        assert!(!BTCRelayPallet::disable_difficulty_check());

        // reduce number of blocks to reduce testing time, but higher than 2016 blocks for difficulty adjustment
        const BLOCKS_TO_TEST: usize = 5_000;

        // load blocks with transactions
        let test_data = get_bitcoin_testdata();

        SecurityPallet::set_active_block_number(1);

        // store all block headers. parachain_genesis is the first block
        // known in the parachain. Any block before will be rejected
        // ensure that first block is at a difficulty interval
        // NOTE: dataset starts at height 691451, first block where X % DIFFICULTY = 0
        // is 691488, hence we skip the first 37 blocks
        let skip_blocks = test_data
            .iter()
            .position(|d| d.height % DIFFICULTY_ADJUSTMENT_INTERVAL == 0)
            .unwrap();
        let parachain_genesis_header = test_data[skip_blocks].get_block_header();
        let parachain_genesis_height = test_data[skip_blocks].height;
        assert_eq!(parachain_genesis_height % DIFFICULTY_ADJUSTMENT_INTERVAL, 0);

        assert_ok!(RuntimeCall::BTCRelay(BTCRelayCall::initialize {
            block_header: parachain_genesis_header,
            block_height: parachain_genesis_height
        })
        .dispatch(origin_of(account_of(ALICE))));

        for block in test_data.iter().skip(skip_blocks + 1).take(BLOCKS_TO_TEST) {
            let block_header = block.get_block_header();
            let prev_header_hash = block_header.hash_prev_block;

            // check that the previously submitted header and the current header are matching
            let best_block_hash = BTCRelayPallet::get_best_block();
            let best_height = BTCRelayPallet::get_best_block_height();

            assert!(best_height == block.height - 1);
            assert!(best_block_hash == prev_header_hash);

            // submit block hashes
            assert_ok!(RuntimeCall::BTCRelay(BTCRelayCall::store_block_header {
                block_header: block.get_block_header(),
                reorg_bound: 10u32,
            })
            .dispatch(origin_of(account_of(ALICE))));

            assert_store_main_chain_header_event(block.height, block.get_block_hash(), account_of(ALICE));
        }
        SecurityPallet::set_active_block_number(1 + CONFIRMATIONS);

        // verify all transactions
        let current_height = btc_relay::Pallet::<Runtime>::get_best_block_height();
        for block in test_data.iter().skip(skip_blocks).take(BLOCKS_TO_TEST) {
            for tx in &block.test_txs {
                let txid = tx.get_txid();
                let merkle_proof = tx.get_merkle_proof();
                if block.height <= current_height - CONFIRMATIONS + 1 {
                    assert_ok!(BTCRelayPallet::_verify_transaction_inclusion(txid, merkle_proof, None));
                } else {
                    // expect to fail due to insufficient confirmations
                    assert_noop!(
                        BTCRelayPallet::_verify_transaction_inclusion(txid, merkle_proof, None),
                        BTCRelayError::BitcoinConfirmations
                    );
                }
            }
        }
    })
}

#[test]
fn integration_test_submit_fork_headers() {
    ExtBuilder::build().execute_without_relay_init(|| {
        const NUM_FORK_HEADERS: u32 = 2;
        const NUM_FORK_HEADERS_PLUS_ONE: u32 = NUM_FORK_HEADERS + 1;
        const REORG_HEIGHT: u32 = NUM_FORK_HEADERS + CONFIRMATIONS;
        const FORK_DEPTH: u32 = CONFIRMATIONS + 1;
        const FORK_ID: u32 = 1;

        // Load blocks with transactions
        // First header in the set is testnet3 genesis
        // Next two headers in the set are fork headers at height 1 and 2
        // Remainder are headers in the canonical chain with height 1, 2, ...
        // https://github.com/bitcoin/bitcoin/blob/d6a59166a1879c1dd5b3a301847961f4b3f17742/test/functional/p2p_dos_header_tree.py#L39
        let test_data = get_fork_testdata();

        SecurityPallet::set_active_block_number(1);

        let genesis_height = 0;
        // Note: the testdata set is old and hence this is a block version below 4
        // Therefore, this is stored directly from the parsed block in the `btc-relay` pallet
        // without going through the `relay` pallet, which checks for the block version when parsing
        let genesis_header = test_data[0];

        assert_ok!(BTCRelayPallet::_initialize(
            account_of(ALICE),
            genesis_header,
            genesis_height
        ));

        // submit the two fork headers first so that they become the main chain
        // chains_index[0]: [0] -> [f1] -> [f2]
        for (index, header) in test_data.iter().enumerate().skip(1).take(NUM_FORK_HEADERS as usize) {
            SecurityPallet::set_active_block_number(index as u32);

            assert_ok!(BTCRelayPallet::_store_block_header(
                &account_of(ALICE),
                header.clone(),
                10u32
            ));
            assert_store_main_chain_header_event(index as u32, header.hash, account_of(ALICE));
        }

        // submit future main chain without genesis
        for (index, header) in test_data.iter().enumerate().skip(1 + NUM_FORK_HEADERS as usize) {
            SecurityPallet::set_active_block_number(index as u32);
            let height: u32 = index as u32 - NUM_FORK_HEADERS;

            assert_ok!(BTCRelayPallet::_store_block_header(
                &account_of(ALICE),
                header.clone(),
                10u32
            ));

            // depending on the height and header, we expect different events and chain state
            match height {
                // store future main chain headers as fork to equal height
                // chains_index[0]: [0] -> [f1] -> [f2]
                //                      \
                // chain_index[1]:       -> [1] -> [2]
                0..=NUM_FORK_HEADERS => assert_store_fork_header_event(FORK_ID, height, header.hash, account_of(ALICE)),
                // store CONFIRMATION - 1 more headers
                // chains_index[0]: [0] -> [f1] -> [f2]
                //                      \
                // chain_index[1]:       -> [1] -> [2] -> [3] -> [4] -> [5] -> [6] -> [7]
                NUM_FORK_HEADERS_PLUS_ONE..REORG_HEIGHT => {
                    assert_store_fork_header_event(FORK_ID, height, header.hash, account_of(ALICE));
                    assert_fork_ahead_of_main_chain_event(NUM_FORK_HEADERS, height, FORK_ID);
                }
                // store one more header to cause a reorg
                // chain_index[0]: [0] -> [1] -> [2] -> [3] -> [4] -> [5] -> [6] -> [7] -> [8]
                //                      \
                // chains_index[1]:     -> [f1] -> [f2]
                REORG_HEIGHT => {
                    assert_chain_reorg_event(header.hash, height, FORK_DEPTH);
                    assert_store_main_chain_header_event(height, header.hash, account_of(ALICE));
                }
                // store the remaining headers
                // chain_index[0]: [0] -> [1] -> [2] -> [3] -> [4] -> [5] -> [6] -> [7] -> [8] -> [9] -> ...
                //                      \
                // chains_index[1]:     -> [f1] -> [f2]
                _ => assert_store_main_chain_header_event(height, header.hash, account_of(ALICE)),
            }
        }
    })
}
