use crate::types::*;

use node_primitives::Moment;
use primitive_types::{U256};
use bitcoin_spv::btcspv;

const SERIALIZE_TRANSACTION_NO_WITNESS: i32 = 0x40000000;


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

impl<T: Parsable> Parsable for Vec<T> {
    fn parse(raw_bytes: &[u8], position: usize) -> Result<(Vec<T>, usize), Error> {
        let mut result: Vec<T> = Vec::new();
        let mut parser = BytesParser::new(&raw_bytes[position..]);
        let count: CompactUint = parser.parse()?;
        for _ in 0..count.value {
            result.push(parser.parse()?);
        }
        Ok((result, parser.position))
    }
}

impl<T, U: Copy> ParsableMeta<U> for Vec<T> where T: ParsableMeta<U> {
    fn parse_with(raw_bytes: &[u8], position: usize, extra: U) -> Result<(Vec<T>, usize), Error> {
        let mut result: Vec<T> = Vec::new();
        let mut parser = BytesParser::new(&raw_bytes[position..]);
        let count: CompactUint = parser.parse()?;
        for _ in 0..count.value {
            result.push(parser.parse_with(extra)?);
        }
        Ok((result, parser.position))
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

impl Parsable for TransactionOutput {
    fn parse(raw_bytes: &[u8], position: usize) -> Result<(TransactionOutput, usize), Error> {
        parse_transaction_output(&raw_bytes[position..])
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

/// Allows to parse the given structure from little-endian encoded bytes
pub trait FromLeBytes: Sized {
    fn from_le_bytes(bytes: &[u8]) -> Self;
}

impl FromLeBytes for BlockHeader {
    fn from_le_bytes(bytes: &[u8]) -> BlockHeader {
        parse_block_header(header_from_bytes(bytes))
    }
}

/// Returns a raw block header from a bytes slice
///
/// # Arguments
///
/// * `bytes` - A slice containing the header
pub fn header_from_bytes(bytes: &[u8]) -> RawBlockHeader {
    let mut result: RawBlockHeader = [0; 80];
    result.copy_from_slice(&bytes);
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
    U256::from_little_endian(&target.to_bytes_le())
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
pub fn extract_previous_block_hash(header: RawBlockHeader) -> H256Le {
    H256Le::from_bytes_le(&btcspv::extract_prev_block_hash_le(header))
}

/// Extracts the merkle root from a block header.
///
/// # Arguments
///
/// * `header` - An 80-byte Bitcoin header
pub fn extract_merkle_root(header: RawBlockHeader) -> H256Le {
    H256Le::from_bytes_le(&btcspv::extract_merkle_root_le(header))
}

/// Parses the raw bitcoin header into a Rust struct
///
/// # Arguments
///
/// * `header` - An 80-byte Bitcoin header
pub fn parse_block_header(raw_header: RawBlockHeader) -> BlockHeader {

    let block_header = BlockHeader {
        merkle_root: extract_merkle_root(raw_header),
        target: extract_target(raw_header),
        timestamp: extract_timestamp(raw_header),
        version: extract_version(raw_header),
        nonce: extract_nonce(raw_header),
        hash_prev_block: extract_previous_block_hash(raw_header),
    };

    return block_header;
}

/// Returns the value of a compactly encoded uint and the number of bytes consumed
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
/// Serialization format is documented below
/// https://github.com/bitcoin/bitcoin/blob/master/src/primitives/transaction.h#L182
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

    let allow_witness = (version & SERIALIZE_TRANSACTION_NO_WITNESS) == 0;

    let mut inputs: Vec<TransactionInput> = parser.parse_with(version)?;

    let mut flags: u8 = 0;
    if inputs.len() == 0 && allow_witness {
        flags = parser.parse()?;
        inputs = parser.parse_with(version)?;
    }

    let outputs: Vec<TransactionOutput> = parser.parse()?;

    if (flags & 1) != 0 && allow_witness {
        flags ^= 1;
        for input in &mut inputs {
            input.with_witness(parser.parse()?);
        }
    }

    let locktime_or_blockheight: u32 = parser.parse()?;
    let (locktime, block_height) = if locktime_or_blockheight < 500_000_000 {
        (None, Some(locktime_or_blockheight))
    } else {
        (Some(locktime_or_blockheight), None)
    };

    if flags != 0 {
        return Err(Error::MalformedTransaction);
    }

    Ok(Transaction {
        version: version,
        inputs: inputs,
        outputs: outputs,
        block_height: block_height,
        locktime: locktime,
    })
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
            witness: None,
        },
        consumed_bytes,
    ))
}

pub fn parse_transaction_output(raw_output: &[u8]) -> Result<(TransactionOutput, usize), Error> {
    let mut parser = BytesParser::new(raw_output);
    let value: i64 = parser.parse()?;
    let script_size: CompactUint = parser.parse()?;
    if script_size.value > 10_000 {
        return Err(Error::MalformedTransaction);
    }
    let script = parser.read(script_size.value as usize)?;
    Ok((
        TransactionOutput {
            value: value,
            script: Vec::from(script),
        },
        parser.position,
    ))
}

pub fn extract_value(raw_output: &[u8]) -> u64 {
    return btcspv::extract_value(raw_output);
}

pub fn extract_address_hash(output_script: &[u8]) -> Result<Vec<u8>, Error> {

    let script_len = output_script.len();
    
    // Witness
    if output_script[0] == 0 {
        if script_len < 2 {
            return Err(Error::MalformedWitnessOutput);
        }
        if output_script[1] == (script_len - 2) as u8 {
            return Ok(output_script[2..].to_vec());
        } else {
            return Err(Error::MalformedWitnessOutput);
        }
    }

    // P2PKH
    // 25 bytes
    // Format:
    // 0x76 (OP_DUP) - 0xa9 (OP_HASH160) - 0x14 (20 bytes len) - <20 bytes pubkey hash> - 0x88 (OP_EQUALVERIFY) - 0xac (OP_CHECKSIG)
    if script_len as u32 == P2PKH_SCRIPT_SIZE && output_script[0..=2] == [OpCode::OpDup as u8, OpCode::OpHash160 as u8, HASH160_SIZE_HEX] {
        if output_script[script_len - 2..] != [OpCode::OpEqualVerify as u8, OpCode::OpCheckSig as u8] {
            return Err(Error::MalformedP2PKHOutput);
        }
        return Ok(output_script[3..script_len-2].to_vec());
    }

    // P2SH
    // 23 bytes
    // Format: 
    // 0xa9 (OP_HASH160) - 0x14 (20 bytes hash) - <20 bytes script hash> - 0x87 (OP_EQUAL)
    if script_len as u32 == P2SH_SCRIPT_SIZE && output_script[0..=1] == [OpCode::OpHash160 as u8, HASH160_SIZE_HEX] {
        if output_script[script_len-1] != OpCode::OpEqual as u8 {
            return Err(Error::MalformedP2SHOutput)
        }
        return Ok(output_script[2..(script_len-1)].to_vec())
    }
    return Err(Error::UnsupportedOutputFormat);
}

pub fn extract_op_return_data(output_script: &[u8]) -> Result<Vec<u8>, Error> {
    if output_script[0] != OpCode::OpReturn as u8 {
        return Err(Error::MalformedOpReturnOutput);
    }
    // Check for max OP_RETURN size
    // 83 in total, see here: https://github.com/bitcoin/bitcoin/blob/f018d0c9cd7f408dac016b6bfc873670de713d27/src/script/standard.h#L30
    if output_script.len() > MAX_OPRETURN_SIZE {
        return Err(Error::MalformedOpReturnOutput);
    }

    Ok(output_script[2..].to_vec())
}


#[cfg(test)]
mod tests {
    use super::*;

    // examples from https://bitcoin.org/en/developer-reference#block-headers

    #[test]
    fn test_parse_block_header() {
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

    fn sample_coinbase_transaction_input() -> String {
        "00000000000000000000000000000000".to_owned() +
        "00000000000000000000000000000000" + // Previous outpoint TXID
        "ffffffff"                         + // Previous outpoint index
        "29"                               + // Bytes in coinbase
        "03"                               + // Bytes in height
        "4e0105"                           + // Height: 328014
        "062f503253482f0472d35454085fffed" +
        "f2400000f90f54696d65202620486561" +
        "6c74682021"                       + // Arbitrary data
        "00000000" // Sequence
    }

    fn sample_transaction_input() -> String {
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
        "ffffffff" // Sequence number: UINT32_MAX
    }

    fn sample_transaction_output() -> String {
        "f0ca052a01000000".to_owned() +      // Satoshis (49.99990000 BTC)
        "19" +                               // Bytes in pubkey script: 25
        "76" +                               // OP_DUP
        "a9" +                               // OP_HASH160
        "14" +                               // Push 20 bytes as data
        "cbc20a7664f2f69e5355aa427045bc15" +
        "e7c6c772" +                         // PubKey hash
        "88" +                               // OP_EQUALVERIFY
        "ac"                                 // OP_CHECKSIG
    }

    fn sample_transaction() -> String {
        "01000000".to_owned() +               // Version
        "02"                  +               // Number of inputs
        &sample_coinbase_transaction_input() +
        &sample_transaction_input() +
        "01" +                                // Number of outputs
        &sample_transaction_output() +
        "00000000"
    }

    /*
    fn sample_malformed_witness_output() -> String {
        "00000000220017".to_owned()
    }

    fn sample_malformed_witness_output_large() -> String {
        "0000000".to_owned() + "0000000000100FF111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111"
    }
   

    fn sample_malformed_p2sh_output() -> String {
        "0000000017a914FF".to_owned()
    }

    fn sample_malformed_p2pkh_output() -> String { 
        "000000001976a914FFFF".to_owned()
    }
    */


    fn sample_valid_p2pkh() -> String {
        "76a914000000000000000000000000000000000000000088ac".to_owned()
    }

    fn sample_valid_p2sh() -> String {
        "a914000000000000000000000000000000000000000087".to_owned()
    }

    #[test]
    fn test_parse_coinbase_transaction_input() {
        let raw_input = sample_coinbase_transaction_input();
        let input_bytes = bitcoin_spv::utils::deserialize_hex(&raw_input).unwrap();
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
        let raw_input = sample_transaction_input();
        let input_bytes = bitcoin_spv::utils::deserialize_hex(&raw_input).unwrap();
        let mut parser = BytesParser::new(&input_bytes);
        let input: TransactionInput = parser.parse_with(2).unwrap();
        assert_eq!(input.coinbase, false);
        assert_eq!(input.sequence, u32::max_value());
        assert_eq!(input.previous_index, 0);
        assert_eq!(input.height, None);
        assert_eq!(input.script.len(), 73);

        let previous_hash =
            H256Le::from_hex_le("7b1eabe0209b1fe794124575ef807057c77ada2138ae4fa8d6c4de0398a14f3f");
        assert_eq!(input.previous_hash, previous_hash);
    }

    #[test]
    fn test_parse_transaction_output() {
        let raw_output = sample_transaction_output();
        let output_bytes = bitcoin_spv::utils::deserialize_hex(&raw_output).unwrap();
        let mut parser = BytesParser::new(&output_bytes);
        let output: TransactionOutput = parser.parse().unwrap();
        assert_eq!(output.value, 4999990000);
        assert_eq!(output.script.len(), 25);
    }

    #[test]
    fn test_parse_transaction() {
        let raw_tx = sample_transaction();
        let tx_bytes = hex::decode(&raw_tx).unwrap();
        let transaction = parse_transaction(&tx_bytes).unwrap();
        let inputs = transaction.inputs;
        let outputs = transaction.outputs;
        assert_eq!(transaction.version, 1);
        assert_eq!(inputs.len(), 2);
        assert_eq!(inputs[0].coinbase, true);
        assert_eq!(inputs[1].coinbase, false);
        assert_eq!(outputs.len(), 1);
        assert_eq!(transaction.locktime, None);
        assert_eq!(transaction.block_height, Some(0));
    }

    #[test]
    fn test_parse_transaction_extended_format() {
        let raw_tx = "020000000001010000000000000000000000000000000000000000000000000000000000000000ffffffff0502cb000101ffffffff02400606950000000017a91466c7060feb882664ae62ffad0051fe843e318e85870000000000000000266a24aa21a9ede5c17d15b8b1fa2811b7e6da66ffa5e1aaa05922c69068bf90cd585b95bb46750120000000000000000000000000000000000000000000000000000000000000000000000000";
        let tx_bytes = hex::decode(&raw_tx).unwrap();
        let transaction = parse_transaction(&tx_bytes).unwrap();
        let inputs = transaction.inputs;
        let outputs = transaction.outputs;
        assert_eq!(transaction.version, 2);
        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs[0].coinbase, true);
        assert_eq!(inputs[0].witness.is_some(), true);
        assert_eq!(outputs.len(), 2);
        assert_eq!(&hex::encode(&outputs[0].script), "a91466c7060feb882664ae62ffad0051fe843e318e8587");
        assert_eq!(&hex::encode(&outputs[1].script), "6a24aa21a9ede5c17d15b8b1fa2811b7e6da66ffa5e1aaa05922c69068bf90cd585b95bb4675");
        assert_eq!(transaction.block_height, Some(0));
        assert_eq!(transaction.locktime, None);
    }

    #[test]
    fn test_extract_address_hash_valid_p2pkh(){
        let p2pkh_script = bitcoin_spv::utils::deserialize_hex(&sample_valid_p2pkh()).unwrap();

        let p2pkh_address: [u8; 20] = [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0];

        let extr_p2pkh = extract_address_hash(&p2pkh_script).unwrap();

        assert_eq!(&extr_p2pkh, &p2pkh_address);
    }

    #[test]
    fn test_extract_address_hash_valid_p2sh(){
        let p2sh_script = bitcoin_spv::utils::deserialize_hex(&sample_valid_p2sh()).unwrap();

        let p2sh_address: [u8; 20] = [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0];

        let extr_p2sh = extract_address_hash(&p2sh_script).unwrap();

        assert_eq!(&extr_p2sh, &p2sh_address);
    }

    /*
    #[test]
    fn test_extract_address_invalid_p2pkh_fails() {
        let p2pkh_script = bitcoin_spv::utils::deserialize_hex(&sample_malformed_p2pkh_output()).unwrap();

        assert_eq!(extract_address_hash(&p2pkh_script).err(), Some(Error::MalformedP2PKHOutput));
    }
    */
}
