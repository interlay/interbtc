#[cfg(test)]
extern crate mocktopus;

extern crate bitcoin_hashes;

use bitcoin_hashes::{hash160::Hash as Hash160, Hash};
use sha2::{Digest, Sha256};

#[cfg(test)]
use mocktopus::macros::mockable;

use crate::{Error, SetCompact};
use sp_core::U256;
use sp_std::{prelude::*, vec};

use crate::{address::Address, types::*};

// https://github.com/bitcoin-core/secp256k1/blob/1e5d50fa93d71d751b95eec6a80f6732879a0071/include/secp256k1.h#L180-L181
const SECP256K1_TAG_PUBKEY_EVEN: u8 = 0x02;
const SECP256K1_TAG_PUBKEY_ODD: u8 = 0x03;

// https://github.com/bitcoin/bitcoin/blob/7fcf53f7b4524572d1d0c9a5fdc388e87eb02416/src/script/script.h#L39
const LOCKTIME_THRESHOLD: u32 = 500_000_000;

/// Type to be parsed from a bytes array
pub(crate) trait Parsable: Sized {
    fn parse(raw_bytes: &[u8], position: usize) -> Result<(Self, usize), Error>;
}

/// Type to be parsed from a bytes array using extra metadata
pub(crate) trait ParsableMeta<Metadata>: Sized {
    fn parse_with(raw_bytes: &[u8], position: usize, extra: Metadata) -> Result<(Self, usize), Error>;
}

/// Macro to generate `Parsable` implementation of uint types
macro_rules! make_parsable_int {
    ($type:ty, $bytes:expr) => {
        impl Parsable for $type {
            fn parse(raw_bytes: &[u8], position: usize) -> Result<($type, usize), Error> {
                if position + $bytes > raw_bytes.len() {
                    return Err(Error::EndOfFile);
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
        let last_byte = sp_std::cmp::min(position + 3, raw_bytes.len());
        let (value, bytes_consumed) = parse_compact_uint(raw_bytes.get(position..last_byte).ok_or(Error::EndOfFile)?)?;
        Ok((CompactUint { value }, bytes_consumed))
    }
}

impl Parsable for BlockHeader {
    fn parse(raw_bytes: &[u8], position: usize) -> Result<(BlockHeader, usize), Error> {
        let slice = raw_bytes.get(position..position + 80).ok_or(Error::EndOfFile)?;
        let header_bytes = RawBlockHeader::from_bytes(slice)?;
        let block_header = parse_block_header(&header_bytes)?;
        Ok((block_header, 80))
    }
}

impl Parsable for H256Le {
    fn parse(raw_bytes: &[u8], position: usize) -> Result<(H256Le, usize), Error> {
        let slice = raw_bytes.get(position..position + 32).ok_or(Error::EndOfFile)?;
        Ok((H256Le::from_bytes_le(slice), 32))
    }
}

impl<T: Parsable> Parsable for Vec<T> {
    fn parse(raw_bytes: &[u8], position: usize) -> Result<(Vec<T>, usize), Error> {
        let mut result: Vec<T> = Vec::new();
        let slice = raw_bytes.get(position..).ok_or(Error::EndOfFile)?;
        let mut parser = BytesParser::new(slice);
        let count: CompactUint = parser.parse()?;
        for _ in 0..count.value {
            result.push(parser.parse()?);
        }
        Ok((result, parser.position))
    }
}

impl<T, U: Copy> ParsableMeta<U> for Vec<T>
where
    T: ParsableMeta<U>,
{
    fn parse_with(raw_bytes: &[u8], position: usize, extra: U) -> Result<(Vec<T>, usize), Error> {
        let mut result: Vec<T> = Vec::new();
        let slice = raw_bytes.get(position..).ok_or(Error::EndOfFile)?;
        let mut parser = BytesParser::new(slice);
        let count: CompactUint = parser.parse()?;
        for _ in 0..count.value {
            result.push(parser.parse_with(extra)?);
        }
        Ok((result, parser.position))
    }
}

impl Parsable for Vec<bool> {
    fn parse(raw_bytes: &[u8], position: usize) -> Result<(Vec<bool>, usize), Error> {
        let byte = *raw_bytes.get(position).ok_or(Error::EndOfFile)?;
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
    fn parse_with(raw_bytes: &[u8], position: usize, version: i32) -> Result<(TransactionInput, usize), Error> {
        let slice = raw_bytes.get(position..).ok_or(Error::EndOfFile)?;
        parse_transaction_input(slice, version)
    }
}

impl Parsable for TransactionOutput {
    fn parse(raw_bytes: &[u8], position: usize) -> Result<(TransactionOutput, usize), Error> {
        let slice = raw_bytes.get(position..).ok_or(Error::EndOfFile)?;
        parse_transaction_output(slice)
    }
}

impl Parsable for U256 {
    fn parse(raw_bytes: &[u8], position: usize) -> Result<(U256, usize), Error> {
        if position + 4 > raw_bytes.len() {
            return Err(Error::EndOfFile);
        }

        let mut bytes: [u8; 4] = Default::default();
        bytes.copy_from_slice(&raw_bytes[position..position + 4]);

        let bits = u32::from_le_bytes(bytes);
        let compact = U256::set_compact(bits).ok_or(Error::InvalidCompact)?;
        Ok((compact, 4))
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
        self.position = self
            .position
            .checked_add(bytes_consumed)
            .ok_or(Error::ArithmeticOverflow)?;
        Ok(result)
    }

    /// Peeks at the next byte without updating the parser head.
    pub(crate) fn next(&self) -> Result<u8, Error> {
        self.raw_bytes.get(self.position).ok_or(Error::EndOfFile).map(|i| *i)
    }

    /// This is the same as `parse` but allows to pass extra data to the parser
    /// Fails if there are not enough bytes to read or if the
    /// underlying `Parsable` parse function fails
    pub(crate) fn parse_with<T, U>(&mut self, extra: U) -> Result<T, Error>
    where
        T: ParsableMeta<U>,
    {
        let (result, bytes_consumed) = T::parse_with(&self.raw_bytes, self.position, extra)?;
        self.position = self
            .position
            .checked_add(bytes_consumed)
            .ok_or(Error::ArithmeticOverflow)?;
        Ok(result)
    }

    /// Reads `bytes_count` from the bytes parser and moves the head
    /// Fails if there are not enough bytes to read
    pub(crate) fn read(&mut self, bytes_count: usize) -> Result<Vec<u8>, Error> {
        let bytes = self
            .raw_bytes
            .get(self.position..self.position + bytes_count)
            .ok_or(Error::EndOfFile)?;
        self.position = self
            .position
            .checked_add(bytes_count)
            .ok_or(Error::ArithmeticOverflow)?;
        Ok(Vec::from(bytes))
    }
}

/// Allows to parse the given structure from little-endian encoded bytes
pub trait FromLeBytes: Sized {
    fn from_le_bytes(bytes: &[u8]) -> Result<Self, Error>;
}

impl FromLeBytes for BlockHeader {
    fn from_le_bytes(bytes: &[u8]) -> Result<BlockHeader, Error> {
        parse_block_header(&RawBlockHeader::from_bytes(bytes)?)
    }
}

// like parse_block_header, but without the version check. This is needed in testing, to test
// with historical data
pub fn parse_block_header_lenient(raw_header: &RawBlockHeader) -> Result<BlockHeader, Error> {
    let mut parser = BytesParser::new(raw_header.as_bytes());
    let version: i32 = parser.parse()?;
    let hash_prev_block: H256Le = parser.parse()?;
    let merkle_root: H256Le = parser.parse()?;
    let timestamp: u32 = parser.parse()?;
    let target: U256 = parser.parse()?;
    let nonce: u32 = parser.parse()?;
    let hash: H256Le = raw_header.hash();

    let block_header = BlockHeader {
        merkle_root,
        target,
        timestamp,
        version,
        hash_prev_block,
        nonce,
        hash,
    };

    Ok(block_header)
}

/// Parses the raw bitcoin header into a Rust struct
///
/// # Arguments
///
/// * `header` - An 80-byte Bitcoin header
pub fn parse_block_header(raw_header: &RawBlockHeader) -> Result<BlockHeader, Error> {
    let block_header = parse_block_header_lenient(raw_header)?;

    if block_header.version < 4 {
        // as per bip65, we reject block versions less than 4. Note that the reason
        // we can hardcode this, is that bitcoin switched to version 4 in december
        // 2015, and the genesis of the bridge will never be set to a genesis from
        // before that date.
        // see https://github.com/bitcoin/bips/blob/master/bip-0065.mediawiki#spv-clients
        return Err(Error::InvalidBlockVersion);
    }

    Ok(block_header)
}

/// Returns the value of a compactly encoded uint and the number of bytes consumed
///
/// # Arguments
///
/// * `varint` - A slice containing the header
pub fn parse_compact_uint(varint: &[u8]) -> Result<(u64, usize), Error> {
    match varint.get(0).ok_or(Error::EndOfFile)? {
        0xfd => {
            let mut num_bytes: [u8; 2] = Default::default();
            num_bytes.copy_from_slice(&varint.get(1..3).ok_or(Error::EndOfFile)?);
            Ok((u16::from_le_bytes(num_bytes) as u64, 3))
        }
        0xfe => {
            let mut num_bytes: [u8; 4] = Default::default();
            num_bytes.copy_from_slice(&varint.get(1..5).ok_or(Error::EndOfFile)?);
            Ok((u32::from_le_bytes(num_bytes) as u64, 5))
        }
        0xff => {
            let mut num_bytes: [u8; 8] = Default::default();
            num_bytes.copy_from_slice(&varint.get(1..9).ok_or(Error::EndOfFile)?);
            Ok((u64::from_le_bytes(num_bytes) as u64, 9))
        }
        &n => Ok((n as u64, 1)),
    }
}

/// Parses a single bitcoin transaction
/// Serialization format is documented below
/// <https://github.com/bitcoin/bitcoin/blob/master/src/primitives/transaction.h#L182>
/// # Arguments
///
/// * `raw_transaction` - the raw bytes of the transaction
#[cfg_attr(test, mockable)]
pub fn parse_transaction(raw_transaction: &[u8]) -> Result<Transaction, Error> {
    let mut parser = BytesParser::new(raw_transaction);
    let version: i32 = parser.parse()?;

    // fail if incorrect version: we only support version 1 and 2
    if version != 1 && version != 2 {
        return Err(Error::MalformedTransaction);
    }

    let allow_witness = (version & SERIALIZE_TRANSACTION_NO_WITNESS) == 0;

    // TODO: bound maximum?
    let mut inputs: Vec<TransactionInput> = parser.parse_with(version)?;

    let mut flags: u8 = 0;
    if inputs.is_empty() && allow_witness {
        flags = parser.parse()?;
        inputs = parser.parse_with(version)?;
    }

    // TODO: bound maximum?
    let outputs: Vec<TransactionOutput> = parser.parse()?;

    if (flags & 1) != 0 && allow_witness {
        flags ^= 1;
        for input in &mut inputs {
            input.with_witness(parser.parse()?);
        }

        if inputs.iter().all(|input| input.witness.is_empty()) {
            // A transaction with a set witness-flag must actually include witnesses in the transaction.
            // see https://github.com/bitcoin/bitcoin/blob/be4171679b8eab8205e04ff86140329bd67878a0/src/primitives/transaction.h#L214-L217
            return Err(Error::MalformedTransaction);
        }
    }

    // https://en.bitcoin.it/wiki/NLockTime
    let locktime_or_blockheight: u32 = parser.parse()?;
    let lock_at = if locktime_or_blockheight < LOCKTIME_THRESHOLD {
        LockTime::BlockHeight(locktime_or_blockheight)
    } else {
        LockTime::Time(locktime_or_blockheight)
    };

    if flags != 0 {
        return Err(Error::MalformedTransaction);
    }

    Ok(Transaction {
        version,
        inputs,
        outputs,
        lock_at,
    })
}

/// Parses a transaction input
fn parse_transaction_input(raw_input: &[u8], version: i32) -> Result<(TransactionInput, usize), Error> {
    let mut parser = BytesParser::new(raw_input);
    let previous_hash: H256Le = parser.parse()?;
    let previous_index: u32 = parser.parse()?;

    // coinbase input has no previous hash
    let is_coinbase = previous_hash == H256Le::zero();

    // fail if transaction is coinbase and previous index is not 0xffffffff
    // previous_hash
    if is_coinbase && previous_index != u32::max_value() {
        return Err(Error::MalformedTransaction);
    }

    let mut script_size: u64 = parser.parse::<CompactUint>()?.value;
    let source = if is_coinbase {
        let height = if version != 2 {
            // version 1 does not include height
            None
        } else {
            // version 2 transactions include a height as the first 4 bytes, see
            // https://github.com/bitcoin/bips/blob/master/bip-0034.mediawiki
            let height_size: u64 = parser.parse::<CompactUint>()?.value;
            script_size = script_size.checked_sub(height_size + 1).ok_or(Error::EndOfFile)?;

            let mut buffer = [0u8; 4];
            let bytes = parser.read(height_size as usize)?;
            buffer[..3].copy_from_slice(bytes.get(0..3).ok_or(Error::EndOfFile)?);
            Some(u32::from_le_bytes(buffer))
        };

        TransactionInputSource::Coinbase(height)
    } else {
        TransactionInputSource::FromOutput(previous_hash, previous_index)
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
            source,
            script,
            sequence,
            witness: vec![],
        },
        consumed_bytes,
    ))
}

fn parse_transaction_output(raw_output: &[u8]) -> Result<(TransactionOutput, usize), Error> {
    let mut parser = BytesParser::new(raw_output);
    let value: i64 = parser.parse()?;
    let script_size: CompactUint = parser.parse()?;
    if script_size.value > 10_000 {
        return Err(Error::MalformedTransaction);
    }
    let script = parser.read(script_size.value as usize)?;
    Ok((
        TransactionOutput {
            value,
            script: script.into(),
        },
        parser.position,
    ))
}

pub(crate) fn extract_address_hash_witness<B: AsRef<[u8]>>(witness_script: B) -> Result<Address, Error> {
    let witness_script = witness_script.as_ref();
    // first check if the witness is the compressed public key
    // https://github.com/bitcoin/bips/blob/master/bip-0141.mediawiki#p2wpkh
    if witness_script.len() == 33 {
        let prefix = witness_script[0];
        // the first byte describes whether the Y-coordinate is even or odd, the remaining
        // 32-bytes are the X-coordinate of the underlying point on the elliptic curve
        if prefix == SECP256K1_TAG_PUBKEY_EVEN || prefix == SECP256K1_TAG_PUBKEY_ODD {
            return Ok(Address::P2WPKHv0(H160::from_slice(
                &Hash160::hash(witness_script).to_vec(),
            )));
        }
        // NOTE: as defined in BIP143, version 0 witness programs do
        // not support uncompressed public keys
    }

    // otherwise assume that the witness is the redeem script
    // https://github.com/bitcoin/bips/blob/master/bip-0141.mediawiki#p2wsh
    let mut hasher = Sha256::default();
    hasher.input(witness_script);
    Ok(Address::P2WSHv0(H256::from_slice(&hasher.result()[..])))
}

pub(crate) fn extract_address_hash_scriptsig(input_script: &[u8]) -> Result<Address, Error> {
    let mut parser = BytesParser::new(input_script);
    let mut p2pkh = true;

    // Multisig OBOE hack -> p2sh
    if parser.next()? == OpCode::Op0 as u8 {
        parser.parse::<u8>()?;
        p2pkh = false;
    }

    let sig_size: u64 = parser.parse::<CompactUint>()?.value;

    // P2WPKH-P2SH (SegWit)
    if parser.next()? == OpCode::Op0 as u8 {
        let sig = parser.read(sig_size as usize)?;
        return Ok(Address::P2SH(H160::from_slice(&Hash160::hash(&sig).to_vec())));
    }

    let _sig = parser.read(sig_size as usize)?;

    let redeem_script_size: u64 = parser.parse::<CompactUint>()?.value;

    // if not p2sh, redeem script is just 33-byte pubkey
    if p2pkh && redeem_script_size != 33 {
        return Err(Error::UnsupportedInputFormat);
    }
    let redeem_script = parser.read(redeem_script_size as usize)?;
    let hash = H160::from_slice(&Hash160::hash(&redeem_script).to_vec());
    Ok(if p2pkh {
        Address::P2PKH(hash)
    } else {
        Address::P2SH(hash)
    })
}

pub(crate) fn extract_op_return_data(output_script: &[u8]) -> Result<Vec<u8>, Error> {
    if *output_script.get(0).ok_or(Error::EndOfFile)? != OpCode::OpReturn as u8 {
        return Err(Error::MalformedOpReturnOutput);
    }
    // Check for max OP_RETURN size
    // 83 in total, see here: https://github.com/bitcoin/bitcoin/blob/f018d0c9cd7f408dac016b6bfc873670de713d27/src/script/standard.h#L30
    if output_script.len() > MAX_OPRETURN_SIZE {
        return Err(Error::MalformedOpReturnOutput);
    }

    let result = output_script.get(2..).ok_or(Error::EndOfFile)?;

    if result.len() != output_script[1] as usize {
        return Err(Error::MalformedOpReturnOutput);
    }

    Ok(result.to_vec())
}

#[cfg(test)]
pub(crate) mod tests {
    use std::str::FromStr;

    use super::*;
    use crate::{Address, PublicKey, Script};
    use frame_support::{assert_err, assert_ok};

    // examples from https://bitcoin.org/en/developer-reference#block-headers

    #[test]
    fn test_parse_block_header() {
        let hex_header = sample_block_header();
        let raw_header = RawBlockHeader::from_hex(&hex_header).unwrap();
        let parsed_header = parse_block_header(&raw_header).unwrap();
        assert_eq!(parsed_header.version, 4);
        assert_eq!(parsed_header.timestamp, 1415239972);
        assert_eq!(
            format!("{:x}", parsed_header.merkle_root),
            "7114b3aa8a049bbc12cdde1008a2dd70e2ed045f698593ca869394ee52aa109d"
        );
        assert_eq!(
            format!("{:x}", parsed_header.hash_prev_block),
            "00000000000000000cca48eb4b330d91e8d946d344ca302a86a280161b0bffb6"
        );
        let expected_target = String::from("680733321990486529407107157001552378184394215934016880640");
        assert_eq!(parsed_header.target.to_string(), expected_target);
    }

    #[test]
    fn test_parse_block_header_with_too_low_version_fails() {
        let hex_header_without_version = "000000b6ff0b1b1680a2862a30ca44d346d9e8910d334beb48ca0c00000000000000009d10aa52ee949386ca9385695f04ede270dda20810decd12bc9b048aaab3147124d95a5430c31b18fe9f0864";
        let valid_header_hex = "04".to_string() + hex_header_without_version;
        let invalid_header_hex = "03".to_string() + hex_header_without_version;

        assert_ok!(parse_block_header(
            &RawBlockHeader::from_hex(&valid_header_hex).unwrap()
        ));
        assert_err!(
            parse_block_header(&RawBlockHeader::from_hex(&invalid_header_hex).unwrap()),
            Error::InvalidBlockVersion
        );
    }

    #[test]
    fn test_parse_compact_uint() {
        let cases = [
            (&[1, 2, 3][..], (1, 1)),
            (&[253, 2, 3][..], (770, 3)),
            (&[254, 2, 3, 8, 1, 8][..], (17302274, 5)),
            (&[255, 6, 0xa, 3, 8, 1, 0xb, 2, 7, 8][..], (504978207276206598, 9)),
        ];
        for (input, expected) in cases.iter() {
            assert_eq!(parse_compact_uint(input).unwrap(), *expected);
        }
    }

    pub fn sample_coinbase_transaction_input() -> String {
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

    pub fn sample_transaction_input() -> String {
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

    pub fn sample_transaction_output() -> String {
        "f0ca052a01000000".to_owned() +      // Satoshis (49.99990000 BTC)
        "19" +                               // Bytes in pubkey script: 25
        "76" +                               // OP_DUP
        "a9" +                               // OP_HASH160
        "14" +                               // Push 20 bytes as data
        "cbc20a7664f2f69e5355aa427045bc15" +
        "e7c6c772" +                         // PubKey hash
        "88" +                               // OP_EQUALVERIFY
        "ac" // OP_CHECKSIG
    }

    pub fn sample_transaction() -> String {
        "01000000".to_owned() +               // Version
        "02"                  +               // Number of inputs
        &sample_coinbase_transaction_input() +
        &sample_transaction_input() +
        "01" +                                // Number of outputs
        &sample_transaction_output() +
        "00000000"
    }

    pub fn sample_extended_transaction() -> String {
        // id: c586389e5e4b3acb9d6c8be1c19ae8ab2795397633176f5a6442a261bbdefc3a
        "0200000000010140d43a99926d43eb0e619bf0b3d83b4a31f60c176beecfb9d35bf45e54d0f7420100000017160014a4b4ca48de0b3fffc15404a1acdc8dbaae226955ffffffff0100e1f5050000000017a9144a1154d50b03292b3024370901711946cb7cccc387024830450221008604ef8f6d8afa892dee0f31259b6ce02dd70c545cfcfed8148179971876c54a022076d771d6e91bed212783c9b06e0de600fab2d518fad6f15a2b191d7fbd262a3e0121039d25ab79f41f75ceaf882411fd41fa670a4c672c23ffaf0e361a969cde0692e800000000".to_owned()
    }

    fn sample_valid_p2pkh() -> String {
        "76a914000000000000000000000000000000000000000088ac".to_owned()
    }

    fn sample_valid_p2sh() -> String {
        "a914000000000000000000000000000000000000000087".to_owned()
    }

    pub fn sample_block_header() -> String {
        "04000000".to_owned() + // ............... Block version: 4
            "b6ff0b1b1680a2862a30ca44d346d9e8" + //
            "910d334beb48ca0c0000000000000000" + // ... Hash of previous block's header
            "9d10aa52ee949386ca9385695f04ede2" + //
            "70dda20810decd12bc9b048aaab31471" + // ... Merkle root
            "24d95a54" + // ........................... Unix time: 1415239972
            "30c31b18" + // ........................... Target: 0x1bc330 * 256**(0x18-3)
            "fe9f0864"
    }

    #[test]
    fn test_parse_coinbase_transaction_input() {
        let raw_input = sample_coinbase_transaction_input();
        let input_bytes = hex::decode(&raw_input).unwrap();
        let mut parser = BytesParser::new(&input_bytes);
        let input: TransactionInput = parser.parse_with(2).unwrap();
        assert!(matches!(input.source, TransactionInputSource::Coinbase(Some(328014))));
        assert_eq!(input.sequence, 0);
        assert_eq!(input.script.len(), 37); // 0x29 - 4
    }

    #[test]
    fn test_parse_transaction_input() {
        let raw_input = sample_transaction_input();
        let input_bytes = hex::decode(&raw_input).unwrap();
        let mut parser = BytesParser::new(&input_bytes);
        let input: TransactionInput = parser.parse_with(2).unwrap();

        let previous_hash = H256Le::from_hex_le("7b1eabe0209b1fe794124575ef807057c77ada2138ae4fa8d6c4de0398a14f3f");

        assert!(matches!(input.source, TransactionInputSource::FromOutput(hash, 0) if hash == previous_hash));
        assert_eq!(input.sequence, u32::max_value());
        assert_eq!(input.script.len(), 73);
    }

    #[test]
    fn test_parse_transaction_output() {
        let raw_output = sample_transaction_output();
        let output_bytes = hex::decode(&raw_output).unwrap();
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
        assert!(matches!(inputs[0].source, TransactionInputSource::Coinbase(_)));
        assert!(matches!(inputs[1].source, TransactionInputSource::FromOutput(_, _)));
        assert_eq!(outputs.len(), 1);
        assert_eq!(transaction.lock_at, LockTime::BlockHeight(0));
    }

    #[test]
    fn test_parse_transaction_extended_format() {
        let raw_tx = sample_extended_transaction();
        let tx_bytes = hex::decode(&raw_tx).unwrap();
        let transaction = parse_transaction(&tx_bytes).unwrap();
        let inputs = transaction.inputs;
        let outputs = transaction.outputs;
        assert_eq!(transaction.version, 2);
        assert_eq!(inputs.len(), 1);
        assert!(matches!(inputs[0].source, TransactionInputSource::FromOutput(_, _)));
        assert_eq!(inputs[0].witness.len(), 2);
        assert_eq!(inputs[0].witness[0].len(), 72);
        assert_eq!(inputs[0].witness[1].len(), 33);
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].value, 100000000);
        assert_eq!(
            &outputs[0].script.as_hex(),
            "a9144a1154d50b03292b3024370901711946cb7cccc387"
        );
        assert_eq!(transaction.lock_at, LockTime::BlockHeight(0));
    }

    #[test]
    fn test_parse_transaction_multi_inputs() {
        let raw_tx = "02000000000105a6f0d82981c7d7fd424c97548be1b246161097532e102c0457f46ad5870698910000000000ffffffffa6f0d82981c7d7fd424c97548be1b246161097532e102c0457f46ad5870698910d00000000ffffffffa6f0d82981c7d7fd424c97548be1b246161097532e102c0457f46ad5870698914c00000000ffffffffa6f0d82981c7d7fd424c97548be1b246161097532e102c0457f46ad5870698912e00000000ffffffffa6f0d82981c7d7fd424c97548be1b246161097532e102c0457f46ad5870698913500000000ffffffff01a032eb0500000000160014c97ec9439f77c079983582847a09b6b5e6fd2e86024830450221008bf5d1ea3868a10a7acd5e793fd5f8a2468b5546d1f1e721d77f7666d457a786022065c9167fd6300be52f593267b3af49be1c8b87c333063cc0f6412e9902b80520012103eec785a16054b40bfe15c287beca7f214f88742501fabbe18251502c0ea0588f02483045022100d4c7892b69a49a44163c9d61d89ea1e9273247bd6c8f332d57abbb30257c2f5c022035b96a00ae2a7fece639af849e281238bc98bc7d971fe906af15a874a4eb1844012103eec785a16054b40bfe15c287beca7f214f88742501fabbe18251502c0ea0588f0247304402204336575b363780eb2b4c7bdee9b0109d3b92965f9ba431beae1c4803d0e0704a0220667228268d99dff834dc4d372063d6dd4f80e0df2b3a0168bd4748e16c70aeec012103eec785a16054b40bfe15c287beca7f214f88742501fabbe18251502c0ea0588f0247304402203b5e9dcca5937a6bae4b844ad598316ef30ad82512a2a08e534b9a2af58dceea02202bef0b6d1f421b6416d3dc0e2d99f78e5e4892933dd5973cdcab005109917ffd012103eec785a16054b40bfe15c287beca7f214f88742501fabbe18251502c0ea0588f0248304502210092f9f9eaecf35f7b11d7f12026874fd2e0f595fb216885110ae53ea94fd5744502203867f4e1af5b4ea84721ea16443d25126e917ab52fc50eb7613ab90423f3df25012103eec785a16054b40bfe15c287beca7f214f88742501fabbe18251502c0ea0588f00000000";
        let tx_bytes = hex::decode(&raw_tx).unwrap();
        let transaction = parse_transaction(&tx_bytes).unwrap();
        let inputs = transaction.inputs;
        let outputs = transaction.outputs;
        assert_eq!(transaction.version, 2);
        assert_eq!(inputs.len(), 5);
        assert_eq!(outputs.len(), 1);
    }

    #[test]
    fn test_parse_transaction_multi_outputs() {
        let raw_tx = "01000000000101109d2e41430bfdec7e6dfb02bf78b5827eeb717ef25210ff3203b0db8c76c9260000000000ffffffff0a1085970000000000160014bbef244bcad13cffb68b5cef3017c742367555221085970000000000160014bbef244bcad13cffb68b5cef3017c742367555221085970000000000160014bbef244bcad13cffb68b5cef3017c742367555221085970000000000160014bbef244bcad13cffb68b5cef3017c742367555221085970000000000160014bbef244bcad13cffb68b5cef3017c742367555221085970000000000160014bbef244bcad13cffb68b5cef3017c742367555221085970000000000160014bbef244bcad13cffb68b5cef3017c742367555221085970000000000160014bbef244bcad13cffb68b5cef3017c742367555221085970000000000160014bbef244bcad13cffb68b5cef3017c742367555221085970000000000160014bbef244bcad13cffb68b5cef3017c7423675552202473044022078d531212bf562a403d8469f78e684d8de5b7998abadba48272f659f73326c6502207f45a0e0b3463940fd30f39fde95464af3549bd0e793ee07c2407311d6fadbaf0121026ccfb8061f235cc110697c0bfb3afb99d82c886672f6b9b5393b25a434c0cbf300000000";
        let tx_bytes = hex::decode(&raw_tx).unwrap();
        let transaction = parse_transaction(&tx_bytes).unwrap();
        let inputs = transaction.inputs;
        let outputs = transaction.outputs;
        assert_eq!(transaction.version, 1);
        assert_eq!(inputs.len(), 1);
        assert_eq!(outputs.len(), 10);
    }

    #[test]
    fn test_parse_transaction_containing_taproot() {
        let raw_tx = "010000000001013413e41f47eecad702082578c35a2925217056fd0a837b22f1a205fe178a010d0500000000ffffffff19771000000000000017a91415f691c1905082c300362d48540846c30855162d877a1000000000000022512038234fa3e3ca718dfadfb540c320180e68798e67e0a9d4f10d98ea33d37caf047a100000000000001976a914d73838271ee26471aa3640915ed7274b49435b6688acee2000000000000016001470eab26ae0074a58802acc7c38cd9941619c408d14250000000000001976a91479ef95650e8284c3be439d888cf2ee2d1d8ef63088ac3129000000000000160014a558dd2db8167e069f580da2482a9b73dc4f5960217f0000000000001976a91409f3607112083fb1ffe3718214a8e5d5eb0da46188ac04a50000000000001600149215c14609d581aacaa54f629e823cc8abd17ee6c7cd00000000000017a9146da59c9a54a5465402884712bbbe140bc68a4f218728f700000000000017a914ed99cbd06b43b4e3741d1457f7af7b24c2e8d12487ae380100000000001976a91448296f6f29c497f59193ab4e7def5f2e03ef2f9988ac654901000000000017a914ba997376b5daaa3707aefdf30cc09745b579df2187a6a301000000000017a914ffed3c6e71adc2b73939d6951f4655ed1432909b87ec9202000000000017a9147759a1bffe2acca168afdb5b106250b02a703b2887d63603000000000016001439fef3095e8a3bce11ce471aa602bf3e3609d8ddae3703000000000017a914ea0d18bbd804d17a1f2f07ed9aa1670721777d2287cd370300000000001976a914bcc6bcffe584761176d8f510896e882f838208d988ac1d3803000000000016001470eb59ad925fdec71ca0ec50cf7c6b9bbe8dc7592f380300000000001976a91447eb6c94d7b2ac0c11eb3957c0844d333e21d02e88ac724803000000000016001470eb59ad925fdec71ca0ec50cf7c6b9bbe8dc759692e050000000000160014ae26178c1a9b4adb6f24f047fa119e034205900c381b10000000000017a914c9e20b0d7e46d07a878585955ca377db833d181587d32b20000000000017a914bbfcd0b601046e1656ba9b74a98ee8d362d5b63687402f200000000000160014ca146a720a30ca404e979df59d3ddca039e8fd58f22fea0000000000220020935f3eb059cd94bd307e6378bd590724f361f0316fd0964eb5952f274dfb7b4f0400483045022100c9fc44a423e31fc792f5d255ae09ffdc0b224cb70fcebacd52183ce2813ba11d022046c8530230f644be4a05f25bd6a2264b99afc7e3e38531d4bde12d477d03f18001473044022027f50b14154123b173286db76e189a32973a13b0b4ca425329533229cf7f8d9a02202cea81a657ee654c63ab4a01a741931378abae036435a1d695622216596d9e27016952210257bf4070df9735de32305f3bc25320d331edb10c662423e06cd1e50bc58d8fa7210246454540c4e36ba6a481347d0194ffe476640289aecfd2d3f3db1328415b9a5c210248e0a3385d6f744ae81779e10f8ccafbbed7d44debf08a2b0d5250e2f0a0e84853aef0210b00";
        let tx_bytes = hex::decode(&raw_tx).unwrap();
        let transaction = parse_transaction(&tx_bytes).unwrap();
        let inputs = transaction.inputs;
        let outputs = transaction.outputs;
        assert_eq!(transaction.version, 1);
        assert_eq!(inputs.len(), 1);
        assert_eq!(outputs.len(), 25);
    }

    #[test]
    fn test_extract_address_hash_valid_p2pkh() {
        let p2pkh_script = hex::decode(&sample_valid_p2pkh()).unwrap();

        let p2pkh_hash: [u8; 20] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

        let script = Script::from(p2pkh_script);
        let payload = Address::from_script_pub_key(&script).unwrap();

        assert!(matches!(payload, Address::P2PKH(hash) if hash.as_bytes() == p2pkh_hash))
    }

    #[test]
    fn test_extract_address_hash_valid_p2sh() {
        let p2sh_script = hex::decode(&sample_valid_p2sh()).unwrap();

        let p2sh_hash: [u8; 20] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

        let script = Script::from(p2sh_script);
        let payload = Address::from_script_pub_key(&script).unwrap();

        assert!(matches!(payload, Address::P2SH(hash) if hash.as_bytes() == p2sh_hash))
    }

    #[test]
    fn test_extract_address_hash_p2pkh_scriptsig_from_public_key() {
        let script_sig = PublicKey([
            2, 168, 49, 109, 0, 14, 227, 106, 112, 84, 59, 37, 153, 238, 121, 44, 66, 8, 181, 64, 248, 19, 137, 27, 47,
            222, 50, 95, 187, 221, 152, 165, 69,
        ])
        .to_p2pkh_script_sig(vec![1; 32]);

        let extr_address = extract_address_hash_scriptsig(script_sig.as_bytes()).unwrap();
        assert_eq!(
            extr_address,
            Address::P2PKH(H160([
                80, 10, 21, 194, 142, 226, 119, 74, 230, 18, 7, 88, 187, 232, 227, 97, 20, 80, 235, 9
            ]))
        );
    }

    #[test]
    fn test_extract_address_hash_p2pkh_in_p2sh_scriptsig_from_public_key() {
        let script_sig = PublicKey([
            2, 139, 220, 235, 13, 249, 164, 152, 179, 4, 175, 217, 170, 84, 218, 179, 182, 247, 109, 48, 57, 152, 241,
            165, 225, 26, 242, 187, 160, 225, 248, 195, 250,
        ])
        .to_p2sh_script_sig(vec![1; 32]);

        let extr_address = extract_address_hash_scriptsig(script_sig.as_bytes()).unwrap();
        assert_eq!(
            extr_address,
            Address::P2SH(H160([
                24, 49, 81, 119, 128, 234, 237, 59, 97, 156, 209, 13, 224, 143, 34, 170, 227, 63, 97, 46
            ]))
        );
    }

    #[test]
    fn test_extract_address_hash_scriptsig() {
        let raw_tx = "0100000001c15041a06deb6b3818b022fac558da4ce2097f0860c8f642105bbad9d29be02a010000006c493046022100cfd2a2d332b29adce119c55a9fadd3c073332024b7e272513e51623ca15993480221009b482d7f7b4d479aff62bdcdaea54667737d56f8d4d63dd03ec3ef651ed9a25401210325f8b039a11861659c9bf03f43fc4ea055f3a71cd60c7b1fd474ab578f9977faffffffff0290d94000000000001976a9148ed243a7be26080a1a8cf96b53270665f1b8dd2388ac4083086b000000001976a9147e7d94d0ddc21d83bfbcfc7798e4547edf0832aa88ac00000000";
        let tx_bytes = hex::decode(&raw_tx).unwrap();
        let transaction = parse_transaction(&tx_bytes).unwrap();

        let address = Address::P2PKH(H160([
            126, 125, 148, 208, 221, 194, 29, 131, 191, 188, 252, 119, 152, 228, 84, 126, 223, 8, 50, 170,
        ]));
        let extr_address = extract_address_hash_scriptsig(&transaction.inputs[0].script).unwrap();

        assert_eq!(&extr_address, &address);
    }

    #[test]
    fn test_extract_address_hash_scriptsig_p2sh() {
        let raw_tx = "0100000001c8cc2b56525e734ff63a13bc6ad06a9e5664df8c67632253a8e36017aee3ee40000000009000483045022100ad0851c69dd756b45190b5a8e97cb4ac3c2b0fa2f2aae23aed6ca97ab33bf88302200b248593abc1259512793e7dea61036c601775ebb23640a0120b0dba2c34b79001455141042f90074d7a5bf30c72cf3a8dfd1381bdbd30407010e878f3a11269d5f74a58788505cdca22ea6eab7cfb40dc0e07aba200424ab0d79122a653ad0c7ec9896bdf51aefeffffff0120f40e00000000001976a9141d30342095961d951d306845ef98ac08474b36a088aca7270400";
        let tx_bytes = hex::decode(&raw_tx).unwrap();
        let transaction = parse_transaction(&tx_bytes).unwrap();

        let address = Address::P2SH(H160([
            233, 195, 221, 12, 7, 170, 199, 97, 121, 235, 199, 106, 108, 120, 212, 214, 124, 108, 22, 10,
        ]));
        let extr_address = extract_address_hash_scriptsig(&transaction.inputs[0].script).unwrap();

        assert_eq!(&extr_address, &address);
    }

    #[test]
    fn test_extract_address_hash_scriptsig_p2wpkh_p2sh_testnet() {
        let expected = Address::P2SH(H160::from_slice(
            &hex::decode("068a6a2ec6be7d6e7aac1657445154c52db0cef8").unwrap(),
        ));
        let actual =
            extract_address_hash_scriptsig(&hex::decode("160014473ca3f4d726ce9c21af7cdc3fcc13264f681b04").unwrap())
                .unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_extract_address_hash_scriptsig_p2sh_segwit() {
        // source: https://blockstream.info/tx/0a0d7b9ab879fbd7a096e856fa5461dbae959ac86d51451c211a65fb8e95f54b?expand
        let raw_tx = "02000000000101a1dcf3ca033463e346339642dd7305e33de4ce5ab179d1e19b1eb146534421660000000017160014a97a9058829417d4c581ad5004b6e46cc680063dfdffffff01b9010000000000001600143b05c08e224ddec538ac7aa2e3b6583b983807a302473044022051480b10ef40d12bce982d1d08176a403f176dd3e51189c07a0a9584ddb8e91602204a02134b2b892904a3519da0044e97da9ae78232f8f7678fea0b6531bf3104130121039dcac4d315739516bf5cea98bc6a9cfb49cb6269beb67c520bc5ecacc3c7d47206c70900";
        let tx_bytes = hex::decode(&raw_tx).unwrap();
        let transaction = parse_transaction(&tx_bytes).unwrap();

        // 35PLQyoXs2sk9QDqMv7bBGowxP1pjwXAMe
        let address = Address::P2SH(H160::from_str(&"288873634ae24a3c9b6792cc7e2a084ec79ef68b").unwrap());
        let extr_address = extract_address_hash_scriptsig(&transaction.inputs[0].script).unwrap();
        assert_eq!(&extr_address, &address);
    }

    /*
    #[test]
    fn test_extract_address_invalid_p2pkh_fails() {
        let p2pkh_script = hex::decode(&sample_malformed_p2pkh_output()).unwrap();

        assert_eq!(extract_address_hash_scriptpubkey(&p2pkh_script).err(), Some(Error::MalformedP2PKHOutput));
    }
    */
}
