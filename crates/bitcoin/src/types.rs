extern crate hex;

use crate::formatter::Formattable;
use crate::parser::*;
use crate::utils::*;
use codec::alloc::string::String;
use codec::{Decode, Encode};
use primitive_types::{H256, U256};
use sp_std::collections::btree_set::BTreeSet;
use sp_std::prelude::*;
use x_core::Error;

pub(crate) const SERIALIZE_TRANSACTION_NO_WITNESS: i32 = 0x4000_0000;

/// Custom Types
/// Bitcoin Raw Block Header type

#[derive(Encode, Decode, Copy, Clone)]
pub struct RawBlockHeader([u8; 80]);

impl RawBlockHeader {
    /// Returns a raw block header from a bytes slice
    ///
    /// # Arguments
    ///
    /// * `bytes` - A slice containing the header
    pub fn from_bytes(bytes: &[u8]) -> Result<RawBlockHeader, Error> {
        if bytes.len() != 80 {
            return Err(Error::InvalidHeaderSize);
        }
        let mut result: [u8; 80] = [0; 80];
        result.copy_from_slice(&bytes);
        Ok(RawBlockHeader(result))
    }

    /// Returns a raw block header from a bytes slice
    ///
    /// # Arguments
    ///
    /// * `bytes` - A slice containing the header
    pub fn from_hex<T: AsRef<[u8]>>(hex_string: T) -> Result<RawBlockHeader, Error> {
        let bytes = hex::decode(hex_string).map_err(|_e| Error::MalformedHeader)?;
        Self::from_bytes(&bytes)
    }

    /// Returns the hash of the block header using Bitcoin's double sha256
    pub fn hash(&self) -> H256Le {
        H256Le::from_bytes_le(&sha256d(self.as_slice()))
    }

    /// Returns the block header as a slice
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

impl PartialEq for RawBlockHeader {
    fn eq(&self, other: &Self) -> bool {
        let self_bytes = &self.0[..];
        let other_bytes = &other.0[..];
        self_bytes == other_bytes
    }
}

impl sp_std::fmt::Debug for RawBlockHeader {
    fn fmt(&self, f: &mut sp_std::fmt::Formatter<'_>) -> sp_std::fmt::Result {
        f.debug_list().entries(self.0.iter()).finish()
    }
}

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
    pub timestamp: u64,
    pub version: i32,
    pub hash_prev_block: H256Le,
    pub nonce: u32,
}

/// Bitcoin transaction input
#[derive(PartialEq, Clone, Debug)]
pub struct TransactionInput {
    pub previous_hash: H256Le,
    pub previous_index: u32,
    pub coinbase: bool,
    pub height: Option<Vec<u8>>, // FIXME: Vec<u8> type here seems weird
    pub script: Vec<u8>,
    pub sequence: u32,
    pub witness: Vec<Vec<u8>>,
}

impl TransactionInput {
    pub fn with_witness(&mut self, witness: Vec<Vec<u8>>) {
        self.witness = witness;
    }
}

/// Bitcoin transaction output
#[derive(PartialEq, Debug, Clone)]
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
    pub block_height: Option<u32>, //FIXME: why is this optional?
    pub locktime: Option<u32>,     //FIXME: why is this optional?
}

impl Transaction {
    pub fn tx_id(&self) -> H256Le {
        sha256d_le(&self.format())
    }

    pub fn hash(&self) -> H256Le {
        sha256d_le(&self.format_with(false))
    }
}

/// Bitcoin Enriched Block Headers
#[derive(Encode, Decode, Default, Clone, Copy, PartialEq, Debug)]
pub struct RichBlockHeader {
    pub block_hash: H256Le,
    pub block_header: BlockHeader,
    pub block_height: u32,
    pub chain_ref: u32,
}

impl RichBlockHeader {
    // Creates a RichBlockHeader given a RawBlockHeader, Blockchain identifier and block height
    pub fn construct(
        raw_block_header: RawBlockHeader,
        chain_ref: u32,
        block_height: u32,
    ) -> Result<RichBlockHeader, Error> {
        Ok(RichBlockHeader {
            block_hash: raw_block_header.hash(),
            block_header: BlockHeader::from_le_bytes(raw_block_header.as_slice())?,
            block_height: block_height,
            chain_ref: chain_ref,
        })
    }
}

/// Representation of a Bitcoin blockchain
#[derive(Encode, Decode, Default, Clone, PartialEq, Debug)]
pub struct BlockChain {
    pub chain_id: u32,
    pub start_height: u32,
    pub max_height: u32,
    pub no_data: BTreeSet<u32>,
    pub invalid: BTreeSet<u32>,
}

/// Represents a bitcoin 32 bytes hash digest encoded in little-endian
#[derive(Encode, Decode, Default, PartialEq, Eq, Clone, Copy, Debug)]
pub struct H256Le {
    content: [u8; 32],
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
        H256Le { content }
    }

    /// Creates a H256Le from big endian bytes
    pub fn from_bytes_be(bytes: &[u8]) -> H256Le {
        let bytes_le = reverse_endianness(bytes);
        let mut content: [u8; 32] = Default::default();
        content.copy_from_slice(&bytes_le);
        H256Le { content }
    }

    pub fn from_hex_le(hex: &str) -> H256Le {
        H256Le::from_bytes_le(&hex::decode(hex).unwrap())
    }

    pub fn from_hex_be(hex: &str) -> H256Le {
        H256Le::from_bytes_be(&hex::decode(hex).unwrap())
    }

    /// Returns the content of the H256Le encoded in big endian
    pub fn to_bytes_be(&self) -> [u8; 32] {
        let mut content: [u8; 32] = Default::default();
        content.copy_from_slice(&reverse_endianness(&self.content[..]));
        content
    }

    /// Returns the content of the H256Le encoded in little endian
    pub fn to_bytes_le(&self) -> [u8; 32] {
        self.content
    }

    /// Returns the content of the H256Le encoded in little endian hex
    pub fn to_hex_le(&self) -> String {
        hex::encode(&self.to_bytes_le())
    }

    /// Returns the content of the H256Le encoded in big endian hex
    pub fn to_hex_be(&self) -> String {
        hex::encode(&self.to_bytes_be())
    }

    /// Returns the value as a U256
    pub fn as_u256(&self) -> U256 {
        U256::from_little_endian(&self.to_bytes_le())
    }

    /// Hashes the value a single time using sha256
    pub fn sha256d(&self) -> Self {
        Self::from_bytes_le(&sha256d(&self.to_bytes_le()))
    }
}

#[cfg(feature = "std")]
impl sp_std::fmt::Display for H256Le {
    fn fmt(&self, f: &mut sp_std::fmt::Formatter<'_>) -> sp_std::fmt::Result {
        write!(f, "0x{}", self.to_hex_be())
    }
}

impl sp_std::fmt::LowerHex for H256Le {
    fn fmt(&self, f: &mut sp_std::fmt::Formatter<'_>) -> sp_std::fmt::Result {
        write!(f, "{}", self.to_hex_be())
    }
}

// Bitcoin Script OpCodes
pub enum OpCode {
    OpDup = 0x76,
    OpHash160 = 0xa9,
    OpEqualVerify = 0x88,
    OpCheckSig = 0xac,
    OpEqual = 0x87,
    OpReturn = 0x6a,
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

impl CompactUint {
    pub(crate) fn from_usize(value: usize) -> CompactUint {
        CompactUint {
            value: value as u64,
        }
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
