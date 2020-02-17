use codec::{Encode, Decode};
use bitcoin_spv::types::{RawHeader};
use bitcoin_spv::validatespv::{parse_header};

/// Custom Types
/// Bitcoin Raw Block Header type
pub type RawBlockHeader = RawHeader;

/// Structs
/// Bitcoin Block Headers
// TODO: Figure out how to set a pointer to the ChainIndex mapping instead
#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct BlockHeader<U256, H256, Moment> {
      pub block_height: U256,
      pub merkle_root: H256,
      pub target: U256,
      pub timestamp: Moment,
      pub chain_ref: U256,
      pub no_data: bool,
      pub invalid: bool,
      // Optional fields
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


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
