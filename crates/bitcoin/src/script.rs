use crate::{formatter::TryFormat, types::*, Error};
use codec::{Decode, Encode};
use scale_info::TypeInfo;

#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};

#[cfg(feature = "std")]
use codec::alloc::string::String;

/// Bitcoin script
#[derive(Encode, Decode, TypeInfo, PartialEq, Debug, Clone)]
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

        // The format is described here https://github.com/bitcoin/bips/blob/master/bip-0034.mediawiki
        // Tl;dr: first byte is number of bytes in the number, following bytes are little-endian
        // representation of the number

        let mut height_bytes = height.to_le_bytes().to_vec();
        for i in (1..4).rev() {
            // remove trailing zeroes, but always keep first byte even if it's zero
            if height_bytes[i] == 0 {
                height_bytes.remove(i);
            } else {
                break;
            }
        }

        // If the most significant byte is >= 0x80 and the value is positive, push a
        // new zero-byte to make the significant byte < 0x80 again.
        // See https://github.com/bitcoin/bitcoin/blob/b565485c24c0feacae559a7f6f7b83d7516ca58d/src/script/script.h#L360-L373
        if let Some(x) = height_bytes.last() {
            if (x & 0x80) != 0 {
                height_bytes.push(0);
            }
        }

        // note: formatting the height_bytes vec automatically prepends the length of the vec, so no need
        // to append it manually
        script.append(height_bytes);
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

    pub fn append<T: TryFormat>(&mut self, value: T) {
        value.try_format(&mut self.bytes).expect("Not bounded");
    }

    pub fn extract_op_return_data(&self) -> Result<Vec<u8>, Error> {
        let output_script = &self.bytes;
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
impl std::convert::TryFrom<&str> for Script {
    type Error = crate::Error;

    fn try_from(hex_string: &str) -> Result<Script, Self::Error> {
        let bytes = hex::decode(hex_string).map_err(|_e| Error::InvalidScript)?;
        Ok(Script { bytes })
    }
}

#[test]
fn test_script_height() {
    assert_eq!(Script::height(7).bytes, vec![1, 7]);
    // 2^7 boundary
    assert_eq!(Script::height(127).bytes, vec![1, 127]);
    assert_eq!(Script::height(128).bytes, vec![2, 128, 0]);
    // 2^8 boundary
    assert_eq!(Script::height(255).bytes, vec![2, 0xff, 0x00]);
    assert_eq!(Script::height(256).bytes, vec![2, 0x00, 0x01]);
    // 2^15 boundary
    assert_eq!(Script::height(32767).bytes, vec![2, 0xff, 0x7f]);
    assert_eq!(Script::height(32768).bytes, vec![3, 0x00, 0x80, 0x00]);
    // 2^16 boundary
    assert_eq!(Script::height(65535).bytes, vec![3, 0xff, 0xff, 0x00]);
    assert_eq!(Script::height(65536).bytes, vec![3, 0x00, 0x00, 0x01]);
}
