// Summa Bitcoin SPV lib
use bitcoin_spv::types::{RawHeader};
use bitcoin_spv::btcspv;

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
    pub version: u32,
    pub hash_prev_block: H256,
    pub nonce: u32
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


/// Extracts the nonce from a block header.
///
/// # Arguments
///
/// * `header` - An 80-byte Bitcoin header
pub fn extract_nonce(header: RawHeader) -> u32 {
    let mut nonce: [u8; 4] = Default::default();
    nonce.copy_from_slice(&header[76..80]);
    u32::from_le_bytes(nonce)
}

/// Extracts the version from a block header.
///
/// # Arguments
///
/// * `header` - An 80-byte Bitcoin header
pub fn extract_version(header: RawHeader) -> u32 {
    let mut version: [u8; 4] = Default::default();
    version.copy_from_slice(&header[0..4]);
    u32::from_le_bytes(version)
}

/// Extracts the target from a block header.
///
/// # Arguments
///
/// * `header` - An 80-byte Bitcoin header
pub fn extract_target(header: RawHeader) -> U256 {
    let target = btcspv::extract_target(header);
    U256::from_little_endian(&target.to_bytes_le()[..])
}

/// Extracts the timestamp from a block header.
///
/// # Arguments
///
/// * `header` - An 80-byte Bitcoin header
pub fn extract_timestamp(header: RawHeader) -> Moment {
    btcspv::extract_timestamp(header) as u64
}

/// Extracts the previous block hash from a block header.
///
/// # Arguments
///
/// * `header` - An 80-byte Bitcoin header
pub fn extract_previous_block_hash(header: RawHeader) -> H256 {
    H256::from_slice(&btcspv::extract_prev_block_hash_le(header)[..])
}

/// Extracts the merkle root from a block header.
///
/// # Arguments
///
/// * `header` - An 80-byte Bitcoin header
pub fn extract_merkle_root(header: RawHeader) -> H256 {
    H256::from_slice(&btcspv::extract_merkle_root_le(header)[..])
}


pub fn parse_block_header(raw_header: RawBlockHeader) -> BlockHeader<U256, H256, Moment> {
    let hash_current_block: H256 = H256::zero();

    let block_header = BlockHeader {
        block_hash: hash_current_block,
        block_height: None,
        merkle_root: extract_merkle_root(raw_header),
        target: extract_target(raw_header),
        timestamp: extract_timestamp(raw_header),
        chain_ref: None,
        version: extract_version(raw_header),
        nonce: extract_nonce(raw_header),
        hash_prev_block: extract_previous_block_hash(raw_header),
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
