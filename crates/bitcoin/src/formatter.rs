use sp_std::vec::Vec;

use crate::types::{
    Address, CompactUint, H256Le, OpCode, Script, Transaction, TransactionInput, TransactionOutput,
    SERIALIZE_TRANSACTION_NO_WITNESS,
};

const WITNESS_FLAG: u8 = 0x01;
const WITNESS_MARKER: u8 = 0x00;

/// Type to be formatted as a bytes array
pub trait Formattable<Options = ()> {
    fn format(&self) -> Vec<u8>;
    fn format_with(&self, _options: Options) -> Vec<u8> {
        self.format()
    }
}

/// Macro to generate `Formattable` implementation of int types
macro_rules! make_formattable_int {
    ($type:ty) => {
        impl Formattable<bool> for $type {
            fn format(&self) -> Vec<u8> {
                Vec::from(&self.to_le_bytes()[..])
            }

            fn format_with(&self, be: bool) -> Vec<u8> {
                if be {
                    Vec::from(&self.to_be_bytes()[..])
                } else {
                    self.format()
                }
            }
        }
    };
}

impl<T, U> Formattable<U> for &T
where
    T: Formattable<U>,
{
    fn format(&self) -> Vec<u8> {
        T::format(self)
    }

    fn format_with(&self, options: U) -> Vec<u8> {
        T::format_with(self, options)
    }
}

// Generate formattable implementation for the basic integers (signed and unsgined) types
make_formattable_int!(u8);
make_formattable_int!(u16);
make_formattable_int!(u32);
make_formattable_int!(u64);
make_formattable_int!(i8);
make_formattable_int!(i16);
make_formattable_int!(i32);
make_formattable_int!(i64);

impl Formattable<()> for H256Le {
    fn format(&self) -> Vec<u8> {
        Vec::from(&self.to_bytes_le()[..])
    }
}

impl Formattable for CompactUint {
    fn format(&self) -> Vec<u8> {
        let mut formatter = Formatter::new();
        if self.value < 0xfd {
            formatter.format(self.value as u8);
        } else if self.value < u16::max_value() as u64 {
            formatter.format(0xfd as u8);
            formatter.format(self.value as u16);
        } else if self.value < u32::max_value() as u64 {
            formatter.format(0xfe as u8);
            formatter.format(self.value as u32);
        } else {
            formatter.format(0xff as u8);
            formatter.format(self.value);
        }
        formatter.result()
    }
}

impl<T, U> Formattable<U> for Vec<T>
where
    for<'a> &'a T: Formattable<U>,
    U: Default + Copy,
{
    fn format(&self) -> Vec<u8> {
        self.format_with(Default::default())
    }

    fn format_with(&self, options: U) -> Vec<u8> {
        let mut formatter = Formatter::new();
        formatter.format(CompactUint {
            value: self.len() as u64,
        });
        for value in self.iter() {
            formatter.format_with(value, options);
        }
        formatter.result()
    }
}

impl Formattable<bool> for TransactionInput {
    fn format(&self) -> Vec<u8> {
        let mut formatter = Formatter::new();
        formatter.format(&self.previous_hash);
        formatter.format(self.previous_index);
        formatter.format(CompactUint::from_usize(self.script.len()));
        self.height.iter().for_each(|h| formatter.output(h));
        formatter.output(&self.script); // we already formatted the length
        formatter.format(self.sequence);
        formatter.result()
    }
}

impl Formattable for Script {
    fn format(&self) -> Vec<u8> {
        self.bytes.format()
    }
}

impl Formattable for &[u8] {
    fn format(&self) -> Vec<u8> {
        Vec::from(*self)
    }
}

impl Formattable for Address {
    fn format(&self) -> Vec<u8> {
        Vec::from(self.as_bytes())
    }
}

impl Formattable for OpCode {
    fn format(&self) -> Vec<u8> {
        vec![*self as u8]
    }
}

impl Formattable for TransactionOutput {
    fn format(&self) -> Vec<u8> {
        let mut formatter = Formatter::new();
        formatter.format(self.value);
        formatter.format(&self.script);
        formatter.result()
    }
}

impl Formattable<bool> for Transaction {
    fn format(&self) -> Vec<u8> {
        self.format_with(true)
    }

    fn format_with(&self, witness: bool) -> Vec<u8> {
        let mut formatter = Formatter::new();
        let allow_witness = (self.version & SERIALIZE_TRANSACTION_NO_WITNESS) == 0;
        // check if any of the inputs has a witness
        let has_witness = allow_witness && self.inputs.iter().any(|v| !v.witness.is_empty());

        formatter.format(self.version);

        if witness && has_witness {
            formatter.format(WITNESS_MARKER);
            formatter.format(WITNESS_FLAG);
        }

        formatter.format(&self.inputs);
        formatter.format(&self.outputs);

        if witness && has_witness {
            for input in self.inputs.iter() {
                formatter.format(&input.witness);
            }
        }

        // only block_height or locktime should ever be Some
        self.block_height
            .or(self.locktime)
            .iter()
            .for_each(|b| formatter.format(b));

        formatter.result()
    }
}

pub(crate) struct Formatter {
    bytes: Vec<u8>,
}

impl Formatter {
    fn new() -> Formatter {
        Formatter { bytes: Vec::new() }
    }

    fn output(&mut self, bytes: &Vec<u8>) {
        self.bytes.extend(bytes);
    }

    fn format<T, U>(&mut self, value: T)
    where
        T: Formattable<U>,
    {
        self.bytes.extend(value.format())
    }

    fn format_with<T, U>(&mut self, value: T, data: U)
    where
        T: Formattable<U>,
    {
        self.bytes.extend(value.format_with(data))
    }

    pub(crate) fn result(&self) -> Vec<u8> {
        self.bytes.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;
    use crate::utils::sha256d_le;

    #[test]
    fn test_format_int_types() {
        assert_eq!(1u8.format(), [1]);
        assert_eq!(1i8.format(), [1]);
        assert_eq!(255u8.format(), [255]);
        assert_eq!((-1i8).format(), [255]);

        assert_eq!(256u16.format(), [0, 1]);
        assert_eq!(256u16.format_with(true), [1, 0]);
        assert_eq!((0xffffu32 + 1).format(), [0, 0, 1, 0]);
        assert_eq!((0xffffffu32 + 1).format(), [0, 0, 0, 1]);
        assert_eq!(u64::max_value().format(), [0xff].repeat(8));
    }

    #[test]
    fn test_format_compact_uint() {
        assert_eq!(CompactUint { value: 0xfa }.format(), [0xfa]);
        assert_eq!(CompactUint { value: 0xff }.format(), [0xfd, 0xff, 0]);
        let u32_cuint = CompactUint { value: 0xffff + 1 };
        assert_eq!(u32_cuint.format(), [0xfe, 0, 0, 1, 0]);
        let u64_cuint = CompactUint {
            value: u64::max_value(),
        };
        assert_eq!(u64_cuint.format(), [0xff].repeat(9));
    }

    #[test]
    fn test_format_transaction_input() {
        let raw_input = parser::tests::sample_transaction_input();
        let input_bytes = hex::decode(&raw_input).unwrap();
        let mut parser = parser::BytesParser::new(&input_bytes);
        let input: TransactionInput = parser.parse_with(2).unwrap();
        let formatted = input.format();
        assert_eq!(formatted, input_bytes);
    }

    #[test]
    fn test_format_transaction_output() {
        let raw_output = parser::tests::sample_transaction_output();
        let output_bytes = hex::decode(&raw_output).unwrap();
        let mut parser = parser::BytesParser::new(&output_bytes);
        let output: TransactionOutput = parser.parse().unwrap();
        let formatted = output.format();
        assert_eq!(formatted, output_bytes);
    }

    #[test]
    fn test_format_transaction() {
        let raw_tx = parser::tests::sample_transaction();
        let tx_bytes = hex::decode(&raw_tx).unwrap();
        let transaction = crate::parser::parse_transaction(&tx_bytes).unwrap();
        let formatted = transaction.format();
        assert_eq!(formatted, tx_bytes);
    }

    #[test]
    fn test_format_extended_transaction() {
        let expected_hash =
            H256Le::from_hex_be("b759d39a8596b70b3a46700b83e1edb247e17ba58df305421864fe7a9ac142ea");
        let expected_txid =
            H256Le::from_hex_be("c586389e5e4b3acb9d6c8be1c19ae8ab2795397633176f5a6442a261bbdefc3a");
        let raw_tx = parser::tests::sample_extended_transaction();
        let tx_bytes = hex::decode(&raw_tx).unwrap();
        let transaction = parser::parse_transaction(&tx_bytes).unwrap();
        let formatted = transaction.format();
        assert_eq!(formatted, tx_bytes);
        let computed_hash = sha256d_le(&formatted);
        assert_eq!(computed_hash, expected_hash);
        let formatted_no_witness = transaction.format_with(false);
        let computed_txid = sha256d_le(&formatted_no_witness);
        assert_eq!(computed_txid, expected_txid);
    }
}
