use super::*;
use crate::Pallet as BtcRelay;
use bitcoin::types::{
    Block, BlockBuilder, Transaction, TransactionBuilder, TransactionInputBuilder, TransactionInputSource,
    TransactionOutput,
};
use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite};
use frame_system::RawOrigin;
use security::Pallet as Security;
use sp_core::{H160, H256, U256};
use sp_std::prelude::*;

fn mine_genesis<T: Config>(account_id: T::AccountId, address: &BtcAddress, height: u32) -> Block {
    let block = BlockBuilder::new()
        .with_version(4)
        .with_coinbase(address, 50, 3)
        .with_timestamp(1588813835)
        .mine(U256::from(2).pow(254.into()))
        .unwrap();

    BtcRelay::<T>::_initialize(account_id, block.header, height).unwrap();

    block
}

fn mine_block_with_one_tx<T: Config>(
    account_id: T::AccountId,
    prev: Block,
    address: &BtcAddress,
    value: i32,
    op_return: &[u8],
) -> (Block, Transaction) {
    let prev_block_hash = prev.header.hash;

    let transaction = TransactionBuilder::new()
        .with_version(2)
        .add_input(
            TransactionInputBuilder::new()
                .with_source(TransactionInputSource::FromOutput(prev.transactions[0].hash(), 0))
                .with_script(&[
                    0, 71, 48, 68, 2, 32, 91, 128, 41, 150, 96, 53, 187, 63, 230, 129, 53, 234, 210, 186, 21, 187, 98,
                    38, 255, 112, 30, 27, 228, 29, 132, 140, 155, 62, 123, 216, 232, 168, 2, 32, 72, 126, 179, 207,
                    142, 8, 99, 8, 32, 78, 244, 166, 106, 160, 207, 227, 61, 210, 172, 234, 234, 93, 59, 159, 79, 12,
                    194, 240, 212, 3, 120, 50, 1, 71, 81, 33, 3, 113, 209, 131, 177, 9, 29, 242, 229, 15, 217, 247,
                    165, 78, 111, 80, 79, 50, 200, 117, 80, 30, 233, 210, 167, 133, 175, 62, 253, 134, 127, 212, 51,
                    33, 2, 128, 200, 184, 235, 148, 25, 43, 34, 28, 173, 55, 54, 189, 164, 187, 243, 243, 152, 7, 84,
                    210, 85, 156, 238, 77, 97, 188, 240, 162, 197, 105, 62, 82, 174,
                ])
                .build(),
        )
        .add_output(TransactionOutput::payment(value.into(), address))
        .add_output(TransactionOutput::op_return(0, op_return))
        .build();

    let block = BlockBuilder::new()
        .with_previous_hash(prev_block_hash)
        .with_version(4)
        .with_coinbase(address, 50, 3)
        .with_timestamp(1588813835)
        .add_transaction(transaction.clone())
        .mine(U256::from(2).pow(254.into()))
        .unwrap();

    BtcRelay::<T>::_store_block_header(&account_id, block.header).unwrap();

    (block, transaction)
}

benchmarks! {
    initialize {
        let height = 0u32;
        let origin: T::AccountId = account("Origin", 0, 0);
        let stake = 100u32;

        let address = BtcAddress::P2PKH(H160::from([0; 20]));
        let block = BlockBuilder::new()
            .with_version(4)
            .with_coinbase(&address, 50, 3)
            .with_timestamp(1588813835)
            .mine(U256::from(2).pow(254.into())).unwrap();

    }: _(RawOrigin::Signed(origin), block.header, height)

    store_block_header {
        let origin: T::AccountId = account("Origin", 0, 0);

        let address = BtcAddress::P2PKH(H160::from([0; 20]));
        let height = 0;
        let stake = 100u32;

        let init_block = BlockBuilder::new()
            .with_version(4)
            .with_coinbase(&address, 50, 3)
            .with_timestamp(1588813835)
            .mine(U256::from(2).pow(254.into())).unwrap();

        let init_block_hash = init_block.header.hash;

        BtcRelay::<T>::_initialize(origin.clone(), init_block.header, height).unwrap();

        let block = BlockBuilder::new()
            .with_previous_hash(init_block_hash)
            .with_version(4)
            .with_coinbase(&address, 50, 3)
            .with_timestamp(1588814835)
            .mine(U256::from(2).pow(254.into())).unwrap();

    }: _(RawOrigin::Signed(origin), block.header)

    verify_and_validate_transaction {
        let origin: T::AccountId = account("Origin", 0, 0);

        let address = BtcAddress::P2PKH(H160::from([0; 20]));

        let height = 0;
        let block = mine_genesis::<T>(origin.clone(), &address, height);

        let value = 0;
        let op_return = H256::zero().as_bytes().to_vec();
        let (block, transaction) = mine_block_with_one_tx::<T>(origin.clone(), block, &address, value, &op_return);

        let tx_id = transaction.tx_id();
        let merkle_proof = block.merkle_proof(&[tx_id]).unwrap();

        Security::<T>::set_active_block_number(100u32.into());

    }: _(RawOrigin::Signed(origin), merkle_proof, Some(0), transaction, value.into(), address, Some(H256::zero()))

    verify_transaction_inclusion {
        let origin: T::AccountId = account("Origin", 0, 0);

        let address = BtcAddress::P2PKH(H160::from([0; 20]));

        let height = 0;
        let block = mine_genesis::<T>(origin.clone(), &address, height);

        let value = 0;
        let op_return = H256::zero().as_bytes().to_vec();
        let (block, transaction) = mine_block_with_one_tx::<T>(origin.clone(), block, &address, value, &op_return);

        let tx_id = transaction.tx_id();
        let tx_block_height = height;
        let merkle_proof = block.merkle_proof(&[tx_id]).unwrap();

        Security::<T>::set_active_block_number(100u32.into());

    }: _(RawOrigin::Signed(origin), tx_id, merkle_proof, Some(0))

    validate_transaction {
        let origin: T::AccountId = account("Origin", 0, 0);

        let address = BtcAddress::P2PKH(H160::from([0; 20]));
        let value = 0;
        let op_return = H256::zero().as_bytes().to_vec();

        let block = mine_genesis::<T>(origin.clone(), &address, 0);
        let (_, transaction) = mine_block_with_one_tx::<T>(origin.clone(), block, &address, value, &op_return);

    }: _(RawOrigin::Signed(origin), transaction, value.into(), address, Some(H256::from_slice(&op_return)))

}

impl_benchmark_test_suite!(BtcRelay, crate::mock::ExtBuilder::build(), crate::mock::Test);
