#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(test)]
mod tests;

#[cfg(test)]
mod mock;

#[cfg(test)]
extern crate mocktopus;

#[cfg(test)]
use mocktopus::macros::mockable;

/// # BTC-Relay implementation
/// This is the implementation of the BTC-Relay following the spec at:
/// https://interlay.gitlab.io/polkabtc-spec/btcrelay-spec/
// Substrate
use frame_support::{
    decl_event, decl_module, decl_storage, dispatch::DispatchResult, ensure, IterableStorageMap,
};
use primitive_types::U256;
use sp_core::H160;
use sp_std::collections::btree_set::BTreeSet;
use sp_std::prelude::*;
use system::ensure_signed;

// Crates
use bitcoin::merkle::{MerkleProof, ProofResult};
use bitcoin::parser::{
    extract_address_hash, extract_op_return_data, parse_block_header, parse_transaction,
};
use bitcoin::types::{
    BlockChain, BlockHeader, H256Le, RawBlockHeader, RichBlockHeader, Transaction,
};
use security::ErrorCode;
use x_core::Error;

/// ## Configuration and Constants
/// The pallet's configuration trait.
/// For further reference, see:
/// https://interlay.gitlab.io/polkabtc-spec/btcrelay-spec/spec/data-model.html
pub trait Trait: system::Trait //+ security::Trait
{
    /// The overarching event type.
    type Event: From<Event> + Into<<Self as system::Trait>::Event>;
}

/// Difficulty Adjustment Interval
pub const DIFFICULTY_ADJUSTMENT_INTERVAL: u32 = 2016;

/// Target Timespan
pub const TARGET_TIMESPAN: u32 = 1_209_600;

// Used in Bitcoin's retarget algorithm
pub const TARGET_TIMESPAN_DIVISOR: u32 = 4;

/// Unrounded Maximum Target
/// 0x00000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
pub const UNROUNDED_MAX_TARGET: U256 = U256([
    <u64>::max_value(),
    <u64>::max_value(),
    <u64>::max_value(),
    0x0000_0000_ffff_ffffu64,
]);

/// Main chain id
pub const MAIN_CHAIN_ID: u32 = 0;

/// Global security parameter k for stable transactions
pub const STABLE_TRANSACTION_CONFIRMATIONS: u32 = 6;

// This pallet's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as BTCRelay {
    /// ## Storage
        /// Store Bitcoin block headers
        BlockHeaders: map hasher(blake2_128_concat) H256Le => RichBlockHeader;

        /// Sorted mapping of BlockChain elements with reference to ChainsIndex
        Chains: map hasher(blake2_128_concat) u32 => u32;

        /// Store the index for each tracked blockchain
        ChainsIndex: map hasher(blake2_128_concat) u32 => Option<BlockChain>;

        /// Stores a mapping from (chain_index, block height) to block hash
        ChainsHashes: double_map hasher(blake2_128_concat) u32, hasher(blake2_128_concat) u32 => H256Le;

        /// Store the current blockchain tip
        BestBlock: H256Le;

        /// Store the height of the best block
        BestBlockHeight: u32;

        /// Track existing BlockChain entries
        ChainCounter: u32;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        // Initializing events
        fn deposit_event() = default;

        // Initialize errors
        // type Error = Error<T>;

        /// One time function to initialize the BTC-Relay with the first block
        /// # Arguments
        ///
        /// * `block_header_bytes` - 80 byte raw Bitcoin block header.
        /// * `block_height` - Bitcoin block height of the submitted
        /// block header.
        fn initialize(
            origin,
            raw_block_header: RawBlockHeader,
            block_height: u32)
            -> DispatchResult
        {
            let _ = ensure_signed(origin)?;

            // Check if BTC-Relay was already initialized
            ensure!(!Self::best_block_exists(), Error::AlreadyInitialized);

            // Parse the block header bytes to extract the required info
            let basic_block_header = parse_block_header(&raw_block_header)?;
            let block_header_hash = raw_block_header.hash();

            // construct the BlockChain struct
            let blockchain = Self::initialize_blockchain(
                    block_height, block_header_hash);
            // Create rich block header
            let block_header = RichBlockHeader {
                block_hash: block_header_hash,
                block_header: basic_block_header,
                block_height: block_height,
                chain_ref: blockchain.chain_id
            };

            // Store a new BlockHeader struct in BlockHeaders
            Self::set_block_header_from_hash(block_header_hash, &block_header);

            // Store a pointer to BlockChain in ChainsIndex
            Self::set_block_chain_from_id(
                MAIN_CHAIN_ID, &blockchain);

            // Store the reference to the new BlockChain in Chains
            Self::set_chain_from_position_and_id(0, MAIN_CHAIN_ID);

            // Set BestBlock and BestBlockHeight to the submitted block
            Self::set_best_block(block_header_hash);
            Self::set_best_block_height(block_height);

            // Emit a Initialized Event
            Self::deposit_event(Event::Initialized(
                    block_height, block_header_hash
                )
            );

            Ok(())
        }

        /// Stores a single new block header
        ///
        /// # Arguments
        ///
        /// * `raw_block_header` - 80 byte raw Bitcoin block header.
        fn store_block_header(
            origin, raw_block_header: RawBlockHeader
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            // Check if BTC _Parachain is in shutdown state.+

            // ensure!(
            //     !<security::Module<T>>::check_parachain_status(
            //         StatusCode::Shutdown),
            //     Error::Shutdown
            // );

            // Parse the block header bytes to extract the required info
            let basic_block_header = Self::verify_block_header(&raw_block_header)?;
            let block_header_hash = raw_block_header.hash();

            let prev_header = Self::get_block_header_from_hash(
                basic_block_header.hash_prev_block
            )?;

            // get the block chain of the previous header
            let prev_blockchain = Self::get_block_chain_from_id(
                prev_header.chain_ref
            )?;

            // Update the current block header
            // check if the prev block is the highest block in the chain
            // load the previous block header block height
            let prev_block_height = prev_header.block_height;

            // update the current block header with height and chain ref
            // Set the height of the block header
            let current_block_height = prev_block_height + 1;

            // Update the blockchain
            // check if we create a new blockchain or extend the existing one
            // print!("Prev max height: {:?} \n", prev_blockchain.max_height);
            let is_fork = prev_blockchain.max_height != prev_block_height;

            let blockchain = if is_fork {
                // create new blockchain element
                Self::create_blockchain(current_block_height, block_header_hash)
            } else {
                // extend the current chain
                Self::extend_blockchain(
                    current_block_height, &block_header_hash, prev_blockchain)?
            };

            // Create rich block header
            let block_header = RichBlockHeader {
                block_hash: block_header_hash,
                block_header: basic_block_header,
                block_height: current_block_height,
                chain_ref: blockchain.chain_id
            };


            // Store a new BlockHeader struct in BlockHeaders
            Self::set_block_header_from_hash(block_header_hash, &block_header);

            // Storing the blockchain depends if we extend or create a new chain
            if is_fork {
                // create a new chain
                // Store a pointer to BlockChain in ChainsIndex
                Self::set_block_chain_from_id(blockchain.chain_id, &blockchain);
                // Store the reference to the blockchain in Chains
                Self::insert_sorted(&blockchain)?;
            } else {
                // extended the chain
                // Update the pointer to BlockChain in ChainsIndex
                <ChainsIndex>::mutate(blockchain.chain_id, |_b| &blockchain);

                // check if ordering of Chains needs updating
                Self::check_and_do_reorg(&blockchain)?;

                if blockchain.chain_id == MAIN_CHAIN_ID {
                    Self::set_best_block(block_header_hash);
                    Self::set_best_block_height(current_block_height)
                }
            };

            // Determine if this block extends the main chain or a fork
            let current_best_block = Self::get_best_block();

            // print!("Best block hash: {:?} \n", current_best_block);
            // print!("Current block hash: {:?} \n", block_header_hash);
            if current_best_block == block_header_hash {
                // extends the main chain
                Self::deposit_event(
                    Event::StoreMainChainHeader(
                        current_block_height,
                        block_header_hash
                    )
                );
            } else {
            // created a new fork or updated an existing one
                Self::deposit_event(
                    Event::StoreForkHeader(
                        blockchain.chain_id,
                        current_block_height,
                        block_header_hash
                    )
                );
            };


            Ok(())
        }

        /// Verifies the inclusion of `tx_id` in block at height `tx_block_height`
        /// # Arguments
        ///
        /// * `tx_id` - The hash of the transaction to check for
        /// * `tx_block_height` - The height of the block in which the
        /// transaction should be included
        /// * `raw_merkle_proof` - The raw merkle proof as returned by
        /// bitcoin `gettxoutproof`
        /// * `confirmations` - The number of confirmations needed to accept
        /// the proof
        /// * `insecure` - determines if checks against recommended global transaction confirmation are to be executed. Recommended: set to `true`
        fn verify_transaction_inclusion(
            origin,
            tx_id: H256Le,
            block_height: u32,
            raw_merkle_proof: Vec<u8>,
            confirmations: u32,
            insecure: bool)
        -> DispatchResult {
            let _ = ensure_signed(origin)?;

            // fail if parachain is not in running state.
            /*
            ensure!(<security::Module<T>>::check_parachain_status(StatusCode::Running),
                Error::<T>::Shutdown);
            */
            Self::_verify_transaction_inclusion(tx_id, block_height, raw_merkle_proof, confirmations, insecure)?;
            Ok(())
        }

        /// Validates a given raw Bitcoin transaction, according to the
        /// supported transaction format (see
        /// https://interlay.gitlab.io/polkabtc-spec/btcrelay-spec/intro/
        /// accepted-format.html)
        /// This DOES NOT check if the transaction is included in a block
        /// nor does it guarantee that the transaction is fully valid according
        /// to the consensus (needs full node).
        ///
        /// # Arguments
        /// * `raw_tx` - raw Bitcoin transaction
        /// * `paymentValue` - value of BTC sent in the 1st /
        /// payment UTXO of the transaction
        /// * `recipientBtcAddress` - 20 byte Bitcoin address of recipient
        /// of the BTC in the 1st  / payment UTXO
        /// * `op_return_id` - 32 byte hash identifier expected in
        /// OP_RETURN (replay protection)
        fn validate_transaction(
            origin,
            raw_tx: Vec<u8>,
            payment_value: i64,
            recipient_btc_address: Vec<u8>,
            op_return_id: Vec<u8>
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            Self::_validate_transaction(raw_tx, payment_value, recipient_btc_address, op_return_id)?;
            Ok(())
        }
    }
}

#[cfg_attr(test, mockable)]
impl<T: Trait> Module<T> {
    pub fn _verify_transaction_inclusion(
        tx_id: H256Le,
        block_height: u32,
        raw_merkle_proof: Vec<u8>,
        confirmations: u32,
        insecure: bool,
    ) -> Result<(), Error> {
        //let main_chain = Self::get_block_chain_from_id(MAIN_CHAIN_ID);
        let best_block_height = Self::get_best_block_height();

        let next_best_fork_id = Self::get_chain_id_from_position(1);
        let chain = Self::get_block_chain_from_id(next_best_fork_id)?;
        let next_best_fork_height = chain.max_height;

        // fail if there is an ongoing fork
        ensure!(
            best_block_height >= next_best_fork_height + STABLE_TRANSACTION_CONFIRMATIONS,
            Error::OngoingFork
        );

        // This call fails if not enough confirmations
        Self::check_confirmations(best_block_height, confirmations, block_height, insecure)?;

        let proof_result = Self::verify_merkle_proof(&raw_merkle_proof)?;
        let rich_header = Self::get_block_header_from_height(
            &Self::get_block_chain_from_id(MAIN_CHAIN_ID)?,
            block_height,
        )?;

        // fail if the transaction hash is invalid
        ensure!(proof_result.transaction_hash == tx_id, Error::InvalidTxid);

        // fail if the merkle root is invalid
        ensure!(
            proof_result.extracted_root == rich_header.block_header.merkle_root,
            Error::InvalidMerkleProof
        );
        Ok(())
    }

    pub fn _validate_transaction(
        raw_tx: Vec<u8>,
        payment_value: i64,
        recipient_btc_address: Vec<u8>,
        op_return_id: Vec<u8>,
    ) -> Result<(), Error> {
        let transaction = Self::parse_transaction(&raw_tx)?;

        // TODO: make 2 a constant
        ensure!(transaction.outputs.len() >= 2, Error::MalformedTransaction);

        // Check if 1st / payment UTXO transfers sufficient value
        // FIXME: returns incorrect value (too large: 9865995930474779817)
        let extr_payment_value = transaction.outputs[0].value;
        ensure!(
            extr_payment_value >= payment_value,
            Error::InsufficientValue
        );

        // Check if 1st / payment UTXO sends to correct address
        let extr_recipient_address = extract_address_hash(&transaction.outputs[0].script)?;
        ensure!(
            extr_recipient_address == recipient_btc_address,
            Error::WrongRecipient
        );

        // Check if 2nd / data UTXO has correct OP_RETURN value
        let extr_op_return_value = extract_op_return_data(&transaction.outputs[1].script)?;
        ensure!(extr_op_return_value == op_return_id, Error::InvalidOpreturn);

        Ok(())
    }

    // ********************************
    // START: Storage getter functions
    // ********************************

    /// Get chain id from position (sorted by max block height)
    fn get_chain_id_from_position(position: u32) -> u32 {
        <Chains>::get(position)
    }
    /// Get the position of the fork in Chains
    fn get_chain_position_from_chain_id(chain_id: u32) -> Result<u32, Error> {
        for (k, v) in <Chains>::iter() {
            if v == chain_id {
                return Ok(k);
            }
        }
        Err(Error::ForkIdNotFound)
    }
    //    match <Chains>::enumerate()
    //        .position(|(_k, v)| v == chain_id)
    //    {
    //        Some(pos) => return Ok(pos as u32),
    //        None => return Err(Error::ForkIdNotFound),
    //    };
    //}
    /// Get a blockchain from the id
    // TODO: the return of this element can an empty element when it was deleted
    // Function should be changed to return a Result or Option
    fn get_block_chain_from_id(chain_id: u32) -> Result<BlockChain, Error> {
        <ChainsIndex>::get(chain_id).ok_or(Error::InvalidChainID)
    }
    /// Get the current best block hash
    fn get_best_block() -> H256Le {
        <BestBlock>::get()
    }
    /// Check if a best block hash is set
    fn best_block_exists() -> bool {
        <BestBlock>::exists()
    }
    /// get the best block height
    fn get_best_block_height() -> u32 {
        <BestBlockHeight>::get()
    }
    /// Get the current chain counter
    fn get_chain_counter() -> u32 {
        <ChainCounter>::get()
    }
    /// Get a block hash from a blockchain
    /// # Arguments
    ///
    /// * `chain_id`: the id of the blockchain to search in
    /// * `block_height`: the height if the block header
    fn get_block_hash(chain_id: u32, block_height: u32) -> Result<H256Le, Error> {
        if !Self::block_exists(chain_id, block_height) {
            return Err(Error::MissingBlockHeight);
        }
        Ok(<ChainsHashes>::get(chain_id, block_height))
    }

    /// Get a block header from its hash
    fn get_block_header_from_hash(block_hash: H256Le) -> Result<RichBlockHeader, Error> {
        if <BlockHeaders>::contains_key(block_hash) {
            return Ok(<BlockHeaders>::get(block_hash));
        }
        Err(Error::BlockNotFound)
    }
    /// Check if a block header exists
    fn block_header_exists(block_hash: H256Le) -> bool {
        <BlockHeaders>::contains_key(block_hash)
    }
    /// Get a block header from
    fn get_block_header_from_height(
        blockchain: &BlockChain,
        block_height: u32,
    ) -> Result<RichBlockHeader, Error> {
        let block_hash = Self::get_block_hash(blockchain.chain_id, block_height)?;
        Self::get_block_header_from_hash(block_hash)
    }
    /// Storage setter functions
    /// Set a new chain with position and id
    fn set_chain_from_position_and_id(position: u32, id: u32) {
        <Chains>::insert(position, id);
    }
    /// Swap chain elements
    fn swap_chain(pos_1: u32, pos_2: u32) {
        <Chains>::swap(pos_1, pos_2)
    }
    /// Remove a chain id from chains
    fn remove_blockchain_from_chain(position: u32) -> Result<(), Error> {
        // swap the element with the last element in the mapping
        let head_index = match <Chains>::iter().nth(0) {
            Some(head) => head.0,
            None => return Err(Error::ForkIdNotFound),
        };
        <Chains>::swap(position, head_index);
        // remove the header (now the value at the initial position)
        <Chains>::remove(head_index);
        Ok(())
    }
    /// Set a new blockchain in ChainsIndex
    fn set_block_chain_from_id(id: u32, chain: &BlockChain) {
        <ChainsIndex>::insert(id, &chain);
    }
    /// Update a blockchain in ChainsIndex
    fn mutate_block_chain_from_id(id: u32, chain: BlockChain) {
        <ChainsIndex>::mutate(id, |b| *b = Some(chain));
    }
    /// Remove a blockchain element from chainindex
    fn remove_blockchain_from_chainindex(id: u32) {
        <ChainsIndex>::remove(id);
    }
    /// Set a new block header
    fn set_block_header_from_hash(hash: H256Le, header: &RichBlockHeader) {
        <BlockHeaders>::insert(hash, header);
    }
    /// update the chain_ref of a block header
    fn mutate_block_header_from_chain_id(hash: &H256Le, chain_ref: u32) {
        <BlockHeaders>::mutate(&hash, |header| header.chain_ref = chain_ref);
    }

    /// Set a new best block
    fn set_best_block(hash: H256Le) {
        <BestBlock>::put(hash);
    }
    /// Set a new best block height
    fn set_best_block_height(height: u32) {
        <BestBlockHeight>::put(height);
    }
    /// Set a new chain counter
    fn increment_chain_counter() -> u32 {
        let new_counter = Self::get_chain_counter() + 1;
        <ChainCounter>::put(new_counter);

        new_counter
    }
    /// Initialize the new main blockchain with a single block
    fn initialize_blockchain(block_height: u32, block_hash: H256Le) -> BlockChain {
        let chain_id = MAIN_CHAIN_ID;

        // generate an empty blockchain
        Self::generate_blockchain(chain_id, block_height, block_hash)
    }
    /// Create a new blockchain element with a new chain id
    fn create_blockchain(block_height: u32, block_hash: H256Le) -> BlockChain {
        // get a new chain id
        let chain_id: u32 = Self::increment_chain_counter();

        // generate an empty blockchain
        Self::generate_blockchain(chain_id, block_height, block_hash)
    }
    /// Generate the raw blockchain from a chain Id and with a single block
    fn generate_blockchain(chain_id: u32, block_height: u32, block_hash: H256Le) -> BlockChain {
        // initialize an empty chain

        Self::insert_block_hash(chain_id, block_height, block_hash);

        BlockChain {
            chain_id,
            start_height: block_height,
            max_height: block_height,
            no_data: BTreeSet::new(),
            invalid: BTreeSet::new(),
        }
    }

    fn insert_block_hash(chain_id: u32, block_height: u32, block_hash: H256Le) {
        <ChainsHashes>::insert(chain_id, block_height, block_hash);
    }

    fn remove_block_hash(chain_id: u32, block_height: u32) {
        <ChainsHashes>::remove(chain_id, block_height);
    }

    fn block_exists(chain_id: u32, block_height: u32) -> bool {
        <ChainsHashes>::contains_key(chain_id, block_height)
    }

    fn _blocks_count(chain_id: u32) -> usize {
        <ChainsHashes>::iter_prefix(chain_id).count()
    }

    /// Add a new block header to an existing blockchain
    fn extend_blockchain(
        block_height: u32,
        block_hash: &H256Le,
        prev_blockchain: BlockChain,
    ) -> Result<BlockChain, Error> {
        let mut blockchain = prev_blockchain;

        if Self::block_exists(blockchain.chain_id, block_height) {
            return Err(Error::DuplicateBlock);
        }
        Self::insert_block_hash(blockchain.chain_id, block_height, *block_hash);

        blockchain.max_height = block_height;

        Ok(blockchain)
    }

    // Get require conformations for stable transactions
    fn get_stable_transaction_confirmations() -> u32 {
        STABLE_TRANSACTION_CONFIRMATIONS
    }
    // *********************************
    // END: Storage getter functions
    // *********************************

    // Wrapper functions around bitcoin lib for testing purposes
    fn parse_transaction(raw_tx: &[u8]) -> Result<Transaction, Error> {
        parse_transaction(&raw_tx)
    }

    fn verify_merkle_proof(raw_merkle_proof: &[u8]) -> Result<ProofResult, Error> {
        let merkle_proof = MerkleProof::parse(&raw_merkle_proof)?;

        merkle_proof.verify_proof()
    }
    /// Parses and verifies a raw Bitcoin block header.
    /// # Arguments
    /// * block_header` - 80-byte block header
    ///
    /// # Returns
    /// * `pure_block_header` - PureBlockHeader representation of the 80-byte block header
    ///
    /// # Panics
    /// If ParachainStatus in Security module is not set to RUNNING
    fn verify_block_header(raw_block_header: &RawBlockHeader) -> Result<BlockHeader, Error> {
        let basic_block_header = parse_block_header(&raw_block_header)?;

        let block_header_hash = raw_block_header.hash();

        // Check that the block header is not yet stored in BTC-Relay
        ensure!(
            !Self::block_header_exists(block_header_hash),
            Error::DuplicateBlock
        );

        // Check that the referenced previous block header exists in BTC-Relay
        let prev_block_header =
            Self::get_block_header_from_hash(basic_block_header.hash_prev_block)?;
        // Check that the PoW hash satisfies the target set in the block header
        ensure!(
            block_header_hash.as_u256() < basic_block_header.target,
            Error::LowDiff
        );

        // Check that the diff. target is indeed correctly set in the block header, i.e., check for re-target.
        let block_height = prev_block_header.block_height + 1;

        let expected_target =
            if block_height >= 2016 && block_height % DIFFICULTY_ADJUSTMENT_INTERVAL == 0 {
                Self::compute_new_target(&prev_block_header, block_height)?
            } else {
                prev_block_header.block_header.target
            };

        ensure!(
            basic_block_header.target == expected_target,
            Error::DiffTargetHeader
        );

        Ok(basic_block_header)
    }

    /// Computes Bitcoin's PoW retarget algorithm for a given block height
    /// # Arguments
    ///  * `prev_block_header`: previous block header
    ///  * `block_height` : block height of new target
    fn compute_new_target(
        prev_block_header: &RichBlockHeader,
        block_height: u32,
    ) -> Result<U256, Error> {
        // get time of last retarget
        let last_retarget_time =
            Self::get_last_retarget_time(prev_block_header.chain_ref, block_height)?;
        // Compute new target
        let actual_timespan = if ((prev_block_header.block_header.timestamp - last_retarget_time)
            as u32)
            < (TARGET_TIMESPAN / TARGET_TIMESPAN_DIVISOR)
        {
            TARGET_TIMESPAN / TARGET_TIMESPAN_DIVISOR
        } else {
            TARGET_TIMESPAN * TARGET_TIMESPAN_DIVISOR
        };

        let new_target = U256::from(actual_timespan) * prev_block_header.block_header.target
            / U256::from(TARGET_TIMESPAN);

        // ensure target does not exceed max. target
        Ok(if new_target > UNROUNDED_MAX_TARGET {
            UNROUNDED_MAX_TARGET
        } else {
            new_target
        })
    }

    /// Returns the timestamp of the last difficulty retarget on the specified BlockChain, given the current block height
    ///
    /// # Arguments
    /// * `chain_ref` - BlockChain identifier
    /// * `block_height` - current block height
    fn get_last_retarget_time(chain_ref: u32, block_height: u32) -> Result<u64, Error> {
        let block_chain = Self::get_block_chain_from_id(chain_ref)?;
        let last_retarget_header = Self::get_block_header_from_height(
            &block_chain,
            block_height - DIFFICULTY_ADJUSTMENT_INTERVAL,
        )?;
        Ok(last_retarget_header.block_header.timestamp)
    }

    /// Swap the main chain with a fork. This method takes the starting height
    /// of the fork and replaces each block in the main chain with the blocks
    /// in the fork. It moves the replaced blocks in the main chain to a new
    /// fork.
    /// Last, it replaces the chain_ref of each block header in the new main
    /// chain to the MAIN_CHAIN_ID and each block header in the new fork to the
    /// new chain id.
    ///
    /// # Arguments
    ///
    /// * `fork` - the fork that is going to become the main chain
    fn swap_main_blockchain(fork: &BlockChain) -> Result<(), Error> {
        // load the main chain
        let mut main_chain = Self::get_block_chain_from_id(MAIN_CHAIN_ID)?;

        // the start height of the fork
        let start_height = fork.start_height;

        // create a new blockchain element to store the part of the main chain
        // that is being forked
        // generate a chain id
        let chain_id = Self::increment_chain_counter();

        // maybe split off the no data elements
        // check if there is a no_data block element
        // that is greater than start_height
        let index_no_data = main_chain
            .no_data
            .iter()
            .position(|&h| h >= start_height)
            .map(|v| v as u32);
        let no_data = match index_no_data {
            Some(index) => main_chain.no_data.split_off(&index),
            None => BTreeSet::new(),
        };

        // maybe split off the invalid elements
        let index_invalid = main_chain
            .invalid
            .iter()
            .position(|&h| h >= start_height)
            .map(|v| v as u32);
        let invalid = match index_invalid {
            Some(index) => main_chain.invalid.split_off(&index),
            None => BTreeSet::new(),
        };

        // store the main chain part that is going to be replaced by the new fork
        // into the forked_main_chain element
        let forked_main_chain: BlockChain = BlockChain {
            chain_id,
            start_height,
            max_height: main_chain.max_height,
            no_data,
            invalid,
        };

        main_chain.max_height = fork.max_height;
        main_chain.no_data.append(&mut fork.no_data.clone());
        main_chain.invalid.append(&mut fork.invalid.clone());

        // get the best block hash
        let best_block = Self::get_block_hash(fork.chain_id, fork.max_height)?;

        // get the position of the fork in Chains
        let position: u32 = Self::get_chain_position_from_chain_id(fork.chain_id)?;

        // Update the stored main chain
        Self::set_block_chain_from_id(MAIN_CHAIN_ID, &main_chain);

        // Set BestBlock and BestBlockHeight to the submitted block
        Self::set_best_block(best_block);
        Self::set_best_block_height(main_chain.max_height);

        // remove the fork from storage
        Self::remove_blockchain_from_chainindex(fork.chain_id);
        Self::remove_blockchain_from_chain(position)?;

        // store the forked main chain
        Self::set_block_chain_from_id(forked_main_chain.chain_id, &forked_main_chain);

        // insert the reference to the forked main chain in Chains
        Self::insert_sorted(&forked_main_chain)?;

        // update all the forked block headers
        for height in fork.start_height..=forked_main_chain.max_height {
            let block_hash = Self::get_block_hash(main_chain.chain_id, height)?;
            Self::insert_block_hash(forked_main_chain.chain_id, height, block_hash);
            Self::mutate_block_header_from_chain_id(&block_hash, forked_main_chain.chain_id);
            Self::remove_block_hash(MAIN_CHAIN_ID, height);
        }

        // update all new main chain block headers
        for height in fork.start_height..=fork.max_height {
            let block = Self::get_block_hash(fork.chain_id, height)?;
            Self::mutate_block_header_from_chain_id(&block, MAIN_CHAIN_ID);
            Self::insert_block_hash(MAIN_CHAIN_ID, height, block);
        }
        <ChainsHashes>::remove_prefix(fork.chain_id);

        Ok(())
    }
    /// Checks if a newly inserted fork results in an update to the sorted
    /// Chains mapping. This happens when the max height of the fork is greater
    /// than the max height of the previous element in the Chains mapping.
    ///
    /// # Arguments
    ///
    /// * `fork` - the blockchain element that may cause a reorg
    fn check_and_do_reorg(fork: &BlockChain) -> Result<(), Error> {
        // Check if the ordering needs updating
        // if the fork is the main chain, we don't need to update the ordering
        if fork.chain_id == MAIN_CHAIN_ID {
            return Ok(());
        }

        // get the position of the fork in Chains
        let fork_position: u32 = Self::get_chain_position_from_chain_id(fork.chain_id)?;
        // print!("fork position {:?}\n", fork_position);
        // check if the previous element in Chains has a lower block_height
        let mut current_position = fork_position;
        let mut current_height = fork.max_height;

        // swap elements as long as previous block height is smaller
        while current_position > 0 {
            // get the previous position
            let prev_position = current_position - 1;
            // get the blockchain id
            let prev_blockchain_id = Self::get_chain_id_from_position(prev_position);
            // get the previous blockchain height
            let prev_height = Self::get_block_chain_from_id(prev_blockchain_id)?.max_height;
            // swap elements if block height is greater
            // print!("curr height {:?}\n", current_height);
            // print!("prev height {:?}\n", prev_height);
            if prev_height < current_height {
                // Check if swap occurs on the main chain element
                // print!("prev chain id {:?}\n", prev_blockchain_id);
                if prev_blockchain_id == MAIN_CHAIN_ID {
                    // if the previous position is the top element
                    // and the current height is more than the
                    // STABLE_TRANSACTION_CONFIRMATIONS ahead
                    // we are swapping the main chain
                    if prev_height + STABLE_TRANSACTION_CONFIRMATIONS < current_height {
                        Self::swap_main_blockchain(&fork)?;

                        // announce the new main chain
                        let new_chain_tip = <BestBlock>::get();
                        let block_height = <BestBlockHeight>::get();
                        let fork_depth = fork.max_height - fork.start_height;
                        // print!("tip {:?}\n", new_chain_tip);
                        // print!("block height {:?}\n", block_height);
                        // print!("depth {:?}\n", fork_depth);
                        Self::deposit_event(Event::ChainReorg(
                            new_chain_tip,
                            block_height,
                            fork_depth,
                        ));
                    } else {
                        Self::deposit_event(Event::ForkAheadOfMainChain(
                            prev_height,     // main chain height
                            fork.max_height, // fork height
                            fork.chain_id,   // fork id
                        ));
                    }
                    // break the while loop
                    break;
                } else {
                    // else, simply swap the chain_id ordering in Chains
                    Self::swap_chain(prev_position, current_position);
                }

                // update the current chain to the previous one
                current_position = prev_position;
                current_height = prev_height;
            } else {
                break;
            }
        }

        Ok(())
    }
    /// Insert a new fork into the Chains mapping sorted by its max height
    ///
    /// # Arguments
    ///
    /// * `blockchain` - new blockchain element
    fn insert_sorted(blockchain: &BlockChain) -> Result<(), Error> {
        // print!("Chain id: {:?}\n", blockchain.chain_id);
        // get a sorted vector over the Chains elements
        // NOTE: LinkedStorageMap iterators are not sorted over the keys
        let mut chains = <Chains>::iter().collect::<Vec<(u32, u32)>>();
        chains.sort_by_key(|k| k.0);

        let max_chain_element = chains.len() as u32;
        // define the position of the new blockchain
        // by default, we insert it as the last element
        let mut position_blockchain = max_chain_element;

        // Starting from the second highest element, find where to insert the new fork
        // the previous element's block height should be higher or equal
        // the next element's block height should be lower or equal
        // NOTE: we never want to insert a new main chain through this function
        for (curr_position, curr_chain_id) in chains.iter().skip(1) {
            // get the height of the current chain_id
            let curr_height = Self::get_block_chain_from_id(*curr_chain_id)?.max_height;

            // if the height of the current blockchain is lower than
            // the new blockchain, it should be inserted at that position
            if curr_height <= blockchain.max_height {
                position_blockchain = *curr_position;
                break;
            };
        }

        // insert the new fork into the chains element
        Self::set_chain_from_position_and_id(max_chain_element, blockchain.chain_id);
        // starting from the last element swap the positions until
        // the new blockchain is at the position_blockchain
        // print!("max element {:?}\n", max_chain_element);
        // print!("position blockchain {:?}\n", position_blockchain);
        for curr_position in (position_blockchain + 1..max_chain_element + 1).rev() {
            // stop when the blockchain element is at it's
            // designated position
            // print!("current position {:?}\n", curr_position);
            if curr_position < position_blockchain {
                break;
            };
            let prev_position = curr_position - 1;
            // swap the current element with the previous one
            // print!("Swapping pos {:?} with pos {:?}\n", curr_position, prev_position);
            Self::swap_chain(curr_position, prev_position);
        }
        Ok(())
    }
    /// Flag an error in a block header. This function is called by the
    /// security pallet.
    ///
    /// # Arguments
    ///
    /// * `block_hash` - the hash of the block header with the error
    /// * `error` - the error code for the block header
    pub fn flag_block_error(block_hash: H256Le, error: ErrorCode) -> Result<(), Error> {
        // Get the chain id of the block header
        let block_header = Self::get_block_header_from_hash(block_hash)?;
        let chain_id = block_header.chain_ref;

        // Get the blockchain element for the chain id
        let mut blockchain = Self::get_block_chain_from_id(chain_id)?;

        // Flag errors in the blockchain entry
        // Check which error we are dealing with
        let newly_flagged = match error {
            ErrorCode::NoDataBTCRelay => blockchain.no_data.insert(block_header.block_height),
            ErrorCode::InvalidBTCRelay => blockchain.invalid.insert(block_header.block_height),
            _ => return Err(Error::UnknownErrorcode),
        };

        // If the block was not already flagged, store the updated blockchain entry
        if newly_flagged {
            Self::mutate_block_chain_from_id(chain_id, blockchain);
            Self::deposit_event(Event::FlagBlockError(block_hash, chain_id, error));
        }

        Ok(())
    }

    /// Clear an error from a block header. This function is called by the
    /// security pallet.
    ///
    /// # Arguments
    ///
    /// * `block_hash` - the hash of the block header being cleared
    /// * `error` - the error code for the block header
    pub fn clear_block_error(block_hash: H256Le, error: ErrorCode) -> Result<(), Error> {
        // Get the chain id of the block header
        let block_header = Self::get_block_header_from_hash(block_hash)?;
        let chain_id = block_header.chain_ref;

        // Get the blockchain element for the chain id
        let mut blockchain = Self::get_block_chain_from_id(chain_id)?;

        // Clear errors in the blockchain entry
        // Check which error we are dealing with
        let block_exists = match error {
            ErrorCode::NoDataBTCRelay => blockchain.no_data.remove(&block_header.block_height),
            ErrorCode::InvalidBTCRelay => blockchain.invalid.remove(&block_header.block_height),
            _ => return Err(Error::UnknownErrorcode),
        };

        if block_exists {
            // Store the updated blockchain entry
            Self::mutate_block_chain_from_id(chain_id, blockchain);

            Self::deposit_event(Event::ClearBlockError(block_hash, chain_id, error));
        }

        Ok(())
    }

    /// Checks if the given transaction confirmations are greater/equal to the
    /// requested confirmations (and/or the global k security parameter)
    ///
    /// # Arguments
    /// * `block_height` - current main chain block height
    /// * `req_confs` - confirmations requested by the caller
    /// * `tx_block_height` - block height of checked transaction
    /// * `insecure` -  determines if checks against recommended global transaction confirmation are to be executed. Recommended: set to `true`
    ///
    pub fn check_confirmations(
        main_chain_height: u32,
        req_confs: u32,
        tx_block_height: u32,
        insecure: bool,
    ) -> Result<(), Error> {
        // insecure call: only checks against user parameter
        if insecure {
            if tx_block_height + req_confs <= main_chain_height {
                Ok(())
            } else {
                Err(Error::Confirmations)
            }
        } else {
            // secure call: checks against max of user- and global security parameter
            let global_confs = Self::get_stable_transaction_confirmations();

            if global_confs > req_confs {
                if tx_block_height + global_confs <= main_chain_height {
                    Ok(())
                } else {
                    Err(Error::InsufficientStableConfirmations)
                }
            } else if tx_block_height + req_confs <= main_chain_height {
                Ok(())
            } else {
                Err(Error::Confirmations)
            }
        }
    }
}

decl_event! {
    pub enum Event {
        Initialized(u32, H256Le),
        StoreMainChainHeader(u32, H256Le),
        StoreForkHeader(u32, u32, H256Le),
        ChainReorg(H256Le, u32, u32),
        ForkAheadOfMainChain(u32, u32, u32),
        VerifyTransaction(H256Le, u32, u32),
        ValidateTransaction(H256Le, u32, H160, H256Le),
        FlagBlockError(H256Le, u32, ErrorCode),
        ClearBlockError(H256Le, u32, ErrorCode),
    }
}
