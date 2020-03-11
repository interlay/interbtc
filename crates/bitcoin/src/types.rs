use bitcoin_spv::types::{RawHeader};

use codec::{Encode, Decode};

/// Custom Types
/// Bitcoin Raw Block Header type
pub type RawBlockHeader = RawHeader;


/// Structs
/// Bitcoin Basic Block Headers
// TODO: Figure out how to set a pointer to the ChainIndex mapping instead
#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct BlockHeader<H256, U256, Timestamp> {
    pub block_hash: H256,
    pub merkle_root: H256,
    pub target: U256,
    pub timestamp: Timestamp,
    pub version: u32,
    pub hash_prev_block: H256,
    pub nonce: u32
}

/// Bitcoin Enriched Block Headers
#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct RichBlockHeader<H256, U256, Timestamp> {
    pub block_header: BlockHeader<H256, U256, Timestamp>,
    pub block_height: U256,
    pub chain_ref: U256,
}

/// Representation of a Bitcoin blockchain
// Note: the chain representation is for now a vector
// TODO: ask if there is a "mapping" type in structs
#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct BlockChain<U256, Map> {
    pub chain_id: U256,
    pub chain: Map,
    pub start_height: U256,
    pub max_height: U256,
    pub no_data: Vec<U256>,
    pub invalid: Vec<U256>,
}
