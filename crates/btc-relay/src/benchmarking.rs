use super::*;
use crate::Pallet as BtcRelay;
use bitcoin::types::{Block, BlockBuilder, H256Le, TransactionBuilder, TransactionInputBuilder};
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use sp_core::{H160, U256};
use sp_std::prelude::*;

fn add_block<T: Config>(caller: T::AccountId, parent_hash: H256Le, seed: usize) -> Block {
    let block = BlockBuilder::new()
        .with_previous_hash(parent_hash)
        .with_version(4)
        .with_coinbase(&BtcAddress::P2PKH(H160::from([0; 20])), 50, 3)
        .add_transaction(
            TransactionBuilder::new()
                .with_version(2)
                .add_input(TransactionInputBuilder::new().with_script(&vec![0; seed]).build())
                .build(),
        )
        .with_timestamp(u32::MAX)
        .mine(U256::from(2).pow(254.into()))
        .unwrap();
    BtcRelay::<T>::_store_block_header(&caller, block.header, u32::MAX).unwrap();
    block
}

#[benchmarks]
pub mod benchmarks {
    use super::*;

    #[benchmark]
    pub fn initialize() {
        let height = 0u32;
        let caller = whitelisted_caller();

        let block = BlockBuilder::new()
            .with_version(4)
            .with_coinbase(&BtcAddress::P2PKH(H160::from([0; 20])), 50, 3)
            .with_timestamp(u32::MAX)
            .mine(U256::from(2).pow(254.into()))
            .unwrap();

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), block.header, height);
    }

    #[benchmark]
    pub fn store_block_header(r: Linear<1, 6>) {
        let caller: T::AccountId = whitelisted_caller();
        let address = BtcAddress::P2PKH(H160::from([0; 20]));
        let stable_conf = BtcRelay::<T>::get_stable_transaction_confirmations();

        let init_block = BlockBuilder::new()
            .with_version(4)
            .with_coinbase(&address, 50, 3)
            .with_timestamp(1588813835)
            .mine(U256::from(2).pow(254.into()))
            .unwrap();

        let mut init_block_hash = init_block.header.hash;
        BtcRelay::<T>::_initialize(caller.clone(), init_block.header, 0).unwrap();

        for i in 0..r {
            let mut block_hash = init_block_hash;
            for _ in 0..stable_conf {
                let block = add_block::<T>(caller.clone(), block_hash, i as usize);
                block_hash = block.header.hash;
            }
        }

        // new fork up to block before swapping the main chain
        for _ in 0..(BestBlockHeight::<T>::get() + stable_conf - 1) {
            let block = add_block::<T>(caller.clone(), init_block_hash, r as usize);
            init_block_hash = block.header.hash;
        }

        let prev_best_block_height = BestBlockHeight::<T>::get();
        assert_eq!(prev_best_block_height, stable_conf);
        assert_eq!(ChainsIndex::<T>::iter().collect::<Vec<_>>().len(), (r + 1) as usize);

        // we can benchmark the worst-case complexity for swapping
        // since we know how many blocks are required
        let block = BlockBuilder::new()
            .with_previous_hash(init_block_hash)
            .with_version(4)
            .with_coinbase(&address, 50, 3)
            .with_timestamp(u32::MAX)
            .mine(U256::from(2).pow(254.into()))
            .unwrap();

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), block.header, u32::MAX);

        // make sure reorg occurred
        assert_eq!(BestBlockHeight::<T>::get(), prev_best_block_height + stable_conf);
        let rich_header = BtcRelay::<T>::get_block_header_from_hash(block.header.hash).unwrap();
        assert_eq!(rich_header.chain_id, MAIN_CHAIN_ID);
    }

    impl_benchmark_test_suite!(BtcRelay, crate::mock::ExtBuilder::build(), crate::mock::Test);
}
