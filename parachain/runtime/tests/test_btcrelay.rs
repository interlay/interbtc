mod bitcoin_data;
mod mock;

use bitcoin_data::get_bitcoin_testdata;
use mock::*;

#[test]
fn integration_test_submit_block_headers_and_verify_transaction_inclusion() {
    ExtBuilder::build().execute_with(|| {
        // load blocks with transactions
        let test_data = get_bitcoin_testdata();

        // store all block headers. parachain_genesis is the first block
        // known in the parachain. Any block before will be rejected
        let parachain_genesis = test_data[0].get_raw_header();
        assert_ok!(BTCRelayCall::initialize(parachain_genesis, block.height)
            .dispatch(origin_of(account_of(ALICE))));
        for block in test_data.iter().skip(1) {
            assert_ok!(BTCRelayCall::store_block_header(block.get_raw())
                .dispatch(origin_of(account_of(ALICE))));

            assert_store_main_chain_header_event(block.height, H256Le::from_hex_be(&block.hash));
        }
        // verify all transaction that have enough confirmations
        let current_height = btc_relay::Module::<Runtime>::get_best_block_height();
        for block in test_data.iter() {
            if block.height < current_height - CONFIRMATIONS {
                for tx in &block.test_txs {
                    let txid = H256Le::from_hex_be(&tx.txid);
                    let raw_merkle_proof =
                        hex::decode(&tx.raw_merkle_proof).expect("Error parsing merkle proof");
                    assert_ok!(BTCRelayCall::verify_transaction_inclusion(
                        txid,
                        block.height,
                        raw_merkle_proof,
                        CONFIRMATIONS,
                        false
                    )
                    .dispatch(origin_of(account_of(ALICE))));
                }
            }
        }
    })
}
