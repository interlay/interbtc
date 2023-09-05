//! Provides conversions between rust-bitcoin and interbtc types.
//! Please note that these operations involve (unbounded) re-encoding
//! and decoding so may be expensive to use.

use crate::{formatter::TryFormat, parser::Parsable};
use rust_bitcoin::consensus::{Decodable, Encodable};

pub use rust_bitcoin;

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

#[derive(Debug)]
pub enum ConversionError {
    ParsingError,
    FormattingError,
}

/// Macro to implement type conversion from interbtc types to rust-bitcoin, using consensus encoding
macro_rules! impl_bitcoin_conversion {
    ($a:path, $b:path) => {
        impl TryFrom<$a> for $b {
            type Error = ConversionError;
            fn try_from(value: $a) -> Result<Self, Self::Error> {
                let mut bytes = Vec::<u8>::new();
                value
                    .try_format(&mut bytes)
                    .map_err(|_| ConversionError::FormattingError)?;
                let result = Self::consensus_decode_from_finite_reader(&mut &bytes[..])
                    .map_err(|_| ConversionError::ParsingError)?;
                Ok(result)
            }
        }
    };
}

/// Macro to implement type conversion to interbtc types from rust-bitcoin, using consensus encoding
macro_rules! impl_interbtc_conversion {
    ($a:path, $b:path) => {
        impl TryFrom<$b> for $a {
            type Error = ConversionError;
            fn try_from(value: $b) -> Result<Self, Self::Error> {
                let mut data: Vec<u8> = Vec::new();
                value
                    .consensus_encode(&mut data)
                    .map_err(|_| ConversionError::FormattingError)?;
                let result = Self::parse(&data, 0).map_err(|_| ConversionError::ParsingError)?;
                Ok(result.0)
            }
        }
    };
}

macro_rules! impl_bidirectional_conversions {
    ($a:path, $b:path) => {
        impl_bitcoin_conversion!($a, $b);
        impl_interbtc_conversion!($a, $b);
    };
}

// NOTE: rust_bitcoin::Script exists but we can't convert to that because it's unsized
impl_bitcoin_conversion!(crate::Script, rust_bitcoin::ScriptBuf);

// Transaction conversions
impl_bidirectional_conversions!(crate::types::Transaction, rust_bitcoin::Transaction);

// Payload -> Address
impl TryFrom<rust_bitcoin::address::Payload> for crate::Address {
    type Error = ConversionError;
    fn try_from(value: rust_bitcoin::address::Payload) -> Result<Self, Self::Error> {
        let bitcoin_script = value.script_pubkey();
        let bitcoin_script_bytes = bitcoin_script.to_bytes();
        let interlay_script = crate::Script::from(bitcoin_script_bytes);
        crate::Address::from_script_pub_key(&interlay_script).map_err(|_| ConversionError::ParsingError)
    }
}

// Address -> Payload
impl TryFrom<crate::Address> for rust_bitcoin::address::Payload {
    type Error = ConversionError;
    fn try_from(value: crate::Address) -> Result<Self, Self::Error> {
        let interlay_script = value.to_script_pub_key();
        let bitcoin_script = rust_bitcoin::blockdata::script::Script::from_bytes(interlay_script.as_bytes());
        rust_bitcoin::address::Payload::from_script(&bitcoin_script).map_err(|_| ConversionError::ParsingError)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_transaction;

    #[test]
    fn test_transaction_compat() {
        // txid eb3db053cd139147f2fd676cf59a491fd5aebc54bddfde829704585b659126fc
        let raw_tx = "0100000000010120e6fb8f0e2cfb8667a140a92d045d5db7c1b56635790bc907c3e71d43720a150e00000017160014641e441c2ba32dd7cf05afde7922144dd106b09bffffffff019dbd54000000000017a914bd847a4912984cf6152547feca51c1b9c2bcbe2787024830450221008f00033064c26cfca4dc98e5dba800b18729c3441dca37b49358ae0df9be7fad02202a81085318466ea66ef390d5dab6737e44a05f7f2e747932ebba917e0098f37d012102c109fc47335c3a2e206d462ad52590b1842aa9d6e0eb9c683c896fa8723590b400000000";
        let tx_bytes = hex::decode(&raw_tx).unwrap();
        let interlay_transaction = parse_transaction(&tx_bytes).unwrap();

        let rust_bitcoin_transaction: rust_bitcoin::Transaction = interlay_transaction.clone().try_into().unwrap();

        // check that the rust-bitcoin type encodes to the same bytes
        let mut re_encoded_bytes: Vec<u8> = Vec::new();
        rust_bitcoin_transaction
            .consensus_encode(&mut re_encoded_bytes)
            .unwrap();
        assert_eq!(tx_bytes, re_encoded_bytes);

        // check that the conversion back works
        assert_eq!(interlay_transaction, rust_bitcoin_transaction.try_into().unwrap());
    }

    #[test]
    fn test_address_compat() {
        let interbtc_address = crate::Address::P2WPKHv0(primitive_types::H160([1; 20]));
        let rust_bitcoin_address: rust_bitcoin::address::Payload = interbtc_address.clone().try_into().unwrap();
        assert_eq!(interbtc_address, rust_bitcoin_address.try_into().unwrap());
    }
}
