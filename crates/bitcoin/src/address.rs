use crate::types::*;
use crate::Error;
use crate::Script;
use codec::{Decode, Encode};
use secp256k1::{
    util::COMPRESSED_PUBLIC_KEY_SIZE, Error as Secp256k1Error, PublicKey as Secp256k1PublicKey,
    SecretKey as Secp256k1SecretKey,
};
use sha2::{Digest, Sha256};
use sp_core::H160;
use sp_std::vec::Vec;

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
                script.append(HASH160_SIZE_HEX);
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

/// Compressed ECDSA (secp256k1 curve) Public Key
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct PublicKey(pub [u8; COMPRESSED_PUBLIC_KEY_SIZE]);

impl Default for PublicKey {
    fn default() -> Self {
        Self([0; COMPRESSED_PUBLIC_KEY_SIZE])
    }
}

impl From<[u8; COMPRESSED_PUBLIC_KEY_SIZE]> for PublicKey {
    fn from(bytes: [u8; COMPRESSED_PUBLIC_KEY_SIZE]) -> Self {
        Self(bytes)
    }
}

/// To avoid the use of OP_RETURN during the issue process, we use an On-chain Key Derivation scheme (OKD) for
/// Bitcoin’s ECDSA (secp256k1 curve). The vault-registry maintains a "master" public key for each registered
/// Vault which can then be used to derive additional deposit addresses on-demand.
impl PublicKey {
    /// Generates an ephemeral "deposit" public key which can be used in Issue
    /// requests to ensure that payments are unique.
    ///
    /// # Arguments
    ///
    /// * `secure_id` - random nonce (as provided by the security module)
    pub fn new_deposit_public_key(&self, secure_id: H256) -> Result<Self, Secp256k1Error> {
        // c = H(V || id)
        let mut hasher = Sha256::default();
        // input compressed public key
        hasher.input(&self.0);
        // input secure id
        hasher.input(secure_id.as_bytes());
        let mut bytes = [0; 32];
        bytes.copy_from_slice(&hasher.result()[..]);

        let secret_key = Secp256k1SecretKey::parse(&bytes)?;
        let mut public_key = Secp256k1PublicKey::parse_compressed(&self.0)?;
        // D = V * c
        public_key.tweak_mul_assign(&secret_key)?;
        Ok(Self(public_key.serialize_compressed()))
    }

    /// Calculates the RIPEMD-160 hash of the compressed public key,
    /// which can be used to formulate an `Address`.
    pub fn to_hash(&self) -> H160 {
        get_hash_from_compressed_public_key(self.0.to_vec())
    }
}

fn get_hash_from_compressed_public_key(data: Vec<u8>) -> H160 {
    use bitcoin_hashes::hash160::Hash as Hash160;
    use bitcoin_hashes::Hash;

    H160::from(Hash160::hash(&data).into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::thread_rng;

    #[test]
    fn test_get_hash_from_compressed_public_key() {
        // "04ff01b82f2f166c719937d5bd856bd919d9d6d495826cde3733cdb0d1084c8d12b311ced5cc235271c4a16a41fb943ab58e96ca6c4e2f85c6368999c8a3ec26b2"
        // "02ff01b82f2f166c719937d5bd856bd919d9d6d495826cde3733cdb0d1084c8d12"

        let public_key =
            hex::decode("02ff01b82f2f166c719937d5bd856bd919d9d6d495826cde3733cdb0d1084c8d12")
                .unwrap();

        assert_eq!(
            get_hash_from_compressed_public_key(public_key),
            H160::from_slice(&hex::decode("84b42bde9034a27ce718af4bfbfb3b2ab842175d").unwrap())
        );
    }

    #[test]
    fn test_public_key_derivation_scheme() {
        // c
        let secure_id = H256::random();
        let secret_key = Secp256k1SecretKey::parse_slice(secure_id.as_bytes()).unwrap();

        // v
        let mut vault_secret_key = Secp256k1SecretKey::random(&mut thread_rng());
        // V
        let vault_public_key = Secp256k1PublicKey::from_secret_key(&vault_secret_key);

        // D = V * c
        let mut deposit_public_key = vault_public_key.clone();
        deposit_public_key.tweak_mul_assign(&secret_key).unwrap();

        // d = v * c
        vault_secret_key.tweak_mul_assign(&secret_key).unwrap();

        assert_eq!(
            deposit_public_key,
            Secp256k1PublicKey::from_secret_key(&vault_secret_key)
        );
    }
}
