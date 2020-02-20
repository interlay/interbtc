// Summa Bitcoin SPV lib
use bitcoin_spv::types::{RawHeader};
use bitcoin_spv::validatespv::{parse_header};

// Substrate imports
use codec::{Encode, Decode};
use primitive_types::{U256, H256};
use node_primitives::{Moment};

/// Custom Types
/// Bitcoin Raw Block Header type
pub type RawBlockHeader = RawHeader;

/// Structs
/// Bitcoin Block Headers
// TODO: Figure out how to set a pointer to the ChainIndex mapping instead
#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct BlockHeader<U256, H256, Timestamp> {
    pub block_hash: H256, 
    pub block_height: U256,
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
pub struct BlockChain<U256, H256> {
    pub chain_id: U256,
    pub chain: Vec<H256>,
    pub max_height: U256,
    pub no_data: bool,
    pub invalid: bool,
}


pub fn parse_block_header(block_header_bytes: Vec<u8>, block_height: U256) -> BlockHeader<U256, H256, Moment> {
    let merkle_root: H256 = H256::zero();
    let timestamp: Moment = 0; 
    let target: U256 = U256::max_value();
    let hash_current_block: H256 = H256::zero();
    // returns a new BlockHeader struct
    let block_header = BlockHeader {
        block_hash: hash_current_block,
        block_height: block_height,
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

    block_header
}


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
