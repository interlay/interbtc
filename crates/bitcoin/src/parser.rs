use crate::types::*;

use node_primitives::Moment;
use primitive_types::{H256, U256};

use bitcoin_spv::btcspv;

/// Type to be parsed from a bytes array
pub(crate) trait Parsable: Sized {
    fn parse(raw_bytes: &[u8], position: usize) -> Result<(Self, usize), Error>;
}

/// Type to be parsed from a bytes array using extra metadata
pub(crate) trait ParsableMeta<Metadata>: Sized {
    fn parse_with(
        raw_bytes: &[u8],
        position: usize,
        extra: Metadata,
    ) -> Result<(Self, usize), Error>;
}

/// Macro to generate `Parsable` implementation of uint types
macro_rules! make_parsable_int {
    ($type:ty, $bytes:expr) => {
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
    };
}

// Generate parsable implementation for the basic integers (signed and unsgined) types
make_parsable_int!(u8, 1);
make_parsable_int!(u16, 2);
make_parsable_int!(u32, 4);
make_parsable_int!(u64, 8);
make_parsable_int!(i8, 1);
make_parsable_int!(i16, 2);
make_parsable_int!(i32, 4);
make_parsable_int!(i64, 8);

impl Parsable for CompactUint {
    fn parse(raw_bytes: &[u8], position: usize) -> Result<(CompactUint, usize), Error> {
        let last_byte = std::cmp::min(position + 3, raw_bytes.len());
        let (value, bytes_consumed) = parse_compact_uint(&raw_bytes[position..last_byte]);
        Ok((CompactUint { value }, bytes_consumed))
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
        Ok((
            H256Le::from_bytes_le(&raw_bytes[position..position + 32]),
            32,
        ))
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


impl ParsableMeta<i32> for TransactionInput {
    fn parse_with(
        raw_bytes: &[u8],
        position: usize,
        version: i32,
    ) -> Result<(TransactionInput, usize), Error> {
        parse_transaction_input(&raw_bytes[position..], version)
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
    pub(crate) fn parse<T: Parsable>(&mut self) -> Result<T, Error> {
        let (result, bytes_consumed) = T::parse(&self.raw_bytes, self.position)?;
        self.position += bytes_consumed;
        Ok(result)
    }

    /// This is the same as `parse` but allows to pass extra data to the parser
    /// Fails if there are not enough bytes to read or if the
    /// underlying `Parsable` parse function fails
    pub(crate) fn parse_with<T, U>(&mut self, extra: U) -> Result<T, Error>
    where
        T: ParsableMeta<U>,
    {
        let (result, bytes_consumed) = T::parse_with(&self.raw_bytes, self.position, extra)?;
        self.position += bytes_consumed;
        Ok(result)
    }

    /// Reads `bytes_count` from the bytes parser and moves the head
    /// Fails if there are not enough bytes to read
    pub(crate) fn read(&mut self, bytes_count: usize) -> Result<Vec<u8>, Error> {
        if self.position + bytes_count > self.raw_bytes.len() {
            return Err(Error::EOS);
        }
        let bytes = &self.raw_bytes[self.position..self.position + bytes_count];
        self.position += bytes_count;
        Ok(Vec::from(bytes))
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

    return block_header;
}

/// Returns the value of the varint
///
/// # Arguments
///
/// * `varint` - A slice containing the header
pub fn parse_compact_uint(varint: &[u8]) -> (u64, usize) {
    match varint[0] {
        0xfd => {
            let mut num_bytes: [u8; 2] = Default::default();
            num_bytes.copy_from_slice(&varint[1..3]);
            (u16::from_le_bytes(num_bytes) as u64, 3)
        }
        0xfe => {
            let mut num_bytes: [u8; 4] = Default::default();
            num_bytes.copy_from_slice(&varint[1..5]);
            (u32::from_le_bytes(num_bytes) as u64, 5)
        }
        0xff => {
            let mut num_bytes: [u8; 8] = Default::default();
            num_bytes.copy_from_slice(&varint[1..9]);
            (u64::from_le_bytes(num_bytes) as u64, 9)
        }
        _ => (varint[0] as u64, 1),
    }
}

/// Parses a single bitcoin transaction
/// # Arguments
///
/// * `raw_transaction` - the raw bytes of the transaction
pub fn parse_transaction(raw_transaction: &[u8]) -> Result<Transaction, Error> {
    let mut parser = BytesParser::new(raw_transaction);
    let version: i32 = parser.parse()?;

    // fail if incorrect version: we only support version 1 and 2
    if version != 1 && version != 2 {
        return Err(Error::MalformedTransaction);
    }

    let tx_in_count: CompactUint = parser.parse()?;
    let mut inputs: Vec<TransactionInput> = Vec::new();
    for _ in 0..tx_in_count.value {
        inputs.push(parser.parse_with(version)?);
    }

    // parser.parse()
    Err(Error::MalformedTransaction)
}

/// Parses a transaction input
pub fn parse_transaction_input(
    raw_input: &[u8],
    version: i32,
) -> Result<(TransactionInput, usize), Error> {
    let mut parser = BytesParser::new(raw_input);
    let previous_hash: H256Le = parser.parse()?;
    let pervious_index: u32 = parser.parse()?;

    // coinbase input has no previous hash
    let is_coinbase = previous_hash == H256Le::zero();

    // fail if transaction is coinbase and previous index is not 0xffffffff
    // previous_hash
    if is_coinbase && pervious_index != u32::max_value() {
        return Err(Error::MalformedTransaction);
    }

    let mut script_size: u64 = parser.parse::<CompactUint>()?.value;
    let height = if is_coinbase && version == 2 {
        script_size -= 4;
        Some(parser.read(4)?)
    } else {
        None
    };

    let script = parser.read(script_size as usize)?;
    // fail if coinbase script is longer than 100 bytes
    if is_coinbase && script.len() > 100 {
        return Err(Error::MalformedTransaction);
    }

    let sequence: u32 = parser.parse()?;
    let consumed_bytes = parser.position;

    Ok((
        TransactionInput {
            previous_hash: previous_hash,
            previous_index: pervious_index,
            coinbase: is_coinbase,
            height: height,
            script: script,
            sequence: sequence,
        },
        consumed_bytes,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_block_header() {
        // example from https://bitcoin.org/en/developer-reference#block-headers
        let hex_header = "02000000".to_owned() + // ............... Block version: 2
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
        assert_eq!(
            format!("{:x}", parsed_header.merkle_root),
            "7114b3aa8a049bbc12cdde1008a2dd70e2ed045f698593ca869394ee52aa109d"
        );
        assert_eq!(
            format!("{:x}", parsed_header.hash_prev_block),
            "00000000000000000cca48eb4b330d91e8d946d344ca302a86a280161b0bffb6"
        );
        let expected_target =
            String::from("680733321990486529407107157001552378184394215934016880640");
        assert_eq!(parsed_header.target.to_string(), expected_target);
    }

    #[test]
    fn test_parse_compact_uint() {
        let cases = [
            (&[1, 2, 3][..], (1, 1)),
            (&[253, 2, 3][..], (770, 3)),
            (&[254, 2, 3, 8, 1, 8][..], (17302274, 5)),
            (
                &[255, 6, 0xa, 3, 8, 1, 0xb, 2, 7, 8][..],
                (504978207276206598, 9),
            ),
        ];
        for (input, expected) in cases.iter() {
            assert_eq!(parse_compact_uint(input), *expected);
        }
    }

    fn coinbase_transaction_input() -> String {
        "00000000000000000000000000000000".to_owned() +
        "00000000000000000000000000000000" + // Previous outpoint TXID
        "ffffffff"                         + // Previous outpoint index
        "29"                               + // Bytes in coinbase
        "03"                               + // Bytes in height
        "4e0105"                           + // Height: 328014
        "062f503253482f0472d35454085fffed" +
        "f2400000f90f54696d65202620486561" +
        "6c74682021"                       + // Arbitrary data
        "00000000"                           // Sequence
    }

    fn transaction_input() -> String {
        "7b1eabe0209b1fe794124575ef807057".to_owned() +
        "c77ada2138ae4fa8d6c4de0398a14f3f" +   // Outpoint TXID
        "00000000" +                           // Outpoint index number
        "49" +                                 // Bytes in sig. script: 73
        "48" +                                 // Push 72 bytes as data
        "30450221008949f0cb400094ad2b5eb3" +
        "99d59d01c14d73d8fe6e96df1a7150de" +
        "b388ab8935022079656090d7f6bac4c9" +
        "a94e0aad311a4268e082a725f8aeae05" +
        "73fb12ff866a5f01" +                   // Secp256k1 signature
        "ffffffff"                             // Sequence number: UINT32_MAX
    }

    #[test]
    fn test_parse_coinbase_transaction_input() {
        let raw_input = coinbase_transaction_input();
        let input_bytes = bitcoin_spv::utils::deserialize_hex(&raw_input[..]).unwrap();
        let mut parser = BytesParser::new(&input_bytes);
        let input: TransactionInput = parser.parse_with(2).unwrap();
        assert_eq!(input.coinbase, true);
        assert_eq!(input.sequence, 0);
        assert_eq!(input.previous_index, u32::max_value());
        let height = input.height.unwrap();
        assert_eq!(height.len(), 4);
        assert_eq!(height[0], 3);
        assert_eq!(input.script.len(), 37); // 0x29 - 4
    }

    #[test]
    fn test_parse_transaction_input() {
        let raw_input = transaction_input();
        let input_bytes = bitcoin_spv::utils::deserialize_hex(&raw_input[..]).unwrap();
        let mut parser = BytesParser::new(&input_bytes);
        let input: TransactionInput = parser.parse_with(2).unwrap();
        assert_eq!(input.coinbase, false);
        assert_eq!(input.sequence, u32::max_value());
        assert_eq!(input.previous_index, 0);
        assert_eq!(input.height, None);
        assert_eq!(input.script.len(), 73);

        let previous_hash = H256Le::from_hex_le("7b1eabe0209b1fe794124575ef807057c77ada2138ae4fa8d6c4de0398a14f3f");
        assert_eq!(input.previous_hash, previous_hash);
    }
}
