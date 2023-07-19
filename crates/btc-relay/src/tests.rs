/// Tests for BTC-Relay
use sp_core::U256;

use crate::{ext, mock::*, types::*, BtcAddress, Error, DIFFICULTY_ADJUSTMENT_INTERVAL};

type Event = crate::Event<Test>;

use crate::{Chains, ChainsIndex};
use bitcoin::{merkle::*, parser::*, types::*};
use frame_support::{assert_err, assert_ok};
use mocktopus::mocking::*;
use sp_std::{
    convert::{TryFrom, TryInto},
    str::FromStr,
};

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
        let chain_id: u32 = 2;
        let block_height: u32 = 100;

        let rich_header = RichBlockHeader::<BlockNumber> {
            block_header: sample_block_header(),
            block_height,
            chain_id,
            para_height: Default::default(),
        };

        BTCRelay::set_block_header_from_hash(rich_header.block_hash(), &rich_header);

        let curr_header = BTCRelay::get_block_header_from_hash(rich_header.block_hash()).unwrap();
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
        let chain_id: u32 = 1;
        let start_height: u32 = 10;
        let block_height: u32 = 100;

        let blockchain = get_empty_block_chain_from_chain_id_and_height(chain_id, start_height, block_height);

        BTCRelay::set_block_chain_from_id(chain_id, &blockchain);

        let curr_blockchain = BTCRelay::get_block_chain_from_id(chain_id).unwrap();

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
        let relayer_id = 3;
        let block_height: u32 = 0;
        let block_header = sample_block_header();
        let block_header_hash = block_header.hash;
        BTCRelay::best_block_exists.mock_safe(|| MockResult::Return(false));

        assert_ok!(BTCRelay::_initialize(relayer_id, block_header, block_height));

        System::assert_has_event(TestEvent::BTCRelay(Event::Initialized {
            block_height,
            block_hash: block_header_hash,
            relayer_id,
        }));
    })
}

#[test]
fn initialize_best_block_already_set_fails() {
    run_test(|| {
        let relayer_id = 3;
        let block_height: u32 = 1;

        BTCRelay::best_block_exists.mock_safe(|| MockResult::Return(true));

        assert_err!(
            BTCRelay::_initialize(relayer_id, sample_block_header(), block_height),
            TestError::AlreadyInitialized
        );
    })
}

#[test]
fn initialize_with_invalid_difficulty_period_should_fail() {
    run_test(|| {
        let relayer_id = 3;
        let block_height: u32 = 2021;
        let block_header = sample_block_header();
        BTCRelay::best_block_exists.mock_safe(|| MockResult::Return(false));

        assert_err!(
            BTCRelay::_initialize(relayer_id, block_header, block_height),
            TestError::InvalidStartHeight
        );
    })
}

#[test]
fn initialize_with_valid_difficulty_period_should_succeed() {
    run_test(|| {
        let relayer_id = 3;
        let block_height: u32 = DIFFICULTY_ADJUSTMENT_INTERVAL;
        let block_header = sample_block_header();
        let block_header_hash = block_header.hash;
        BTCRelay::best_block_exists.mock_safe(|| MockResult::Return(false));

        assert_ok!(BTCRelay::_initialize(relayer_id, block_header, block_height));

        System::assert_has_event(TestEvent::BTCRelay(Event::Initialized {
            block_height,
            block_hash: block_header_hash,
            relayer_id,
        }));
    })
}

/// store_block_header function
#[test]
fn store_block_header_on_mainchain_succeeds() {
    run_test(|| {
        BTCRelay::verify_block_header.mock_safe(|_, _, _| MockResult::Return(Ok(())));
        BTCRelay::block_header_exists.mock_safe(|_| MockResult::Return(true));

        let chain_id: u32 = 0;
        let start_height: u32 = 0;
        let block_height: u32 = 100;
        let block_header = sample_block_header();

        let rich_header = RichBlockHeader::<BlockNumber> {
            block_header,
            block_height,
            chain_id,
            para_height: Default::default(),
        };
        BTCRelay::get_block_header_from_hash.mock_safe(move |_| MockResult::Return(Ok(rich_header)));

        let prev_blockchain = get_empty_block_chain_from_chain_id_and_height(chain_id, start_height, block_height);
        BTCRelay::get_block_chain_from_id.mock_safe(move |_: u32| MockResult::Return(Ok(prev_blockchain.clone())));

        let block_header_hash = block_header.hash;
        assert_ok!(BTCRelay::_store_block_header(&3, rich_header.block_header));

        let store_main_event = TestEvent::BTCRelay(Event::StoreMainChainHeader {
            block_height: block_height + 1,
            block_hash: block_header_hash,
            relayer_id: 3,
        });
        assert!(System::events().iter().any(|a| a.event == store_main_event));
    })
}

#[test]
fn store_block_header_on_fork_succeeds() {
    run_test(|| {
        BTCRelay::verify_block_header.mock_safe(|_, _, _| MockResult::Return(Ok(())));
        BTCRelay::block_header_exists.mock_safe(|_| MockResult::Return(true));

        let chain_id: u32 = 1;
        let start_height: u32 = 20;
        let block_height: u32 = 100;
        let block_header = sample_block_header();

        let rich_header = RichBlockHeader::<BlockNumber> {
            block_header,
            block_height: block_height - 1,
            chain_id,
            para_height: Default::default(),
        };
        BTCRelay::get_block_header_from_hash.mock_safe(move |_| MockResult::Return(Ok(rich_header)));

        let prev_blockchain = get_empty_block_chain_from_chain_id_and_height(chain_id, start_height, block_height);
        BTCRelay::get_block_chain_from_id.mock_safe(move |_: u32| MockResult::Return(Ok(prev_blockchain.clone())));

        let block_header_hash = block_header.hash;

        // simulate that initialize has been called
        BTCRelay::increment_chain_counter().unwrap();

        assert_ok!(BTCRelay::_store_block_header(&3, block_header));

        let store_fork_event = TestEvent::BTCRelay(Event::StoreForkHeader {
            chain_id,
            fork_height: block_height,
            block_hash: block_header_hash,
            relayer_id: 3,
        });
        assert!(System::events().iter().any(|a| a.event == store_fork_event));
    })
}

mod store_block_header_tests {
    use std::iter::successors;

    use crate::MAIN_CHAIN_ID;

    use super::*;
    fn from_prev(nonce: u32, prev: H256Le) -> BlockHeader {
        let mut ret = BlockHeader {
            nonce,
            hash_prev_block: prev,
            ..sample_block_header()
        };
        ret.update_hash().unwrap();
        ret
    }

    fn check_store_block_header_invariants() {
        let mainchain = ChainsIndex::<Test>::get(0).unwrap();

        // cache chains and chains_index for readability
        let mut chains = Chains::<Test>::iter().collect::<Vec<_>>();
        chains.sort_by_key(|k| k.0);
        let mut chains_index = ChainsIndex::<Test>::iter().collect::<Vec<_>>();
        chains_index.sort_by_key(|k| k.0);

        // The keys in ``Chains`` MUST be consecutive, i.e. for each ``i``, if ``Chains[i]`` does not exist,
        // ``Chains[i+1]`` MUST NOT exist either.
        for (arr_idx, (key, _value)) in chains.iter().enumerate() {
            assert_eq!(arr_idx as u32, *key);
        }
        let chains = chains.into_iter().map(|(_, value)| value).collect::<Vec<_>>();

        // The keys in ``ChainsIndex`` MUST be consecutive, i.e. for each ``i``, if ``ChainsIndex[i]`` does not exist,
        // ``ChainsIndex[i+1]`` MUST NOT exist either.
        for (arr_idx, (key, _value)) in chains_index.iter().enumerate() {
            assert_eq!(arr_idx as u32, *key);
        }
        let chains_index = chains_index.into_iter().map(|(_, value)| value).collect::<Vec<_>>();

        // for all i > 0, `ChainsIndex[i].maxHeight < ChainsIndex[0].maxHeight + STABLE_BITCOIN_CONFIRMATIONS`
        for chain in chains_index.iter().skip(1) {
            assert!(chain.max_height < mainchain.max_height + BTCRelay::get_stable_transaction_confirmations());
        }

        // The values in ``Chains`` MUST be such that for each ``0 < i < j``, ``ChainsIndex[Chains[i]].maxHeight >=
        // ChainsIndex[Chains[j]].maxHeight``.
        for i in 1..chains.len() - 1 {
            if chains_index[chains[i] as usize].max_height < chains_index[chains[i + 1] as usize].max_height {
                assert!(chains_index[chains[i] as usize].max_height >= chains_index[chains[i + 1] as usize].max_height);
            }
        }

        // ChainsIndex[i].chainRef = i
        for (idx, chain) in chains_index.iter().enumerate() {
            assert_eq!(idx as u32, chain.chain_id);
        }

        // BestBlock MUST refer the latest block from the main chain
        assert_eq!(
            BTCRelay::get_block_hash(MAIN_CHAIN_ID, mainchain.max_height).unwrap(),
            BTCRelay::get_best_block()
        );

        // BestBlockHeight MUST be equal to ``ChainsIndex[0].maxHeight``
        assert_eq!(BTCRelay::get_best_block_height(), mainchain.max_height);

        // the ``chainRef`` stored inside blocks MUST agree with the actual chain they are stored on
        for (chain_idx, _, hash) in crate::ChainsHashes::<Test>::iter() {
            assert_eq!(crate::BlockHeaders::<Test>::get(hash).chain_id, chain_idx);
        }

        // sanity check: BlockHeaders is indexed by richblock.block_header.hash
        for (hash, rich_block) in crate::BlockHeaders::<Test>::iter() {
            assert_eq!(rich_block.block_header.hash, hash);
        }

        // ChainsHashes MUST ONLY contain items for heights that are in the corresponding chain
        for (chain_idx, height, _hash) in crate::ChainsHashes::<Test>::iter() {
            let chain = &chains_index[chain_idx as usize];
            assert!(height >= chain.start_height && height <= chain.max_height);
        }
        // For each chain, ChainsHashes MUST contain exactly `chain_length` hashes
        for chain in chains_index {
            let num_blocks = crate::ChainsHashes::<Test>::iter()
                .map(|(chain_idx, _, _)| chain_idx)
                .filter(|&chain_idx| chain_idx == chain.chain_id)
                .count();
            assert_eq!(num_blocks as u32, chain.max_height - chain.start_height + 1);
        }
        // the number of ChainsHashes must match the number of submitted blockheaders
        assert_eq!(
            crate::ChainsHashes::<Test>::iter().count(),
            crate::BlockHeaders::<Test>::iter().count()
        );
    }

    fn assert_is_block(height: u32, block_header: &BlockHeader) {
        Security::set_active_block_number(ext::security::active_block_number::<Test>() + 1000);

        BTCRelay::ensure_no_ongoing_fork.mock_safe(|_| MockResult::Return(Ok(())));
        assert_ok!(BTCRelay::verify_block_header_inclusion(block_header.hash, Some(0)));
        BTCRelay::ensure_no_ongoing_fork.clear_mock();

        let chain = BTCRelay::get_block_chain_from_id(0).unwrap();
        assert_eq!(
            BTCRelay::get_block_header_from_height(&chain, height)
                .unwrap()
                .block_header,
            block_header.clone()
        );
    }

    #[test]
    fn store_block_header_simple_fork_succeeds() {
        run_test(|| {
            BTCRelay::verify_block_header.mock_safe(|_, _, _| MockResult::Return(Ok(())));

            let mut genesis = sample_block_header();
            genesis.nonce = 11;
            genesis.update_hash().unwrap();

            let block_height = 0;
            assert_ok!(BTCRelay::_initialize(3, genesis, block_height));

            // second block to the mainchain - otherwise we can't create a fork
            let a2 = from_prev(12, genesis.hash);

            let mut blocks = vec![a2];

            // create a new fork, and make it overtake the main chain
            let mut prev = genesis.hash;
            for i in 0..10 {
                blocks.push(from_prev(31 + i, prev));
                prev = blocks[blocks.len() - 1].hash;
            }

            for x in blocks.iter() {
                store_header_and_check_invariants(x);
            }

            assert_is_block(block_height, &genesis);
            for (idx, block) in blocks.iter().skip(1).enumerate() {
                assert_is_block(block_height + 1 + idx as u32, block);
            }
            // block a2 used to be in the mainchain, but is not anymore
            assert_err!(
                BTCRelay::verify_block_header_inclusion(a2.hash, Some(0)),
                TestError::InvalidChainID
            );
        })
    }

    fn assert_best_block(block_header: &BlockHeader, height: u32) {
        assert_eq!(BTCRelay::get_best_block_height(), height);
        assert_is_block(height, &block_header);
    }

    fn assert_ongoing_fork() {
        assert_err!(
            BTCRelay::ensure_no_ongoing_fork(BTCRelay::get_best_block_height()),
            TestError::OngoingFork
        );
    }

    fn store_header_and_check_invariants(block: &BlockHeader) {
        check_store_block_header_invariants();
        assert_ok!(BTCRelay::_store_block_header(&3, block.clone()));
        Security::set_active_block_number(
            ext::security::active_block_number::<Test>() + BTCRelay::parachain_confirmations(),
        );
        check_store_block_header_invariants();
    }

    #[test]
    fn store_block_header_fork_of_fork_succeeds() {
        run_test(|| {
            BTCRelay::verify_block_header.mock_safe(|_, _, _| MockResult::Return(Ok(())));

            let mut genesis = sample_block_header();
            genesis.update_hash().unwrap();

            // create the following tree shape:
            // f1 --> temp_fork_1
            //    \-> f2 --> temp_fork_2
            //           \-> f3 --> ... --> f10

            // corresponds to f1..f10
            let final_chain =
                sp_std::iter::successors(Some(genesis), |prev| Some(from_prev(prev.nonce + 1, prev.hash)))
                    .take(10)
                    .collect::<Vec<_>>();

            let block_height = 0;
            assert_ok!(BTCRelay::_initialize(3, final_chain[0].clone(), block_height));
            assert_best_block(&final_chain[0], block_height);

            // submit a temporary block such that final_chain[1] will be considered a fork
            let temp_fork_1 = from_prev(100, final_chain[0].hash);
            store_header_and_check_invariants(&temp_fork_1);

            store_header_and_check_invariants(&final_chain[1]);
            assert_ongoing_fork();

            // submit a temporary block such that final_chain[2] will be considered a fork
            let temp_fork_2 = from_prev(101, final_chain[1].hash);
            store_header_and_check_invariants(&temp_fork_2);
            assert_ongoing_fork();

            // main chain and fork currently have same height - we can submit CONFIRMATION-1 block without reorg
            for block in final_chain
                .iter()
                .skip(2) // 2 already submitted
                .take(BTCRelay::get_stable_transaction_confirmations() as usize - 1)
            {
                store_header_and_check_invariants(&block);
                assert_is_block(block_height + 1, &temp_fork_1);
            }

            // we did reorg, but temp_fork_2 has height of 12, so it is still considered an ongoing fork
            store_header_and_check_invariants(
                &final_chain[BTCRelay::get_stable_transaction_confirmations() as usize + 1],
            );

            assert_ongoing_fork();

            for (idx, block) in final_chain
                .iter()
                .enumerate()
                .skip(BTCRelay::get_stable_transaction_confirmations() as usize + 2)
            {
                // all blocks in final chain that have been submitted so far must now be usable
                for (idx, block) in final_chain.iter().enumerate().take(idx - 1) {
                    assert_is_block(block_height + idx as u32, block);
                }

                store_header_and_check_invariants(block);
            }

            // temp_fork_1 used to be in the mainchain, but is not anymore
            assert_err!(
                BTCRelay::verify_block_header_inclusion(temp_fork_1.hash, Some(0)),
                TestError::InvalidChainID
            );
            // temp_fork_2 is not included
            assert_err!(
                BTCRelay::verify_block_header_inclusion(temp_fork_2.hash, Some(0)),
                TestError::InvalidChainID
            );
        })
    }

    fn parse_from_hex(hex_string: &str) -> BlockHeader {
        BlockHeader::from_hex(hex_string).unwrap()
    }

    #[test]
    fn store_block_header_genesis_block_is_stored() {
        run_test(|| {
            BTCRelay::verify_block_header.mock_safe(|_, _, _| MockResult::Return(Ok(())));

            let genesis = sample_block_header();
            assert_ok!(BTCRelay::_initialize(3, genesis, 0));

            // store some blocks so that we don't get confirmation errors
            let blocks = successors(Some(genesis.clone()), |prev| Some(from_prev(0, prev.hash))).skip(1);
            for block in blocks.take(10) {
                store_header_and_check_invariants(&block);
            }

            // check that genesis block has been stored and is usable
            assert_ok!(BTCRelay::verify_block_header_inclusion(genesis.hash, None));
        })
    }

    #[test]
    pub fn test_real_world_fork() {
        run_test(|| {
            // data from the july 2015 fork: https://github.com/ethereum/btcrelay/tree/develop/test/headers/fork/20150704
            let relayer_id = 3;

            let genesis = parse_from_hex("03000000e6a65096db85d2ed2dfab33dea50b338341e1aeb5d0ce411000000000000000098420532fa55a0bca5f043f8f8f16a2b73761e822178692cb99ced039e2a32a0ea3f97558e4116183a9cc34a");
            assert_ok!(BTCRelay::_initialize(relayer_id, genesis, 0)); // 363730

            let fork_blocks: Vec<BlockHeader> = vec![
                "020000009e576e5a71f5af67e4dac515f8f3c02e536bb452d720a3060000000000000000699ff8e92806063b23a2e70009ec8d10cfc97a9a8a9a508f1b6b189b06845c7d644097558e411618b943d647",
                "03000000999d430cd9260dad128c49dd83ebd42c0bb425aa29c89c00000000000000000077e5e98057f2461764ddc0595b2b3521d803e923ae76498403154757ab06473d7c4397558e4116189f56edaa",
                "03000000b4905331a06377b2943509c5bc009986d2d55cd319255f150000000000000000334590846af20915ea38e90a4f4c5cd3cd42273ac2be57c59eb132f2be40cde08c4997558e411618d91e0fa5",
                "0300000061ec31a5ef18153ae5ba6a936973ad47e399e1e40ea2b70c00000000000000003b068951f10a9bdedcb70591dfab1a603830c5adee7301154d9266eb719f4d31164a97558e411618fd16e98a",
                "0300000018aa81e306208ec21658952c4a158f5a1d7dd80f5ed6660900000000000000009e932c2f6dd01e59b22d4603bae975f0c860433839a338418ebccb0b01e0db32504b97558e4116183f7dd1b1",
                "03000000e75d18cb65a02c0312b3093d10baeec721a466f5d6bf01130000000000000000844b3dd9043c983c02129b130bc651eb460a6d78df909bcd4ce50c602d8801c68a4d97558e41161822dd0c89",
            ].into_iter().map(parse_from_hex).collect();

            for block in fork_blocks.iter() {
                store_header_and_check_invariants(block);
            }

            let reorg_blocks:Vec<BlockHeader> = vec![
                "030000009e576e5a71f5af67e4dac515f8f3c02e536bb452d720a306000000000000000022862bfc426ccbe5868e8ee0d0abb9e89aa86a6031fa597323a081dcb29b15cff54897558e4116184b082d3b",
                "0300000053ef2a88b2b244409f03fee87f819ef14690c23033e2280c00000000000000009e90b9ad092c3f37393f163f70fcc054cbc7dc38210f213840fa9cf9791493b3954997558e4116186d536b51",
                "03000000d691c32ec84e22c0b9c8fbec184c0ec5f113b16e21e04212000000000000000092ccf4a5399e2436948a6917471135340a51967704bff3c55e57f5f0af6ca7d4275397558e411618d0abe918",
                "03000000eea345978c6b095148670d6128e1cc9069ac6bb3075c35060000000000000000b00ecf72f6d247a60eca5fc70d760939139cc0bc008d483c90b43e22596e0ed1dc5497558e41161884e655a3",
                "0300000036191cd0a5e5b1f04dec4cbb97170883fa621013e18a35150000000000000000d8f418aa2714981e26938ccd1620649a5c6fbe839eabc133ac0fac49deafe7dcb75597558e41161810d85d32",
                "03000000deab448a286a2873fcb3eac032aa1fbb13b7c96a3f24950600000000000000005df3fffaf0b0d3db741bf96cbf35830e3497f0634c819779281b4a2e5d301d65cd5697558e411618983a2772",

                "0300000033c784021ffbbdfff3bea3b4b9a7caa7f4f8c60713f7bc0300000000000000006e28294eb3195a9fe49845bd090bd69afe1e2b9301a8da1c27fd14a819d86da9a25797558e4116181625905a",
                "0300000015adb823bc91b2b706c0b259132723d10de12541857c4e010000000000000000658af03884b873d1e024f18425c32202ae54f6a2824ea05c81277b34fb325d77035997558e4116188ca36a47",
                "03000000948fad4ea4d24d9103ffaa1555fdd75a461e8e1a1f1fb61000000000000000009b13f566d729a53a70b0a5adc85211ba3cebe3cc098dcabd5502f5e462560c16af5a97558e411618c73c2f92",
                "03000000aca153703e2d0b0e73416d79a7e33abb7fea7bd19b5e600e00000000000000005c9ae978db9578eaf13deec63a9d83aa2e5b5c7b116ffd86ac517433a780bc4e4c5c97558e4116188c4a6f0e",
                "03000000d04ed892f51d8b4d51b66819b5c9689236977417f3a0f40800000000000000007cb5f9a5c9a065fcf4d21be7cc80d505ae1ef2a401ba481477f94aff07a91db3585c97558e41161826e96c22",
                "03000000b01d7251bb00e0798ef7fac2cbbe022e3093a8b006709d0b0000000000000000e58c392166f00accd7f773b4ca12fa1b2c1ccba33b8daca3274eb0f36394dd89456097558e41161839fc9a66",
                // "030000005fbd386a5032a0aa1c428416d2d0a9e62b3f31021bb695000000000000000000c46349cf6bddc4451f5df490ad53a83a58018e519579755800845fd4b0e39e79f46197558e41161884eabd86",
            ].into_iter().map(parse_from_hex).collect();

            for (idx, block) in reorg_blocks.iter().take(11).enumerate() {
                store_header_and_check_invariants(block);

                for previous_block in reorg_blocks.iter().take(idx + 1) {
                    assert_err!(
                        BTCRelay::verify_block_header_inclusion(previous_block.hash, None),
                        TestError::OngoingFork
                    );
                }
            }

            store_header_and_check_invariants(&reorg_blocks[reorg_blocks.len() - 1]);

            // now everything in main should verify, while nothing in the fork does.
            for block in reorg_blocks.iter().take(7) {
                assert_ok!(BTCRelay::verify_block_header_inclusion(block.hash, None));
            }
            for block in reorg_blocks.iter().skip(7) {
                assert_err!(
                    BTCRelay::verify_block_header_inclusion(block.hash, None),
                    TestError::BitcoinConfirmations
                );
            }
            for block in fork_blocks.iter() {
                assert_err!(
                    BTCRelay::verify_block_header_inclusion(block.hash, None),
                    TestError::InvalidChainID
                );
            }
        })
    }
}

#[test]
fn store_block_header_no_prev_block_fails() {
    run_test(|| {
        assert_err!(
            BTCRelay::_store_block_header(&3, sample_block_header()),
            TestError::BlockNotFound,
        );
    })
}

#[test]
fn check_and_do_reorg_fork_id_not_found() {
    run_test(|| {
        let chain_id: u32 = 99;
        let start_height: u32 = 3;
        let block_height: u32 = 10;

        let blockchain = get_empty_block_chain_from_chain_id_and_height(chain_id, start_height, block_height);

        assert_err!(BTCRelay::reorganize_chains(&blockchain), TestError::ForkIdNotFound);
    })
}

#[test]
fn check_and_do_reorg_swap_fork_position() {
    run_test(|| {
        // insert the main chain in Chains and ChainsIndex
        let main_chain_id: u32 = 0;
        let main_start_height: u32 = 3;
        let main_block_height: u32 = 110;
        let main_position: u32 = 0;
        let main = get_empty_block_chain_from_chain_id_and_height(main_chain_id, main_start_height, main_block_height);
        BTCRelay::set_chain_from_position_and_id(main_position, main_chain_id);
        BTCRelay::set_block_chain_from_id(main_chain_id, &main);

        // insert the fork chain in Chains and ChainsIndex
        let fork_chain_id: u32 = 4;
        let fork_start_height: u32 = 20;
        let fork_block_height: u32 = 100;
        let fork_position: u32 = 2;
        let fork = get_empty_block_chain_from_chain_id_and_height(fork_chain_id, fork_start_height, fork_block_height);
        BTCRelay::set_chain_from_position_and_id(fork_position, fork_chain_id);
        BTCRelay::set_block_chain_from_id(fork_chain_id, &fork);

        // insert the swap chain in Chains and ChainsIndex
        let swap_chain_id: u32 = 3;
        let swap_start_height: u32 = 43;
        let swap_block_height: u32 = 99;
        let swap_position: u32 = 1;
        let swap = get_empty_block_chain_from_chain_id_and_height(swap_chain_id, swap_start_height, swap_block_height);
        BTCRelay::set_chain_from_position_and_id(swap_position, swap_chain_id);
        BTCRelay::set_block_chain_from_id(swap_chain_id, &swap);

        // check that fork is at its initial position
        let current_position = BTCRelay::get_chain_position_from_chain_id(fork_chain_id).unwrap();

        assert_eq!(current_position, fork_position);

        assert_ok!(BTCRelay::reorganize_chains(&fork));
        // assert that positions have been swapped
        let new_position = BTCRelay::get_chain_position_from_chain_id(fork_chain_id).unwrap();
        assert_eq!(new_position, swap_position);

        // assert the main chain has not changed
        let curr_main_chain = BTCRelay::get_block_chain_from_id(main_chain_id);
        assert_eq!(curr_main_chain, Ok(main));
    })
}

#[test]
fn check_and_do_reorg_new_fork_is_main_chain() {
    run_test(|| {
        // insert the main chain in Chains and ChainsIndex
        let main_chain_id: u32 = 0;
        let main_start_height: u32 = 4;
        let main_block_height: u32 = 110;
        let main_position: u32 = 0;
        let main = get_empty_block_chain_from_chain_id_and_height(main_chain_id, main_start_height, main_block_height);
        BTCRelay::set_chain_from_position_and_id(main_position, main_chain_id);
        BTCRelay::set_block_chain_from_id(main_chain_id, &main);

        // insert the fork chain in Chains and ChainsIndex
        let fork_chain_id: u32 = 4;
        let fork_block_height: u32 = 117;
        let fork_position: u32 = 1;
        let fork = get_empty_block_chain_from_chain_id_and_height(fork_chain_id, main_start_height, fork_block_height);
        BTCRelay::set_chain_from_position_and_id(fork_position, fork_chain_id);
        BTCRelay::set_block_chain_from_id(fork_chain_id, &fork);

        // set the best block
        let best_block_hash = H256Le::zero();
        BTCRelay::set_best_block(best_block_hash);
        BTCRelay::set_best_block_height(fork_block_height);

        // check that fork is at its initial position
        let current_position = BTCRelay::get_chain_position_from_chain_id(fork_chain_id).unwrap();

        assert_eq!(current_position, fork_position);

        BTCRelay::swap_main_blockchain.mock_safe(move |_| MockResult::Return(Ok((best_block_hash, fork_block_height))));

        assert_ok!(BTCRelay::reorganize_chains(&fork));
        // assert that the new main chain is set
        let reorg_event = TestEvent::BTCRelay(Event::ChainReorg {
            new_chain_tip_hash: best_block_hash,
            new_chain_tip_height: fork_block_height,
            fork_depth: fork.max_height - fork.start_height,
        });
        assert!(System::events().iter().any(|a| a.event == reorg_event));
    })
}
#[test]
fn check_and_do_reorg_new_fork_below_stable_transaction_confirmations() {
    run_test(|| {
        // insert the main chain in Chains and ChainsIndex
        let main_chain_id: u32 = 0;
        let main_start_height: u32 = 4;
        let main_block_height: u32 = 110;
        let main_position: u32 = 0;
        let main = get_empty_block_chain_from_chain_id_and_height(main_chain_id, main_start_height, main_block_height);
        BTCRelay::set_chain_from_position_and_id(main_position, main_chain_id);
        BTCRelay::set_block_chain_from_id(main_chain_id, &main);

        // insert the fork chain in Chains and ChainsIndex
        let fork_chain_id: u32 = 4;
        let fork_block_height: u32 = 113;
        let fork_position: u32 = 1;
        let fork = get_empty_block_chain_from_chain_id_and_height(fork_chain_id, main_start_height, fork_block_height);
        BTCRelay::set_chain_from_position_and_id(fork_position, fork_chain_id);
        BTCRelay::set_block_chain_from_id(fork_chain_id, &fork);

        // set the best block
        let best_block_hash = H256Le::zero();
        BTCRelay::set_best_block(best_block_hash);
        BTCRelay::set_best_block_height(fork_block_height);

        // check that fork is at its initial position
        let current_position = BTCRelay::get_chain_position_from_chain_id(fork_chain_id).unwrap();

        assert_eq!(current_position, fork_position);

        BTCRelay::swap_main_blockchain.mock_safe(move |_| MockResult::Return(Ok((best_block_hash, fork_block_height))));

        assert_ok!(BTCRelay::reorganize_chains(&fork));
        // assert that the fork has not overtaken the main chain
        let ahead_event = TestEvent::BTCRelay(Event::ForkAheadOfMainChain {
            main_chain_height: main_block_height,
            fork_height: fork_block_height,
            fork_id: fork_chain_id,
        });
        assert!(System::events().iter().any(|a| a.event == ahead_event));
    })
}

/// insert_sorted
#[test]
fn insert_sorted_succeeds() {
    run_test(|| {
        // insert the main chain in Chains and ChainsIndex
        let main_chain_id: u32 = 0;
        let main_start_height: u32 = 60;
        let main_block_height: u32 = 110;
        let main_position: u32 = 0;
        let main = get_empty_block_chain_from_chain_id_and_height(main_chain_id, main_start_height, main_block_height);
        BTCRelay::set_block_chain_from_id(main_chain_id, &main);
        assert_eq!(Ok(()), BTCRelay::insert_sorted(&main));

        let curr_main_pos = BTCRelay::get_chain_position_from_chain_id(main_chain_id).unwrap();
        assert_eq!(curr_main_pos, main_position);
        // insert the swap chain in Chains and ChainsIndex
        let swap_chain_id: u32 = 3;
        let swap_start_height: u32 = 70;
        let swap_block_height: u32 = 99;
        let swap_position: u32 = 1;
        let swap = get_empty_block_chain_from_chain_id_and_height(swap_chain_id, swap_start_height, swap_block_height);
        BTCRelay::set_block_chain_from_id(swap_chain_id, &swap);
        assert_eq!(Ok(()), BTCRelay::insert_sorted(&swap));

        let curr_swap_pos = BTCRelay::get_chain_position_from_chain_id(swap_chain_id).unwrap();
        assert_eq!(curr_swap_pos, swap_position);

        // insert the fork chain in Chains and ChainsIndex
        let fork_chain_id: u32 = 4;
        let fork_start_height: u32 = 77;
        let fork_block_height: u32 = 100;
        let fork_position: u32 = 1;
        let new_swap_pos: u32 = 2;
        let fork = get_empty_block_chain_from_chain_id_and_height(fork_chain_id, fork_start_height, fork_block_height);
        BTCRelay::set_block_chain_from_id(fork_chain_id, &fork);
        assert_eq!(Ok(()), BTCRelay::insert_sorted(&fork));

        let curr_fork_pos = BTCRelay::get_chain_position_from_chain_id(fork_chain_id).unwrap();
        assert_eq!(curr_fork_pos, fork_position);
        let curr_swap_pos = BTCRelay::get_chain_position_from_chain_id(swap_chain_id).unwrap();
        assert_eq!(curr_swap_pos, new_swap_pos);
    })
}

/// verify_block_header
#[test]
fn test_verify_block_header_no_retarget_succeeds() {
    run_test(|| {
        let chain_id: u32 = 0;
        // no retarget at block 100
        let block_height: u32 = 100;
        let genesis_header = sample_parsed_genesis_header(chain_id, block_height);

        let block_header = BlockHeader::from_hex(sample_raw_first_header()).unwrap();

        // Not duplicate block
        BTCRelay::block_header_exists.mock_safe(move |_| MockResult::Return(false));

        assert_ok!(BTCRelay::verify_block_header(
            &block_header,
            genesis_header.block_height + 1,
            genesis_header
        ));
    })
}

#[test]
fn test_verify_block_header_correct_retarget_increase_succeeds() {
    run_test(|| {
        let chain_id: u32 = 0;
        // Next block requires retarget
        let block_height: u32 = 2015;
        // Sample interval with INCREASING target
        let retarget_headers = sample_retarget_interval_increase();

        let prev_block_header_rich =
            RichBlockHeader::<BlockNumber>::new(retarget_headers[1], chain_id, block_height, Default::default());

        let curr_block_header = retarget_headers[2];
        // Prev block exists
        BTCRelay::get_block_header_from_hash.mock_safe(move |_| MockResult::Return(Ok(prev_block_header_rich)));
        // Not duplicate block
        BTCRelay::block_header_exists.mock_safe(move |_| MockResult::Return(false));
        // Compute new target returns target of submitted header (i.e., correct)
        BTCRelay::compute_new_target.mock_safe(move |_, _| MockResult::Return(Ok(curr_block_header.target)));

        let block_header = retarget_headers[2];
        assert_ok!(BTCRelay::verify_block_header(
            &block_header,
            prev_block_header_rich.block_height + 1,
            prev_block_header_rich
        ));
    })
}

#[test]
fn test_verify_block_header_correct_retarget_decrease_succeeds() {
    run_test(|| {
        let chain_id: u32 = 0;
        // Next block requires retarget
        let block_height: u32 = 2015;
        // Sample interval with DECREASING target
        let retarget_headers = sample_retarget_interval_decrease();

        let prev_block_header_rich =
            RichBlockHeader::<BlockNumber>::new(retarget_headers[1], chain_id, block_height, Default::default());

        let curr_block_header = retarget_headers[2];
        // Not duplicate block
        BTCRelay::block_header_exists.mock_safe(move |_| MockResult::Return(false));
        // Compute new target returns target of submitted header (i.e., correct)
        BTCRelay::compute_new_target.mock_safe(move |_, _| MockResult::Return(Ok(curr_block_header.target)));

        let block_header = &retarget_headers[2];
        assert_ok!(BTCRelay::verify_block_header(
            &block_header,
            prev_block_header_rich.block_height + 1,
            prev_block_header_rich
        ));
    })
}

#[test]
fn test_verify_block_header_missing_retarget_succeeds() {
    run_test(|| {
        let chain_id: u32 = 0;
        // Next block requires retarget
        let block_height: u32 = 2015;
        let retarget_headers = sample_retarget_interval_increase();

        let prev_block_header_rich =
            RichBlockHeader::<BlockNumber>::new(retarget_headers[1], chain_id, block_height, Default::default());

        let curr_block_header = retarget_headers[2];
        // Not duplicate block
        BTCRelay::block_header_exists.mock_safe(move |_| MockResult::Return(false));
        // Compute new target returns HIGHER target
        BTCRelay::compute_new_target.mock_safe(move |_, _| MockResult::Return(Ok(curr_block_header.target + 1)));

        let block_header = retarget_headers[2];
        assert_err!(
            BTCRelay::verify_block_header(
                &block_header,
                prev_block_header_rich.block_height + 1,
                prev_block_header_rich
            ),
            TestError::DiffTargetHeader
        );
    })
}

#[test]
fn test_compute_new_target() {
    let chain_id: u32 = 0;
    // no retarget at block 100
    let block_height: u32 = 2016;
    let retarget_headers = sample_retarget_interval_increase();

    let last_retarget_time = retarget_headers[0].timestamp as u64;
    let prev_block_header =
        RichBlockHeader::<BlockNumber>::new(retarget_headers[1], chain_id, block_height, Default::default());

    let curr_block_header = retarget_headers[2];

    BTCRelay::get_last_retarget_time.mock_safe(move |_, _| MockResult::Return(Ok(last_retarget_time)));

    let new_target = BTCRelay::compute_new_target(&prev_block_header, block_height).unwrap();

    assert_eq!(new_target, curr_block_header.target);
}

#[test]
fn test_verify_block_header_duplicate_fails() {
    run_test(|| {
        let chain_id: u32 = 0;
        // no retarget at block 100
        let block_height: u32 = 100;
        let genesis_header = sample_parsed_genesis_header(chain_id, block_height);

        let rich_first_header = sample_parsed_first_block(chain_id, 101);

        // Prev block is genesis
        BTCRelay::get_block_header_from_hash.mock_safe(move |_| MockResult::Return(Ok(genesis_header)));
        // submitted block ALREADY EXISTS
        BTCRelay::block_header_exists.mock_safe(move |block_hash| {
            assert_eq!(&block_hash, &rich_first_header.block_header.hash);
            MockResult::Return(true)
        });

        let first_header = BlockHeader::from_hex(sample_raw_first_header()).unwrap();
        assert_err!(
            BTCRelay::verify_block_header(&first_header, genesis_header.block_height + 1, genesis_header),
            TestError::DuplicateBlock
        );
    })
}

#[test]
fn test_verify_block_header_low_diff_fails() {
    run_test(|| {
        let chain_id: u32 = 0;
        // no retarget at block 100
        let block_height: u32 = 100;
        let genesis_header = sample_parsed_genesis_header(chain_id, block_height);

        // block header with high target but weak hash
        let first_header_weak = BlockHeader::from_hex(sample_raw_first_header_low_diff()).unwrap();

        // submitted block does not yet exist
        BTCRelay::block_header_exists.mock_safe(move |_| MockResult::Return(false));

        assert_err!(
            BTCRelay::verify_block_header(&first_header_weak, genesis_header.block_height + 1, genesis_header),
            TestError::LowDiff
        );
    });
}

#[test]
fn test_validate_transaction_succeeds_with_payment() {
    run_test(|| {
        let minimum_btc: i64 = 2500200000;
        let recipient_btc_address =
            BtcAddress::P2SH(H160::from_str(&"66c7060feb882664ae62ffad0051fe843e318e85").unwrap());

        let outputs = vec![sample_valid_payment_output()];

        let transaction = sample_transaction_parsed(&outputs);

        assert_ok!(BTCRelay::_validate_transaction(
            transaction,
            minimum_btc,
            recipient_btc_address,
            None,
        ));
    });
}

#[test]
fn test_validate_transaction_succeeds_with_payment_and_op_return() {
    run_test(|| {
        let minimum_btc: i64 = 2500200000;
        let recipient_btc_address =
            BtcAddress::P2SH(H160::from_str(&"66c7060feb882664ae62ffad0051fe843e318e85").unwrap());
        let op_return_id =
            hex::decode("e5c17d15b8b1fa2811b7e6da66ffa5e1aaa05922c69068bf90cd585b95bb4675".to_owned()).unwrap();

        let outputs = vec![sample_valid_payment_output(), sample_valid_data_output()];

        let transaction = sample_transaction_parsed(&outputs);

        assert_ok!(BTCRelay::_validate_transaction(
            transaction,
            minimum_btc,
            recipient_btc_address,
            Some(H256::from_slice(&op_return_id))
        ));
    });
}

#[test]
fn test_validate_transaction_succeeds_with_op_return_and_payment() {
    run_test(|| {
        let minimum_btc: i64 = 2500200000;
        let recipient_btc_address =
            BtcAddress::P2SH(H160::from_str(&"66c7060feb882664ae62ffad0051fe843e318e85").unwrap());
        let op_return_id =
            hex::decode("e5c17d15b8b1fa2811b7e6da66ffa5e1aaa05922c69068bf90cd585b95bb4675".to_owned()).unwrap();

        let outputs = vec![sample_valid_data_output(), sample_valid_payment_output()];

        let transaction = sample_transaction_parsed(&outputs);

        assert_ok!(BTCRelay::_validate_transaction(
            transaction,
            minimum_btc,
            recipient_btc_address,
            Some(H256::from_slice(&op_return_id))
        ));
    });
}

#[test]
fn test_validate_transaction_succeeds_with_payment_and_refund_and_op_return() {
    run_test(|| {
        let minimum_btc: i64 = 2500200000;
        let recipient_btc_address =
            BtcAddress::P2SH(H160::from_str(&"66c7060feb882664ae62ffad0051fe843e318e85").unwrap());
        let op_return_id =
            hex::decode("e5c17d15b8b1fa2811b7e6da66ffa5e1aaa05922c69068bf90cd585b95bb4675".to_owned()).unwrap();

        let outputs = vec![
            sample_valid_payment_output(),
            sample_wrong_recipient_payment_output(),
            sample_valid_data_output(),
        ];

        let transaction = sample_transaction_parsed(&outputs);

        assert_ok!(BTCRelay::_validate_transaction(
            transaction,
            minimum_btc,
            recipient_btc_address,
            Some(H256::from_slice(&op_return_id))
        ));
    });
}

#[test]
fn test_validate_transaction_invalid_no_outputs_fails() {
    run_test(|| {
        let minimum_btc: i64 = 2500200000;
        let recipient_btc_address =
            BtcAddress::P2SH(H160::from_str(&"66c7060feb882664ae62ffad0051fe843e318e85").unwrap());
        let op_return_id =
            hex::decode("ede5c17d15b8b1fa2811b7e6da66ffa5e1aaa05922c69068bf90cd585b95bb46".to_owned()).unwrap();
        // missing required data output
        let outputs = vec![sample_valid_payment_output()];

        let transaction = sample_transaction_parsed(&outputs);

        assert_err!(
            BTCRelay::_validate_transaction(
                transaction,
                minimum_btc,
                recipient_btc_address,
                Some(H256::from_slice(&op_return_id))
            ),
            TestError::InvalidOpReturnTransaction
        )
    });
}

#[test]
fn test_validate_transaction_insufficient_payment_value_fails() {
    run_test(|| {
        let minimum_btc: i64 = 2500200000;
        let recipient_btc_address =
            BtcAddress::P2SH(H160::from_str(&"66c7060feb882664ae62ffad0051fe843e318e85").unwrap());
        let op_return_id =
            hex::decode("e5c17d15b8b1fa2811b7e6da66ffa5e1aaa05922c69068bf90cd585b95bb4675".to_owned()).unwrap();

        let outputs = vec![sample_insufficient_value_payment_output(), sample_valid_data_output()];

        let transaction = sample_transaction_parsed(&outputs);

        assert_err!(
            BTCRelay::_validate_transaction(
                transaction,
                minimum_btc,
                recipient_btc_address,
                Some(H256::from_slice(&op_return_id))
            ),
            TestError::InvalidPaymentAmount
        )
    });
}

#[test]
fn test_validate_transaction_wrong_recipient_fails() {
    run_test(|| {
        let minimum_btc: i64 = 2500200000;
        let recipient_btc_address =
            BtcAddress::P2SH(H160::from_str(&"66c7060feb882664ae62ffad0051fe843e318e85").unwrap());
        let op_return_id =
            hex::decode("e5c17d15b8b1fa2811b7e6da66ffa5e1aaa05922c69068bf90cd585b95bb4675".to_owned()).unwrap();

        let outputs = vec![
            sample_wrong_recipient_payment_output(),
            sample_wrong_recipient_payment_output(),
            sample_valid_data_output(),
        ];

        let transaction = sample_transaction_parsed(&outputs);

        assert_err!(
            BTCRelay::_validate_transaction(
                transaction,
                minimum_btc,
                recipient_btc_address,
                Some(H256::from_slice(&op_return_id))
            ),
            TestError::InvalidOpReturnTransaction
        )
    });
}

#[test]
fn test_validate_transaction_incorrect_opreturn_fails() {
    run_test(|| {
        let minimum_btc: i64 = 2500200000;
        let recipient_btc_address =
            BtcAddress::P2SH(H160::from_str(&"66c7060feb882664ae62ffad0051fe843e318e85").unwrap());
        let op_return_id =
            hex::decode("0000000000000000000000000000000000000000000000000000000000000000".to_owned()).unwrap();

        let outputs = vec![sample_valid_payment_output(), sample_incorrect_data_output()];

        let transaction = sample_transaction_parsed(&outputs);

        assert_err!(
            BTCRelay::_validate_transaction(
                transaction,
                minimum_btc,
                recipient_btc_address,
                Some(H256::from_slice(&op_return_id))
            ),
            TestError::InvalidOpReturnTransaction
        )
    });
}

#[test]
fn test_verify_transaction_inclusion_succeeds() {
    run_test(|| {
        let chain_id = 0;
        let fork_ref = 1;
        let start = 10;
        let main_chain_height = 300;
        let fork_chain_height = 280;
        let confirmations = None;
        let rich_block_header = sample_rich_tx_block_header(chain_id, main_chain_height);

        let main = get_empty_block_chain_from_chain_id_and_height(chain_id, start, main_chain_height);

        let fork = get_empty_block_chain_from_chain_id_and_height(fork_ref, start, fork_chain_height);

        BTCRelay::get_chain_id_from_position.mock_safe(move |_| MockResult::Return(Ok(fork_ref)));
        BTCRelay::get_block_chain_from_id.mock_safe(move |id| {
            if id == chain_id {
                MockResult::Return(Ok(main.clone()))
            } else {
                MockResult::Return(Ok(fork.clone()))
            }
        });

        BTCRelay::get_best_block_height.mock_safe(move || MockResult::Return(main_chain_height));

        BTCRelay::block_matches_merkle_root.mock_safe(move |_, _| MockResult::Return(true));

        BTCRelay::get_block_header_from_hash.mock_safe(move |_| MockResult::Return(Ok(rich_block_header)));

        BTCRelay::check_bitcoin_confirmations.mock_safe(|_, _, _| MockResult::Return(Ok(())));

        BTCRelay::check_parachain_confirmations.mock_safe(|_| MockResult::Return(Ok(())));

        assert_ok!(BTCRelay::_verify_transaction_inclusion(
            sample_unchecked_transaction(),
            confirmations
        ));
    });
}

#[test]
fn test_verify_transaction_inclusion_empty_fork_succeeds() {
    run_test(|| {
        let chain_id = 0;
        let start = 10;
        let main_chain_height = 300;
        let confirmations = None;
        let rich_block_header = sample_rich_tx_block_header(chain_id, main_chain_height);

        let main = get_empty_block_chain_from_chain_id_and_height(chain_id, start, main_chain_height);

        BTCRelay::get_block_chain_from_id.mock_safe(move |id| {
            if id == chain_id {
                MockResult::Return(Ok(main.clone()))
            } else {
                MockResult::Return(Err(TestError::InvalidChainID.into()))
            }
        });

        BTCRelay::get_best_block_height.mock_safe(move || MockResult::Return(main_chain_height));

        BTCRelay::get_block_header_from_hash.mock_safe(move |_| MockResult::Return(Ok(rich_block_header)));

        BTCRelay::check_bitcoin_confirmations.mock_safe(|_, _, _| MockResult::Return(Ok(())));

        BTCRelay::check_parachain_confirmations.mock_safe(|_| MockResult::Return(Ok(())));

        BTCRelay::block_matches_merkle_root.mock_safe(move |_, _| MockResult::Return(true));

        assert_ok!(BTCRelay::_verify_transaction_inclusion(
            sample_unchecked_transaction(),
            confirmations,
        ));
    });
}

#[test]
fn test_verify_transaction_inclusion_invalid_tx_id_fails() {
    run_test(|| {
        let chain_id = 0;
        let fork_ref = 1;
        let start = 10;
        let main_chain_height = 300;
        let fork_chain_height = 280;
        let confirmations = None;
        let rich_block_header = sample_rich_tx_block_header(chain_id, main_chain_height);

        let main = get_empty_block_chain_from_chain_id_and_height(chain_id, start, main_chain_height);

        let fork = get_empty_block_chain_from_chain_id_and_height(fork_ref, start, fork_chain_height);

        BTCRelay::get_chain_id_from_position.mock_safe(move |_| MockResult::Return(Ok(fork_ref)));
        BTCRelay::get_block_chain_from_id.mock_safe(move |id| {
            if id == chain_id {
                MockResult::Return(Ok(main.clone()))
            } else {
                MockResult::Return(Ok(fork.clone()))
            }
        });

        BTCRelay::get_best_block_height.mock_safe(move || MockResult::Return(main_chain_height));

        BTCRelay::get_block_header_from_hash.mock_safe(move |_| MockResult::Return(Ok(rich_block_header)));

        BTCRelay::check_bitcoin_confirmations.mock_safe(|_, _, _| MockResult::Return(Ok(())));

        BTCRelay::check_parachain_confirmations.mock_safe(|_| MockResult::Return(Ok(())));

        let mut tx = sample_unchecked_transaction();

        // Mismatching TXID
        tx.coinbase_proof.merkle_proof.hashes[0] = H256Le::from_bytes_le(
            &hex::decode("0000000000000000000000000000000000000000000000000000000000000000".to_owned()).unwrap(),
        );

        assert_err!(
            BTCRelay::_verify_transaction_inclusion(tx, confirmations,),
            TestError::InvalidTxid
        );
    });
}

#[test]
fn test_verify_transaction_inclusion_invalid_merkle_root_fails() {
    run_test(|| {
        let chain_id = 0;
        let fork_ref = 1;
        let start = 10;
        let main_chain_height = 300;
        let fork_chain_height = 280;
        let confirmations = None;
        let mut rich_block_header = sample_rich_tx_block_header(chain_id, main_chain_height);

        // Mismatching merkle root
        let invalid_merkle_root = H256Le::from_bytes_le(
            &hex::decode("0000000000000000000000000000000000000000000000000000000000000000".to_owned()).unwrap(),
        );
        rich_block_header.block_header.merkle_root = invalid_merkle_root;

        let main = get_empty_block_chain_from_chain_id_and_height(chain_id, start, main_chain_height);

        let fork = get_empty_block_chain_from_chain_id_and_height(fork_ref, start, fork_chain_height);

        BTCRelay::get_chain_id_from_position.mock_safe(move |_| MockResult::Return(Ok(fork_ref)));
        BTCRelay::get_block_chain_from_id.mock_safe(move |id| {
            if id == chain_id {
                MockResult::Return(Ok(main.clone()))
            } else {
                MockResult::Return(Ok(fork.clone()))
            }
        });

        BTCRelay::get_best_block_height.mock_safe(move || MockResult::Return(main_chain_height));

        BTCRelay::get_block_header_from_hash.mock_safe(move |_| MockResult::Return(Ok(rich_block_header)));

        BTCRelay::check_bitcoin_confirmations.mock_safe(|_, _, _| MockResult::Return(Ok(())));

        BTCRelay::check_parachain_confirmations.mock_safe(|_| MockResult::Return(Ok(())));

        // merkle root does not match block
        BTCRelay::block_matches_merkle_root.mock_safe(move |_, _| MockResult::Return(false));

        assert_err!(
            BTCRelay::_verify_transaction_inclusion(sample_unchecked_transaction(), confirmations,),
            TestError::InvalidMerkleProof
        );
    });
}

#[test]
fn test_verify_transaction_inclusion_fails_with_ongoing_fork() {
    run_test(|| {
        BTCRelay::get_chain_id_from_position.mock_safe(|_| MockResult::Return(Ok(1)));
        BTCRelay::get_block_chain_from_id.mock_safe(|_| MockResult::Return(Ok(BlockChain::default())));

        let confirmations = None;

        assert_err!(
            BTCRelay::_verify_transaction_inclusion(sample_unchecked_transaction(), confirmations,),
            TestError::OngoingFork
        );
    });
}

#[test]
fn test_get_and_verify_issue_payment_with_tx_containing_taproot() {
    run_test(|| {
        BTCRelay::_verify_transaction_inclusion.mock_safe(|_, _| {
            let raw_tx = "010000000001013413e41f47eecad702082578c35a2925217056fd0a837b22f1a205fe178a010d0500000000ffffffff19771000000000000017a91415f691c1905082c300362d48540846c30855162d877a1000000000000022512038234fa3e3ca718dfadfb540c320180e68798e67e0a9d4f10d98ea33d37caf047a100000000000001976a914d73838271ee26471aa3640915ed7274b49435b6688acee2000000000000016001470eab26ae0074a58802acc7c38cd9941619c408d14250000000000001976a91479ef95650e8284c3be439d888cf2ee2d1d8ef63088ac3129000000000000160014a558dd2db8167e069f580da2482a9b73dc4f5960217f0000000000001976a91409f3607112083fb1ffe3718214a8e5d5eb0da46188ac04a50000000000001600149215c14609d581aacaa54f629e823cc8abd17ee6c7cd00000000000017a9146da59c9a54a5465402884712bbbe140bc68a4f218728f700000000000017a914ed99cbd06b43b4e3741d1457f7af7b24c2e8d12487ae380100000000001976a91448296f6f29c497f59193ab4e7def5f2e03ef2f9988ac654901000000000017a914ba997376b5daaa3707aefdf30cc09745b579df2187a6a301000000000017a914ffed3c6e71adc2b73939d6951f4655ed1432909b87ec9202000000000017a9147759a1bffe2acca168afdb5b106250b02a703b2887d63603000000000016001439fef3095e8a3bce11ce471aa602bf3e3609d8ddae3703000000000017a914ea0d18bbd804d17a1f2f07ed9aa1670721777d2287cd370300000000001976a914bcc6bcffe584761176d8f510896e882f838208d988ac1d3803000000000016001470eb59ad925fdec71ca0ec50cf7c6b9bbe8dc7592f380300000000001976a91447eb6c94d7b2ac0c11eb3957c0844d333e21d02e88ac724803000000000016001470eb59ad925fdec71ca0ec50cf7c6b9bbe8dc759692e050000000000160014ae26178c1a9b4adb6f24f047fa119e034205900c381b10000000000017a914c9e20b0d7e46d07a878585955ca377db833d181587d32b20000000000017a914bbfcd0b601046e1656ba9b74a98ee8d362d5b63687402f200000000000160014ca146a720a30ca404e979df59d3ddca039e8fd58f22fea0000000000220020935f3eb059cd94bd307e6378bd590724f361f0316fd0964eb5952f274dfb7b4f0400483045022100c9fc44a423e31fc792f5d255ae09ffdc0b224cb70fcebacd52183ce2813ba11d022046c8530230f644be4a05f25bd6a2264b99afc7e3e38531d4bde12d477d03f18001473044022027f50b14154123b173286db76e189a32973a13b0b4ca425329533229cf7f8d9a02202cea81a657ee654c63ab4a01a741931378abae036435a1d695622216596d9e27016952210257bf4070df9735de32305f3bc25320d331edb10c662423e06cd1e50bc58d8fa7210246454540c4e36ba6a481347d0194ffe476640289aecfd2d3f3db1328415b9a5c210248e0a3385d6f744ae81779e10f8ccafbbed7d44debf08a2b0d5250e2f0a0e84853aef0210b00";
            let tx_bytes = hex::decode(&raw_tx).unwrap();
            let transaction = parse_transaction(&tx_bytes).unwrap();

            MockResult::Return(Ok(transaction))
        });

        // check the last output address
        let raw_address = "935f3eb059cd94bd307e6378bd590724f361f0316fd0964eb5952f274dfb7b4f";
        let address_bytes = hex::decode(&raw_address).unwrap();
        let address_hash = H256::from_slice(&address_bytes);
        let recipient_btc_address = BtcAddress::P2WSHv0(address_hash);

        assert_ok!(
            BTCRelay::get_and_verify_issue_payment::<i64>(sample_unchecked_transaction(), recipient_btc_address),
            15347698
        );
    })
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
        Security::set_active_block_number(5 + PARACHAIN_CONFIRMATIONS);
        assert_ok!(BTCRelay::check_parachain_confirmations(0));
    });
}

#[test]
fn test_check_parachain_confirmations_insufficient_confs_fails() {
    run_test(|| {
        Security::set_active_block_number(0);
        assert_err!(
            BTCRelay::check_parachain_confirmations(0),
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
        let main_chain_id: u32 = 0;
        let main_start_height: u32 = 3;
        let main_block_height: u32 = 110;
        let main_position: u32 = 0;
        let main = get_empty_block_chain_from_chain_id_and_height(main_chain_id, main_start_height, main_block_height);
        BTCRelay::set_chain_from_position_and_id(main_position, main_chain_id);
        BTCRelay::set_block_chain_from_id(main_chain_id, &main);

        assert_eq!(Ok(main), BTCRelay::get_block_chain_from_id(main_chain_id));
    });
}

#[test]
fn store_generated_block_headers() {
    let target = U256::from(2).pow(254.into());
    let miner = BtcAddress::P2PKH(H160::from_str(&"66c7060feb882664ae62ffad0051fe843e318e85").unwrap());

    run_test(|| {
        let mut last_block = BlockBuilder::new().with_coinbase(&miner, 50, 0).mine(target).unwrap();
        assert_ok!(BTCRelay::_initialize(3, last_block.header, 0));
        for i in 1..20 {
            last_block = BlockBuilder::new()
                .with_coinbase(&miner, 50, i)
                .with_previous_hash(last_block.header.hash)
                .mine(target)
                .unwrap();
            assert_ok!(BTCRelay::_store_block_header(&3, last_block.header));
        }
        let main_chain: BlockChain = BTCRelay::get_block_chain_from_id(crate::MAIN_CHAIN_ID).unwrap();
        assert_eq!(main_chain.start_height, 0);
        assert_eq!(main_chain.max_height, 19);
    })
}

mod op_return_payment_data_tests {
    use super::*;
    use itertools::Itertools;

    fn permutations(transaction: Transaction) -> impl Iterator<Item = Transaction> {
        let n = transaction.outputs.len();
        let outputs = transaction.outputs.clone();
        outputs.into_iter().permutations(n).map(move |x| Transaction {
            outputs: x.clone(),
            ..transaction.clone()
        })
    }

    fn dummy_address1() -> BtcAddress {
        BtcAddress::P2SH(H160::from_str(&"66c7060feb882664ae62ffad0051fe843e318e85").unwrap())
    }

    fn dummy_address2() -> BtcAddress {
        BtcAddress::P2SH(H160::from_str(&"0000000000000000000000000000000000000000").unwrap())
    }
    fn dummy_address3() -> BtcAddress {
        BtcAddress::P2SH(H160::from_str(&"1000000000000000000000000000000000000000").unwrap())
    }

    #[test]
    fn test_constructing_op_return_payment_data_with_zero_outputs_fails() {
        run_test(|| {
            let transaction = TransactionBuilder::new()
                .with_version(2)
                .add_output(TransactionOutput::op_return(0, &[0; 32]))
                .build();

            for transaction in permutations(transaction) {
                assert_err!(
                    OpReturnPaymentData::<Test>::try_from(transaction),
                    Error::<Test>::InvalidOpReturnTransaction
                );
            }
        })
    }

    #[test]
    fn test_constructing_op_return_payment_data_with_one_output_succeeds() {
        run_test(|| {
            let transaction = TransactionBuilder::new()
                .with_version(2)
                .add_output(TransactionOutput::payment(252345, &dummy_address1()))
                .add_output(TransactionOutput::op_return(0, &[0; 32]))
                .build();

            for transaction in permutations(transaction) {
                assert_ok!(OpReturnPaymentData::<Test>::try_from(transaction));
            }
        })
    }

    #[test]
    fn test_constructing_op_return_payment_data_with_two_outputs_succeeds() {
        run_test(|| {
            let transaction = TransactionBuilder::new()
                .with_version(2)
                .add_output(TransactionOutput::payment(252345, &dummy_address1()))
                .add_output(TransactionOutput::payment(252345, &dummy_address2()))
                .add_output(TransactionOutput::op_return(0, &[0; 32]))
                .build();

            for transaction in permutations(transaction) {
                assert_ok!(OpReturnPaymentData::<Test>::try_from(transaction));
            }
        })
    }

    #[test]
    fn test_constructing_op_return_payment_data_with_too_many_outputs_fails() {
        run_test(|| {
            let transaction = TransactionBuilder::new()
                .with_version(2)
                .add_output(TransactionOutput::payment(252345, &dummy_address1()))
                .add_output(TransactionOutput::payment(252345, &dummy_address2()))
                .add_output(TransactionOutput::payment(252345, &dummy_address3()))
                .add_output(TransactionOutput::op_return(0, &[0; 32]))
                .build();

            for transaction in permutations(transaction) {
                assert_err!(
                    OpReturnPaymentData::<Test>::try_from(transaction),
                    Error::<Test>::InvalidOpReturnTransaction
                );
            }
        })
    }
    #[test]
    fn test_constructing_op_return_payment_data_with_two_identical_outputs_fails() {
        run_test(|| {
            let transaction = TransactionBuilder::new()
                .with_version(2)
                .add_output(TransactionOutput::payment(252345, &dummy_address1()))
                .add_output(TransactionOutput::payment(252344145, &dummy_address1()))
                .add_output(TransactionOutput::op_return(0, &[0; 32]))
                .build();

            for transaction in permutations(transaction) {
                assert_err!(
                    OpReturnPaymentData::<Test>::try_from(transaction),
                    Error::<Test>::InvalidOpReturnTransaction
                );
            }
        })
    }
    #[test]
    fn test_constructing_op_return_payment_data_with_invalid_op_return_len_fails() {
        run_test(|| {
            let transaction = TransactionBuilder::new()
                .with_version(2)
                .add_output(TransactionOutput::payment(252345, &dummy_address1()))
                .add_output(TransactionOutput::op_return(0, &[0; 31]))
                .build();

            for transaction in permutations(transaction) {
                assert_err!(
                    OpReturnPaymentData::<Test>::try_from(transaction),
                    Error::<Test>::InvalidOpReturnTransaction
                );
            }
        })
    }
    #[test]
    fn test_constructing_op_return_payment_data_with_two_op_returns_fails() {
        run_test(|| {
            let transaction = TransactionBuilder::new()
                .with_version(2)
                .add_output(TransactionOutput::payment(252345, &dummy_address1()))
                .add_output(TransactionOutput::op_return(0, &[0; 32]))
                .add_output(TransactionOutput::op_return(0, &[1; 32]))
                .build();

            for transaction in permutations(transaction) {
                assert_err!(
                    OpReturnPaymentData::<Test>::try_from(transaction),
                    Error::<Test>::InvalidOpReturnTransaction
                );
            }
        })
    }
    #[test]
    fn test_constructing_op_return_payment_data_with_op_return_value_fails() {
        run_test(|| {
            let transaction = TransactionBuilder::new()
                .with_version(2)
                .add_output(TransactionOutput::payment(252345, &dummy_address1()))
                .add_output(TransactionOutput::op_return(1, &[0; 32]))
                .build();

            for transaction in permutations(transaction) {
                assert_err!(
                    OpReturnPaymentData::<Test>::try_from(transaction),
                    Error::<Test>::InvalidOpReturnTransaction
                );
            }
        })
    }

    #[test]
    fn test_ensure_valid_payment_to_succeeds() {
        run_test(|| {
            let amount = 12345;
            let op_return = H256::from_slice(&[5; 32]);
            let transaction = TransactionBuilder::new()
                .with_version(2)
                .add_output(TransactionOutput::payment(amount, &dummy_address1()))
                .add_output(TransactionOutput::payment(123, &dummy_address2()))
                .add_output(TransactionOutput::op_return(0, op_return.as_bytes()))
                .build();

            for transaction in permutations(transaction) {
                let payment_data = OpReturnPaymentData::<Test>::try_from(transaction).unwrap();
                assert_ok!(
                    payment_data.ensure_valid_payment_to(amount, dummy_address1(), Some(op_return)),
                    Some(dummy_address2())
                );
            }
        })
    }

    #[test]
    fn test_ensure_valid_payment_to_single_payment_succeeds() {
        run_test(|| {
            let amount = 12345;
            let op_return = H256::from_slice(&[5; 32]);
            let transaction = TransactionBuilder::new()
                .with_version(2)
                .add_output(TransactionOutput::payment(amount, &dummy_address1()))
                .add_output(TransactionOutput::op_return(0, op_return.as_bytes()))
                .build();

            for transaction in permutations(transaction) {
                let payment_data = OpReturnPaymentData::<Test>::try_from(transaction).unwrap();
                assert_ok!(
                    payment_data.ensure_valid_payment_to(amount, dummy_address1(), Some(op_return)),
                    None
                );
            }
        })
    }

    #[test]
    fn test_ensure_valid_payment_to_without_opreturn_check_succeeds() {
        run_test(|| {
            let amount = 12345;
            let op_return = H256::from_slice(&[5; 32]);
            let transaction = TransactionBuilder::new()
                .with_version(2)
                .add_output(TransactionOutput::payment(amount, &dummy_address1()))
                .add_output(TransactionOutput::payment(123, &dummy_address2()))
                .add_output(TransactionOutput::op_return(0, op_return.as_bytes()))
                .build();

            for transaction in permutations(transaction) {
                let payment_data = OpReturnPaymentData::<Test>::try_from(transaction).unwrap();
                assert_ok!(
                    payment_data.ensure_valid_payment_to(amount, dummy_address1(), None),
                    Some(dummy_address2())
                );
            }
        })
    }

    #[test]
    fn test_ensure_valid_payment_to_wrong_op_return_fails() {
        run_test(|| {
            let amount = 12345;
            let op_return = H256::from_slice(&[5; 32]);
            let transaction = TransactionBuilder::new()
                .with_version(2)
                .add_output(TransactionOutput::payment(amount, &dummy_address1()))
                .add_output(TransactionOutput::op_return(0, &[0; 32]))
                .build();

            for transaction in permutations(transaction) {
                let payment_data = OpReturnPaymentData::<Test>::try_from(transaction).unwrap();
                assert_err!(
                    payment_data.ensure_valid_payment_to(amount, dummy_address1(), Some(op_return)),
                    Error::<Test>::InvalidPayment
                );
            }
        })
    }

    #[test]
    fn test_ensure_valid_payment_to_wrong_recipient_fails() {
        run_test(|| {
            let amount = 12345;
            let op_return = H256::from_slice(&[5; 32]);
            let transaction = TransactionBuilder::new()
                .with_version(2)
                .add_output(TransactionOutput::payment(amount, &dummy_address1()))
                .add_output(TransactionOutput::payment(123, &dummy_address2()))
                .add_output(TransactionOutput::op_return(0, op_return.as_bytes()))
                .build();

            for transaction in permutations(transaction) {
                let payment_data = OpReturnPaymentData::<Test>::try_from(transaction).unwrap();
                assert_err!(
                    payment_data.ensure_valid_payment_to(amount, dummy_address3(), Some(op_return)),
                    Error::<Test>::InvalidPayment
                );
            }
        })
    }

    #[test]
    fn test_ensure_valid_payment_to_with_invalid_amount_fails() {
        run_test(|| {
            let amount = 12345;
            let op_return = H256::from_slice(&[5; 32]);
            let transaction = TransactionBuilder::new()
                .with_version(2)
                .add_output(TransactionOutput::payment(amount, &dummy_address1()))
                .add_output(TransactionOutput::payment(123, &dummy_address2()))
                .add_output(TransactionOutput::op_return(0, op_return.as_bytes()))
                .build();

            for transaction in permutations(transaction) {
                let payment_data = OpReturnPaymentData::<Test>::try_from(transaction).unwrap();
                assert_err!(
                    payment_data.ensure_valid_payment_to(amount - 1, dummy_address1(), Some(op_return)),
                    Error::<Test>::InvalidPaymentAmount
                );
            }
        })
    }
}

#[test]
fn test_check_and_do_reorg() {
    use crate::{Chains, ChainsIndex};
    use bitcoin::types::BlockChain;

    // data taken from testnet fork
    run_test(|| {
        Chains::<Test>::insert(0, 0);
        Chains::<Test>::insert(2, 7);

        ChainsIndex::<Test>::insert(
            0,
            BlockChain {
                chain_id: 0,
                start_height: 1_892_642,
                max_height: 1_897_317,
            },
        );

        ChainsIndex::<Test>::insert(
            2,
            BlockChain {
                chain_id: 2,
                start_height: 1_893_831,
                max_height: 1_893_831,
            },
        );

        ChainsIndex::<Test>::insert(
            4,
            BlockChain {
                chain_id: 4,
                start_height: 1_895_256,
                max_height: 1_895_256,
            },
        );

        ChainsIndex::<Test>::insert(
            6,
            BlockChain {
                chain_id: 6,
                start_height: 1_896_846,
                max_height: 1_896_846,
            },
        );

        ChainsIndex::<Test>::insert(
            7,
            BlockChain {
                chain_id: 7,
                start_height: 1_897_317,
                max_height: 1_897_910,
            },
        );

        BTCRelay::swap_main_blockchain.mock_safe(|_| MockResult::Return(Ok((Default::default(), Default::default()))));

        // we should skip empty `Chains`, this can occur if the
        // previous index is accidentally deleted
        assert_ok!(BTCRelay::reorganize_chains(&BlockChain {
            chain_id: 7,
            start_height: 1_897_317,
            max_height: 1_897_910,
        }));
    })
}

#[test]
pub fn test_has_request_expired() {
    run_test(|| {
        fn has_request_expired_after(period: u64, parachain_blocks: u64, bitcoin_blocks: u32) -> bool {
            let opentime = 1000;
            let btc_open_height = 100;

            BTCRelay::set_best_block_height(btc_open_height + bitcoin_blocks);
            Security::set_active_block_number(opentime + parachain_blocks);
            BTCRelay::has_request_expired(opentime, btc_open_height, period).unwrap()
        }

        // NOTE: mocks configure 100 parachain blocks per bitcoin block

        // boundary - just barely expired
        assert!(has_request_expired_after(600, 601, 7));
        // blockchain blocks not expired
        assert!(!has_request_expired_after(600, 600, 7));
        // bitcoin blocks not expired
        assert!(!has_request_expired_after(600, 601, 6));

        // test that the number of bitcoin blocks required for expiry is rounded up

        // boundary - just barely expired
        assert!(has_request_expired_after(601, 602, 8));
        // blockchain blocks not expired
        assert!(!has_request_expired_after(601, 601, 8));
        // bitcoin blocks not expired
        assert!(!has_request_expired_after(601, 602, 7));
    })
}

/// # Util functions

const SAMPLE_MERKLE_ROOT: &str = "1EE1FB90996CA1D5DCD12866BA9066458BF768641215933D7D8B3A10EF79D090";

fn sample_block_header() -> BlockHeader {
    let mut ret = BlockHeader {
        merkle_root: H256Le::from_hex_le(SAMPLE_MERKLE_ROOT),
        target: 123.into(),
        timestamp: 1601494682,
        version: 2,
        hash_prev_block: H256Le::from_hex_be("0000000000000000000e84948eaacb9b03382782f16f2d8a354de69f2e5a2a68"),
        hash: H256Le::from_hex_be("0000000000000000000000000000000000000000000000000000000000000000"),
        nonce: 0,
    };
    ret.update_hash().unwrap();
    ret
}

fn sample_unchecked_transaction() -> FullTransactionProof {
    let coinbase_tx_hex = "01000000010000000000000000000000000000000000000000000000000000000000000000ffffffff08044b6d0b1a020b02ffffffff0100f2052a01000000434104e8e37f1556b53b557405fc7924c861e640c8f99ebb3feb09ae69a84bea1f125940309beec02fb815ea5e68782c32da123b4585bc2f23731f1f1c62c9727dba9dac00000000";
    let raw_coinbase_tx = hex::decode(coinbase_tx_hex).unwrap();
    let coinbase_tx = parse_transaction(&raw_coinbase_tx).unwrap();

    let coinbase_proof_hex = "010000006fd2c5a8fac33dbe89bb2a2947a73eed2afc3b1d4f886942df08000000000000b152eca4364850f3424c7ac2b337d606c5ca0a3f96f1554f8db33d2f6f130bbed325a04e4b6d0b1a85790e6b0a00000005e1af205960ae338a37174b407ee71067c3cd7f04d48a5cec7e13f6eccb61dcbca314970cd7c647d1cc0a477e1a2122b98205b6924b73001b8dab20ee81c2f4f740213c81f059806fb8c1b91d0a7397a57156cfc3a17b71d095c244aafc1eb1158be15fc2ab11ef3e079568d43b2b09ed5a5690fb13ecb1032f7aab99238a1847e827331b1fe7a2689fbc23d14cd21317c699596cbca222182a489322ece1fa74021f00";
    let coinbase_raw_proof = hex::decode(coinbase_proof_hex).unwrap();
    let coinbase_proof = MerkleProof::parse(&coinbase_raw_proof).unwrap();

    // txid 8d30eb0f3e65b8d8a9f26f6f73fc5aafa5c0372f9bb38aa38dd4c9dd1933e090
    let user_tx_hex = "010000000168a59c95a89ed5e9af00e90a7823156b02b7811000c63170bb2440d8db6a1869000000008a473044022050c32cf6cd888178268701a636b189dc3f026ee3ebd230fd77018e54044aac77022055aa7fa73c524dd4f0be02694683a21eb03d5d2f2c519d7dc7110b742c417517014104aa5c77986a87b93b03d949013e629601b6dbdbd5fc09f3bef9263b64b3c38d79d443fafa2fbf422a203fe433adf6e071f3172a53747739ce72c640fe7e514981ffffffff0140420f00000000001976a91449cf380abdb86449efc694988bf0f447739f73cd88ac00000000";
    let raw_user_tx = hex::decode(user_tx_hex).unwrap();
    let user_tx = parse_transaction(&raw_user_tx).unwrap();

    let user_proof_hex = "010000006fd2c5a8fac33dbe89bb2a2947a73eed2afc3b1d4f886942df08000000000000b152eca4364850f3424c7ac2b337d606c5ca0a3f96f1554f8db33d2f6f130bbed325a04e4b6d0b1a85790e6b0a000000038d9d737b484e96eed701c4b3728aea80aa7f2a7f57125790ed9998f9050a1bef90e03319ddc9d48da38ab39b2f37c0a5af5afc736f6ff2a9d8b8653e0feb308d84251842a4c0f0e188e1c2bf643ec37a1402dd86a25a9ab5004633467d16e313013d";
    let user_raw_proof = hex::decode(user_proof_hex).unwrap();
    let user_proof = MerkleProof::parse(&user_raw_proof).unwrap();

    FullTransactionProof {
        coinbase_proof: PartialTransactionProof {
            transaction: coinbase_tx,
            tx_encoded_len: raw_coinbase_tx.len() as u32,
            merkle_proof: coinbase_proof,
        },
        user_tx_proof: PartialTransactionProof {
            transaction: user_tx,
            tx_encoded_len: raw_user_tx.len() as u32,
            merkle_proof: user_proof,
        },
    }
}

fn get_empty_block_chain_from_chain_id_and_height(chain_id: u32, start_height: u32, block_height: u32) -> BlockChain {
    let blockchain = BlockChain {
        chain_id,
        start_height,
        max_height: block_height,
    };

    blockchain
}

fn sample_raw_genesis_header() -> String {
    "01000000".to_owned() + "a7c3299ed2475e1d6ea5ed18d5bfe243224add249cce99c5c67cc9fb00000000601c73862a0a7238e376f497783c8ecca2cf61a4f002ec8898024230787f399cb575d949ffff001d3a5de07f"
}

fn sample_parsed_genesis_header(chain_id: u32, block_height: u32) -> RichBlockHeader<BlockNumber> {
    let genesis_header = BlockHeader::from_hex(sample_raw_genesis_header()).unwrap();
    RichBlockHeader::<BlockNumber> {
        block_header: genesis_header,
        block_height,
        chain_id,
        para_height: Default::default(),
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

fn sample_parsed_first_block(chain_id: u32, block_height: u32) -> RichBlockHeader<BlockNumber> {
    let block_header = BlockHeader::from_hex(sample_raw_first_header()).unwrap();
    RichBlockHeader::<BlockNumber> {
        block_header,
        block_height,
        chain_id,
        para_height: Default::default(),
    }
}

fn sample_retarget_interval_increase() -> [BlockHeader; 3] {
    // block height 66528
    let last_retarget_header = BlockHeader::from_hex("01000000".to_owned() + "4e8e5cf3c4e4b8f63a9cf88beb2dbaba1949182101ae4e5cf54ad100000000009f2a2344e8112b0d7bd8089414106ee5f17bb6cd64078883e1b661fa251aac6bed1d3c4cf4a3051c4dcd2b02").unwrap();
    // block height 66543
    let prev_block_header = BlockHeader::from_hex("01000000".to_owned()  + "1e321d88cb25946c4ca521eece3752803c021f9403fc4e0171203a0500000000317057f8b50414848a5a3a26d9eb8ace3d6f5495df456d0104dd1421159faf5029293c4cf4a3051c73199005").unwrap();
    // block height 68544
    let curr_header = BlockHeader::from_hex("01000000".to_owned() + "fb57c71ccd211b3de4ccc2e23b50a7cdb72aab91e60737b3a2bfdf030000000088a88ad9df68925e880e5d52b7e50cef225871c68b40a2cd0bca1084cd436037f388404cfd68011caeb1f801").unwrap();

    [last_retarget_header, prev_block_header, curr_header]
}

fn sample_retarget_interval_decrease() -> [BlockHeader; 3] {
    // block height 558432
    let last_retarget_header = BlockHeader::from_hex("00c0ff2f".to_owned() + "6550b5dae76559589e3e3e135237072b6bc498949da6280000000000000000005988783435f506d2ccfbadb484e56d6f1d5dfdd480650acae1e3b43d3464ea73caf13b5c33d62f171d508fdb").unwrap();
    // block height 560447
    let prev_block_header = BlockHeader::from_hex("00000020".to_owned()  + "d8e8e54ca5e33522b94fbba5de736efc55ff75e832cf2300000000000000000007b395f80858ee022c9c3c2f0f5cee4bd807039f0729b0559ae4326c3ba77d6b209f4e5c33d62f1746ee356d").unwrap();
    // block height 560448
    let curr_header = BlockHeader::from_hex("00000020".to_owned() + "6b05bd2c4a06b3d8503a033c2593396a25a79e1dcadb140000000000000000001b08df3d42cd9a38d8b66adf9dc5eb464f503633bd861085ffff723634531596a1a24e5c35683017bf67b72a").unwrap();

    [last_retarget_header, prev_block_header, curr_header]
}

fn sample_rich_tx_block_header(chain_id: u32, block_height: u32) -> RichBlockHeader<BlockNumber> {
    let block_header = BlockHeader::from_hex("0000003096cb3d93696c4f56c10da153963d35abf4692c07b2b3bf0702fb4cb32a8682211ee1fb90996ca1d5dcd12866ba9066458bf768641215933d7d8b3a10ef79d090e8a13a5effff7f2005000000".to_owned()).unwrap();
    RichBlockHeader::<BlockNumber> {
        block_header,
        block_height,
        chain_id,
        para_height: Default::default(),
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
        script: "6a20e5c17d15b8b1fa2811b7e6da66ffa5e1aaa05922c69068bf90cd585b95bb4675"
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
        source: TransactionInputSource::FromOutput(H256Le::from_bytes_le(&spent_output_txid), 0),
        script: hex::decode("16001443feac9ca9d20883126e30e962ca11fda07f808b".to_owned()).unwrap(),
        sequence: 4294967295,
        witness: vec![],
    };

    inputs.push(input);

    Transaction {
        version: 2,
        inputs,
        outputs: outputs.to_vec(),
        lock_at: LockTime::BlockHeight(203),
    }
}
