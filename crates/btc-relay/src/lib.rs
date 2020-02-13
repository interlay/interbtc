#![cfg_attr(not(feature = "std"), no_std)]

/// For more guidance on FRAME pallets, see the example.
/// https://github.com/paritytech/substrate/blob/master/frame/example/src/lib.rs

/// # BTC-Relay implementation
/// This is the implementation of the BTC-Relay following the spec at:
/// https://interlay.gitlab.io/polkabtc-spec/btcrelay-spec/

use frame_support::{decl_module, decl_storage, decl_event, decl_error, dispatch::DispatchResult, ensure};
use {system::ensure_signed, timestamp};
use sp_core::{U256, H256, H160};
use codec::{Encode, Decode};

/// ## Custom Types
/// Bitcoin Raw Block Header type
pub type RawBlockHeader = [u8; 80];

/// ## Configuration and Constants
/// The pallet's configuration trait.
/// For further reference, see: 
/// https://interlay.gitlab.io/polkabtc-spec/btcrelay-spec/spec/data-model.html
pub trait Trait: timestamp::Trait + system::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    
    /// Difficulty Adjustment Interval
    const DIFFICULTY_ADJUSTMENT_INTERVAL: u16 = 2016;

    /// Target Timespan
    const TARGET_TIMESPAN: u64 = 1209600;
    
    /// Unrounded Maximum Target
    /// 0x00000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
    const UNROUNDED_MAX_TARGET: U256 = U256([0x00000000ffffffffu64, <u64>::max_value(), <u64>::max_value(), <u64>::max_value()]);
}


/// ## Structs
/// Bitcoin Block Headers
// TODO: Figure out how to set a pointer to the ChainIndex mapping instead
#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct BlockHeader<U256, H256, Moment> {
      block_height: U256,
      merkle_root: H256,
      target: U256,
      timestamp: Moment,
      chain_ref: U256,
      no_data: bool,
      invalid: bool,
      // Optional fields
      version: u32,
      hash_prev_lock: H256,
      nonce: u32
}

/// Representation of a Bitcoin blockchain
// Note: the chain representation is for now a vector
// TODO: ask if there is a "mapping" type in structs
#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct BlockChain<U256, H256> {
    chain_id: U256,
    chain: Vec<H256>,
    max_height: U256,
    no_data: bool,
    invalid: bool,
}


// This pallet's storage items.
decl_storage! {
	trait Store for Module<T: Trait> as BTCRelay {
    /// ## Storage
        /// Store Bitcoin block headers
        BlockHeaders get(fn blockheader): map H256 => BlockHeader<U256, H256, T::Moment>;
        
        // TODO: Chains implementation with priority queue
        /// Priority queue of BlockChain elements
        Chains get(fn chain): Vec<BlockChain<U256, H256>>;

        /// Store the index for each tracked blockchain
        ChainsIndex get(fn chainindex): map U256 => BlockChain<U256, H256>;
        
        /// Store the current blockchain tip
        BestBlock get(fn bestblock): H256;

        /// Store the height of the best block
        BestBlockHeight get(fn bestblockheight): U256;

        /// Track existing BlockChain entries
        ChainCounter get(fn chaincounter): U256;
		// Just a dummy storage item.
		// Here we are declaring a StorageValue, `Something` as a Option<u32>
		// `get(fn something)` is the default getter which returns either the stored `u32` or `None` if nothing stored
		// Something get(fn something): Option<u32>;
	}
}

// The pallet's dispatchable functions.
decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// Initializing events
		// this is needed only if you are using events in your pallet
		fn deposit_event() = default;
        
        // Initialize errors
        type Error = Error<T>;

        fn initialize(
            origin,
            //block_header_bytes: RawBlockHeader,
            block_height: U256) 
            // -> Result<U256, Error<T>> {
        {
            let sender = ensure_signed(origin)?;
            
            // Check if BTC-Relay was already initialized
            let bestblock = Self::bestblock();
            ensure!(bestblock.is_zero(), Error::<T>::AlreadyInitialized);
                        
            // Parse the block header bytes to extract the required info
            let merkle_root: H256 = H256::zero();
            let timestamp: T::Moment;
            let n_bits_to_target: u32 = 0;
            let target: U256 = U256::max_value();
            let hash_current_block: H256 = H256::zero();

            // Store the new BlockChain in Chains
            let chain_id: U256 = U256::zero(); 
            let mut chain: Vec<H256> = Vec::new();
            chain.push(hash_current_block);
            let mut blockchain: Vec<BlockChain<U256, H256>> = Vec::new();
            blockchain.push(BlockChain {
                chain_id: chain_id,
                chain: chain,
                max_height: block_height,
                no_data: false,
                invalid: false,
            });

            <Chains>::put(blockchain);
            
            // Insert a pointer to BlockChain in ChainsIndex

            // Store a new BlockHeader struct in BlockHeaders
            let block_header = BlockHeader {
                block_height: block_height,
                merkle_root: merkle_root,
                target: target,
                timestamp: timestamp,
                chain_ref: chain_id,
                no_data: false,
                invalid: false,
            }; 
            // Set BestBlock and BestBlockHeight to the submitted block
            <BestBlockHeight>::mutate(|n| *n = block_height);
            // Emit a Initialized Event
            Self::deposit_event(RawEvent::Initialized(block_height, hash_current_block)) 
            // Ok(Self::bestblockheight())
        }

		// Just a dummy entry point.
		// function that can be called by the external world as an extrinsics call
		// takes a parameter of the type `AccountId`, stores it and emits an event
		// pub fn do_something(origin, something: u32) -> DispatchResult {
			// TODO: You only need this if you want to check it was signed.
			// let who = ensure_signed(origin)?;

			// TODO: Code to execute when something calls this.
			// For example: the following line stores the passed in u32 in the storage
			// Something::put(something);

			// here we are raising the Something event
			// Self::deposit_event(RawEvent::SomethingStored(something, who));
			//Ok(())
		// }
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
		// Just a dummy event.
		// Event `Something` is declared with a parameter of the type `u32` and `AccountId`
		// To emit this event, we call the deposit funtion, from our runtime funtions
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

/// tests for this pallet
#[cfg(test)]
mod tests {
	use super::*;

	use sp_core::H256;
	use frame_support::{impl_outer_origin, assert_ok, parameter_types, weights::Weight};
	use sp_runtime::{
		traits::{BlakeTwo256, IdentityLookup}, testing::Header, Perbill,
	};

	impl_outer_origin! {
		pub enum Origin for Test {}
	}

	// For testing the pallet, we construct most of a mock runtime. This means
	// first constructing a configuration type (`Test`) which `impl`s each of the
	// configuration traits of modules we want to use.
	#[derive(Clone, Eq, PartialEq)]
	pub struct Test;
	parameter_types! {
		pub const BlockHashCount: u64 = 250;
		pub const MaximumBlockWeight: Weight = 1024;
		pub const MaximumBlockLength: u32 = 2 * 1024;
		pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
	}
	impl system::Trait for Test {
		type Origin = Origin;
		type Call = ();
		type Index = u64;
		type BlockNumber = u64;
		type Hash = H256;
		type Hashing = BlakeTwo256;
		type AccountId = u64;
		type Lookup = IdentityLookup<Self::AccountId>;
		type Header = Header;
		type Event = ();
		type BlockHashCount = BlockHashCount;
		type MaximumBlockWeight = MaximumBlockWeight;
		type MaximumBlockLength = MaximumBlockLength;
		type AvailableBlockRatio = AvailableBlockRatio;
		type Version = ();
		type ModuleToIndex = ();
	}
	impl Trait for Test {
		type Event = ();
	}
	type TemplateModule = Module<Test>;

	// This function basically just builds a genesis storage key/value store according to
	// our desired mockup.
	fn new_test_ext() -> sp_io::TestExternalities {
		system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
	}

	#[test]
	fn it_works_for_default_value() {
		new_test_ext().execute_with(|| {
			// Just a dummy test for the dummy funtion `do_something`
			// calling the `do_something` function with a value 42
			assert_ok!(TemplateModule::do_something(Origin::signed(1), 42));
			// asserting that the stored value is equal to what we stored
			assert_eq!(TemplateModule::something(), Some(42));
		});
	}
}
