use crate::types::*;
use crate::Error;
use crate::Script;
use codec::{Decode, Encode};
use sp_core::H160;

/// A Bitcoin address is a serialized identifier that represents the destination for a payment.
/// Address prefixes are used to indicate the network as well as the format. Since the Parachain
/// follows SPV assumptions we do not need to know which network a payment is included in.
#[derive(Encode, Decode, Clone, Ord, PartialOrd, PartialEq, Eq, Debug, Copy)]
#[cfg_attr(
    feature = "std",
    derive(serde::Serialize, serde::Deserialize, std::hash::Hash)
)]
pub enum Address {
    P2PKH(H160),
    P2SH(H160),
    P2WPKHv0(H160),
}

#[cfg(feature = "std")]
impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let addr = match self {
            Self::P2PKH(hash) => (hash, "p2pkh"),
            Self::P2SH(hash) => (hash, "p2sh"),
            Self::P2WPKHv0(hash) => (hash, "p2wpkh"),
        };
        write!(f, "{} ({})", addr.0, addr.1)
    }
}

impl Address {
    pub fn from_script(script: &Script) -> Result<Self, Error> {
        if script.is_p2pkh() {
            // 0x76 (OP_DUP) - 0xa9 (OP_HASH160) - 0x14 (20 bytes len) - <20 bytes pubkey hash> - 0x88 (OP_EQUALVERIFY) - 0xac (OP_CHECKSIG)
            Ok(Self::P2PKH(H160::from_slice(&script.as_bytes()[3..23])))
        } else if script.is_p2sh() {
            // 0xa9 (OP_HASH160) - 0x14 (20 bytes hash) - <20 bytes script hash> - 0x87 (OP_EQUAL)
            Ok(Self::P2SH(H160::from_slice(&script.as_bytes()[2..22])))
        } else if script.is_p2wpkh_v0() {
            // 0x00 0x14 (20 bytes len) - <20 bytes hash>
            Ok(Self::P2WPKHv0(H160::from_slice(&script.as_bytes()[2..])))
        } else {
            Err(Error::InvalidBtcAddress)
        }
    }

    pub fn to_script(&self) -> Script {
        match self {
            Self::P2PKH(pub_key_hash) => {
                let mut script = Script::new();
                script.append(OpCode::OpDup);
                script.append(OpCode::OpHash160);
                script.append(HASH160_SIZE_HEX);
                script.append(pub_key_hash);
                script.append(OpCode::OpEqualVerify);
                script.append(OpCode::OpCheckSig);
                script
            }
            Self::P2SH(script_hash) => {
                let mut script = Script::new();
                script.append(OpCode::OpHash160);
                script.append(HASH160_SIZE_HEX);
                script.append(script_hash);
                script.append(OpCode::OpEqual);
                script
            }
            Self::P2WPKHv0(pub_key_hash) => {
                let mut script = Script::new();
                script.append(OpCode::Op0);
                script.append(pub_key_hash);
                script
            }
        }
    }

    pub fn hash(&self) -> H160 {
        match *self {
            Address::P2PKH(hash) => hash,
            Address::P2SH(hash) => hash,
            Address::P2WPKHv0(hash) => hash,
        }
    }

    #[cfg(feature = "std")]
    pub fn random() -> Self {
        Address::P2PKH(H160::random())
    }
}

impl Default for Address {
    fn default() -> Self {
        Self::P2PKH(H160::zero())
    }
}
