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
use bitcoin::types::{RichBlockHeader, BlockChain};
use bitcoin::parser::{header_from_bytes, parse_block_header};
use security::{ErrorCodes};

/// ## Configuration and Constants
/// The pallet's configuration trait.
/// For further reference, see: 
/// https://interlay.gitlab.io/polkabtc-spec/btcrelay-spec/spec/data-model.html
pub trait Trait: system::Trait {
    /// The overarching event type.
    type Event: From<Event> + Into<<Self as system::Trait>::Event>;
    
}

/// Difficulty Adjustment Interval
pub const DIFFICULTY_ADJUSTMENT_INTERVAL: u16 = 2016;

/// Target Timespan
pub const TARGET_TIMESPAN: u64 = 1209600;

/// Unrounded Maximum Target
/// 0x00000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
pub const UNROUNDED_MAX_TARGET: U256 = U256([0x00000000ffffffffu64, <u64>::max_value(), <u64>::max_value(), <u64>::max_value()]);

/// Main chain id
pub const MAIN_CHAIN_ID: U256 = U256([0u64, 0u64, 0u64, 0u64]);

// This pallet's storage items.
decl_storage! {
	trait Store for Module<T: Trait> as BTCRelay {
    /// ## Storage
        /// Store Bitcoin block headers
        BlockHeaders get(fn blockheader): map H256 => RichBlockHeader<H256, U256, Moment>;
        
        /// Sorted mapping of BlockChain elements with reference to ChainsIndex
        Chains get(fn chain): map U256 => U256;

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
            let raw_block_header = header_from_bytes(&block_header_bytes);
            let basic_block_header = parse_block_header(raw_block_header);
            let block_header_hash = basic_block_header.block_hash; 

            // get a new chain id
            let chain_id: U256 = Self::increment_chain_counter(); 
            
            // Create rich block header
            let block_header = RichBlockHeader {
                block_header: basic_block_header,
                block_height: block_height,
                chain_ref: chain_id
            };

            // construct the BlockChain struct
            let blockchain = Self::create_chain(&chain_id, &block_height, &block_header_hash)
                .map_err(|_e| <Error<T>>::AlreadyInitialized)?;
            
            // Store a new BlockHeader struct in BlockHeaders
            <BlockHeaders>::insert(&block_header_hash, &block_header);

            // Store a pointer to BlockChain in ChainsIndex
            <ChainsIndex>::insert(&chain_id, &blockchain); 
  
            // Store the new BlockChain in Chains
            <Chains>::insert(MAIN_CHAIN_ID, &chain_id);

            // Set BestBlock and BestBlockHeight to the submitted block
            <BestBlock>::put(&block_header_hash);
            <BestBlockHeight>::put(&block_height);

            // Emit a Initialized Event
            Self::deposit_event(Event::Initialized(block_height, block_header_hash));
            
            Ok(())
        }
    
        fn store_block_header(origin, block_header_bytes: Vec<u8>)
        -> DispatchResult {
            let _ = ensure_signed(origin)?;
            // TODO: Check if BTC _Parachain is in shutdown state.

            // Parse the block header bytes to extract the required info
            let raw_block_header = header_from_bytes(&block_header_bytes);
            let basic_block_header = parse_block_header(raw_block_header);
            let block_header_hash = basic_block_header.block_hash; 
           
            // TODO: call verify_block_header
            

            // get the block header of the previous block
            ensure!(<BlockHeaders>::exists(basic_block_header.hash_prev_block), Error::<T>::PrevBlock);
            let prev_header = Self::blockheader(basic_block_header.hash_prev_block);

            // get the block chain of the previous header
            let prev_blockchain = Self::chainindex(prev_header.chain_ref);
              
            // Update the current block header
            // check if the prev block is the highest block in the chain
            // load the previous block header block height
            let prev_block_height = prev_header.block_height;
            
            // compare the prev header block height with the max height of the chain
            let current_chain_id = match prev_blockchain.max_height {
                // if the max height of that chain is the prev header, extend on this chain
                prev_block_height => prev_header.chain_ref,
                // if not, create a new chain id
                _ => Self::increment_chain_counter(),
            };
            
            // update the current block header structure with height and chain ref
            // Set the height of the block header
            let current_block_height = prev_block_height
                .checked_add(U256::from("1"))
                .ok_or("Overflow on block height")?;
            
            // Create rich block header
            let block_header = RichBlockHeader {
                block_header: basic_block_header,
                block_height: current_block_height,
                chain_ref: current_chain_id
            };
            
            // Update the blockchain
            // check if we create a new blockchain or extend the existing one
            let blockchain = match current_chain_id {
                // extend the current chain
                prev_chain_id => Self::extend_chain(
                    &current_block_height, &block_header_hash, prev_blockchain)
                    .map_err(|_e| <Error<T>>::DuplicateBlock)?,
                // create new blockchain element
                _ => Self::create_chain(
                    &current_chain_id, &current_block_height, &block_header_hash)
                    .map_err(|_e| <Error<T>>::DuplicateBlock)?,
            };

            // Store a new BlockHeader struct in BlockHeaders
            <BlockHeaders>::insert(&block_header_hash, &block_header);

            // Storing the blockchain depends if we extend or create a new chain
            match current_chain_id {
                // extended the chain
                prev_chain_id => {
                    // Update the pointer to BlockChain in ChainsIndex
                    <ChainsIndex>::mutate(&current_chain_id, |_b| blockchain); 

                    // TODO: call checkAndDoReorg
                }
                _ => {
                    // Store a pointer to BlockChain in ChainsIndex
                    <ChainsIndex>::insert(&current_chain_id, &blockchain); 
                }
            };

            // Emit a block store event
            let longest_chain_height = Self::bestblockheight();
            match current_block_height {
                longest_chain_height => Self::deposit_event(
                    Event::StoreMainChainHeader(current_block_height, block_header_hash)),
                _ => Self::deposit_event(
                    Event::StoreForkHeader(current_chain_id, current_block_height, block_header_hash)),
            };

            Ok(())
        }

        fn verify_transaction_inclusion(
            origin,
            tx_id: H256,
            tx_block_height: U256,
            tx_index: u64,
            merkle_proof: Vec<u8>,
            confirmations: U256)
        -> DispatchResult {
            let _ = ensure_signed(origin)?;

            // TODO: check if Parachain is in error status
            
            // TODO: check no data blocks

            Ok(())

        }
        
        fn flag_block_error(origin, block_hash: H256, error: ErrorCodes)
            -> DispatchResult {
           
            // TODO: ensure this is a staked relayer
            let _ = ensure_signed(origin)?;
            
            // Get the chain id of the block header
            ensure!(<BlockHeaders>::exists(block_hash), Error::<T>::BlockNotFound);
            let block_header = Self::blockheader(block_hash);
            let chain_id = block_header.chain_ref;

            // Get the blockchain element for the chain id
            let mut blockchain = Self::chainindex(&chain_id);

            // Flag errors in the blockchain entry
            // Check which error we are dealing with
            match error {
                ErrorCodes::NoDataBTCRelay => blockchain
                    .no_data
                    .push(block_header.block_height),
                ErrorCodes::InvalidBTCRelay => blockchain
                    .invalid
                    .push(block_header.block_height),
                _ => return Err(<Error<T>>::UnknownErrorcode.into()),
            };

            // Store the updated blockchain entry
            <ChainsIndex>::mutate(&chain_id, |_b| blockchain);

            Self::deposit_event(Event::FlagBlockError(block_hash, chain_id, error));
            Ok (())
        }
        
        fn clear_block_error(origin, block_hash: H256, error: ErrorCodes)
            -> DispatchResult {
           
            // TODO: ensure this is a staked relayer
            let _ = ensure_signed(origin)?;
            
            // Get the chain id of the block header
            ensure!(<BlockHeaders>::exists(block_hash), Error::<T>::BlockNotFound);
            let block_header = Self::blockheader(block_hash);
            let chain_id = block_header.chain_ref;

            // Get the blockchain element for the chain id
            let mut blockchain = Self::chainindex(&chain_id);

            // Clear errors in the blockchain entry
            // Check which error we are dealing with
            match error {
                ErrorCodes::NoDataBTCRelay => {
                    let index = blockchain.no_data
                        .iter()
                        .position(|x| *x == block_header.block_height)
                        .unwrap();
                    blockchain.no_data.remove(index);
                },
                ErrorCodes::InvalidBTCRelay => {
                    let index = blockchain.invalid
                        .iter()
                        .position(|x| *x == block_header.block_height)
                        .unwrap();
                    blockchain.invalid.remove(index);
                },
                _ => return Err(<Error<T>>::UnknownErrorcode.into()),
            };

            // Store the updated blockchain entry
            <ChainsIndex>::mutate(&chain_id, |_b| blockchain);

            Self::deposit_event(Event::ClearBlockError(block_hash, chain_id, error));
            Ok (())
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
    fn create_chain(
        chain_id: &U256,
        block_height: &U256,
        block_hash: &H256)
        -> Result<BlockChain<U256, BTreeMap<U256, H256>>, Error<T>> 
    {
        let mut chain = BTreeMap::new();

        if let Some(_) = chain.insert(*block_height, *block_hash) {
            return Err(<Error<T>>::DuplicateBlock.into())
        }
                
        let blockchain = BlockChain {
                    chain_id: *chain_id,
                    chain: chain,
                    start_height: *block_height,
                    max_height: *block_height,
                    no_data: vec![],
                    invalid: vec![],
        };
        Ok(blockchain)
    }
    fn extend_chain(
        block_height: &U256,
        block_hash: &H256,
        prev_blockchain: BlockChain<U256, BTreeMap<U256, H256>>) 
        -> Result<BlockChain<U256, BTreeMap<U256, H256>>, Error<T>> 
    {

        let mut blockchain = prev_blockchain;
        
        if let Some(_) = blockchain.chain.insert(*block_height, *block_hash) {
            return Err(<Error<T>>::DuplicateBlock.into())
        }
                
        blockchain.max_height = *block_height;

        Ok(blockchain)
    }
            
}

decl_event! {
	pub enum Event {
        Initialized(U256, H256),
        StoreMainChainHeader(U256, H256),
        StoreForkHeader(U256, U256, H256),
        ChainReorg(H256, U256, U256),
        VerifyTransaction(H256, U256, U256),
        ValidateTransaction(H256, U256, H160, H256),
        FlagBlockError(H256, U256, ErrorCodes),
        ClearBlockError(H256, U256, ErrorCodes),
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
        MissingBlockHeight,
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
        UnknownErrorcode,
        BlockNotFound,
        AlreadyReported,
    }
}

