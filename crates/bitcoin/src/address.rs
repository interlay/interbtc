use crate::{types::*, Error, Script};
use bitcoin_hashes::{hash160::Hash as Hash160, Hash};
use codec::{Decode, Encode};
use sha2::{Digest, Sha256};
use sp_core::{H160, H256};
use sp_std::vec::Vec;

use secp256k1::{constants::PUBLIC_KEY_SIZE, Error as Secp256k1Error, PublicKey as Secp256k1PublicKey};

/// A Bitcoin address is a serialized identifier that represents the destination for a payment.
/// Address prefixes are used to indicate the network as well as the format. Since the Parachain
/// follows SPV assumptions we do not need to know which network a payment is included in.
#[derive(Encode, Decode, Clone, Ord, PartialOrd, PartialEq, Eq, Debug, Copy)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize, std::hash::Hash))]
pub enum Address {
    // input: {signature} {pubkey}
    // output: OP_DUP OP_HASH160 {hash160(pubkey)} OP_EQUALVERIFY OP_CHECKSIG
    // witness: <>
    P2PKH(H160),
    // input: [redeem_script_sig ...] {redeem_script}
    // output: OP_HASH160 {hash160(redeem_script)} OP_EQUAL
    // witness: <?>
    P2SH(H160),
    // input: <>
    // output: OP_0 {hash160(pubkey)}
    // witness: {signature} {pubkey}
    P2WPKHv0(H160),
    // input: <>
    // output: OP_0 {sha256(redeem_script)}
    // witness: [redeem_script_sig ...] {redeem_script}
    P2WSHv0(H256),
}

impl Address {
    pub fn from_script_pub_key(script: &Script) -> Result<Self, Error> {
        const OP_DUP: u8 = OpCode::OpDup as u8;
        const OP_HASH_160: u8 = OpCode::OpHash160 as u8;
        const OP_EQUAL_VERIFY: u8 = OpCode::OpEqualVerify as u8;
        const OP_CHECK_SIG: u8 = OpCode::OpCheckSig as u8;
        const OP_EQUAL: u8 = OpCode::OpEqual as u8;
        const OP_0: u8 = OpCode::Op0 as u8;

        match script.as_bytes() {
            &[OP_DUP, OP_HASH_160, HASH160_SIZE_HEX, ref addr @ .., OP_EQUAL_VERIFY, OP_CHECK_SIG]
                if addr.len() == HASH160_SIZE_HEX as usize =>
            {
                Ok(Self::P2PKH(H160::from_slice(addr)))
            }
            &[OP_HASH_160, HASH160_SIZE_HEX, ref addr @ .., OP_EQUAL] if addr.len() == HASH160_SIZE_HEX as usize => {
                Ok(Self::P2SH(H160::from_slice(addr)))
            }
            &[OP_0, HASH256_SIZE_HEX, ref addr @ ..] if addr.len() == HASH256_SIZE_HEX as usize => {
                Ok(Self::P2WSHv0(H256::from_slice(addr)))
            }
            &[OP_0, HASH160_SIZE_HEX, ref addr @ ..] if addr.len() == HASH160_SIZE_HEX as usize => {
                Ok(Self::P2WPKHv0(H160::from_slice(addr)))
            }
            _ => Err(Error::InvalidBtcAddress),
        }
    }

    pub fn to_script_pub_key(&self) -> Script {
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
            Self::P2WSHv0(script_hash) => {
                let mut script = Script::new();
                script.append(OpCode::Op0);
                script.append(HASH256_SIZE_HEX);
                script.append(script_hash);
                script
            }
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

pub mod global {
    use secp256k1::{ffi::types::AlignedType, AllPreallocated, Secp256k1};
    use sp_std::{ops::Deref, vec, vec::Vec};
    // this is what lazy_static uses internally
    use spin::Once;

    pub struct GlobalContext {
        __private: (),
    }

    pub static SECP256K1: &GlobalContext = &GlobalContext { __private: () };

    impl Deref for GlobalContext {
        type Target = Secp256k1<AllPreallocated<'static>>;

        fn deref(&self) -> &Self::Target {
            static ONCE: Once<()> = Once::new();
            static mut BUFFER: Vec<AlignedType> = vec![];
            static mut CONTEXT: Option<Secp256k1<AllPreallocated<'static>>> = None;
            ONCE.call_once(|| unsafe {
                BUFFER = vec![AlignedType::zeroed(); Secp256k1::preallocate_size()];
                let ctx = Secp256k1::preallocated_new(&mut BUFFER).unwrap();
                CONTEXT = Some(ctx);
            });
            unsafe { CONTEXT.as_ref().unwrap() }
        }
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
        self.new_deposit_public_key_with_secret(&self.new_secret_key(secure_id))
    }

    fn new_deposit_public_key_with_secret(&self, secret_key: &[u8; 32]) -> Result<Self, Secp256k1Error> {
        let mut public_key = Secp256k1PublicKey::from_slice(&self.0)?;
        // D = V * c
        public_key.mul_assign(global::SECP256K1, secret_key)?;
        Ok(Self(public_key.serialize()))
    }

    /// Calculates the RIPEMD-160 hash of the compressed public key,
    /// which can be used to formulate an `Address`.
    pub fn to_hash(&self) -> H160 {
        H160::from(Hash160::hash(&self.0).into_inner())
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Construct the p2pkh scriptSig for this compressed pubKey
    /// given the signature. Note: we do not check signatures on
    /// verification, but this should be non-empty.
    pub fn to_p2pkh_script_sig(&self, sig: Vec<u8>) -> Script {
        let mut script = Script::new();
        script.append(&sig);
        script.append(self.0.to_vec());
        script
    }

    /// Construct the redeemScript for a one-signature-required
    /// p2sh transaction.
    pub(crate) fn to_redeem_script(&self) -> Vec<u8> {
        let mut redeem_script = self.0.to_vec();
        redeem_script.push(OpCode::OpCheckSig as u8);
        redeem_script
    }

    /// Construct the scriptSig for a one-signature-required
    /// p2sh transaction, given the key's signature. Note: we
    /// do not verify that the signature is valid but this field
    /// must be non-empty for parsing to succeed.
    pub fn to_p2sh_script_sig(&self, sig: Vec<u8>) -> Script {
        let mut script = Script::new();
        script.append(OpCode::Op0);
        script.append(&sig);
        script.append(self.to_redeem_script());
        script
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frame_support::assert_err;
    use secp256k1::{rand::rngs::OsRng, Secp256k1, SecretKey as Secp256k1SecretKey};

    #[test]
    fn test_public_key_to_hash() {
        // "04ff01b82f2f166c719937d5bd856bd919d9d6d495826cde3733cdb0d1084c8d12b311ced5cc235271c4a16a41fb943ab58e96ca6c4e2f85c6368999c8a3ec26b2"
        // "02ff01b82f2f166c719937d5bd856bd919d9d6d495826cde3733cdb0d1084c8d12"

        let public_key = PublicKey([
            2, 255, 1, 184, 47, 47, 22, 108, 113, 153, 55, 213, 189, 133, 107, 217, 25, 217, 214, 212, 149, 130, 108,
            222, 55, 51, 205, 176, 209, 8, 76, 141, 18,
        ]);

        assert_eq!(
            public_key.to_hash(),
            H160::from_slice(&hex::decode("84b42bde9034a27ce718af4bfbfb3b2ab842175d").unwrap())
        );
    }

    #[test]
    fn test_check_secret_key_constraints() {
        assert_err!(
            Secp256k1SecretKey::from_slice(&[0; 32]),
            Secp256k1Error::InvalidSecretKey
        );

        // https://en.bitcoin.it/wiki/Private_key
        assert_err!(
            Secp256k1SecretKey::from_slice(
                &hex::decode("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141").unwrap()
            ),
            Secp256k1Error::InvalidSecretKey
        );
    }

    #[test]
    fn test_new_deposit_public_key() {
        let secp = Secp256k1::new();
        let mut rng = OsRng::new().unwrap();

        // c
        let secure_id = H256::random();

        // v
        let mut vault_secret_key = Secp256k1SecretKey::new(&mut rng);
        // V
        let vault_public_key = Secp256k1PublicKey::from_secret_key(&secp, &vault_secret_key);
        let vault_public_key = PublicKey(vault_public_key.serialize());

        // D = V * c
        let deposit_public_key = vault_public_key.new_deposit_public_key(secure_id).unwrap();

        // d = v * c
        vault_secret_key
            .mul_assign(&vault_public_key.new_secret_key(secure_id))
            .unwrap();

        assert_eq!(
            deposit_public_key,
            PublicKey(Secp256k1PublicKey::from_secret_key(&secp, &vault_secret_key).serialize())
        );
    }

    #[test]
    fn test_new_deposit_public_key_static() {
        // bcrt1qzrkyemjkaxq48zwlnhxvear8fh6lvkwszxy7dm
        let old_public_key = PublicKey([
            2, 123, 236, 243, 192, 100, 34, 40, 51, 111, 129, 130, 160, 64, 129, 135, 11, 184, 68, 84, 83, 198, 234,
            196, 150, 13, 208, 86, 34, 150, 10, 59, 247,
        ]);

        let secret_key = &[
            137, 16, 46, 159, 212, 158, 232, 178, 197, 253, 105, 137, 102, 159, 70, 217, 110, 211, 254, 82, 216, 4,
            105, 171, 102, 252, 54, 190, 114, 91, 11, 69,
        ];

        // bcrt1qn9mgwncjtnavx23utveqqcrxh3zjtll58pc744
        let new_public_key = old_public_key.new_deposit_public_key_with_secret(secret_key).unwrap();

        assert_eq!(
            new_public_key,
            PublicKey([
                2, 151, 202, 113, 10, 9, 43, 125, 187, 101, 157, 152, 191, 94, 12, 236, 133, 229, 16, 233, 221, 52,
                150, 183, 243, 61, 110, 8, 152, 132, 99, 49, 189,
            ])
        );
    }
}
