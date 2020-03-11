use crate::types::{RawBlockHeader, BlockHeader};

use primitive_types::{U256, H256};
use node_primitives::{Moment};

use bitcoin_spv::btcspv;


pub fn header_from_bytes(bytes: &[u8]) -> RawBlockHeader {
    let mut result: RawBlockHeader = [0; 80];
    result.copy_from_slice(&bytes[..]);
    result
}

/// Extracts the nonce from a block header.
///
/// # Arguments
///
/// * `header` - An 80-byte Bitcoin header
pub fn extract_nonce(header: RawBlockHeader) -> u32 {
    let mut nonce: [u8; 4] = Default::default();
    nonce.copy_from_slice(&header[76..80]);
    u32::from_le_bytes(nonce)
}

/// Extracts the version from a block header.
///
/// # Arguments
///
/// * `header` - An 80-byte Bitcoin header
pub fn extract_version(header: RawBlockHeader) -> u32 {
    let mut version: [u8; 4] = Default::default();
    version.copy_from_slice(&header[0..4]);
    u32::from_le_bytes(version)
}

/// Extracts the target from a block header.
///
/// # Arguments
///
/// * `header` - An 80-byte Bitcoin header
pub fn extract_target(header: RawBlockHeader) -> U256 {
    let target = btcspv::extract_target(header);
    U256::from_little_endian(&target.to_bytes_le()[..])
}

/// Extracts the timestamp from a block header.
///
/// # Arguments
///
/// * `header` - An 80-byte Bitcoin header
pub fn extract_timestamp(header: RawBlockHeader) -> Moment {
    btcspv::extract_timestamp(header) as u64
}

/// Extracts the previous block hash from a block header.
///
/// # Arguments
///
/// * `header` - An 80-byte Bitcoin header
pub fn extract_previous_block_hash(header: RawBlockHeader) -> H256 {
    let hash_le = &btcspv::extract_prev_block_hash_le(header)[..];
    H256::from_slice(&bitcoin_spv::utils::reverse_endianness(hash_le)[..])
}

/// Extracts the merkle root from a block header.
///
/// # Arguments
///
/// * `header` - An 80-byte Bitcoin header
pub fn extract_merkle_root(header: RawBlockHeader) -> H256 {
    let root_le = &btcspv::extract_merkle_root_le(header)[..];
    H256::from_slice(&bitcoin_spv::utils::reverse_endianness(root_le)[..])
}


pub fn parse_block_header(raw_header: RawBlockHeader) -> BlockHeader<H256, U256, Moment> {
    let hash_current_block: H256 = H256::zero();

    let block_header = BlockHeader {
        merkle_root: extract_merkle_root(raw_header),
        target: extract_target(raw_header),
        timestamp: extract_timestamp(raw_header),
        version: extract_version(raw_header),
        nonce: extract_nonce(raw_header),
        hash_prev_block: extract_previous_block_hash(raw_header),

        block_hash: hash_current_block,
    };

    return block_header
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_block_header() {
        // example from https://bitcoin.org/en/developer-reference#block-headers
        let hex_header =
            "02000000".to_string() + // ............... Block version: 2
            "b6ff0b1b1680a2862a30ca44d346d9e8" + //
            "910d334beb48ca0c0000000000000000" + // ... Hash of previous block's header
            "9d10aa52ee949386ca9385695f04ede2" + //
            "70dda20810decd12bc9b048aaab31471" + // ... Merkle root
            "24d95a54" + // ........................... Unix time: 1415239972
            "30c31b18" + // ........................... Target: 0x1bc330 * 256**(0x18-3)
            "fe9f0864";
        let raw_header = bitcoin_spv::utils::deserialize_hex(&hex_header[..]).unwrap();
        let parsed_header = parse_block_header(header_from_bytes(&raw_header));
        assert_eq!(parsed_header.version, 2);
        assert_eq!(parsed_header.timestamp, 1415239972);
        assert_eq!(format!("{:x}", parsed_header.merkle_root),
                   "7114b3aa8a049bbc12cdde1008a2dd70e2ed045f698593ca869394ee52aa109d");
        assert_eq!(format!("{:x}", parsed_header.hash_prev_block),
                   "00000000000000000cca48eb4b330d91e8d946d344ca302a86a280161b0bffb6");
        let expected_target = String::from("680733321990486529407107157001552378184394215934016880640");
        assert_eq!(parsed_header.target.to_string(), expected_target);
    }
}
