use super::*;
use crate::Module as BtcRelay;
use bitcoin::formatter::Formattable;
use bitcoin::types::{
    Address, Block, BlockBuilder, RawBlockHeader, Transaction, TransactionBuilder,
    TransactionInputBuilder, TransactionOutput,
};
use frame_benchmarking::{account, benchmarks};
use frame_system::Module as System;
use frame_system::RawOrigin;
use sp_core::{H256, U256};
use sp_std::prelude::*;

fn mine_genesis<T: Trait>(address: &Address, height: u32) -> Block {
    let block = BlockBuilder::new()
        .with_version(2)
        .with_coinbase(address, 50, 3)
        .with_timestamp(1588813835)
        .mine(U256::from(2).pow(254.into()));

    let block_header = RawBlockHeader::from_bytes(&block.header.format()).unwrap();
    BtcRelay::<T>::_initialize(block_header, height).unwrap();

    block
}

fn mine_block_with_one_tx<T: Trait>(
    prev: Block,
    address: &Address,
    value: i32,
    op_return: &[u8],
) -> (Block, Transaction) {
    let prev_block_hash = prev.header.hash();

    let transaction = TransactionBuilder::new()
        .with_version(2)
        .add_input(
            TransactionInputBuilder::new()
                .with_coinbase(false)
                .with_previous_hash(prev.transactions[0].hash())
                .build(),
        )
        .add_output(TransactionOutput::p2pkh(value.into(), address))
        .add_output(TransactionOutput::op_return(0, op_return))
        .build();

    let block = BlockBuilder::new()
        .with_previous_hash(prev_block_hash)
        .with_version(2)
        .with_coinbase(address, 50, 3)
        .with_timestamp(1588813835)
        .add_transaction(transaction.clone())
        .mine(U256::from(2).pow(254.into()));

    let block_header = RawBlockHeader::from_bytes(&block.header.format()).unwrap();
    BtcRelay::<T>::_store_block_header(block_header).unwrap();

    (block, transaction)
}

benchmarks! {
    _ {}

    initialize {
        let height = 0;
        let origin: T::AccountId = account("Origin", 0, 0);

        let address = Address::from([0; 20]);
        let block = BlockBuilder::new()
            .with_version(2)
            .with_coinbase(&address, 50, 3)
            .with_timestamp(1588813835)
            .mine(U256::from(2).pow(254.into()));
        let block_header = RawBlockHeader::from_bytes(&block.header.format()).unwrap();

    }: _(RawOrigin::Signed(origin), block_header, height.into())
    verify {
        assert_eq!(BestBlockHeight::get(), height);
    }

    store_block_header {
        let origin: T::AccountId = account("Origin", 0, 0);

        let address = Address::from([0; 20]);

        let height = 0;
        let confirmations = 6;

        let init_block = BlockBuilder::new()
            .with_version(2)
            .with_coinbase(&address, 50, 3)
            .with_timestamp(1588813835)
            .mine(U256::from(2).pow(254.into()));

        let init_block_hash = init_block.header.hash();
        let raw_block_header = RawBlockHeader::from_bytes(&init_block.header.format())
            .expect("could not serialize block header");

        BtcRelay::<T>::_initialize(raw_block_header, height).unwrap();

        let block = BlockBuilder::new()
            .with_previous_hash(init_block_hash)
            .with_version(2)
            .with_coinbase(&address, 50, 3)
            .with_timestamp(1588814835)
            .mine(U256::from(2).pow(254.into()));

        let raw_block_header = RawBlockHeader::from_bytes(&block.header.format())
            .expect("could not serialize block header");

    }: _(RawOrigin::Signed(origin), raw_block_header)

    // TODO: parameterize
    store_block_headers {
        let origin: T::AccountId = account("Origin", 0, 0);

        let address = Address::from([0; 20]);

        let height = 0;
        let confirmations = 6;

        let init_block = BlockBuilder::new()
            .with_version(2)
            .with_coinbase(&address, 50, 3)
            .with_timestamp(1588813835)
            .mine(U256::from(2).pow(254.into()));

        let block_hash_0 = init_block.header.hash();
        let raw_block_header_0 = RawBlockHeader::from_bytes(&init_block.header.format())
            .expect("could not serialize block header");

        BtcRelay::<T>::_initialize(raw_block_header_0, height).unwrap();

        let block = BlockBuilder::new()
            .with_previous_hash(block_hash_0)
            .with_version(2)
            .with_coinbase(&address, 50, 3)
            .with_timestamp(1588814835)
            .mine(U256::from(2).pow(254.into()));

        let block_hash_1 = block.header.hash();
        let raw_block_header_1 = RawBlockHeader::from_bytes(&block.header.format())
            .expect("could not serialize block header");

        let block = BlockBuilder::new()
            .with_previous_hash(block_hash_1)
            .with_version(2)
            .with_coinbase(&address, 50, 3)
            .with_timestamp(1588814835)
            .mine(U256::from(2).pow(254.into()));

        let raw_block_header_2 = RawBlockHeader::from_bytes(&block.header.format())
            .expect("could not serialize block header");


    }: _(RawOrigin::Signed(origin), vec![raw_block_header_1, raw_block_header_2])

    verify_and_validate_transaction {
        let origin: T::AccountId = account("Origin", 0, 0);

        let address = Address::from([0; 20]);
        let mut height = 0;
        let block = mine_genesis::<T>(&address, height);
        height += 1;

        let value = 0;
        let op_return = H256::zero().as_bytes().to_vec();
        let (block, transaction) = mine_block_with_one_tx::<T>(block, &address, value, &op_return);

        let tx_id = transaction.tx_id();
        let tx_block_height = height;
        let proof = block.merkle_proof(&vec![tx_id]).format();
        let raw_tx = transaction.format_with(true);

        System::<T>::set_block_number(100.into());

    }: _(RawOrigin::Signed(origin), tx_id, proof, 0, true, raw_tx, value.into(), address.as_bytes().to_vec(), op_return)

    verify_transaction_inclusion {
        let origin: T::AccountId = account("Origin", 0, 0);

        let address = Address::from([0; 20]);
        let mut height = 0;
        let block = mine_genesis::<T>(&address, height);
        height += 1;

        let value = 0;
        let op_return = H256::zero().as_bytes().to_vec();
        let (block, transaction) = mine_block_with_one_tx::<T>(block, &address, value, &op_return);

        let tx_id = transaction.tx_id();
        let tx_block_height = height;
        let proof = block.merkle_proof(&vec![tx_id]).format();

        System::<T>::set_block_number(100.into());

    }: _(RawOrigin::Signed(origin), tx_id, proof, 0, true)

    validate_transaction {
        let origin: T::AccountId = account("Origin", 0, 0);

        let address = Address::from([0; 20]);
        let value = 0;
        let op_return = H256::zero().as_bytes().to_vec();

        let block = mine_genesis::<T>(&address, 0);
        let (_, transaction) = mine_block_with_one_tx::<T>(block, &address, value, &op_return);

        let raw_tx = transaction.format_with(true);

    }: _(RawOrigin::Signed(origin), raw_tx, value.into(), address.as_bytes().to_vec(), op_return)

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{ExtBuilder, Test};
    use frame_support::assert_ok;

    #[test]
    fn test_benchmarks() {
        ExtBuilder::build().execute_with(|| {
            assert_ok!(test_benchmark_initialize::<Test>());
            assert_ok!(test_benchmark_store_block_header::<Test>());
            assert_ok!(test_benchmark_store_block_headers::<Test>());
            assert_ok!(test_benchmark_verify_and_validate_transaction::<Test>());
            assert_ok!(test_benchmark_verify_transaction_inclusion::<Test>());
            assert_ok!(test_benchmark_validate_transaction::<Test>());
        });
    }
}
