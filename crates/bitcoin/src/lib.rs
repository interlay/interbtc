// Summa Bitcoin SPV lib
use bitcoin_spv::types::{RawHeader};
use bitcoin_spv::validatespv::{parse_header};

// Substrate imports
use codec::{Encode, Decode};
use primitive_types::{U256, H256};
use node_primitives::{Moment};

// IndexMap
// use core::hash::BuildHasherDefault;
// use indexmap::IndexMap;
// use twox_hash::XxHash64;

/// Custom Types
/// Bitcoin Raw Block Header type
pub type RawBlockHeader = RawHeader;

// NOTE: This is a more efficient type for mappings (compared to BTreeMap),
// but currently not supported by Substrate
// /// A mapping type for blockchains
// pub type Map<K, V> = IndexMap<K, V, BuildHasherDefault<XxHash64>>;

/// Structs
/// Bitcoin Block Headers
// TODO: Figure out how to set a pointer to the ChainIndex mapping instead
#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct BlockHeader<U256, H256, Timestamp> {
    pub block_hash: H256, 
    pub block_height: Option<U256>,
    pub merkle_root: H256,
    pub target: U256,
    pub timestamp: Timestamp,
    pub chain_ref: Option<U256>,
    pub no_data: bool,
    pub invalid: bool,
    pub version: Option<u32>,
    pub hash_prev_block: Option<H256>,
    pub nonce: Option<u32>
}

/// Representation of a Bitcoin blockchain
// Note: the chain representation is for now a vector
// TODO: ask if there is a "mapping" type in structs
#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct BlockChain<U256, Map> {
    pub chain_id: U256,
    pub chain: Map,
    pub max_height: U256,
    pub no_data: bool,
    pub invalid: bool,
}


pub fn parse_block_header(block_header_bytes: Vec<u8>) -> BlockHeader<U256, H256, Moment> {
    let merkle_root: H256 = H256::zero();
    let timestamp: Moment = Moment::default(); 
    let target: U256 = U256::max_value();
    let hash_current_block: H256 = H256::zero();
    // returns a new BlockHeader struct
    let block_header = BlockHeader {
        block_hash: hash_current_block,
        block_height: None,
        merkle_root: merkle_root,
        target: target,
        timestamp: timestamp,
        chain_ref: None,
        no_data: false,
        invalid: false,
        version: None,
        nonce: None,
        hash_prev_block: None
    };    

    return block_header
}


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
