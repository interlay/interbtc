extern crate hex;

use primitive_types::{U256, H256};
use codec::{Encode, Decode};
use node_primitives::{Moment};
use sp_std::collections::btree_map::BTreeMap;
use sp_std::collections::btree_set::BTreeSet;
use bitcoin_spv::types::{RawHeader};
use crate::utils::*;
use crate::parser::*;
/// Custom Types
/// Bitcoin Raw Block Header type


pub type RawBlockHeader = RawHeader;

// #[derive(Encode, Decode, Default, Copy, Clone, PartialEq)]
// struct RawBlockHeader(pub [u8; 32]);

// impl RawBlockHeader {
//     fn hash(&self) -> H256Le {

//     }
// }

// Constants
pub const P2PKH_SCRIPT_SIZE: u32 = 25;
pub const P2SH_SCRIPT_SIZE: u32 = 23;
pub const HASH160_SIZE_HEX: u8 = 0x14;
pub const MAX_OPRETURN_SIZE: usize = 83;
/// Structs
/// Bitcoin Basic Block Headers
// TODO: Figure out how to set a pointer to the ChainIndex mapping instead
#[derive(Encode, Decode, Default, Copy, Clone, PartialEq, Debug)]
//#[cfg_attr(feature = "std", derive(Debug))]
pub struct BlockHeader {
    pub merkle_root: H256Le,
    pub target: U256,
    pub timestamp: Moment,
    pub version: u32,
    pub hash_prev_block: H256Le,
    pub nonce: u32
}

impl BlockHeader {

    pub fn block_hash_le(bytes: &[u8]) -> H256Le{
        sha256d_le(bytes)
    }

    pub fn block_hash_be(bytes: &[u8]) -> H256{
        sha256d_be(bytes)
    }
}

/// Bitcoin transaction input
#[derive(PartialEq, Clone, Debug)]
pub struct TransactionInput {
    pub previous_hash: H256Le,
    pub previous_index: u32,
    pub coinbase: bool,
    pub height: Option<Vec<u8>>,
    pub script: Vec<u8>,
    pub sequence: u32,
    pub witness: Option<Vec<u8>>,
}

impl TransactionInput {
    pub fn with_witness(&mut self, witness: Vec<u8>) -> () {
        self.witness = Some(witness);
    }
}

/// Bitcoin transaction output
#[derive(PartialEq, Debug)]
pub struct TransactionOutput {
    pub value: i64,
    pub script: Vec<u8>,
}

/// Bitcoin transaction
#[derive(PartialEq, Debug)]
pub struct Transaction {
    pub version: i32,
    pub inputs: Vec<TransactionInput>,
    pub outputs: Vec<TransactionOutput>,
    pub block_height: Option<u32>,
    pub locktime: Option<u32>,
}


impl Transaction {
    pub fn tx_id(raw_tx: &[u8]) -> H256Le {
        sha256d_le(&raw_tx)
    }
}

/// Bitcoin Enriched Block Headers
#[derive(Encode, Decode, Default, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct RichBlockHeader {
    pub block_hash: H256Le,
    pub block_header: BlockHeader,
    pub block_height: u32,
    pub chain_ref: u32,
}

impl RichBlockHeader {
    
    // Creates a RichBlockHeader given a RawBlockHeader, Blockchain identifier and block height
    pub fn construct_rich_block_header(raw_block_header: RawBlockHeader, chain_ref: u32, block_height: u32) -> RichBlockHeader {
        RichBlockHeader {
            block_hash: BlockHeader::block_hash_le(&raw_block_header),
            block_header: BlockHeader::from_le_bytes(&raw_block_header),
            block_height: block_height,
            chain_ref: chain_ref,
        }
    }
}

/// Representation of a Bitcoin blockchain
#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct BlockChain {
    pub chain_id: u32,
    pub chain: BTreeMap<u32,H256Le>,
    pub start_height: u32,
    pub max_height: u32,
    pub no_data: BTreeSet<u32>,
    pub invalid: BTreeSet<u32>,
}

/// Represents a bitcoin 32 bytes hash digest encoded in little-endian
#[derive(Encode, Decode, Default, PartialEq, Eq, Clone, Copy, Debug)]
//#[cfg_attr(feature="std", derive(Debug))]
pub struct H256Le {
    content: [u8; 32]
}

impl H256Le {
    /// Creates a new H256Le hash equals to zero
    pub fn zero() -> H256Le {
        H256Le { content: [0; 32] }
    }

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
        hex::encode(&self.to_bytes_be())
    }

    pub fn as_u256(&self) -> U256 {
        U256::from_little_endian(&self.to_bytes_le())
    }
}

#[cfg(feature="std")]
impl std::fmt::Display for H256Le {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{}", self.to_hex_be())
    }
}

impl std::fmt::LowerHex for H256Le {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex_be())
    }
}


/// Errors which can be returned by the bitcoin crate
#[derive(Clone, Copy, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub enum Error {
    /// Reached EOS without finishing to parse bytes
    EOS,

    /// Format of the proof is not correct
    MalformedProof,

    /// Format of the proof is correct but does not yield the correct
    /// merkle root
    InvalidProof,

    /// Format of the transaction is invalid
    MalformedTransaction,

    /// Format of the BIP141 witness transaction output is invalid
    MalformedWitnessOutput,

    // Format of the P2PKH transaction output is invalid
    MalformedP2PKHOutput,

    // Format of the P2SH transaction output is invalid
    MalformedP2SHOutput,

    /// Format of the OP_RETURN transaction output is invalid
    MalformedOpReturnOutput,

    // Output does not match format of supported output types (Witness, P2PKH, P2SH)
    UnsupportedOutputFormat
}


impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::EOS => write!(f, "reached EOS before parsing end"),
            Error::MalformedProof => write!(f, "merkle proof is malformed"),
            Error::InvalidProof => write!(f, "invalid merkle proof"),
            Error::MalformedTransaction => write!(f, "invalid transaction format"),
            Error::MalformedWitnessOutput => write!(f, "invalid witness output format"),
            Error::MalformedP2PKHOutput => write!(f, "invalid P2PKH output format"),
            Error::MalformedP2SHOutput => write!(f, "invalid P2SH output format"),
            Error::MalformedOpReturnOutput => write!(f, "invalid OP_RETURN output format"),
            Error::UnsupportedOutputFormat => write!(f, "unsupported output type. Currently supported: Witness, P2PKH, P2SH")
        }
    }
}

// Bitcoin Script OpCodes
pub enum OpCode {
    OpDup = 0x76,
    OpHash160 = 0xa9,
    OpEqualVerify = 0x88,
    OpCheckSig = 0xac, 
    OpEqual = 0x87,
    OpReturn = 0x6a
}

impl PartialEq<H256Le> for H256 {
    fn eq(&self, other: &H256Le) -> bool {
        let bytes_le = H256Le::from_bytes_be(self.as_bytes());
        bytes_le == *other
    }
}

impl PartialEq<H256> for H256Le {
    fn eq(&self, other: &H256) -> bool {
        *other == *self
    }
}


pub(crate) struct CompactUint {
    pub(crate) value: u64,
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

    #[test]
    fn test_partial_eq() {
        let mut bytes: [u8; 32] = [0; 32];
        bytes[0] = 5;
        bytes[1] = 10;
        let h256 = H256::from_slice(&bytes);
        let h256_le = H256Le::from_bytes_be(&bytes);
        assert_eq!(h256, h256_le);
        assert_eq!(h256_le, h256);
    }
}
