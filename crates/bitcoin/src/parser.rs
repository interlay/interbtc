use crate::types::*;

use primitive_types::{U256, H256};
use node_primitives::{Moment};

use bitcoin_spv::btcspv;


pub(crate) trait Parsable: Sized {
    fn parse(raw_bytes: &[u8], position: usize) -> Result<(Self, usize), Error>;
}

/// Macro to generate `Parsable` implementation of uint types
macro_rules! make_uint_parsable {
    ($type:ty, $bytes:expr) => (
        impl Parsable for $type {
            fn parse(raw_bytes: &[u8], position: usize) -> Result<($type, usize), Error> {
                if position + $bytes > raw_bytes.len() {
                    return Err(Error::EOS);
                }
                let mut value_bytes: [u8; $bytes] = Default::default();
                value_bytes.copy_from_slice(&raw_bytes[position..position + $bytes]);
                Ok((<$type>::from_le_bytes(value_bytes), $bytes))
            }
        }
    );
}

make_uint_parsable!(u8, 1);
make_uint_parsable!(u16, 2);
make_uint_parsable!(u32, 4);
make_uint_parsable!(u64, 8);

impl Parsable for VarInt {
    fn parse(raw_bytes: &[u8], position: usize) -> Result<(VarInt, usize), Error> {
        let last_byte = std::cmp::min(position + 3, raw_bytes.len());
        let (value, bytes_consumed) = parse_varint(&raw_bytes[position..last_byte]);
        Ok((VarInt { value }, bytes_consumed))
    }
}

impl Parsable for BlockHeader {
    fn parse(raw_bytes: &[u8], position: usize) -> Result<(BlockHeader, usize), Error> {
        if position + 80 > raw_bytes.len() {
            return Err(Error::EOS);
        }
        let header_bytes = header_from_bytes(&raw_bytes[position..position + 80]);
        let block_header = parse_block_header(header_bytes);
        Ok((block_header, 80))
    }
}

impl Parsable for H256Le {
    fn parse(raw_bytes: &[u8], position: usize) -> Result<(H256Le, usize), Error> {
        if position + 32 > raw_bytes.len() {
            return Err(Error::EOS);
        }
        Ok((H256Le::from_bytes_le(&raw_bytes[position..position + 32]), 32))
    }
}

impl Parsable for Vec<bool> {
    fn parse(raw_bytes: &[u8], position: usize) -> Result<(Vec<bool>, usize), Error> {
        let byte = raw_bytes[position];
        let mut flag_bits = Vec::new();
        for i in 0..8 {
            let mask = 1 << i;
            let bit = (byte & mask) != 0;
            flag_bits.push(bit);
        }
        Ok((flag_bits, 1))
    }
}

/// BytesParser is a stateful parser for raw bytes
/// The head of the parser is updated for each `read` or `parse` operation
pub(crate) struct BytesParser {
    raw_bytes: Vec<u8>,
    position: usize,
}

impl BytesParser {
    /// Creates a new `BytesParser` to parse the given raw bytes
    pub(crate) fn new(bytes: &[u8]) -> BytesParser {
        BytesParser {
            raw_bytes: Vec::from(bytes),
            position: 0,
        }
    }

    /// Parses a `Parsable` object and updates the parser head
    /// to the next byte after the parsed object
    /// Fails if there are not enough bytes to read or if the
    /// underlying `Parsable` parse function fails
    pub(crate) fn parse<T: Parsable> (&mut self) -> Result<T, Error> {
        let (result, bytes_consumed) = T::parse(&self.raw_bytes, self.position)?;
        self.position += bytes_consumed;
        Ok(result)
    }
}


/// Returns a raw block header from a bytes slice
///
/// # Arguments
///
/// * `bytes` - A slice containing the header
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

/// Parses the raw bitcoin header into a Rust struct
///
/// # Arguments
///
/// * `header` - An 80-byte Bitcoin header
pub fn parse_block_header(raw_header: RawBlockHeader) -> BlockHeader {
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

/// Returns the value of the varint
///
/// # Arguments
///
/// * `varint` - A slice containing the header
pub fn parse_varint(varint: &[u8]) -> (u64, usize) {
    match varint[0] {
        0xfd => {
            let mut num_bytes: [u8; 2] = Default::default();
            num_bytes.copy_from_slice(&varint[1..3]);
            (u16::from_le_bytes(num_bytes) as u64, 3)
        },
        0xfe => {
            let mut num_bytes: [u8; 4] = Default::default();
            num_bytes.copy_from_slice(&varint[1..5]);
            (u32::from_le_bytes(num_bytes) as u64, 5)
        },
        0xff => {
            let mut num_bytes: [u8; 8] = Default::default();
            num_bytes.copy_from_slice(&varint[1..9]);
            (u64::from_le_bytes(num_bytes) as u64, 9)
        },
        _    => (varint[0] as u64, 1)
    }
}


pub fn parse_transaction(_raw_transaction: &[u8]) -> Result<Transaction, Error> {
    Err(Error::InvalidProof)
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

    #[test]
    fn test_parse_varint() {
        let cases = [
            (&[1, 2, 3][..], (1, 1)),
            (&[253, 2, 3][..], (770, 3)),
            (&[254, 2, 3, 8, 1, 8][..], (17302274, 5)),
            (&[255, 6, 0xa, 3, 8, 1, 0xb, 2, 7, 8][..], (504978207276206598, 9)),
        ];
        for (input, expected) in cases.iter() {
            assert_eq!(parse_varint(input), *expected);
        }
    }
}
