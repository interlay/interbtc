mod bitcoin_data;
mod mock;

use bitcoin_data::get_bitcoin_testdata;
use mock::{assert_eq, *};

type BTCRelayError = btc_relay::Error<Runtime>;

#[test]
fn integration_test_submit_block_headers_and_verify_transaction_inclusion() {
    ExtBuilder::build().execute_without_relay_init(|| {
        // load blocks with transactions
        let test_data = get_bitcoin_testdata();

        SecurityPallet::set_active_block_number(1);

        // store all block headers. parachain_genesis is the first block
        // known in the parachain. Any block before will be rejected
        let parachain_genesis_height = test_data[0].height;
        let parachain_genesis_header = test_data[0].get_raw_header();

        assert_ok!(Call::Relay(RelayCall::initialize {
            raw_block_header: parachain_genesis_header,
            block_height: parachain_genesis_height
        })
        .dispatch(origin_of(account_of(ALICE))));
        for block in test_data.iter().skip(1) {
            assert_ok!(Call::Relay(RelayCall::store_block_header {
                raw_block_header: block.get_raw_header()
            })
            .dispatch(origin_of(account_of(ALICE))));

            assert_store_main_chain_header_event(block.height, block.get_block_hash(), account_of(ALICE));
        }
        SecurityPallet::set_active_block_number(1 + CONFIRMATIONS);
        // verify all transaction
        let current_height = btc_relay::Pallet::<Runtime>::get_best_block_height();
        for block in test_data.iter() {
            for tx in &block.test_txs {
                let txid = tx.get_txid();
                let raw_merkle_proof = tx.get_raw_merkle_proof();
                if block.height <= current_height - CONFIRMATIONS + 1 {
                    assert_ok!(Call::BTCRelay(BTCRelayCall::verify_transaction_inclusion {
                        tx_id: txid,
                        raw_merkle_proof: raw_merkle_proof,
                        confirmations: None,
                    })
                    .dispatch(origin_of(account_of(ALICE))));
                } else {
                    // expect to fail due to insufficient confirmations
                    assert_noop!(
                        Call::BTCRelay(BTCRelayCall::verify_transaction_inclusion {
                            tx_id: txid,
                            raw_merkle_proof: raw_merkle_proof,
                            confirmations: None,
                        })
                        .dispatch(origin_of(account_of(ALICE))),
                        BTCRelayError::BitcoinConfirmations
                    );
                }
            }
        }
    })
}

#[test]
fn integration_test_btc_relay_with_parachain_shutdown_fails() {
    ExtBuilder::build().execute_with(|| {
        SecurityPallet::set_status(StatusCode::Shutdown);

        assert_noop!(
            Call::BTCRelay(BTCRelayCall::verify_and_validate_transaction {
                raw_merkle_proof: Default::default(),
                confirmations: Default::default(),
                raw_tx: Default::default(),
                expected_btc: Default::default(),
                recipient_btc_address: Default::default(),
                op_return_id: Default::default()
            })
            .dispatch(origin_of(account_of(ALICE))),
            SecurityError::ParachainShutdown
        );
        assert_noop!(
            Call::BTCRelay(BTCRelayCall::verify_transaction_inclusion {
                tx_id: Default::default(),
                raw_merkle_proof: Default::default(),
                confirmations: Default::default()
            })
            .dispatch(origin_of(account_of(ALICE))),
            SecurityError::ParachainShutdown
        );
        assert_noop!(
            Call::BTCRelay(BTCRelayCall::validate_transaction {
                raw_tx: Default::default(),
                expected_btc: Default::default(),
                recipient_btc_address: Default::default(),
                op_return_id: Default::default()
            })
            .dispatch(origin_of(account_of(ALICE))),
            SecurityError::ParachainShutdown
        );
    })
}
