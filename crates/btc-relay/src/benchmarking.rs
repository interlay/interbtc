use super::*;
use crate::Pallet as BtcRelay;
use bitcoin::types::{Block, BlockBuilder, H256Le, TransactionBuilder, TransactionInputBuilder};
use frame_benchmarking::v2::*;
use frame_support::assert_ok;
use frame_system::RawOrigin;
use sp_core::{H160, U256};
use sp_std::prelude::*;

const SECURE_BITCOIN_CONFIRMATIONS: u32 = 6;

fn initialize_relay<T: Config>(caller: T::AccountId) -> Block {
    let init_block = BlockBuilder::new()
        .with_version(4)
        .with_coinbase(&BtcAddress::P2PKH(H160::from([0; 20])), 50, 3)
        .with_timestamp(u32::MAX)
        .mine(U256::from(2).pow(254.into()))
        .unwrap();
    assert_ok!(BtcRelay::<T>::_initialize(caller, init_block.header, 0));
    init_block
}

fn new_block<T: Config>(parent_hash: H256Le, seed: usize) -> Block {
    BlockBuilder::new()
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
        .unwrap()
}

fn add_new_block_to_relay<T: Config>(caller: T::AccountId, parent_hash: H256Le, seed: usize) -> Block {
    let block = new_block::<T>(parent_hash, seed);
    assert_ok!(BtcRelay::<T>::_store_block_header(&caller, block.header));
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
    pub fn store_block_header() {
        let caller: T::AccountId = whitelisted_caller();

        let init_block = initialize_relay::<T>(caller.clone());
        let init_block_hash = init_block.header.hash;
        let block = new_block::<T>(init_block_hash, 0);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), block.header, u32::MAX);

        // make sure block is stored
        let rich_header = BtcRelay::<T>::get_block_header_from_hash(block.header.hash).unwrap();
        assert_eq!(rich_header.chain_id, MAIN_CHAIN_ID);
    }

    #[benchmark]
    pub fn store_block_header_new_fork_sorted(f: Linear<1, 6>) {
        let caller: T::AccountId = whitelisted_caller();

        let init_block = initialize_relay::<T>(caller.clone());
        let init_block_hash = init_block.header.hash;
        add_new_block_to_relay::<T>(caller.clone(), init_block_hash, 0);

        for i in 1..f {
            add_new_block_to_relay::<T>(caller.clone(), init_block_hash, i as usize);
        }

        let block = new_block::<T>(init_block_hash, f as usize);

        #[extrinsic_call]
        store_block_header(RawOrigin::Signed(caller), block.header, u32::MAX);

        // make sure fork is stored
        let rich_header = BtcRelay::<T>::get_block_header_from_hash(block.header.hash).unwrap();
        assert_eq!(rich_header.chain_id, MAIN_CHAIN_ID + f);
    }

    #[benchmark]
    pub fn store_block_header_new_fork_unsorted(f: Linear<1, 6>) {
        let caller: T::AccountId = whitelisted_caller();

        let init_block = initialize_relay::<T>(caller.clone());
        let init_block_hash = init_block.header.hash;
        let block_1 = add_new_block_to_relay::<T>(caller.clone(), init_block_hash, 0);

        for i in 1..f {
            add_new_block_to_relay::<T>(caller.clone(), init_block_hash, i as usize);
        }

        let _block_2_1 = add_new_block_to_relay::<T>(caller.clone(), block_1.header.hash, (f + 1) as usize);
        let block_2_2 = new_block::<T>(block_1.header.hash, (f + 2) as usize);

        #[extrinsic_call]
        store_block_header(RawOrigin::Signed(caller), block_2_2.header, u32::MAX);

        // make sure fork is stored
        let rich_header = BtcRelay::<T>::get_block_header_from_hash(block_2_2.header.hash).unwrap();
        let fork_position = BtcRelay::<T>::get_chain_position_from_chain_id(rich_header.chain_id).unwrap();
        assert_eq!(fork_position, MAIN_CHAIN_ID + 1);
    }

    #[benchmark]
    pub fn store_block_header_reorganize_chains(f: Linear<3, 6>) {
        let caller: T::AccountId = whitelisted_caller();
        StableBitcoinConfirmations::<T>::put(SECURE_BITCOIN_CONFIRMATIONS);

        let init_block = initialize_relay::<T>(caller.clone());
        let mut init_block_hash = init_block.header.hash;

        for i in 1..f {
            let mut block_hash = init_block_hash;
            for _ in 0..SECURE_BITCOIN_CONFIRMATIONS {
                let block = add_new_block_to_relay::<T>(caller.clone(), block_hash, i as usize);
                block_hash = block.header.hash;
            }
        }

        // new fork up to block before swapping the main chain
        for _ in 1..(BestBlockHeight::<T>::get() + SECURE_BITCOIN_CONFIRMATIONS) {
            let block = add_new_block_to_relay::<T>(caller.clone(), init_block_hash, f as usize);
            init_block_hash = block.header.hash;
        }

        let prev_best_block_height = BestBlockHeight::<T>::get();
        assert_eq!(prev_best_block_height, SECURE_BITCOIN_CONFIRMATIONS);
        assert_eq!(ChainsIndex::<T>::iter().collect::<Vec<_>>().len(), f as usize);

        // we can benchmark the worst-case complexity for swapping
        // since we know how many blocks are required
        let block = new_block::<T>(init_block_hash, f as usize);

        #[extrinsic_call]
        store_block_header(RawOrigin::Signed(caller), block.header, u32::MAX);

        // make sure reorg occurred
        assert_eq!(
            BestBlockHeight::<T>::get(),
            prev_best_block_height + SECURE_BITCOIN_CONFIRMATIONS
        );
        let rich_header = BtcRelay::<T>::get_block_header_from_hash(block.header.hash).unwrap();
        assert_eq!(rich_header.chain_id, MAIN_CHAIN_ID);
    }

    impl_benchmark_test_suite!(BtcRelay, crate::mock::ExtBuilder::build(), crate::mock::Test);
}
