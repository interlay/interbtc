use crate::{formatter::TryFormat, parser::Parsable};
pub use rust_bitcoin;
use rust_bitcoin::consensus::{Decodable, Encodable};

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

#[derive(Debug)]
pub enum ConversionError {
    ParsingError,
    FormattingError,
}

pub trait ConvertFromInterlayBitcoin {
    type Output;
    fn to_rust_bitcoin(&self) -> Result<Self::Output, ConversionError>;
}

pub trait ConvertToInterlayBitcoin {
    type Output;
    fn to_interlay(&self) -> Result<Self::Output, ConversionError>;
}

/// Macro to implement type conversion from interlay type to rust-bitcoin, using consensus encoding
macro_rules! impl_bitcoin_conversion {
    ($a:path, $b:path) => {
        impl ConvertFromInterlayBitcoin for $a {
            type Output = $b;
            fn to_rust_bitcoin(&self) -> Result<Self::Output, ConversionError> {
                let mut bytes = Vec::<u8>::new();
                self.try_format(&mut bytes)
                    .map_err(|_| ConversionError::FormattingError)?;

                let result = Self::Output::consensus_decode_from_finite_reader(&mut &bytes[..])
                    .map_err(|_| ConversionError::ParsingError)?;

                Ok(result)
            }
        }
    };
}
/// Macro to implement type conversion to interlay type from rust-bitcoin, using consensus encoding
macro_rules! impl_to_interlay_bitcoin_conversion {
    ($a:path, $b:path) => {
        impl ConvertToInterlayBitcoin for $b {
            type Output = $a;
            fn to_interlay(&self) -> Result<Self::Output, ConversionError> {
                let mut data: Vec<u8> = Vec::new();
                self.consensus_encode(&mut data)
                    .map_err(|_| ConversionError::FormattingError)?;
                let result = Self::Output::parse(&data, 0).map_err(|_| ConversionError::ParsingError)?;
                Ok(result.0)
            }
        }
    };
}

macro_rules! impl_bidirectional_bitcoin_conversion {
    ($a:path, $b:path) => {
        impl_bitcoin_conversion!($a, $b);
        impl_to_interlay_bitcoin_conversion!($a, $b);
    };
}

// there also exists rust_bitcoin::Script but we can't convert to that since it's unsized
impl_bitcoin_conversion!(crate::Script, rust_bitcoin::ScriptBuf);

// Transcation conversions
impl_bidirectional_bitcoin_conversion!(crate::types::Transaction, rust_bitcoin::Transaction);

// Address <--> Payload
impl ConvertToInterlayBitcoin for rust_bitcoin::address::Payload {
    type Output = crate::Address;
    fn to_interlay(&self) -> Result<Self::Output, ConversionError> {
        let bitcoin_script = self.script_pubkey();
        let bitcoin_script_bytes = bitcoin_script.to_bytes();
        let interlay_script = crate::Script::from(bitcoin_script_bytes);
        Ok(crate::Address::from_script_pub_key(&interlay_script).map_err(|_| ConversionError::ParsingError)?)
    }
}
impl ConvertFromInterlayBitcoin for crate::Address {
    type Output = rust_bitcoin::address::Payload;
    fn to_rust_bitcoin(&self) -> Result<Self::Output, ConversionError> {
        let interlay_script = self.to_script_pub_key();
        let bitcoin_script = rust_bitcoin::blockdata::script::Script::from_bytes(interlay_script.as_bytes());
        Ok(rust_bitcoin::address::Payload::from_script(&bitcoin_script).map_err(|_| ConversionError::ParsingError)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_transaction;
    #[test]
    fn test_bitcoin_compat() {
        // txid eb3db053cd139147f2fd676cf59a491fd5aebc54bddfde829704585b659126fc
        let raw_tx = "0100000000010120e6fb8f0e2cfb8667a140a92d045d5db7c1b56635790bc907c3e71d43720a150e00000017160014641e441c2ba32dd7cf05afde7922144dd106b09bffffffff019dbd54000000000017a914bd847a4912984cf6152547feca51c1b9c2bcbe2787024830450221008f00033064c26cfca4dc98e5dba800b18729c3441dca37b49358ae0df9be7fad02202a81085318466ea66ef390d5dab6737e44a05f7f2e747932ebba917e0098f37d012102c109fc47335c3a2e206d462ad52590b1842aa9d6e0eb9c683c896fa8723590b400000000";
        let tx_bytes = hex::decode(&raw_tx).unwrap();
        let interlay_transaction = parse_transaction(&tx_bytes).unwrap();

        let rust_bitcoin_transaction = interlay_transaction.to_rust_bitcoin().unwrap();

        // check that the rust-bitcoin type encoded to the same bytes
        let mut reencoded_bytes: Vec<u8> = Vec::new();
        rust_bitcoin_transaction.consensus_encode(&mut reencoded_bytes).unwrap();
        assert_eq!(tx_bytes, reencoded_bytes);

        // check that the conversion back works
        assert_eq!(interlay_transaction, rust_bitcoin_transaction.to_interlay().unwrap());
    }
}
