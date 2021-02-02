use crate::types::*;
use crate::Error;
use crate::Script;
use bitcoin_hashes::hash160::Hash as Hash160;
use bitcoin_hashes::Hash;
use codec::{Decode, Encode};
use sha2::{Digest, Sha256};
use sp_core::H160;

use secp256k1::{
    util::COMPRESSED_PUBLIC_KEY_SIZE as PUBLIC_KEY_SIZE, Error as Secp256k1Error,
    PublicKey as Secp256k1PublicKey, SecretKey as Secp256k1SecretKey,
};

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
pub struct PublicKey(pub [u8; PUBLIC_KEY_SIZE]);

impl Default for PublicKey {
    fn default() -> Self {
        Self([0; PUBLIC_KEY_SIZE])
    }
}

impl From<[u8; PUBLIC_KEY_SIZE]> for PublicKey {
    fn from(bytes: [u8; PUBLIC_KEY_SIZE]) -> Self {
        Self(bytes)
    }
}

impl Into<[u8; PUBLIC_KEY_SIZE]> for PublicKey {
    fn into(self) -> [u8; PUBLIC_KEY_SIZE] {
        self.0
    }
}

#[cfg(feature = "std")]
impl serde::Serialize for PublicKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut slice = [0u8; 2 + 2 * PUBLIC_KEY_SIZE];
        impl_serde::serialize::serialize_raw(&mut slice, &self.0, serializer)
    }
}

#[cfg(feature = "std")]
impl<'de> serde::Deserialize<'de> for PublicKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut bytes = [0u8; PUBLIC_KEY_SIZE];
        impl_serde::serialize::deserialize_check_len(
            deserializer,
            impl_serde::serialize::ExpectedLen::Exact(&mut bytes),
        )?;
        Ok(PublicKey(bytes))
    }
}

/// To avoid the use of OP_RETURN during the issue process, we use an On-chain Key Derivation scheme (OKD) for
/// Bitcoin’s ECDSA (secp256k1 curve). The vault-registry maintains a "master" public key for each registered
/// Vault which can then be used to derive additional deposit addresses on-demand. Each new issue request triggers
/// the computation of a deposit address. The scheme works as follows:
///
/// ### Preliminaries
///
/// A Vault has a private/public keypair `(v, V)`, where `V = v·G` and `G` is the base point of the secp256k1 curve.
/// Upon registration, the Vault submits public key `V` to the BTC-Parachain storage.
///
/// ### OKD scheme
///
/// 1. Computes `c = H(V || id)`, where `id` is the unique issue identifier, generated on-chain by the BTC-Parachain
///    using the user's AccountId and an internal auto-incrementing nonce as input.
/// 2. Generates a new public key ("deposit public key") `D = V·c` and then the corresponding BTC RIPEMD-160 hash-based
///    address `addr(D)` ('deposit' address) using `D` as input.
/// 3. Stores `D` and `addr(D)` alongside the id of the issue request.
/// 4. The vault knows that the private key of `D` is `c·v`, where `c = H(V || id)` is publicly known (so it can be
///    computed by the vault off-chain, or stored on-chain for convenience). The vault can now import the private key
//     `c·v` into its Bitcoin wallet to gain access to the deposited BTC (required for redeem).
impl PublicKey {
    fn new_secret_key(&self, secure_id: H256) -> [u8; 32] {
        let mut hasher = Sha256::default();
        // input compressed public key
        hasher.input(&self.0);
        // input secure id
        hasher.input(secure_id.as_bytes());

        let mut bytes = [0; 32];
        bytes.copy_from_slice(&hasher.result()[..]);
        bytes
    }

    /// Generates an ephemeral "deposit" public key which can be used in Issue
    /// requests to ensure that payments are unique.
    ///
    /// # Arguments
    ///
    /// * `secure_id` - random nonce (as provided by the security module)
    pub fn new_deposit_public_key(&self, secure_id: H256) -> Result<Self, Secp256k1Error> {
        // libsecp256k1 (set_b32 -> check_overflow) will ensure that secret keys are non-zero
        // and do not exceed the maximum allowed value
        let secret_key = Secp256k1SecretKey::parse(&self.new_secret_key(secure_id))?;
        self.new_deposit_public_key_with_secret(secret_key)
    }

    fn new_deposit_public_key_with_secret(
        &self,
        secret_key: Secp256k1SecretKey,
    ) -> Result<Self, Secp256k1Error> {
        let mut public_key = Secp256k1PublicKey::parse_compressed(&self.0)?;
        // D = V * c
        public_key.tweak_mul_assign(&secret_key)?;
        Ok(Self(public_key.serialize_compressed()))
    }

    /// Calculates the RIPEMD-160 hash of the compressed public key,
    /// which can be used to formulate an `Address`.
    pub fn to_hash(&self) -> H160 {
        H160::from(Hash160::hash(&self.0).into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frame_support::assert_err;
    use rand::thread_rng;

    #[test]
    fn test_public_key_to_hash() {
        // "04ff01b82f2f166c719937d5bd856bd919d9d6d495826cde3733cdb0d1084c8d12b311ced5cc235271c4a16a41fb943ab58e96ca6c4e2f85c6368999c8a3ec26b2"
        // "02ff01b82f2f166c719937d5bd856bd919d9d6d495826cde3733cdb0d1084c8d12"

        let public_key = PublicKey([
            2, 255, 1, 184, 47, 47, 22, 108, 113, 153, 55, 213, 189, 133, 107, 217, 25, 217, 214,
            212, 149, 130, 108, 222, 55, 51, 205, 176, 209, 8, 76, 141, 18,
        ]);

        assert_eq!(
            public_key.to_hash(),
            H160::from_slice(&hex::decode("84b42bde9034a27ce718af4bfbfb3b2ab842175d").unwrap())
        );
    }

    #[test]
    fn test_check_secret_key_constraints() {
        let minimum_scalar = secp256k1::curve::Scalar([0; 8]);
        assert_err!(
            Secp256k1SecretKey::parse_slice(&hex::decode(format!("{:x}", minimum_scalar)).unwrap()),
            Secp256k1Error::InvalidSecretKey
        );

        // https://en.bitcoin.it/wiki/Private_key
        let maximum_scalar = secp256k1::curve::Scalar([
            0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFE, 0xBAAEDCE6, 0xAF48A03B, 0xBFD25E8C,
            0xD0364141,
        ]);
        assert_err!(
            Secp256k1SecretKey::parse_slice(&hex::decode(format!("{:x}", maximum_scalar)).unwrap()),
            Secp256k1Error::InvalidSecretKey
        );
    }

    #[test]
    fn test_new_deposit_public_key() {
        // c
        let secure_id = H256::random();
        let secret_key = Secp256k1SecretKey::parse_slice(secure_id.as_bytes()).unwrap();

        // v
        let mut vault_secret_key = Secp256k1SecretKey::random(&mut thread_rng());
        // V
        let vault_public_key = PublicKey(
            Secp256k1PublicKey::from_secret_key(&vault_secret_key).serialize_compressed(),
        );

        // D = V * c
        let deposit_public_key = vault_public_key
            .new_deposit_public_key_with_secret(secret_key.clone())
            .unwrap();

        // d = v * c
        vault_secret_key.tweak_mul_assign(&secret_key).unwrap();

        assert_eq!(
            deposit_public_key,
            PublicKey(
                Secp256k1PublicKey::from_secret_key(&vault_secret_key).serialize_compressed(),
            )
        );
    }

    #[test]
    fn test_new_deposit_public_key_static() {
        // bcrt1qzrkyemjkaxq48zwlnhxvear8fh6lvkwszxy7dm
        let old_public_key = PublicKey([
            2, 123, 236, 243, 192, 100, 34, 40, 51, 111, 129, 130, 160, 64, 129, 135, 11, 184, 68,
            84, 83, 198, 234, 196, 150, 13, 208, 86, 34, 150, 10, 59, 247,
        ]);

        let secret_key = Secp256k1SecretKey::parse(&[
            137, 16, 46, 159, 212, 158, 232, 178, 197, 253, 105, 137, 102, 159, 70, 217, 110, 211,
            254, 82, 216, 4, 105, 171, 102, 252, 54, 190, 114, 91, 11, 69,
        ])
        .unwrap();

        // bcrt1qn9mgwncjtnavx23utveqqcrxh3zjtll58pc744
        let new_public_key = old_public_key
            .new_deposit_public_key_with_secret(secret_key)
            .unwrap();

        assert_eq!(
            new_public_key,
            PublicKey([
                2, 151, 202, 113, 10, 9, 43, 125, 187, 101, 157, 152, 191, 94, 12, 236, 133, 229,
                16, 233, 221, 52, 150, 183, 243, 61, 110, 8, 152, 132, 99, 49, 189,
            ])
        );
    }
}
