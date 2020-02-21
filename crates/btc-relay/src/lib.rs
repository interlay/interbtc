#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(test)]
mod tests;

/// For more guidance on FRAME pallets, see the example.
/// https://github.com/paritytech/substrate/blob/master/frame/example/src/lib.rs

/// # BTC-Relay implementation
/// This is the implementation of the BTC-Relay following the spec at:
/// https://interlay.gitlab.io/polkabtc-spec/btcrelay-spec/

// Substrate
use frame_support::{decl_module, decl_storage, decl_event, decl_error, dispatch::DispatchResult, ensure};
use {system::ensure_signed};
use node_primitives::{Moment};
use sp_core::{U256, H256, H160};
use sp_std::collections::btree_map::BTreeMap;

// Crates
use bitcoin::{BlockHeader, BlockChain, parse_block_header};

/// ## Configuration and Constants
/// The pallet's configuration trait.
/// For further reference, see: 
/// https://interlay.gitlab.io/polkabtc-spec/btcrelay-spec/spec/data-model.html
pub trait Trait: system::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    
}

// This pallet's storage items.
decl_storage! {
	trait Store for Module<T: Trait> as BTCRelay {
    /// ## Storage
        /// Store Bitcoin block headers
        BlockHeaders get(fn blockheader): map H256 => BlockHeader<U256, H256, Moment>;
        
        /// Vector of BlockChain elements
        Chains get(fn chain): Vec<BlockChain<U256, BTreeMap<U256, H256>>>;

        /// Store the index for each tracked blockchain
        ChainsIndex get(fn chainindex): map U256 => BlockChain<U256, BTreeMap<U256, H256>>;
        
        /// Store the current blockchain tip
        BestBlock get(fn bestblock): H256;

        /// Store the height of the best block
        BestBlockHeight get(fn bestblockheight): U256;

        /// Track existing BlockChain entries
        ChainCounter get(fn chaincounter): U256;
	}
}

// The pallet's dispatchable functions.
decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// Initializing events
		fn deposit_event() = default;
        
        // Initialize errors
        type Error = Error<T>;

        /// Difficulty Adjustment Interval
        const DIFFICULTY_ADJUSTMENT_INTERVAL: u16 = 2016;

        /// Target Timespan
        const TARGET_TIMESPAN: u64 = 1209600;
        
        /// Unrounded Maximum Target
        /// 0x00000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
        const UNROUNDED_MAX_TARGET: U256 = U256([0x00000000ffffffffu64, <u64>::max_value(), <u64>::max_value(), <u64>::max_value()]);

        fn initialize(
            origin,
            block_header_bytes: Vec<u8>,
            block_height: U256) 
            -> DispatchResult
        {
            let _ = ensure_signed(origin)?;
            
            // Check if BTC-Relay was already initialized
            ensure!(!<BestBlock>::exists(), Error::<T>::AlreadyInitialized);

            // Parse the block header bytes to extract the required info
            let mut block_header: BlockHeader<U256, H256, Moment> = parse_block_header(block_header_bytes);
            // Set the height of the block header
            block_header.block_height = Some(block_height);

            // get a new chain id
            let chain_id: U256 = Self::increment_chain_counter(); 
            // set the chain id as a reference in the block_header
            block_header.chain_ref = Some(chain_id);

            // construct the BlockChain struct
            let mut chain: BTreeMap<U256, H256> = BTreeMap::new();
            if let Some(_) = chain.insert(block_height, block_header.block_hash) {
                return Err(<Error<T>>::AlreadyInitialized.into())
            }
                    
            let blockchain = BlockChain {
                        chain_id: chain_id,
                        chain: chain,
                        max_height: block_height,
                        no_data: false,
                        invalid: false,
            };
            
            // Store a new BlockHeader struct in BlockHeaders
            <BlockHeaders>::insert(&block_header.block_hash, &block_header);

            // Store a pointer to BlockChain in ChainsIndex
            <ChainsIndex>::insert(&chain_id, &blockchain); 
  
            // Store the new BlockChain in Chains
            <Chains>::put(vec!(&blockchain));

            // Set BestBlock and BestBlockHeight to the submitted block
            <BestBlock>::put(&block_header.block_hash);
            <BestBlockHeight>::put(&block_height);

            // Emit a Initialized Event
            Self::deposit_event(RawEvent::Initialized(block_height, block_header.block_hash));
            
            Ok(())
        }
    
        fn store_block_header(origin, block_header_bytes: Vec<u8>)
        -> DispatchResult {
            let _ = ensure_signed(origin)?;
            // TODO: Check if BTC _Parachain is in shutdown state.

            // TODO: call verify_block_header
            
            // TODO: call the parse block header function
            // Parse the block header bytes to extract the required info
            let mut block_header: BlockHeader<U256, H256, Moment> = parse_block_header(block_header_bytes);
            
            // get the previous block header by the hash
            // let prev_block_header = Self::blockheader(block_header.hash_prev_block);
            
            // get the block chain of the previous header
            // let blockchain = Self::chainindex(prev_block_header.chain_ref);
            
            // check if the prev block is the highest block in the chain
            // extend the chain or store a new fork
            // match prev_block_header.block_height {
            //     blockchain.max_height => store_main_header(), 
            //     _ => store_fork_header(),
            // }
            
            


            Ok(())
        }
	}
}

/// Utility functions
impl<T: Trait> Module<T> {
    fn increment_chain_counter() -> U256 {
        let new_counter = <ChainCounter>::get() + 1;
        <ChainCounter>::put(new_counter);

        return new_counter;
    }
    fn store_main_header(block_header: &BlockHeader<U256, H256, Moment>) {
        <BlockHeaders>::insert(block_header.block_hash, block_header);
    }
}

decl_event! {
	pub enum Event<T> where 
        AccountId = <T as system::Trait>::AccountId,
        H256 = H256,
        U256 = U256,
    {
        Initialized(U256, H256),
        StoreMainChainHeader(U256, H256),
        StoreForkHeader(U256, U256, H256),
        ChainReorg(H256, U256, U256),
        VerifyTransaction(H256, U256, U256),
        ValidateTransaction(H256, U256, H160, H256),
		SomethingStored(u32, AccountId),
	}
}

// TODO: how to include message in errors?
decl_error! {
    pub enum Error for Module<T: Trait> {
        AlreadyInitialized,
        NotMainChain,
        ForkPrevBlock,
        NotFork,
        InvalidForkId,
        InvalidHeaderSize,
        DuplicateBlock,
        PrevBlock,
        LowDiff,
        DiffTargetHeader,
        MalformedTxid,
        Confirmations,
        InvalidMerkleProof,
        ForkIdNotFound,
        Partial,
        Invalid,
        Shutdown,
        InvalidTxid,
        InsufficientValue,
        TxFormat,
        WrongRecipient,
        InvalidOpreturn,
        InvalidTxVersion,
        NotOpReturn,
    }
}

