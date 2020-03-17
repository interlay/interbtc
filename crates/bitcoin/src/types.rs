use primitive_types::{U256, H256};
use codec::{Encode, Decode};
use node_primitives::{Moment};

use bitcoin_spv::types::{RawHeader};

/// Custom Types
/// Bitcoin Raw Block Header type
pub type RawBlockHeader = RawHeader;


/// Structs
/// Bitcoin Basic Block Headers
// TODO: Figure out how to set a pointer to the ChainIndex mapping instead
#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct BlockHeader {
    pub block_hash: H256,
    pub merkle_root: H256,
    pub target: U256,
    pub timestamp: Moment,
    pub version: u32,
    pub hash_prev_block: H256,
    pub nonce: u32
}

/// Bitcoin Enriched Block Headers
#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct RichBlockHeader {
    pub block_header: BlockHeader,
    pub block_height: u32,
    pub chain_ref: u32,
}

/// Representation of a Bitcoin blockchain
#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct BlockChain<Map> {
    pub chain_id: u32,
    pub chain: Map,
    pub start_height: u32,
    pub max_height: u32,
    pub no_data: Vec<u32>,
    pub invalid: Vec<u32>,
}

/// Represents a bitcoin 32 bytes hash digest encoded in little-endian
#[derive(Default, PartialEq, Clone, Copy)]
#[cfg_attr(feature="std", derive(Debug))]
pub struct H256Le {
    content: [u8; 32]
}

impl H256Le {
    /// Creates a H256Le from little endian bytes
    pub fn from_bytes_le(bytes: &[u8]) -> H256Le {
        let mut content: [u8; 32] = Default::default();
        content.copy_from_slice(&bytes);
        H256Le { content: content }
    }

    /// Creates a H256Le from big endian bytes
    pub fn from_bytes_be(bytes: &[u8]) -> H256Le {
        let bytes_le = bitcoin_spv::utils::reverse_endianness(bytes);
        let mut content: [u8; 32] = Default::default();
        content.copy_from_slice(&bytes_le);
        H256Le { content: content }
    }

    pub fn from_hex_le(hex: &str) -> H256Le {
        H256Le::from_bytes_le(&bitcoin_spv::utils::deserialize_hex(hex).unwrap())
    }

    pub fn from_hex_be(hex: &str) -> H256Le {
        H256Le::from_bytes_be(&bitcoin_spv::utils::deserialize_hex(hex).unwrap())
    }

    /// Returns the content of the H256Le encoded in big endian
    pub fn to_bytes_be(&self) -> [u8; 32] {
        let mut content: [u8; 32] = Default::default();
        content.copy_from_slice(&bitcoin_spv::utils::reverse_endianness(&self.content[..]));
        content
    }

    /// Returns the content of the H256Le encoded in little endian
    pub fn to_bytes_le(&self) -> [u8; 32] {
        self.content.clone()
    }

    /// Returns the content of the H256Le encoded in little endian hex
    pub fn to_hex_le(&self) -> String {
        bitcoin_spv::utils::serialize_hex(&self.to_bytes_le())
    }

    /// Returns the content of the H256Le encoded in big endian hex
    pub fn to_hex_be(&self) -> String {
        bitcoin_spv::utils::serialize_hex(&self.to_bytes_be())
    }
}

#[cfg(feature="std")]
impl std::fmt::Display for H256Le {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex_be())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_h256() {
        let mut bytes: [u8; 32] = [0; 32];
        bytes[0] = 5;
        bytes[1] = 10;
        let content = H256Le::from_bytes_le(&bytes);
        assert_eq!(content.to_bytes_le(), bytes);
        let bytes_be = content.to_bytes_be();
        assert_eq!(bytes_be[31], 5);
        assert_eq!(bytes_be[30], 10);
        let content_be = H256Le::from_bytes_be(&bytes);
        assert_eq!(content_be.to_bytes_be(), bytes);
    }
}
