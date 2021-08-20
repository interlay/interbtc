use crate::{formatter::Formattable, parser::extract_op_return_data, types::*, Error};
use sp_std::{prelude::*, vec};

#[cfg(feature = "std")]
use codec::alloc::string::String;

/// Bitcoin script
#[derive(PartialEq, Debug, Clone)]
pub struct Script {
    pub(crate) bytes: Vec<u8>,
}

impl Default for Script {
    fn default() -> Self {
        Script { bytes: vec![] }
    }
}

impl Script {
    pub fn new() -> Script {
        Self::default()
    }

    pub(crate) fn height(height: u32) -> Script {
        let mut script = Script::new();
        script.append(OpCode::Op3);
        let bytes = height.to_le_bytes();
        script.append(&bytes[0..=2]);
        script
    }

    pub fn op_return(return_content: &[u8]) -> Script {
        let mut script = Script::new();
        script.append(OpCode::OpReturn);
        script.append(return_content.len() as u8);
        script.append(return_content);
        script
    }

    pub fn is_p2wpkh_v0(&self) -> bool {
        // first byte is version
        self.len() == P2WPKH_V0_SCRIPT_SIZE as usize
            && self.bytes[0] == OpCode::Op0 as u8
            && self.bytes[1] == HASH160_SIZE_HEX
    }

    pub fn is_p2wsh_v0(&self) -> bool {
        // first byte is version
        self.len() == P2WSH_V0_SCRIPT_SIZE as usize
            && self.bytes[0] == OpCode::Op0 as u8
            && self.bytes[1] == HASH256_SIZE_HEX
    }

    pub fn is_p2pkh(&self) -> bool {
        self.len() == P2PKH_SCRIPT_SIZE as usize
            && self.bytes[0] == OpCode::OpDup as u8
            && self.bytes[1] == OpCode::OpHash160 as u8
            && self.bytes[2] == HASH160_SIZE_HEX
            && self.bytes[23] == OpCode::OpEqualVerify as u8
            && self.bytes[24] == OpCode::OpCheckSig as u8
    }

    pub fn is_p2sh(&self) -> bool {
        self.len() == P2SH_SCRIPT_SIZE as usize
            && self.bytes[0] == OpCode::OpHash160 as u8
            && self.bytes[1] == HASH160_SIZE_HEX
            && self.bytes[22] == OpCode::OpEqual as u8
    }

    pub fn append<T: Formattable<U>, U>(&mut self, value: T) {
        self.bytes.extend(&value.format())
    }

    pub fn extract_op_return_data(&self) -> Result<Vec<u8>, Error> {
        extract_op_return_data(&self.bytes)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    #[cfg(feature = "std")]
    pub fn as_hex(&self) -> String {
        hex::encode(&self.bytes)
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl From<Vec<u8>> for Script {
    fn from(bytes: Vec<u8>) -> Script {
        Script { bytes }
    }
}

#[cfg(feature = "std")]
impl sp_std::convert::TryFrom<&str> for Script {
    type Error = crate::Error;

    fn try_from(hex_string: &str) -> Result<Script, Self::Error> {
        let bytes = hex::decode(hex_string).map_err(|_e| Error::InvalidScript)?;
        Ok(Script { bytes })
    }
}
