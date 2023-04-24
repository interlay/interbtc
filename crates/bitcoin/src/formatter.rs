use sp_core::U256;
use sp_std::{prelude::*, vec, vec::Vec};

use crate::{merkle::MerkleProof, script::*, types::*, Error, GetCompact};

const WITNESS_FLAG: u8 = 0x01;
const WITNESS_MARKER: u8 = 0x00;

trait CheckedReduce: Sized {
    fn checked_reduce(&mut self, amount: Self) -> Result<(), Error>;
}

impl CheckedReduce for u32 {
    fn checked_reduce(&mut self, amount: Self) -> Result<(), Error> {
        self.checked_sub(amount)
            .map(|new_self| *self = new_self)
            .ok_or(Error::ArithmeticUnderflow)
    }
}

/// Type to be formatted as a bytes array
pub trait TryFormat<Options: Default = ()>: Sized {
    fn format_size(&self, options: Options) -> u32;
    fn try_format(&self, length_bound: &mut u32) -> Result<Vec<u8>, Error>;
    fn try_format_with(&self, _options: Options, length_bound: &mut u32) -> Result<Vec<u8>, Error> {
        self.try_format(length_bound)
    }
}

/// Macro to generate `TryFormat` implementation of int types
macro_rules! make_try_format_int {
    ($type:ty) => {
        impl TryFormat<bool> for $type {
            fn format_size(&self, _: bool) -> u32 {
                sp_std::mem::size_of::<Self>() as u32
            }

            fn try_format(&self, length_bound: &mut u32) -> Result<Vec<u8>, Error> {
                length_bound.checked_reduce(self.format_size(true))?;
                Ok(Vec::from(&self.to_le_bytes()[..]))
            }

            fn try_format_with(&self, be: bool, length_bound: &mut u32) -> Result<Vec<u8>, Error> {
                if be {
                    length_bound.checked_reduce(self.format_size(true))?;
                    Ok(Vec::from(&self.to_be_bytes()[..]))
                } else {
                    self.try_format(length_bound)
                }
            }
        }
    };
}

impl<T, U> TryFormat<U> for &T
where
    T: TryFormat<U>,
    U: Default,
{
    fn format_size(&self, options: U) -> u32 {
        T::format_size(self, options)
    }

    fn try_format(&self, length_bound: &mut u32) -> Result<Vec<u8>, Error> {
        T::try_format(self, length_bound)
    }

    fn try_format_with(&self, options: U, length_bound: &mut u32) -> Result<Vec<u8>, Error> {
        T::try_format_with(self, options, length_bound)
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

impl TryFormat<()> for bool {
    fn format_size(&self, _: ()) -> u32 {
        sp_std::mem::size_of::<u8>() as u32
    }

    fn try_format(&self, length_bound: &mut u32) -> Result<Vec<u8>, Error> {
        (*self as u8).try_format(length_bound)
    }
}

impl TryFormat<()> for H256Le {
    fn format_size(&self, _: ()) -> u32 {
        Self::len_bytes() as u32
    }

    fn try_format(&self, length_bound: &mut u32) -> Result<Vec<u8>, Error> {
        length_bound.checked_reduce(self.format_size(()))?;
        Ok(Vec::from(&self.to_bytes_le()[..]))
    }
}

impl TryFormat for CompactUint {
    fn format_size(&self, _: ()) -> u32 {
        match self.value {
            0..=0xFC => 1,
            0xFD..=0xFFFF => 3,
            0x10000..=0xFFFFFFFF => 5,
            _ => 9,
        }
    }

    fn try_format(&self, length_bound: &mut u32) -> Result<Vec<u8>, Error> {
        let mut formatter = Formatter::new();
        if self.value < 0xfd {
            formatter.try_format(self.value as u8, length_bound)?;
        } else if self.value < u16::max_value() as u64 {
            formatter.try_format(0xfd_u8, length_bound)?;
            formatter.try_format(self.value as u16, length_bound)?;
        } else if self.value < u32::max_value() as u64 {
            formatter.try_format(0xfe_u8, length_bound)?;
            formatter.try_format(self.value as u32, length_bound)?;
        } else {
            formatter.try_format(0xff_u8, length_bound)?;
            formatter.try_format(self.value, length_bound)?;
        }
        Ok(formatter.result())
    }
}

impl<T, U> TryFormat<U> for Vec<T>
where
    for<'a> &'a T: TryFormat<U>,
    U: Default + Copy,
{
    fn format_size(&self, options: U) -> u32 {
        let mut size = CompactUint::from_usize(self.len()).format_size(());
        for value in self.iter() {
            size += value.format_size(options);
        }
        size
    }

    fn try_format(&self, length_bound: &mut u32) -> Result<Vec<u8>, Error> {
        self.try_format_with(Default::default(), length_bound)
    }

    fn try_format_with(&self, options: U, length_bound: &mut u32) -> Result<Vec<u8>, Error> {
        let mut formatter = Formatter::new();
        formatter.try_format(
            CompactUint {
                value: self.len() as u64,
            },
            length_bound,
        )?;
        for value in self.iter() {
            formatter.try_format_with(value, options, length_bound)?;
        }
        Ok(formatter.result())
    }
}

impl TryFormat for TransactionInput {
    fn format_size(&self, _: ()) -> u32 {
        let mut size = 0;
        size += H256Le::len_bytes() as u32;
        size += sp_std::mem::size_of::<u32>() as u32;
        if let TransactionInputSource::Coinbase(Some(height)) = self.source {
            size += Script::height(height).format_size(());
        }
        size += self.script.format_size(false);
        size += self.sequence.format_size(false);
        size
    }

    fn try_format(&self, length_bound: &mut u32) -> Result<Vec<u8>, Error> {
        let mut formatter = Formatter::new();
        let (previous_hash, previous_index) = match self.source {
            TransactionInputSource::Coinbase(_) => (H256Le::zero(), u32::max_value()),
            TransactionInputSource::FromOutput(hash, index) => (hash, index),
        };
        formatter.try_format(&previous_hash, length_bound)?;
        formatter.try_format(previous_index, length_bound)?;
        formatter.try_format(CompactUint::from_usize(self.script.len()), length_bound)?;
        if let TransactionInputSource::Coinbase(Some(height)) = self.source {
            formatter.try_format(Script::height(height).as_bytes(), length_bound)?;
        }
        formatter.try_output(&self.script, length_bound)?; // we already formatted the length
        formatter.try_format(self.sequence, length_bound)?;
        Ok(formatter.result())
    }
}

impl TryFormat for Script {
    fn format_size(&self, _: ()) -> u32 {
        self.bytes.format_size(false)
    }

    fn try_format(&self, length_bound: &mut u32) -> Result<Vec<u8>, Error> {
        self.bytes.try_format(length_bound)
    }
}

impl TryFormat for &[u8] {
    fn format_size(&self, _: ()) -> u32 {
        self.len() as u32
    }

    fn try_format(&self, length_bound: &mut u32) -> Result<Vec<u8>, Error> {
        length_bound.checked_reduce(self.format_size(()))?;
        Ok(Vec::from(*self))
    }
}

impl TryFormat for H160 {
    fn format_size(&self, _: ()) -> u32 {
        Self::len_bytes() as u32
    }

    fn try_format(&self, length_bound: &mut u32) -> Result<Vec<u8>, Error> {
        length_bound.checked_reduce(self.format_size(()))?;
        Ok(Vec::from(self.as_bytes()))
    }
}

impl TryFormat for H256 {
    fn format_size(&self, _: ()) -> u32 {
        Self::len_bytes() as u32
    }

    fn try_format(&self, length_bound: &mut u32) -> Result<Vec<u8>, Error> {
        length_bound.checked_reduce(self.format_size(()))?;
        Ok(Vec::from(self.as_bytes()))
    }
}

impl TryFormat for OpCode {
    fn format_size(&self, _: ()) -> u32 {
        sp_std::mem::size_of::<u8>() as u32
    }

    fn try_format(&self, length_bound: &mut u32) -> Result<Vec<u8>, Error> {
        length_bound.checked_reduce(self.format_size(()))?;
        Ok(vec![*self as u8])
    }
}

impl TryFormat for TransactionOutput {
    fn format_size(&self, _: ()) -> u32 {
        let mut size = 0;
        size += self.value.format_size(false);
        size += self.script.format_size(());
        size
    }

    fn try_format(&self, length_bound: &mut u32) -> Result<Vec<u8>, Error> {
        let mut formatter = Formatter::new();
        formatter.try_format(self.value, length_bound)?;
        formatter.try_format(&self.script, length_bound)?;
        Ok(formatter.result())
    }
}

impl TryFormat<bool> for Transaction {
    fn format_size(&self, witness: bool) -> u32 {
        let mut size = 0;
        size += self.version.format_size(witness);
        let allow_witness = (self.version & SERIALIZE_TRANSACTION_NO_WITNESS) == 0;
        if witness && allow_witness && self.has_witness() {
            size += WITNESS_MARKER.format_size(witness);
            size += WITNESS_FLAG.format_size(witness);
            for input in self.inputs.iter() {
                size += input.witness.format_size(witness);
            }
        }
        size += self.inputs.format_size(());
        size += self.outputs.format_size(());
        match self.lock_at {
            LockTime::BlockHeight(b) | LockTime::Time(b) => size += b.format_size(witness),
        };
        size
    }

    fn try_format(&self, length_bound: &mut u32) -> Result<Vec<u8>, Error> {
        self.try_format_with(true, length_bound)
    }

    fn try_format_with(&self, witness: bool, length_bound: &mut u32) -> Result<Vec<u8>, Error> {
        let mut formatter = Formatter::new();
        let allow_witness = (self.version & SERIALIZE_TRANSACTION_NO_WITNESS) == 0;
        // NOTE: doesn't format witnesses for tx_id
        let format_witness = witness && allow_witness && self.has_witness();

        formatter.try_format(self.version, length_bound)?;

        if format_witness {
            formatter.try_format(WITNESS_MARKER, length_bound)?;
            formatter.try_format(WITNESS_FLAG, length_bound)?;
        }

        formatter.try_format(&self.inputs, length_bound)?;
        formatter.try_format(&self.outputs, length_bound)?;

        if format_witness {
            for input in self.inputs.iter() {
                formatter.try_format(&input.witness, length_bound)?;
            }
        }

        match self.lock_at {
            LockTime::BlockHeight(b) | LockTime::Time(b) => formatter.try_format(b, length_bound)?,
        };

        Ok(formatter.result())
    }
}

// https://developer.bitcoin.org/reference/block_chain.html#target-nbits
impl TryFormat for U256 {
    fn format_size(&self, _: ()) -> u32 {
        sp_std::mem::size_of::<u32>() as u32
    }

    fn try_format(&self, _length_bound: &mut u32) -> Result<Vec<u8>, Error> {
        let bits = self.clone().get_compact().ok_or(Error::InvalidCompact)?;
        Ok(bits.to_le_bytes().to_vec())
    }
}

impl TryFormat for BlockHeader {
    fn format_size(&self, _: ()) -> u32 {
        let mut size = 0;
        size += self.version.format_size(false);
        size += self.hash_prev_block.format_size(());
        size += self.merkle_root.format_size(());
        size += self.timestamp.format_size(false);
        size += self.target.format_size(());
        size += self.nonce.format_size(false);
        size
    }

    fn try_format(&self, length_bound: &mut u32) -> Result<Vec<u8>, Error> {
        let mut formatter = Formatter::new();
        formatter.try_format(self.version, length_bound)?;
        formatter.try_format(self.hash_prev_block, length_bound)?;
        formatter.try_format(self.merkle_root, length_bound)?;
        formatter.try_format(self.timestamp, length_bound)?;
        formatter.try_format(self.target, length_bound)?;
        formatter.try_format(self.nonce, length_bound)?;
        Ok(formatter.result())
    }
}

impl TryFormat for Block {
    fn format_size(&self, _: ()) -> u32 {
        let mut size = 0;
        size += self.header.format_size(());
        size += self.transactions.format_size(true);
        size
    }

    fn try_format(&self, length_bound: &mut u32) -> Result<Vec<u8>, Error> {
        let mut formatter = Formatter::new();
        formatter.try_format(self.header, length_bound)?;
        formatter.try_format(&self.transactions, length_bound)?;
        Ok(formatter.result())
    }
}

/// Block header (80 bytes)
/// Number of transactions in the block (unsigned int, 4 bytes, little endian)
/// Number of hashes (varint, 1 - 3 bytes)
/// Hashes (N * 32 bytes, little endian)
/// Number of bytes of flag bits (varint, 1 - 3 bytes)
/// Flag bits (little endian)
impl TryFormat for MerkleProof {
    fn format_size(&self, _: ()) -> u32 {
        let mut size = 0;
        size += self.block_header.format_size(());
        size += self.transactions_count.format_size(false);
        size += self.hashes.format_size(());
        let len = ((self.flag_bits.len() + 7) / 8) as u32;
        size += 1 + len;
        size
    }

    fn try_format(&self, length_bound: &mut u32) -> Result<Vec<u8>, Error> {
        let mut formatter = Formatter::new();
        formatter.try_format(self.block_header, length_bound)?;
        formatter.try_format(self.transactions_count, length_bound)?;
        formatter.try_format(self.hashes.clone(), length_bound)?;

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
        formatter.try_format(bytes.len() as u8, length_bound)?;
        formatter.try_output(&bytes, length_bound)?;

        Ok(formatter.result())
    }
}

pub(crate) struct Formatter {
    bytes: Vec<u8>,
}

impl Formatter {
    fn new() -> Formatter {
        Formatter { bytes: Vec::new() }
    }

    fn try_output(&mut self, bytes: &[u8], length_bound: &mut u32) -> Result<(), Error> {
        length_bound.checked_reduce(bytes.len() as u32)?;
        self.bytes.extend(bytes);
        Ok(())
    }

    fn try_format<T, U>(&mut self, value: T, length_bound: &mut u32) -> Result<(), Error>
    where
        T: TryFormat<U>,
        U: Default,
    {
        self.bytes.extend(value.try_format(length_bound)?);
        Ok(())
    }

    fn try_format_with<T, U>(&mut self, value: T, data: U, length_bound: &mut u32) -> Result<(), Error>
    where
        T: TryFormat<U>,
        U: Default,
    {
        self.bytes.extend(value.try_format_with(data, length_bound)?);
        Ok(())
    }

    pub(crate) fn result(self) -> Vec<u8> {
        self.bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parser, utils::sha256d_le};
    use frame_support::{assert_err, assert_ok};

    #[test]
    fn test_format_int_types() {
        assert_eq!(1u8.try_format(&mut u32::max_value()).unwrap(), [1]);
        assert_eq!(1i8.try_format(&mut u32::max_value()).unwrap(), [1]);
        assert_eq!(255u8.try_format(&mut u32::max_value()).unwrap(), [255]);
        assert_eq!((-1i8).try_format(&mut u32::max_value()).unwrap(), [255]);

        assert_eq!(256u16.try_format(&mut u32::max_value()).unwrap(), [0, 1]);
        assert_eq!(256u16.try_format_with(true, &mut u32::max_value()).unwrap(), [1, 0]);
        assert_eq!((0xffffu32 + 1).try_format(&mut u32::max_value()).unwrap(), [0, 0, 1, 0]);
        assert_eq!(
            (0xffffffu32 + 1).try_format(&mut u32::max_value()).unwrap(),
            [0, 0, 0, 1]
        );
        assert_eq!(
            u64::max_value().try_format(&mut u32::max_value()).unwrap(),
            [0xff].repeat(8)
        );
    }

    #[test]
    fn test_format_compact_uint() {
        assert_eq!(
            CompactUint { value: 0xfa }.try_format(&mut u32::max_value()).unwrap(),
            [0xfa]
        );
        assert_eq!(CompactUint { value: 0xfa }.format_size(()), 1);
        assert_eq!(
            CompactUint { value: 0xff }.try_format(&mut u32::max_value()).unwrap(),
            [0xfd, 0xff, 0]
        );
        assert_eq!(CompactUint { value: 0xff }.format_size(()), 3);
        let u32_cuint = CompactUint { value: 0xffff + 1 };
        assert_eq!(u32_cuint.try_format(&mut u32::max_value()).unwrap(), [0xfe, 0, 0, 1, 0]);
        let u64_cuint = CompactUint {
            value: u64::max_value(),
        };
        assert_eq!(u64_cuint.try_format(&mut u32::max_value()).unwrap(), [0xff].repeat(9));
    }

    #[test]
    fn test_format_transaction_input() {
        let raw_input = parser::tests::sample_transaction_input();
        let input_bytes = hex::decode(&raw_input).unwrap();
        let mut parser = parser::BytesParser::new(&input_bytes);
        let input: TransactionInput = parser.parse_with(2).unwrap();
        let formatted = input.try_format(&mut u32::max_value()).unwrap();
        assert_eq!(formatted, input_bytes);
        assert_eq!(formatted.len() as u32, input.format_size(()));
    }

    #[test]
    fn test_format_transaction_output() {
        let raw_output = parser::tests::sample_transaction_output();
        let output_bytes = hex::decode(&raw_output).unwrap();
        let mut parser = parser::BytesParser::new(&output_bytes);
        let output: TransactionOutput = parser.parse().unwrap();
        let formatted = output.try_format(&mut u32::max_value()).unwrap();
        assert_eq!(formatted, output_bytes);
        assert_eq!(formatted.len() as u32, output.format_size(()));
    }

    #[test]
    fn test_format_transaction() {
        let raw_tx = parser::tests::sample_transaction();
        let tx_bytes = hex::decode(&raw_tx).unwrap();
        let transaction = crate::parser::parse_transaction(&tx_bytes).unwrap();
        let formatted = transaction.try_format(&mut u32::max_value()).unwrap();
        assert_eq!(formatted, tx_bytes);
        assert_eq!(formatted.len() as u32, transaction.format_size(true));
    }

    #[test]
    fn test_format_transaction_bound_fails() {
        let raw_tx = parser::tests::sample_transaction();
        let tx_bytes = hex::decode(&raw_tx).unwrap();
        let transaction = crate::parser::parse_transaction(&tx_bytes).unwrap();
        assert_ok!(transaction.try_format(&mut (tx_bytes.len() as u32)));
        assert_err!(
            transaction.try_format(&mut (tx_bytes.len() as u32 - 1)),
            Error::ArithmeticUnderflow
        );
    }

    #[test]
    fn test_format_extended_transaction() {
        let expected_hash = H256Le::from_hex_be("b759d39a8596b70b3a46700b83e1edb247e17ba58df305421864fe7a9ac142ea");
        let expected_txid = H256Le::from_hex_be("c586389e5e4b3acb9d6c8be1c19ae8ab2795397633176f5a6442a261bbdefc3a");
        let raw_tx = parser::tests::sample_extended_transaction();
        let tx_bytes = hex::decode(&raw_tx).unwrap();
        let transaction = parser::parse_transaction(&tx_bytes).unwrap();
        let formatted = transaction.try_format(&mut u32::max_value()).unwrap();
        assert_eq!(formatted, tx_bytes);
        let computed_hash = sha256d_le(&formatted);
        assert_eq!(computed_hash, expected_hash);
        let formatted_no_witness = transaction.try_format_with(false, &mut u32::max_value()).unwrap();
        let computed_txid = sha256d_le(&formatted_no_witness);
        assert_eq!(computed_txid, expected_txid);
    }

    #[test]
    fn test_format_block_header() {
        let hex_header = parser::tests::sample_block_header();
        let parsed_header = BlockHeader::from_hex(&hex_header).unwrap();
        assert_eq!(
            hex::encode(parsed_header.try_format(&mut u32::max_value()).unwrap()),
            hex_header
        );
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

        assert_eq!(
            hex::encode(parsed_header.try_format(&mut u32::max_value()).unwrap()),
            hex_header
        );
    }

    // taken from https://bitcoin.org/en/developer-reference#block-headers
    #[test]
    fn test_format_u256() {
        let value = U256::from_dec_str("680733321990486529407107157001552378184394215934016880640").unwrap();
        let result = value.try_format(&mut u32::max_value()).unwrap();
        let expected = hex::decode("30c31b18").unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_format_u256_testnet() {
        // 0xb8e4a3e93640d7a4623e92589e40960b4b20420478d7ed60662176c323cf4caa
        let value = U256::from_dec_str("1260618571951953247774709397757627131971305851995253681160192").unwrap();
        let result = value.try_format(&mut u32::max_value()).unwrap();
        let expected = hex::decode("d4c8001a").unwrap();
        assert_eq!(result, expected);
    }

    const PROOF_HEX: &str = "00004020e2ac770a4f511b7ed2f3b638fe12d39ff52b8ced104d360500000000000000006f5ca47842fdd12f46a274ce7060c701d0c1fcff294a826e19b88e8f3dcdbca8f560135e8b64051816587c9c1f0100000bc21da39408e165a8368a7df46a17af25b4c5e3778b45222e48da632412b3be56e3b1196586e514fba3145219e3d9edb1e0e2c71b4cedaf013d8512d121f55e1ae120e954338e4d63d0a446a466b4ec548704366a89c2513c0c47818e4f8af8fa141bcda354451c2a48425704decd178df3c2c518c2fee2a593058b2c2c2ddee80ebc68aa38c161fcbf32f336b9d06feb652893be3326b0fd755cf61e575a56d7cb6b4944a2e74e3fdb583885c9dd4849ab2fd974207d9693a3062d9ba5eb0ea1b7c2d9841297396526c43af19fa8e67f3a6c07f9c8333eda575556df0e8b86a65982f24022336589fae3d56d69d73474024ced4f3a63c7205623d5bd22daf8a58e69b4748539fcdc24e0241f8231278b560340a3eb112f2fd041dc7bd1a0f6ddc37b916c24b0f96a1e9e13b4ffc7ad9c3805cadb91520435821edd439ca70198c92187deb1dde075366006d963632a0fd1ca510b362bbd6cf1805ac70becd3d303ff2d00";

    #[test]
    fn test_format_merkle_proof() {
        let proof = MerkleProof::parse(&hex::decode(PROOF_HEX).unwrap()).unwrap();
        let expected = hex::decode(PROOF_HEX).unwrap();
        assert_eq!(proof.try_format(&mut u32::max_value()).unwrap(), expected);
    }
}
