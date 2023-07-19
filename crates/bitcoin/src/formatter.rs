use sp_core::U256;
use sp_std::{prelude::*, vec, vec::Vec};

use crate::{merkle::MerkleProof, script::*, types::*, Error, GetCompact};

pub(crate) const WITNESS_FLAG: u8 = 0x01;
pub(crate) const WITNESS_MARKER: u8 = 0x00;

pub trait Writer {
    fn write(&mut self, buf: &[u8]) -> Result<(), Error>;
}

impl Writer for Vec<u8> {
    fn write(&mut self, buf: &[u8]) -> Result<(), Error> {
        self.extend_from_slice(buf);
        Ok(())
    }
}

pub(crate) struct BoundedWriter {
    length_bound: u32,
    bytes: Vec<u8>,
}

impl BoundedWriter {
    pub(crate) fn new(length_bound: u32) -> Self {
        Self {
            length_bound,
            bytes: Vec::new(),
        }
    }

    fn checked_reduce(&mut self, bytes: u32) -> Result<(), Error> {
        self.length_bound
            .checked_sub(bytes)
            .map(|new_self| self.length_bound = new_self)
            .ok_or(Error::BoundExceeded)
    }

    pub(crate) fn result(&self) -> Vec<u8> {
        self.bytes.clone()
    }
}

impl Writer for BoundedWriter {
    fn write(&mut self, buf: &[u8]) -> Result<(), Error> {
        self.checked_reduce(buf.len() as u32)?;
        self.bytes.extend_from_slice(buf);
        Ok(())
    }
}

/// Type to be formatted as a bytes array
pub trait TryFormat: Sized {
    fn try_format<W: Writer>(&self, w: &mut W) -> Result<(), Error>;
}

/// Macro to generate `TryFormat` implementation of int types
macro_rules! make_try_format_int {
    ($type:ty) => {
        impl TryFormat for $type {
            fn try_format<W: Writer>(&self, w: &mut W) -> Result<(), Error> {
                w.write(&self.to_le_bytes())
            }
        }
    };
}

impl<T> TryFormat for &T
where
    T: TryFormat,
{
    fn try_format<W: Writer>(&self, w: &mut W) -> Result<(), Error> {
        T::try_format(self, w)
    }
}

// Generate `TryFormat` implementation for the basic integers (signed and unsgined) types
make_try_format_int!(u8);
make_try_format_int!(u16);
make_try_format_int!(u32);
make_try_format_int!(u64);
make_try_format_int!(i8);
make_try_format_int!(i16);
make_try_format_int!(i32);
make_try_format_int!(i64);

impl TryFormat for bool {
    fn try_format<W: Writer>(&self, w: &mut W) -> Result<(), Error> {
        (*self as u8).try_format(w)
    }
}

impl TryFormat for H256Le {
    fn try_format<W: Writer>(&self, w: &mut W) -> Result<(), Error> {
        w.write(&self.to_bytes_le())
    }
}

impl TryFormat for CompactUint {
    fn try_format<W: Writer>(&self, w: &mut W) -> Result<(), Error> {
        if self.value < 0xfd {
            (self.value as u8).try_format(w)?;
        } else if self.value < u16::max_value() as u64 {
            0xfd_u8.try_format(w)?;
            (self.value as u16).try_format(w)?;
        } else if self.value < u32::max_value() as u64 {
            0xfe_u8.try_format(w)?;
            (self.value as u32).try_format(w)?;
        } else {
            0xff_u8.try_format(w)?;
            self.value.try_format(w)?;
        }
        Ok(())
    }
}

impl<T> TryFormat for Vec<T>
where
    for<'a> &'a T: TryFormat,
{
    fn try_format<W: Writer>(&self, w: &mut W) -> Result<(), Error> {
        CompactUint {
            value: self.len() as u64,
        }
        .try_format(w)?;
        for value in self.iter() {
            value.try_format(w)?;
        }
        Ok(())
    }
}

impl TryFormat for TransactionInput {
    fn try_format<W: Writer>(&self, w: &mut W) -> Result<(), Error> {
        let (previous_hash, previous_index) = match self.source {
            TransactionInputSource::Coinbase(_) => (H256Le::zero(), u32::max_value()),
            TransactionInputSource::FromOutput(hash, index) => (hash, index),
        };
        previous_hash.try_format(w)?;
        previous_index.try_format(w)?;

        if let TransactionInputSource::Coinbase(Some(height)) = self.source {
            let height_bytes = Script::height(height);
            // account for the height in version 2 blocks
            let script_len = self.script.len().saturating_add(height_bytes.len());

            CompactUint::from_usize(script_len).try_format(w)?;
            height_bytes.as_bytes().try_format(w)?;
        } else {
            CompactUint::from_usize(self.script.len()).try_format(w)?;
        }
        w.write(&self.script)?; // we already formatted the length
        self.sequence.try_format(w)?;
        Ok(())
    }
}

impl TryFormat for Script {
    fn try_format<W: Writer>(&self, w: &mut W) -> Result<(), Error> {
        self.bytes.try_format(w)
    }
}

impl TryFormat for &[u8] {
    fn try_format<W: Writer>(&self, w: &mut W) -> Result<(), Error> {
        w.write(*self)
    }
}

impl TryFormat for H160 {
    fn try_format<W: Writer>(&self, w: &mut W) -> Result<(), Error> {
        w.write(self.as_bytes())
    }
}

impl TryFormat for H256 {
    fn try_format<W: Writer>(&self, w: &mut W) -> Result<(), Error> {
        w.write(self.as_bytes())
    }
}

impl TryFormat for OpCode {
    fn try_format<W: Writer>(&self, w: &mut W) -> Result<(), Error> {
        (*self as u8).try_format(w)
    }
}

impl TryFormat for TransactionOutput {
    fn try_format<W: Writer>(&self, w: &mut W) -> Result<(), Error> {
        self.value.try_format(w)?;
        self.script.try_format(w)?;
        Ok(())
    }
}

impl TryFormat for LockTime {
    fn try_format<W: Writer>(&self, w: &mut W) -> Result<(), Error> {
        match self {
            LockTime::BlockHeight(b) | LockTime::Time(b) => b.try_format(w),
        }
    }
}

impl TryFormat for Transaction {
    fn try_format<W: Writer>(&self, w: &mut W) -> Result<(), Error> {
        self.version.try_format(w)?;

        if !self.has_witness() {
            self.inputs.try_format(w)?;
            self.outputs.try_format(w)?;
        } else {
            WITNESS_MARKER.try_format(w)?;
            WITNESS_FLAG.try_format(w)?;
            self.inputs.try_format(w)?;
            self.outputs.try_format(w)?;
            for input in self.inputs.iter() {
                input.witness.try_format(w)?;
            }
        }

        self.lock_at.try_format(w)?;
        Ok(())
    }
}

// https://developer.bitcoin.org/reference/block_chain.html#target-nbits
impl TryFormat for U256 {
    fn try_format<W: Writer>(&self, w: &mut W) -> Result<(), Error> {
        let bits = self.clone().get_compact().ok_or(Error::InvalidCompact)?;
        w.write(&bits.to_le_bytes())
    }
}

impl TryFormat for BlockHeader {
    fn try_format<W: Writer>(&self, w: &mut W) -> Result<(), Error> {
        self.version.try_format(w)?;
        self.hash_prev_block.try_format(w)?;
        self.merkle_root.try_format(w)?;
        self.timestamp.try_format(w)?;
        self.target.try_format(w)?;
        self.nonce.try_format(w)?;
        Ok(())
    }
}

impl TryFormat for Block {
    fn try_format<W: Writer>(&self, w: &mut W) -> Result<(), Error> {
        self.header.try_format(w)?;
        self.transactions.try_format(w)?;
        Ok(())
    }
}

/// Block header (80 bytes)
/// Number of transactions in the block (unsigned int, 4 bytes, little endian)
/// Number of hashes (varint, 1 - 3 bytes)
/// Hashes (N * 32 bytes, little endian)
/// Number of bytes of flag bits (varint, 1 - 3 bytes)
/// Flag bits (little endian)
impl TryFormat for MerkleProof {
    fn try_format<W: Writer>(&self, w: &mut W) -> Result<(), Error> {
        self.block_header.try_format(w)?;
        self.transactions_count.try_format(w)?;
        self.hashes.clone().try_format(w)?;

        let len = self
            .flag_bits
            .len()
            .checked_add(7)
            .ok_or(Error::ArithmeticOverflow)?
            .checked_div(8)
            .ok_or(Error::ArithmeticUnderflow)?;
        let mut bytes: Vec<u8> = vec![0; len];
        for p in 0..self.flag_bits.len() {
            bytes[p.checked_div(8).ok_or(Error::ArithmeticUnderflow)?] |= (self.flag_bits[p] as u8) << (p % 8) as u8;
        }
        bytes.try_format(w)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parser, utils::sha256d_le};
    use frame_support::{assert_err, assert_ok};

    fn try_format<T: TryFormat>(data: T) -> Vec<u8> {
        let mut writer = Vec::new();
        data.try_format(&mut writer).unwrap();
        writer
    }

    #[test]
    fn test_format_int_types() {
        assert_eq!(try_format(1u8), [1]);
        assert_eq!(try_format(1i8), [1]);
        assert_eq!(try_format(255u8), [255]);
        assert_eq!(try_format(-1i8), [255]);

        assert_eq!(try_format(256u16), [0, 1]);
        assert_eq!(try_format(0xffffu32 + 1), [0, 0, 1, 0]);
        assert_eq!(try_format(0xffffffu32 + 1), [0, 0, 0, 1]);
        assert_eq!(try_format(u64::max_value()), [0xff].repeat(8));
    }

    #[test]
    fn test_format_compact_uint() {
        assert_eq!(try_format(CompactUint { value: 0xfa }), [0xfa]);
        assert_eq!(try_format(CompactUint { value: 0xff }), [0xfd, 0xff, 0]);
        let u32_cuint = CompactUint { value: 0xffff + 1 };
        assert_eq!(try_format(u32_cuint), [0xfe, 0, 0, 1, 0]);
        let u64_cuint = CompactUint {
            value: u64::max_value(),
        };
        assert_eq!(try_format(u64_cuint), [0xff].repeat(9));
    }

    #[test]
    fn test_format_transaction_input() {
        let raw_input = parser::tests::sample_transaction_input();
        let input_bytes = hex::decode(&raw_input).unwrap();
        let mut parser = parser::BytesParser::new(&input_bytes);
        let input: TransactionInput = parser.parse_with(2).unwrap();
        let formatted = try_format(input);
        assert_eq!(formatted, input_bytes);
    }

    #[test]
    fn test_format_transaction_output() {
        let raw_output = parser::tests::sample_transaction_output();
        let output_bytes = hex::decode(&raw_output).unwrap();
        let mut parser = parser::BytesParser::new(&output_bytes);
        let output: TransactionOutput = parser.parse().unwrap();
        let formatted = try_format(output);
        assert_eq!(formatted, output_bytes);
    }

    #[test]
    fn test_format_transaction() {
        let raw_tx = parser::tests::sample_transaction();
        let tx_bytes = hex::decode(&raw_tx).unwrap();
        let transaction = crate::parser::parse_transaction(&tx_bytes).unwrap();
        let formatted = try_format(transaction);
        assert_eq!(formatted, tx_bytes);
    }

    #[test]
    fn test_format_transaction_bounded_writer_fails() {
        let raw_tx = parser::tests::sample_transaction();
        let tx_bytes = hex::decode(&raw_tx).unwrap();
        let transaction = crate::parser::parse_transaction(&tx_bytes).unwrap();
        assert_ok!(transaction.try_format(&mut BoundedWriter::new(tx_bytes.len() as u32)));
        assert_err!(
            transaction.try_format(&mut BoundedWriter::new(tx_bytes.len() as u32 - 1)),
            Error::BoundExceeded
        );
    }

    #[test]
    fn test_format_extended_transaction() {
        let expected_hash = H256Le::from_hex_be("b759d39a8596b70b3a46700b83e1edb247e17ba58df305421864fe7a9ac142ea");
        let expected_txid = H256Le::from_hex_be("c586389e5e4b3acb9d6c8be1c19ae8ab2795397633176f5a6442a261bbdefc3a");
        let raw_tx = parser::tests::sample_extended_transaction();
        let tx_bytes = hex::decode(&raw_tx).unwrap();
        let transaction = parser::parse_transaction(&tx_bytes).unwrap();
        let formatted = try_format(transaction.clone());
        assert_eq!(formatted, tx_bytes);
        let computed_hash = sha256d_le(&formatted);
        assert_eq!(computed_hash, expected_hash);
        let mut formatted_no_witness = vec![];
        transaction.format_no_witness(&mut formatted_no_witness).unwrap();
        let computed_txid = sha256d_le(&formatted_no_witness);
        assert_eq!(computed_txid, expected_txid);
    }

    #[test]
    fn test_format_block_header() {
        let hex_header = parser::tests::sample_block_header();
        let parsed_header = BlockHeader::from_hex(&hex_header).unwrap();
        assert_eq!(hex::encode(try_format(parsed_header)), hex_header);
    }

    #[test]
    fn test_format_block_header_testnet() {
        let hex_header = "00000020b0b3d77b97015b519553423c96642b33ca534c50ecefd133640000000000000029a0a725684aeca24af83e3ba0a3e3ee56adfdf032d19e5acba6d0a262e1580ca354915fd4c8001ac42a7b3a".to_string();
        let raw_header = hex::decode(&hex_header).unwrap();
        let parsed_header = BlockHeader::from_bytes(&raw_header).unwrap();

        assert_eq!(
            parsed_header,
            BlockHeader {
                merkle_root: H256Le::from_hex_be("0c58e162a2d0a6cb5a9ed132f0fdad56eee3a3a03b3ef84aa2ec4a6825a7a029"),
                target: U256::from_dec_str("1260618571951953247774709397757627131971305851995253681160192").unwrap(),
                timestamp: 1603359907,
                version: 536870912,
                hash: sha256d_le(&raw_header),
                hash_prev_block: H256Le::from_hex_be(
                    "000000000000006433d1efec504c53ca332b64963c425395515b01977bd7b3b0"
                ),
                nonce: 981150404,
            }
        );

        assert_eq!(hex::encode(try_format(parsed_header)), hex_header);
    }

    // taken from https://bitcoin.org/en/developer-reference#block-headers
    #[test]
    fn test_format_u256() {
        let value = U256::from_dec_str("680733321990486529407107157001552378184394215934016880640").unwrap();
        let result = try_format(value);
        let expected = hex::decode("30c31b18").unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_format_u256_testnet() {
        // 0xb8e4a3e93640d7a4623e92589e40960b4b20420478d7ed60662176c323cf4caa
        let value = U256::from_dec_str("1260618571951953247774709397757627131971305851995253681160192").unwrap();
        let result = try_format(value);
        let expected = hex::decode("d4c8001a").unwrap();
        assert_eq!(result, expected);
    }

    const PROOF_HEX: &str = "00004020e2ac770a4f511b7ed2f3b638fe12d39ff52b8ced104d360500000000000000006f5ca47842fdd12f46a274ce7060c701d0c1fcff294a826e19b88e8f3dcdbca8f560135e8b64051816587c9c1f0100000bc21da39408e165a8368a7df46a17af25b4c5e3778b45222e48da632412b3be56e3b1196586e514fba3145219e3d9edb1e0e2c71b4cedaf013d8512d121f55e1ae120e954338e4d63d0a446a466b4ec548704366a89c2513c0c47818e4f8af8fa141bcda354451c2a48425704decd178df3c2c518c2fee2a593058b2c2c2ddee80ebc68aa38c161fcbf32f336b9d06feb652893be3326b0fd755cf61e575a56d7cb6b4944a2e74e3fdb583885c9dd4849ab2fd974207d9693a3062d9ba5eb0ea1b7c2d9841297396526c43af19fa8e67f3a6c07f9c8333eda575556df0e8b86a65982f24022336589fae3d56d69d73474024ced4f3a63c7205623d5bd22daf8a58e69b4748539fcdc24e0241f8231278b560340a3eb112f2fd041dc7bd1a0f6ddc37b916c24b0f96a1e9e13b4ffc7ad9c3805cadb91520435821edd439ca70198c92187deb1dde075366006d963632a0fd1ca510b362bbd6cf1805ac70becd3d303ff2d00";

    #[test]
    fn test_format_merkle_proof() {
        let proof = MerkleProof::parse(&hex::decode(PROOF_HEX).unwrap()).unwrap();
        let expected = hex::decode(PROOF_HEX).unwrap();
        assert_eq!(try_format(proof), expected);
    }
}
