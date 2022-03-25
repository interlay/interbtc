//! # BTC-Relay Pallet
//!
//! Based on the [specification](https://spec.interlay.io/spec/btc-relay/index.html).
//!
//! This pallet implements a Bitcoin light client to store and verify block headers in accordance
//! with SPV assumptions - i.e. longest chain.
//!
//! Unless otherwise stated, the primary source of truth for code contained herein is the
//! [Bitcoin Core repository](https://github.com/bitcoin/bitcoin), though implementation
//! details may vary.
//!
//! ## Overview
//!
//! The BTC-Relay pallet provides functions for:
//!
//! - Initializing and updating the relay.
//! - Transaction inclusion verification.
//! - Transaction validation.
//!
//! ### Terminology
//!
//! - **Bitcoin Confirmations:** The minimum number of Bitcoin confirmations a Bitcoin block header must have to be seen
//!   as included in the main chain.
//!
//! - **Parachain Confirmations:** The minimum number of Parachain confirmations a Bitcoin block header must have to be
//!   usable in transaction inclusion verification.

#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

mod ext;

pub mod types;

#[cfg(any(feature = "runtime-benchmarks", test))]
mod benchmarking;

mod default_weights;
pub use default_weights::WeightInfo;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod mock;

#[cfg(test)]
extern crate mocktopus;

#[cfg(test)]
use mocktopus::macros::mockable;

use frame_support::{
    dispatch::{DispatchError, DispatchResult},
    ensure, runtime_print,
    traits::Get,
    transactional,
};
use frame_system::ensure_signed;
use sp_core::{H256, U256};
use sp_runtime::traits::{CheckedAdd, CheckedDiv, CheckedSub, One};
use sp_std::{
    convert::{TryFrom, TryInto},
    prelude::*,
};

// Crates
pub use bitcoin::{self, Address as BtcAddress, PublicKey as BtcPublicKey};
use bitcoin::{
    merkle::{MerkleProof, ProofResult},
    parser::{parse_block_header, parse_transaction},
    types::{BlockChain, BlockHeader, H256Le, RawBlockHeader, Transaction, Value},
    Error as BitcoinError, SetCompact,
};
pub use types::{OpReturnPaymentData, RichBlockHeader};

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::pallet]
    #[pallet::generate_store(trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config + security::Config {
        /// The overarching event type.
        type Event: From<Event<Self>>
            + Into<<Self as frame_system::Config>::Event>
            + IsType<<Self as frame_system::Config>::Event>;

        /// Weight information for the extrinsics in this module.
        type WeightInfo: WeightInfo;

        #[pallet::constant]
        type ParachainBlocksPerBitcoinBlock: Get<<Self as frame_system::Config>::BlockNumber>;
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Verifies the inclusion of `tx_id` into the relay, and validates the given raw Bitcoin transaction, according
        /// to the supported transaction format (see <https://spec.interlay.io/intro/accepted-format.html>)
        ///
        /// # Arguments
        ///
        /// * `raw_merkle_proof` - The raw merkle proof as returned by bitcoin `gettxoutproof`
        /// * `confirmations` - The number of confirmations needed to accept the proof. If `none`, the value stored in
        ///   the StableBitcoinConfirmations storage item is used.
        /// * `raw_tx` - raw Bitcoin transaction
        /// * `expected_btc` - expected amount of BTC (satoshis) sent to the recipient
        /// * `recipient_btc_address` - 20 byte Bitcoin address of recipient of the BTC in the 1st  / payment UTXO
        /// * `op_return_id` - 32 byte hash identifier expected in OP_RETURN (replay protection)
        #[pallet::weight(<T as Config>::WeightInfo::verify_and_validate_transaction())]
        #[transactional]
        pub fn verify_and_validate_transaction(
            origin: OriginFor<T>,
            raw_merkle_proof: Vec<u8>,
            confirmations: Option<u32>,
            raw_tx: Vec<u8>,
            expected_btc: Value,
            recipient_btc_address: BtcAddress,
            op_return_id: Option<H256>,
        ) -> DispatchResultWithPostInfo {
            let _ = ensure_signed(origin)?;

            let transaction = Self::parse_transaction(&raw_tx)?;
            let merkle_proof = Self::parse_merkle_proof(&raw_merkle_proof)?;
            Self::_verify_transaction_inclusion(transaction.tx_id(), merkle_proof, confirmations)?;
            Self::_validate_transaction(transaction, expected_btc, recipient_btc_address, op_return_id)?;
            Ok(().into())
        }

        /// Verifies the inclusion of `tx_id`
        ///
        /// # Arguments
        ///
        /// * `tx_id` - The hash of the transaction to check for
        /// * `raw_merkle_proof` - The raw merkle proof as returned by bitcoin `gettxoutproof`
        /// * `confirmations` - The number of confirmations needed to accept the proof. If `none`, the value stored in
        ///   the `StableBitcoinConfirmations` storage item is used.
        ///
        /// # <weight>
        /// Key: C (len of chains), P (len of positions)
        /// - Storage Reads:
        /// 	- One storage read to check if inclusion check is disabled. O(1)
        /// 	- One storage read to retrieve best block height. O(1)
        /// 	- One storage read to check if transaction is in active fork. O(1)
        /// 	- One storage read to retrieve block header. O(1)
        /// 	- One storage read to check that parachain is not shutdown. O(1)
        /// 	- One storage read to check stable bitcoin confirmations. O(1)
        /// 	- One storage read to check stable parachain confirmations. O(1)
        /// # </weight>
        #[pallet::weight(<T as Config>::WeightInfo::verify_transaction_inclusion())]
        #[transactional]
        pub fn verify_transaction_inclusion(
            origin: OriginFor<T>,
            tx_id: H256Le,
            raw_merkle_proof: Vec<u8>,
            confirmations: Option<u32>,
        ) -> DispatchResultWithPostInfo {
            let _ = ensure_signed(origin)?;

            let merkle_proof = Self::parse_merkle_proof(&raw_merkle_proof)?;
            Self::_verify_transaction_inclusion(tx_id, merkle_proof, confirmations)?;
            Ok(().into())
        }

        /// Validates a given raw Bitcoin transaction, according to the supported transaction
        /// format (see <https://spec.interlay.io/intro/accepted-format.html>)
        /// This DOES NOT check if the transaction is included in a block, nor does it guarantee that the
        /// transaction is fully valid according to the consensus (needs full node).
        ///
        /// # Arguments
        /// * `raw_tx` - raw Bitcoin transaction
        /// * `expected_btc` - expected amount of BTC (satoshis) sent to the recipient
        /// * `recipient_btc_address` - expected Bitcoin address of recipient (p2sh, p2pkh, p2wpkh)
        /// * `op_return_id` - 32 byte hash identifier expected in OP_RETURN (replay protection)
        #[pallet::weight(<T as Config>::WeightInfo::validate_transaction())]
        #[transactional]
        pub fn validate_transaction(
            origin: OriginFor<T>,
            raw_tx: Vec<u8>,
            expected_btc: Value,
            recipient_btc_address: BtcAddress,
            op_return_id: Option<H256>,
        ) -> DispatchResultWithPostInfo {
            let _ = ensure_signed(origin)?;

            let transaction = Self::parse_transaction(&raw_tx)?;

            Self::_validate_transaction(transaction, expected_btc, recipient_btc_address, op_return_id)?;
            Ok(().into())
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        Initialized {
            block_height: u32,
            block_hash: H256Le,
            relayer_id: T::AccountId,
        },
        StoreMainChainHeader {
            block_height: u32,
            block_hash: H256Le,
            relayer_id: T::AccountId,
        },
        StoreForkHeader {
            chain_id: u32,
            fork_height: u32,
            block_hash: H256Le,
            relayer_id: T::AccountId,
        },
        ChainReorg {
            new_chain_tip_hash: H256Le,
            new_chain_tip_height: u32,
            fork_depth: u32,
        },
        ForkAheadOfMainChain {
            main_chain_height: u32,
            fork_height: u32,
            fork_id: u32,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Already initialized
        AlreadyInitialized,
        /// Start height must be start of difficulty period
        InvalidStartHeight,
        /// Missing the block at this height
        MissingBlockHeight,
        /// Invalid block header size
        InvalidHeaderSize,
        /// Block already stored
        DuplicateBlock,
        /// Block already stored and is not head
        OutdatedBlock,
        /// Previous block hash not found
        PrevBlock,
        /// Invalid chain ID
        InvalidChainID,
        /// PoW hash does not meet difficulty target of header
        LowDiff,
        /// Incorrect difficulty target specified in block header
        DiffTargetHeader,
        /// Malformed transaction identifier
        MalformedTxid,
        /// Transaction has less confirmations of Bitcoin blocks than required
        BitcoinConfirmations,
        /// Transaction has less confirmations of Parachain blocks than required
        ParachainConfirmations,
        /// Current fork ongoing
        OngoingFork,
        /// Merkle proof is malformed
        MalformedMerkleProof,
        /// Invalid merkle proof
        InvalidMerkleProof,
        /// BTC Parachain has shut down
        Shutdown,
        /// Transaction hash does not match given txid
        InvalidTxid,
        /// Invalid payment amount
        InvalidPaymentAmount,
        /// Transaction has incorrect format
        MalformedTransaction,
        /// Incorrect recipient Bitcoin address
        InvalidPayment,
        /// Incorrect transaction output format
        InvalidOutputFormat,
        /// Incorrect identifier in OP_RETURN field
        InvalidOpReturn,
        /// Invalid transaction version
        InvalidTxVersion,
        /// Error code not applicable to blocks
        UnknownErrorcode,
        /// Blockchain with requested ID not found
        ForkIdNotFound,
        /// Block header not found for given hash
        BlockNotFound,
        /// Error code already reported
        AlreadyReported,
        /// Unauthorized staked relayer
        UnauthorizedRelayer,
        /// Overflow of chain counter
        ChainCounterOverflow,
        /// Overflow of block height
        BlockHeightOverflow,
        /// Underflow of stored blockchains counter
        ChainsUnderflow,
        /// EndOfFile reached while parsing
        EndOfFile,
        /// Format of the header is invalid
        MalformedHeader,
        /// Invalid block header version
        InvalidBlockVersion,
        /// Format of the BIP141 witness transaction output is invalid
        MalformedWitnessOutput,
        // Format of the P2PKH transaction output is invalid
        MalformedP2PKHOutput,
        // Format of the P2SH transaction output is invalid
        MalformedP2SHOutput,
        /// Format of the OP_RETURN transaction output is invalid
        MalformedOpReturnOutput,
        // Output does not match format of supported output types (Witness, P2PKH, P2SH)
        UnsupportedOutputFormat,
        // Input does not match format of supported input types (Witness, P2PKH, P2SH)
        UnsupportedInputFormat,
        /// User supplied an invalid address
        InvalidBtcHash,
        /// User supplied an invalid script
        InvalidScript,
        /// Specified invalid Bitcoin address
        InvalidBtcAddress,
        /// Arithmetic overflow
        ArithmeticOverflow,
        /// Arithmetic underflow
        ArithmeticUnderflow,
        /// TryInto failed on integer
        TryIntoIntError,
        /// Transaction does meet the requirements to be considered valid
        InvalidTransaction,
        /// Transaction does meet the requirements to be a valid op-return payment
        InvalidOpReturnTransaction,
        /// Invalid compact value in header
        InvalidCompact,
    }

    /// Store Bitcoin block headers
    #[pallet::storage]
    pub(super) type BlockHeaders<T: Config> =
        StorageMap<_, Blake2_128Concat, H256Le, RichBlockHeader<T::BlockNumber>, ValueQuery>;

    /// Priority queue of BlockChain elements, ordered by the maximum height (descending).
    /// The first index into this mapping (0) is considered to be the longest chain. The value
    /// of the entry is the index into `ChainsIndex` to retrieve the `BlockChain`.
    #[pallet::storage]
    pub(super) type Chains<T: Config> = StorageMap<_, Blake2_128Concat, u32, u32>;

    /// Auxiliary mapping of chains ids to `BlockChain` entries. The first index into this
    /// mapping (0) is considered to be the Bitcoin main chain.
    #[pallet::storage]
    pub(super) type ChainsIndex<T: Config> = StorageMap<_, Blake2_128Concat, u32, BlockChain>;

    /// Stores a mapping from (chain_index, block_height) to block hash
    #[pallet::storage]
    pub(super) type ChainsHashes<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, u32, Blake2_128Concat, u32, H256Le, ValueQuery>;

    /// Store the current blockchain tip
    #[pallet::storage]
    pub(super) type BestBlock<T: Config> = StorageValue<_, H256Le, ValueQuery>;

    /// Store the height of the best block
    #[pallet::storage]
    pub(super) type BestBlockHeight<T: Config> = StorageValue<_, u32, ValueQuery>;

    /// BTC height when the relay was initialized
    #[pallet::storage]
    pub(super) type StartBlockHeight<T: Config> = StorageValue<_, u32, ValueQuery>;

    /// Increment-only counter used to track new BlockChain entries
    #[pallet::storage]
    pub(super) type ChainCounter<T: Config> = StorageValue<_, u32, ValueQuery>;

    /// Global security parameter k for stable Bitcoin transactions
    #[pallet::storage]
    #[pallet::getter(fn bitcoin_confirmations)]
    pub(super) type StableBitcoinConfirmations<T: Config> = StorageValue<_, u32, ValueQuery>;

    /// Global security parameter k for stable Parachain transactions
    #[pallet::storage]
    #[pallet::getter(fn parachain_confirmations)]
    pub(super) type StableParachainConfirmations<T: Config> = StorageValue<_, T::BlockNumber, ValueQuery>;

    /// Whether the module should perform difficulty checks.
    #[pallet::storage]
    #[pallet::getter(fn disable_difficulty_check)]
    pub(super) type DisableDifficultyCheck<T: Config> = StorageValue<_, bool, ValueQuery>;

    /// Whether the module should perform inclusion checks.
    #[pallet::storage]
    #[pallet::getter(fn disable_inclusion_check)]
    pub(super) type DisableInclusionCheck<T: Config> = StorageValue<_, bool, ValueQuery>;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        /// Global security parameter k for stable Bitcoin transactions
        pub bitcoin_confirmations: u32,
        /// Global security parameter k for stable Parachain transactions
        pub parachain_confirmations: T::BlockNumber,
        /// Whether the module should perform difficulty checks.
        pub disable_difficulty_check: bool,
        /// Whether the module should perform inclusion checks.
        pub disable_inclusion_check: bool,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                bitcoin_confirmations: Default::default(),
                parachain_confirmations: Default::default(),
                disable_difficulty_check: Default::default(),
                disable_inclusion_check: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            StableBitcoinConfirmations::<T>::put(self.bitcoin_confirmations);
            StableParachainConfirmations::<T>::put(self.parachain_confirmations);
            DisableDifficultyCheck::<T>::put(self.disable_difficulty_check);
            DisableInclusionCheck::<T>::put(self.disable_inclusion_check);
        }
    }
}

/// Difficulty Adjustment Interval
pub const DIFFICULTY_ADJUSTMENT_INTERVAL: u32 = 2016;

/// Target Spacing: 10 minutes (600 seconds)
// https://github.com/bitcoin/bitcoin/blob/5ba5becbb5d8c794efe579caeea7eea64f895a13/src/chainparams.cpp#L78
pub const TARGET_SPACING: u32 = 10 * 60;

/// Accepted maximum number of transaction outputs for validation of redeem/replace/refund
/// See: <https://spec.interlay.io/intro/accepted-format.html#accepted-bitcoin-transaction-format>
pub const ACCEPTED_MAX_TRANSACTION_OUTPUTS: usize = 3;

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

#[cfg_attr(test, mockable)]
impl<T: Config> Pallet<T> {
    pub fn initialize(relayer: T::AccountId, basic_block_header: BlockHeader, block_height: u32) -> DispatchResult {
        // Check if BTC-Relay was already initialized
        ensure!(!Self::best_block_exists(), Error::<T>::AlreadyInitialized);

        // header must be the start of a difficulty period
        ensure!(
            Self::disable_difficulty_check() || block_height % DIFFICULTY_ADJUSTMENT_INTERVAL == 0,
            Error::<T>::InvalidStartHeight
        );

        // construct the BlockChain struct
        Self::create_and_store_blockchain(block_height, &basic_block_header)?;

        // Set BestBlock and BestBlockHeight to the submitted block
        Self::update_chain_head(&basic_block_header, block_height);

        StartBlockHeight::<T>::set(block_height);

        // Emit a Initialized Event
        Self::deposit_event(Event::<T>::Initialized {
            block_height,
            block_hash: basic_block_header.hash,
            relayer_id: relayer,
        });

        Ok(())
    }

    /// wraps _store_block_header, but differentiates between DuplicateError and OutdatedError
    #[transactional]
    pub fn store_block_header(relayer: &T::AccountId, basic_block_header: BlockHeader) -> DispatchResult {
        let ret = Self::_store_block_header(relayer, basic_block_header);
        if let Err(err) = ret {
            if err == DispatchError::from(Error::<T>::DuplicateBlock) {
                // if this is not the chain head, return OutdatedBlock error
                let this_header_hash = basic_block_header.hash;
                let best_header_hash = Self::get_best_block();
                ensure!(this_header_hash == best_header_hash, Error::<T>::OutdatedBlock);
            }
        }
        ret
    }

    fn _store_block_header(relayer: &T::AccountId, basic_block_header: BlockHeader) -> DispatchResult {
        let prev_header = Self::get_block_header_from_hash(basic_block_header.hash_prev_block)?;

        // check if the prev block is the highest block in the chain
        // load the previous block header block height
        let prev_block_height = prev_header.block_height;

        // update the current block header with height and chain ref
        // Set the height of the block header
        let current_block_height = prev_block_height.checked_add(1).ok_or(Error::<T>::ArithmeticOverflow)?;

        // get the block chain of the previous header
        let prev_blockchain = Self::get_block_chain_from_id(prev_header.chain_id)?;

        // ensure the block header is valid
        Self::verify_block_header(&basic_block_header, current_block_height, prev_header)?;

        // Update the blockchain
        // check if we create a new blockchain or extend the existing one
        runtime_print!("Prev max height: {:?}", prev_blockchain.max_height);
        runtime_print!("Prev block height: {:?}", prev_block_height);
        let is_new_fork = prev_blockchain.max_height != prev_block_height;
        runtime_print!("Fork detected: {:?}", is_new_fork);

        let chain_id = if is_new_fork {
            // create new blockchain element
            Self::create_and_store_blockchain(current_block_height, &basic_block_header)?
        } else {
            // extend the current chain
            let blockchain = Self::extend_blockchain(current_block_height, &basic_block_header, prev_blockchain)?;

            // Update the pointer to BlockChain in ChainsIndex
            // todo: remove - this is already done in extend_blockchain
            ChainsIndex::<T>::mutate(blockchain.chain_id, |_b| &blockchain);

            if blockchain.chain_id != MAIN_CHAIN_ID {
                // if we added a block to a fork, we may need to reorder the chains
                Self::reorganize_chains(&blockchain)?;
            } else {
                Self::update_chain_head(&basic_block_header, current_block_height);
            }
            blockchain.chain_id
        };

        // Determine if this block extends the main chain or a fork
        let current_best_block = Self::get_best_block();

        if current_best_block == basic_block_header.hash {
            // extends the main chain
            Self::deposit_event(Event::<T>::StoreMainChainHeader {
                block_height: current_block_height,
                block_hash: basic_block_header.hash,
                relayer_id: relayer.clone(),
            });
        } else {
            // created a new fork or updated an existing one
            Self::deposit_event(Event::<T>::StoreForkHeader {
                chain_id,
                fork_height: current_block_height,
                block_hash: basic_block_header.hash,
                relayer_id: relayer.clone(),
            });
        };

        Ok(())
    }

    pub fn parse_raw_block_header(raw_block_header: &RawBlockHeader) -> Result<BlockHeader, DispatchError> {
        Ok(parse_block_header(raw_block_header).map_err(Error::<T>::from)?)
    }

    // helper for the dispatchable
    fn _validate_transaction(
        transaction: Transaction,
        expected_btc: Value,
        recipient_btc_address: BtcAddress,
        op_return_id: Option<H256>,
    ) -> Result<(), DispatchError> {
        match op_return_id {
            Some(op_return) => {
                Self::validate_op_return_transaction(transaction, recipient_btc_address, expected_btc, op_return)?;
            }
            None => {
                let payment = Self::get_issue_payment::<i64>(transaction, recipient_btc_address)?;
                ensure!(payment.1 == expected_btc, Error::<T>::InvalidPaymentAmount);
            }
        };
        Ok(())
    }

    /// interface to the issue pallet; verifies inclusion, and returns the first input
    /// address (for refunds) and the payment amount
    pub fn get_and_verify_issue_payment<V: TryFrom<Value>>(
        merkle_proof: MerkleProof,
        transaction: Transaction,
        recipient_btc_address: BtcAddress,
    ) -> Result<(BtcAddress, V), DispatchError> {
        // Verify that the transaction is indeed included in the main chain
        Self::_verify_transaction_inclusion(transaction.tx_id(), merkle_proof, None)?;

        Self::get_issue_payment(transaction, recipient_btc_address)
    }

    fn get_issue_payment<V: TryFrom<i64>>(
        transaction: Transaction,
        recipient_btc_address: BtcAddress,
    ) -> Result<(BtcAddress, V), DispatchError> {
        let input_address = transaction
            .inputs
            .get(0)
            .ok_or(Error::<T>::MalformedTransaction)?
            .extract_address()
            .map_err(|_| Error::<T>::MalformedTransaction)?;

        // using the on-chain key derivation scheme we only expect a simple
        // payment to the vault's new deposit address
        let extr_payment_value = transaction
            .outputs
            .into_iter()
            .find_map(|x| match x.extract_address() {
                Ok(address) if address == recipient_btc_address => Some(x.value),
                _ => None,
            })
            .ok_or(Error::<T>::MalformedTransaction)?
            .try_into()
            .map_err(|_| Error::<T>::InvalidPaymentAmount)?;

        Ok((input_address, extr_payment_value))
    }

    /// interface to redeem,replace,refund to check that the payment is included and is valid
    pub fn verify_and_validate_op_return_transaction<V: TryInto<Value>>(
        merkle_proof: MerkleProof,
        transaction: Transaction,
        recipient_btc_address: BtcAddress,
        expected_btc: V,
        op_return_id: H256,
    ) -> Result<(), DispatchError> {
        // Verify that the transaction is indeed included in the main chain
        Self::_verify_transaction_inclusion(transaction.tx_id(), merkle_proof, None)?;

        // Parse transaction and check that it matches the given parameters
        Self::validate_op_return_transaction(transaction, recipient_btc_address, expected_btc, op_return_id)?;
        Ok(())
    }

    pub fn _verify_transaction_inclusion(
        tx_id: H256Le,
        merkle_proof: MerkleProof,
        confirmations: Option<u32>,
    ) -> Result<(), DispatchError> {
        if Self::disable_inclusion_check() {
            return Ok(());
        }
        let proof_result = Self::verify_merkle_proof(&merkle_proof)?;

        let block_hash = merkle_proof.block_header.hash;
        let stored_block_header = Self::verify_block_header_inclusion(block_hash, confirmations)?;

        // fail if the transaction hash is invalid
        ensure!(proof_result.transaction_hash == tx_id, Error::<T>::InvalidTxid);

        // fail if the merkle root is invalid
        ensure!(
            proof_result.extracted_root == stored_block_header.merkle_root,
            Error::<T>::InvalidMerkleProof
        );

        Ok(())
    }

    pub fn verify_block_header_inclusion(
        block_hash: H256Le,
        confirmations: Option<u32>,
    ) -> Result<BlockHeader, DispatchError> {
        let best_block_height = Self::get_best_block_height();
        Self::ensure_no_ongoing_fork(best_block_height)?;

        let rich_header = Self::get_block_header_from_hash(block_hash)?;

        ensure!(rich_header.chain_id == MAIN_CHAIN_ID, Error::<T>::InvalidChainID);

        let block_height = rich_header.block_height;

        // This call fails if not enough confirmations
        Self::check_bitcoin_confirmations(best_block_height, confirmations, block_height)?;

        // This call fails if the block was stored too recently
        Self::check_parachain_confirmations(rich_header.para_height)?;

        Ok(rich_header.block_header)
    }

    /// Checks if transaction is valid. Returns the return-to-self address, if any, for theft checking purposes
    fn validate_op_return_transaction<V: TryInto<i64>>(
        transaction: Transaction,
        recipient_btc_address: BtcAddress,
        expected_btc: V,
        op_return_id: H256,
    ) -> Result<Option<BtcAddress>, DispatchError> {
        let payment_data = OpReturnPaymentData::<T>::try_from(transaction)?;
        payment_data.ensure_valid_payment_to(
            expected_btc.try_into().map_err(|_| Error::<T>::InvalidPaymentAmount)?,
            recipient_btc_address,
            Some(op_return_id),
        )
    }

    pub fn is_fully_initialized() -> Result<bool, DispatchError> {
        if !StartBlockHeight::<T>::exists() {
            return Ok(false);
        }

        let required_height = StartBlockHeight::<T>::get()
            .checked_add(StableBitcoinConfirmations::<T>::get())
            .ok_or(Error::<T>::ArithmeticOverflow)?;
        let best = BestBlockHeight::<T>::get();
        Ok(best >= required_height)
    }

    pub fn has_request_expired(
        opentime: T::BlockNumber,
        btc_open_height: u32,
        period: T::BlockNumber,
    ) -> Result<bool, DispatchError> {
        Ok(ext::security::parachain_block_expired::<T>(opentime, period)?
            && Self::bitcoin_block_expired(btc_open_height, period)?)
    }

    pub fn bitcoin_expiry_height(btc_open_height: u32, period: T::BlockNumber) -> Result<u32, DispatchError> {
        // calculate num_bitcoin_blocks as ceil(period / ParachainBlocksPerBitcoinBlock)
        let num_bitcoin_blocks: u32 = period
            .checked_add(&T::ParachainBlocksPerBitcoinBlock::get())
            .ok_or(Error::<T>::ArithmeticOverflow)?
            .checked_sub(&T::BlockNumber::one())
            .ok_or(Error::<T>::ArithmeticUnderflow)?
            .checked_div(&T::ParachainBlocksPerBitcoinBlock::get())
            .ok_or(Error::<T>::ArithmeticUnderflow)?
            .try_into()
            .map_err(|_| Error::<T>::TryIntoIntError)?;

        Ok(btc_open_height
            .checked_add(num_bitcoin_blocks)
            .ok_or(Error::<T>::ArithmeticOverflow)?)
    }

    pub fn bitcoin_block_expired(btc_open_height: u32, period: T::BlockNumber) -> Result<bool, DispatchError> {
        let expiration_height = Self::bitcoin_expiry_height(btc_open_height, period)?;

        // Note that we check stictly greater than. This ensures that at least
        // `num_bitcoin_blocks` FULL periods have expired.
        Ok(Self::get_best_block_height() > expiration_height)
    }

    // ********************************
    // START: Storage getter functions
    // ********************************

    /// Get chain id from position (sorted by max block height)
    fn get_chain_id_from_position(position: u32) -> Result<u32, DispatchError> {
        Chains::<T>::get(position).ok_or(Error::<T>::InvalidChainID.into())
    }

    /// Get the position of the fork in Chains
    fn get_chain_position_from_chain_id(chain_id: u32) -> Result<u32, DispatchError> {
        for (k, v) in Chains::<T>::iter() {
            if v == chain_id {
                return Ok(k);
            }
        }
        Err(Error::<T>::ForkIdNotFound.into())
    }

    /// Get a blockchain from the id
    fn get_block_chain_from_id(chain_id: u32) -> Result<BlockChain, DispatchError> {
        ChainsIndex::<T>::get(chain_id).ok_or(Error::<T>::InvalidChainID.into())
    }

    /// Get the current best block hash
    pub fn get_best_block() -> H256Le {
        BestBlock::<T>::get()
    }

    /// Check if a best block hash is set
    fn best_block_exists() -> bool {
        BestBlock::<T>::exists()
    }

    /// get the best block height
    pub fn get_best_block_height() -> u32 {
        BestBlockHeight::<T>::get()
    }

    /// Get the current chain counter
    fn get_chain_counter() -> u32 {
        ChainCounter::<T>::get()
    }

    /// Get a block hash from a blockchain
    ///
    /// # Arguments
    ///
    /// * `chain_id`: the id of the blockchain to search in
    /// * `block_height`: the height of the block header
    fn get_block_hash(chain_id: u32, block_height: u32) -> Result<H256Le, DispatchError> {
        if !Self::block_exists(chain_id, block_height) {
            return Err(Error::<T>::MissingBlockHeight.into());
        }
        Ok(ChainsHashes::<T>::get(chain_id, block_height))
    }

    /// Get a block header from its hash
    fn get_block_header_from_hash(block_hash: H256Le) -> Result<RichBlockHeader<T::BlockNumber>, DispatchError> {
        BlockHeaders::<T>::try_get(block_hash).or(Err(Error::<T>::BlockNotFound.into()))
    }

    /// Check if a block header exists
    pub fn block_header_exists(block_hash: H256Le) -> bool {
        BlockHeaders::<T>::contains_key(block_hash)
    }

    /// Get a block header from
    fn get_block_header_from_height(
        blockchain: &BlockChain,
        block_height: u32,
    ) -> Result<RichBlockHeader<T::BlockNumber>, DispatchError> {
        let block_hash = Self::get_block_hash(blockchain.chain_id, block_height)?;
        Self::get_block_header_from_hash(block_hash)
    }

    /// Storage setter functions
    /// Set a new chain with position and id
    fn set_chain_from_position_and_id(position: u32, id: u32) {
        Chains::<T>::insert(position, id);
    }

    /// Swap chain elements
    fn swap_chain(pos_1: u32, pos_2: u32) {
        // swaps the values of two keys
        Chains::<T>::swap(pos_1, pos_2)
    }

    /// Set a new blockchain in ChainsIndex
    fn set_block_chain_from_id(id: u32, chain: &BlockChain) {
        ChainsIndex::<T>::insert(id, &chain);
    }

    /// Update a blockchain in ChainsIndex
    fn mutate_block_chain_from_id(id: u32, chain: BlockChain) {
        ChainsIndex::<T>::mutate(id, |b| *b = Some(chain));
    }

    /// Set a new block header
    fn set_block_header_from_hash(hash: H256Le, header: &RichBlockHeader<T::BlockNumber>) {
        BlockHeaders::<T>::insert(hash, header);
    }

    /// Set a new best block
    fn set_best_block(hash: H256Le) {
        BestBlock::<T>::put(hash);
    }

    /// Set a new best block height
    fn set_best_block_height(height: u32) {
        BestBlockHeight::<T>::put(height);
    }

    /// Set a new chain counter
    fn increment_chain_counter() -> Result<u32, DispatchError> {
        let ret = Self::get_chain_counter();
        let next_value = ret.checked_add(1).ok_or(Error::<T>::ArithmeticOverflow)?;
        ChainCounter::<T>::put(next_value);

        Ok(ret)
    }

    /// Create a new blockchain element with a new chain id
    fn create_and_store_blockchain(block_height: u32, basic_block_header: &BlockHeader) -> Result<u32, DispatchError> {
        // get a new chain id
        let chain_id = Self::increment_chain_counter()?;

        // generate an empty blockchain
        let blockchain = Self::generate_blockchain(chain_id, block_height, basic_block_header.hash);

        // Store a pointer to BlockChain in ChainsIndex
        Self::set_block_chain_from_id(blockchain.chain_id, &blockchain);

        // Store the reference to the blockchain in Chains
        Self::insert_sorted(&blockchain)?;

        Self::store_rich_header(basic_block_header.clone(), block_height, blockchain.chain_id);

        Ok(blockchain.chain_id)
    }

    /// Generate the raw blockchain from a chain Id and with a single block
    fn generate_blockchain(chain_id: u32, block_height: u32, block_hash: H256Le) -> BlockChain {
        // initialize an empty chain

        Self::insert_block_hash(chain_id, block_height, block_hash);

        BlockChain {
            chain_id,
            start_height: block_height,
            max_height: block_height,
        }
    }

    fn insert_block_hash(chain_id: u32, block_height: u32, block_hash: H256Le) {
        ChainsHashes::<T>::insert(chain_id, block_height, block_hash);
    }

    fn block_exists(chain_id: u32, block_height: u32) -> bool {
        ChainsHashes::<T>::contains_key(chain_id, block_height)
    }

    /// Add a new block header to an existing blockchain
    fn extend_blockchain(
        block_height: u32,
        basic_block_header: &BlockHeader,
        prev_blockchain: BlockChain,
    ) -> Result<BlockChain, DispatchError> {
        let mut blockchain = prev_blockchain;

        if Self::block_exists(blockchain.chain_id, block_height) {
            return Err(Error::<T>::DuplicateBlock.into());
        }
        Self::insert_block_hash(blockchain.chain_id, block_height, basic_block_header.hash);

        blockchain.max_height = block_height;
        Self::set_block_chain_from_id(blockchain.chain_id, &blockchain);

        Self::store_rich_header(basic_block_header.clone(), block_height, blockchain.chain_id);

        Ok(blockchain)
    }

    // Get require conformations for stable transactions
    fn get_stable_transaction_confirmations() -> u32 {
        Self::bitcoin_confirmations()
    }

    // *********************************
    // END: Storage getter functions
    // *********************************

    // Wrapper functions around bitcoin lib for testing purposes
    pub fn parse_transaction(raw_tx: &[u8]) -> Result<Transaction, DispatchError> {
        Ok(parse_transaction(&raw_tx).map_err(Error::<T>::from)?)
    }

    pub fn parse_merkle_proof(raw_merkle_proof: &[u8]) -> Result<MerkleProof, DispatchError> {
        MerkleProof::parse(&raw_merkle_proof).map_err(|err| Error::<T>::from(err).into())
    }

    fn verify_merkle_proof(merkle_proof: &MerkleProof) -> Result<ProofResult, DispatchError> {
        merkle_proof.verify_proof().map_err(|err| Error::<T>::from(err).into())
    }

    /// Parses and verifies a raw Bitcoin block header.
    ///
    /// # Arguments
    ///
    /// * `block_header` - block header
    /// * `block_height` - Height of the new block
    /// * `prev_block_header` - the previous block header in the chain
    ///
    /// # Returns
    ///
    /// * `Ok(())` iff header is valid
    fn verify_block_header(
        block_header: &BlockHeader,
        block_height: u32,
        prev_block_header: RichBlockHeader<T::BlockNumber>,
    ) -> Result<(), DispatchError> {
        // Check that the block header is not yet stored in BTC-Relay
        ensure!(
            !Self::block_header_exists(block_header.hash),
            Error::<T>::DuplicateBlock
        );

        // Check that the PoW hash satisfies the target set in the block header
        ensure!(block_header.hash.as_u256() < block_header.target, Error::<T>::LowDiff);

        if Self::disable_difficulty_check() {
            return Ok(());
        }

        let expected_target =
            if block_height >= DIFFICULTY_ADJUSTMENT_INTERVAL && block_height % DIFFICULTY_ADJUSTMENT_INTERVAL == 0 {
                Self::compute_new_target(&prev_block_header, block_height)?
            } else {
                prev_block_header.block_header.target
            };

        ensure!(block_header.target == expected_target, Error::<T>::DiffTargetHeader);

        Ok(())
    }

    /// Computes Bitcoin's PoW retarget algorithm for a given block height
    ///
    /// # Arguments
    ///
    /// * `prev_block_header`: previous block header
    /// * `block_height` : block height of new target
    fn compute_new_target(
        prev_block_header: &RichBlockHeader<T::BlockNumber>,
        block_height: u32,
    ) -> Result<U256, DispatchError> {
        // time of last retarget (first block in current difficulty period)
        let first_block_time = Self::get_last_retarget_time(prev_block_header.chain_id, block_height)?;
        let last_block_time = prev_block_header.block_header.timestamp as u64;
        let previous_target = prev_block_header.block_header.target;

        // compute new target
        Ok(U256::set_compact(
            bitcoin::pow::calculate_next_work_required(previous_target, first_block_time, last_block_time)
                .map_err(Error::<T>::from)?,
        )
        .ok_or(Error::<T>::InvalidCompact)?)
    }

    /// Returns the timestamp of the last difficulty retarget on the specified BlockChain, given the current block
    /// height
    ///
    /// # Arguments
    ///
    /// * `chain_id` - BlockChain identifier
    /// * `block_height` - current block height
    fn get_last_retarget_time(chain_id: u32, block_height: u32) -> Result<u64, DispatchError> {
        let block_chain = Self::get_block_chain_from_id(chain_id)?;
        let period_start_height = block_height - DIFFICULTY_ADJUSTMENT_INTERVAL;
        let last_retarget_header = Self::get_block_header_from_height(&block_chain, period_start_height)?;
        Ok(last_retarget_header.block_header.timestamp as u64)
    }

    /// Swap the main chain with a fork. The fork is not necessarily a direct fork of the main
    /// chain - it can be a fork of another fork. As such, this function iterates over (child-parent)
    /// pairs, starting at the latest block, until the main chain is reached. All intermediate forks
    /// that the iteration passes through are updated: their start_height is increased appropriately.
    /// Each block header that is iterated over is moved to the main chain. Then, any blocks that used
    /// to be in the main-chain, but that are being replaced by the fork, are moved into the fork that
    /// overtook the mainchain.The start_height and max_height of the mainchain and the fork are updated
    /// appropriately. Finally, the best_block and best_block_height are updated.
    ///
    /// # Arguments
    ///
    /// * `fork` - the fork that is going to become the main chain
    ///
    /// # Returns
    ///
    /// Ok((best_block_hash, best_block_height)) if successful, Err otherwise
    fn swap_main_blockchain(fork: &BlockChain) -> Result<(H256Le, u32), DispatchError> {
        let new_best_block = Self::get_block_hash(fork.chain_id, fork.max_height)?;

        // Set BestBlock and BestBlockHeight to the submitted block
        Self::set_best_block(new_best_block);
        Self::set_best_block_height(fork.max_height);

        // traverse (child-parent) links until the mainchain is reached
        for pair in Self::enumerate_chain_links(new_best_block) {
            let (child, parent) = pair?;

            // if we reached a different fork, we need to update it
            if parent.chain_id != child.chain_id {
                let mut new_fork = Self::get_block_chain_from_id(parent.chain_id)?;
                // update the start height of the parent's chain. There is guaranteed to be at least
                // one block in this chain that is not becoming part of the new main chain, because
                // if there were not, then the child would have been added directly to this chain,
                // rather than creating a new fork.
                new_fork.start_height = parent
                    .block_height
                    .checked_add(1)
                    .ok_or(Error::<T>::ArithmeticOverflow)?;
                // If we reached the main chain, we store the remaining forked old main chain where
                // the longest chain used to be.
                if parent.chain_id == MAIN_CHAIN_ID {
                    new_fork.chain_id = fork.chain_id;
                }

                // update the storage
                Self::mutate_block_chain_from_id(new_fork.chain_id, new_fork.clone());
            }

            // transfer the child block to the main chain, and if main already has a block at this
            // height, transfer it to `fork`
            Self::swap_block_to_mainchain(child, fork.chain_id)?;

            // main chain reached, stop iterating
            if parent.chain_id == MAIN_CHAIN_ID {
                break;
            }
        }

        // update the max_height of main chain
        Self::mutate_block_chain_from_id(
            MAIN_CHAIN_ID,
            BlockChain {
                max_height: fork.max_height,
                ..Self::get_block_chain_from_id(MAIN_CHAIN_ID)?
            },
        );

        // we swapped main chain and `fork`, so it will need to be resorted. The new max_height of this fork
        // is strictly smaller than before, so do a single bubble sort pass to the right
        let start = Self::get_chain_position_from_chain_id(fork.chain_id)?;
        // ideally we'd iterate over start..Chains::<T>::len(), but unfortunately Chains does not implement
        // len, so we resort to making the outer loop infinite, and break when the next key does not exist.
        // This works because we maintain an invariant that states that keys in `Chains` are consecutive.
        for i in start..u32::MAX - 1 {
            // this is the last fork - we can stop iterating now
            if !Chains::<T>::contains_key(i + 1) {
                break;
            }

            let height1 = Self::get_block_chain_from_id(Self::get_chain_id_from_position(i)?)?.max_height;
            let height2 = Self::get_block_chain_from_id(Self::get_chain_id_from_position(i + 1)?)?.max_height;
            if height1 < height2 {
                Self::swap_chain(i, i + 1);
            } else {
                break;
            }
        }

        Ok((new_best_block, fork.max_height))
    }

    /// Transfers the given block to the main chain. If this would overwrite a block already in the
    /// main chain, then the overwritten block is moved to to `chain_id_for_old_main_blocks`.
    fn swap_block_to_mainchain(
        block: RichBlockHeader<T::BlockNumber>,
        chain_id_for_old_main_blocks: u32,
    ) -> Result<(), DispatchError> {
        let block_height = block.block_height;

        // store the hash we will overwrite, if it exists
        let replaced_block_hash = ChainsHashes::<T>::try_get(MAIN_CHAIN_ID, block_height);

        // remove from old chain and insert into the new one
        ChainsHashes::<T>::remove(block.chain_id, block_height);
        ChainsHashes::<T>::insert(MAIN_CHAIN_ID, block_height, block.block_header.hash);

        // update the chainref of the block
        BlockHeaders::<T>::mutate(&block.block_header.hash, |header| header.chain_id = MAIN_CHAIN_ID);

        // if there was a block at block_height in the mainchain, we need to move it
        if let Ok(replaced_block_hash) = replaced_block_hash {
            ChainsHashes::<T>::insert(chain_id_for_old_main_blocks, block_height, replaced_block_hash);
            BlockHeaders::<T>::mutate(&replaced_block_hash, |header| {
                header.chain_id = chain_id_for_old_main_blocks
            });
        }

        Ok(())
    }

    // returns (child, parent)
    fn enumerate_chain_links(
        start: H256Le,
    ) -> impl Iterator<Item = Result<(RichBlockHeader<T::BlockNumber>, RichBlockHeader<T::BlockNumber>), DispatchError>>
    {
        let child = Self::get_block_header_from_hash(start);

        let first = match child {
            Ok(child_block) => match Self::get_block_header_from_hash(child_block.block_header.hash_prev_block) {
                Ok(parent) => Some(Ok((child_block, parent))),
                Err(e) => Some(Err(e)),
            },
            Err(e) => Some(Err(e)),
        };

        sp_std::iter::successors(first, |prev| match prev {
            Ok((_, child)) => match Self::get_block_header_from_hash(child.block_header.hash_prev_block) {
                Err(e) => Some(Err(e)),
                Ok(parent) => Some(Ok((child.clone(), parent.clone()))),
            },
            Err(_) => None,
        })
    }
    /// Checks if a newly inserted fork results in an update to the sorted
    /// Chains mapping. This happens when the max height of the fork is greater
    /// than the max height of the previous element in the Chains mapping.
    ///
    /// # Arguments
    ///
    /// * `fork` - the blockchain element that may cause a reorg
    fn reorganize_chains(fork: &BlockChain) -> Result<(), DispatchError> {
        // get the position of the fork in Chains
        let fork_position: u32 = Self::get_chain_position_from_chain_id(fork.chain_id)?;
        // check if the previous element in Chains has a lower block_height
        let mut current_position = fork_position;
        let mut current_height = fork.max_height;

        // swap elements as long as previous block height is smaller
        while current_position > 0 {
            // get the previous position
            let prev_position = current_position - 1;
            // get the blockchain id
            let prev_blockchain_id = if let Ok(chain_id) = Self::get_chain_id_from_position(prev_position) {
                chain_id
            } else {
                // swap chain positions if previous doesn't exist and retry
                Self::swap_chain(prev_position, current_position);
                continue;
            };

            // get the previous blockchain height
            let prev_height = Self::get_block_chain_from_id(prev_blockchain_id)?.max_height;
            // swap elements if block height is greater
            if prev_height < current_height {
                // Check if swap occurs on the main chain element
                if prev_blockchain_id == MAIN_CHAIN_ID {
                    // if the previous position is the top element
                    // and the current height is more than the
                    // STABLE_TRANSACTION_CONFIRMATIONS ahead
                    // we are swapping the main chain
                    if prev_height + Self::get_stable_transaction_confirmations() <= current_height {
                        // Swap the mainchain. As an optimization, this function returns the
                        // new best block hash and its height
                        let (new_chain_tip_hash, new_chain_tip_height) = Self::swap_main_blockchain(&fork)?;

                        // announce the new main chain
                        let fork_depth = fork.max_height - fork.start_height;
                        Self::deposit_event(Event::<T>::ChainReorg {
                            new_chain_tip_hash,
                            new_chain_tip_height,
                            fork_depth,
                        });
                    } else {
                        Self::deposit_event(Event::<T>::ForkAheadOfMainChain {
                            main_chain_height: prev_height,
                            fork_height: fork.max_height,
                            fork_id: fork.chain_id,
                        });
                    }
                    // successful reorg
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
    fn insert_sorted(blockchain: &BlockChain) -> Result<(), DispatchError> {
        // get a sorted vector over the Chains elements
        // NOTE: LinkedStorageMap iterators are not sorted over the keys
        let mut chains = Chains::<T>::iter().collect::<Vec<(u32, u32)>>();
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
        for curr_position in (position_blockchain + 1..max_chain_element + 1).rev() {
            // TODO: this is a useless check
            // stop when the blockchain element is at it's
            // designated position
            if curr_position < position_blockchain {
                break;
            };
            let prev_position = curr_position - 1;
            // swap the current element with the previous one
            Self::swap_chain(curr_position, prev_position);
        }
        Ok(())
    }

    /// Checks if the given transaction confirmations are greater/equal to the
    /// requested confirmations (and/or the global k security parameter)
    ///
    /// # Arguments
    ///
    /// * `block_height` - current main chain block height
    /// * `confirmations` - The number of confirmations requested. If `none`,
    /// the value stored in the StableBitcoinConfirmations storage item is used.
    /// * `tx_block_height` - block height of checked transaction
    pub fn check_bitcoin_confirmations(
        main_chain_height: u32,
        req_confs: Option<u32>,
        tx_block_height: u32,
    ) -> Result<(), DispatchError> {
        let required_confirmations = req_confs.unwrap_or_else(Self::get_stable_transaction_confirmations);

        let required_mainchain_height = tx_block_height
            .checked_add(required_confirmations)
            .ok_or(Error::<T>::ArithmeticOverflow)?
            .checked_sub(1)
            .unwrap_or_default();

        if main_chain_height >= required_mainchain_height {
            Ok(())
        } else {
            Err(Error::<T>::BitcoinConfirmations.into())
        }
    }

    /// Checks if the given bitcoin block has been stored for a sufficient
    /// amount of blocks. This should give sufficient time for staked relayers
    /// to flag potentially invalid blocks.
    ///
    /// # Arguments
    /// * `para_height` - height of the parachain when the block was stored
    pub fn check_parachain_confirmations(para_height: T::BlockNumber) -> Result<(), DispatchError> {
        let current_height = ext::security::active_block_number::<T>();

        ensure!(
            para_height + Self::parachain_confirmations() <= current_height,
            Error::<T>::ParachainConfirmations
        );

        Ok(())
    }

    fn ensure_no_ongoing_fork(best_block_height: u32) -> Result<(), DispatchError> {
        // check if there is a next best fork
        match Self::get_chain_id_from_position(1) {
            // if yes, check that the main chain is at least Self::confirmations() ahead
            Ok(id) => {
                let next_best_fork_height = Self::get_block_chain_from_id(id)?.max_height;

                runtime_print!("Best block height: {}", best_block_height);
                runtime_print!("Next best fork height: {}", next_best_fork_height);
                // fail if there is an ongoing fork
                ensure!(
                    best_block_height >= next_best_fork_height + Self::get_stable_transaction_confirmations(),
                    Error::<T>::OngoingFork
                );
            }
            // else, do nothing if there is no fork
            Err(_) => {}
        }
        Ok(())
    }

    fn store_rich_header(basic_block_header: BlockHeader, block_height: u32, chain_id: u32) {
        let para_height = ext::security::active_block_number::<T>();
        let block_header = RichBlockHeader::new(basic_block_header, chain_id, block_height, para_height);
        Self::set_block_header_from_hash(basic_block_header.hash, &block_header);
    }

    fn update_chain_head(basic_block_header: &BlockHeader, block_height: u32) {
        Self::set_best_block(basic_block_header.hash);
        Self::set_best_block_height(block_height);
    }

    /// For internal testing
    pub fn set_disable_difficulty_check(disabled: bool) {
        DisableDifficultyCheck::<T>::put(disabled);
    }
}

impl<T: Config> From<BitcoinError> for Error<T> {
    fn from(err: BitcoinError) -> Self {
        match err {
            BitcoinError::MalformedMerkleProof => Self::MalformedMerkleProof,
            BitcoinError::InvalidMerkleProof => Self::InvalidMerkleProof,
            BitcoinError::EndOfFile => Self::EndOfFile,
            BitcoinError::MalformedHeader => Self::MalformedHeader,
            BitcoinError::InvalidBlockVersion => Self::InvalidBlockVersion,
            BitcoinError::MalformedTransaction => Self::MalformedTransaction,
            BitcoinError::UnsupportedInputFormat => Self::UnsupportedInputFormat,
            BitcoinError::MalformedWitnessOutput => Self::MalformedWitnessOutput,
            BitcoinError::MalformedP2PKHOutput => Self::MalformedP2PKHOutput,
            BitcoinError::MalformedP2SHOutput => Self::MalformedP2SHOutput,
            BitcoinError::UnsupportedOutputFormat => Self::UnsupportedOutputFormat,
            BitcoinError::MalformedOpReturnOutput => Self::MalformedOpReturnOutput,
            BitcoinError::InvalidHeaderSize => Self::InvalidHeaderSize,
            BitcoinError::InvalidBtcHash => Self::InvalidBtcHash,
            BitcoinError::InvalidScript => Self::InvalidScript,
            BitcoinError::InvalidBtcAddress => Self::InvalidBtcAddress,
            BitcoinError::ArithmeticOverflow => Self::ArithmeticOverflow,
            BitcoinError::ArithmeticUnderflow => Self::ArithmeticUnderflow,
            BitcoinError::InvalidCompact => Self::InvalidCompact,
        }
    }
}
