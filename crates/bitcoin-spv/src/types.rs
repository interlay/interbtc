extern crate serde_json;

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::btcspv;
use crate::utils;
use crate::validatespv;

/// A raw bitcoin header.
pub type RawHeader = [u8; 80];

/// A bitoin double-sha256 digest
pub type Hash256Digest = [u8; 32];

/// A bitcoin rmd160-of-sha256 digest
pub type Hash160Digest = [u8; 20];

#[doc(hidden)]
pub type RawBytes = Vec<u8>;

/// enum for bitcoin-spv errors
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum SPVError {
    /// Overran a checked read on a slice
    ReadOverrun,
    /// VarInt represents a number larger than 253.
    /// Large VarInts are not supported.
    LargeVarInt,
    /// Called `extract_op_return_data` on an output without an op_return.
    MalformattedOpReturnOutput,
    /// `extract_hash` identified a SH output prefix without a SH postfix.
    MalformattedP2SHOutput,
    /// `extract_hash` identified a PKH output prefix without a PKH postfix.
    MalformattedP2PKHOutput,
    /// `extract_hash` identified a Witness output with a bad length tag.
    MalformattedWitnessOutput,
    /// `extract_hash` could not identify the output type.
    MalformattedOutput,
    /// Header not exactly 80 bytes.
    WrongLengthHeader,
    /// Header does not meet its own difficulty target.
    InsufficientWork,
    /// Header in chain does not correctly reference parent header.
    InvalidChain,
    /// When validating a `BitcoinHeader`, the `hash` field is not the digest
    /// of the raw header.
    WrongDigest,
    /// When validating a `BitcoinHeader`, the `hash` and `hash_le` fields
    /// do not match.
    NonMatchingDigests,
    /// When validating a `BitcoinHeader`, the `merkle_root` field does not
    /// match the root found in the raw header.
    WrongMerkleRoot,
    /// When validating a `BitcoinHeader`, the `merkle_root` and
    /// `merkle_root_le` fields do not match.
    NonMatchingMerkleRoots,
    /// When validating a `BitcoinHeader`, the `prevhash` field does not
    /// match the parent hash found in the raw header.
    WrongPrevHash,
    /// When validating a `BitcoinHeader`, the `prevhash` and
    /// `prevhash_le` fields do not match.
    NonMatchingPrevhashes,
    /// A `vin` (transaction input vector) is malformatted.
    InvalidVin,
    /// A `vout` (transaction output vector) is malformatted.
    InvalidVout,
    /// When validating an `SPVProof`, the `tx_id` field is not the digest
    /// of the `version`, `vin`, `vout`, and `locktime`.
    WrongTxID,
    /// When validating an `SPVProof`, the `intermediate_nodes` is not a valid
    /// merkle proof connecting the `tx_id_le` to the `confirming_header`.
    BadMerkleProof,
    /// Any other error
    UnknownError,
}

/// Enum for transaction input types
#[derive(PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum InputType {
    /// Unknown input type, likely an error
    InputNone,
    /// Legacy input
    Legacy,
    /// Witness-over-scripthash Compatibility input
    Compatibility,
    /// Witness input
    Witness,
}

/// Enum for transaction output types
#[derive(PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum OutputType {
    /// Unknown output type, likely an error.
    OutputNone,
    /// Witness pubkeyhash output script.
    WPKH,
    /// Witness scripthash output script.
    WSH,
    /// OpReturn data output.
    OpReturn,
    /// Legacy pubkeyhash output script.
    PKH,
    /// Legacy scripthash output script.
    SH,
    /// Any other output script.
    Nonstandard,
}

/// BitcoinHeader is a parsed Bitcoin header with height information appended.
#[derive(Clone, Deserialize, Serialize)]
pub struct BitcoinHeader {
    /// The double-sha2 digest encoded BE.
    #[serde(with = "internal_ser::digest_ser")]
    pub hash: Hash256Digest,
    /// The 80-byte raw header.
    #[serde(with = "internal_ser::raw_ser")]
    pub raw: RawHeader,
    /// The double-sha2 digest encoded LE.
    #[serde(with = "internal_ser::digest_ser")]
    pub hash_le: Hash256Digest,
    /// The height of the header
    pub height: u32,
    /// The double-sha2 digest of the parent encoded BE.
    #[serde(with = "internal_ser::digest_ser")]
    pub prevhash: Hash256Digest,
    /// The double-sha2 digest of the parent encoded LE.
    #[serde(with = "internal_ser::digest_ser")]
    pub prevhash_le: Hash256Digest,
    /// The double-sha2 merkle tree root of the block transactions encoded BE.
    #[serde(with = "internal_ser::digest_ser")]
    pub merkle_root: Hash256Digest,
    /// The double-sha2 merkle tree root of the block transactions encoded LE.
    #[serde(with = "internal_ser::digest_ser")]
    pub merkle_root_le: Hash256Digest,
}

impl BitcoinHeader {
    /// Checks validity of an entire Bitcoin header
    ///
    /// # Arguments
    ///
    /// * `self` - The Bitcoin header
    ///
    /// # Errors
    ///
    /// * Errors if any of the Bitcoin header elements are invalid.
    pub fn validate(&self) -> Result<(), SPVError> {
        let raw_as_slice = &self.raw[..];
        if self.hash_le[..] != btcspv::hash256(raw_as_slice) {
            return Err(SPVError::WrongDigest);
        }
        if self.hash_le[..] != utils::reverse_endianness(&self.hash.to_vec())[..] {
            return Err(SPVError::NonMatchingDigests);
        }
        if self.merkle_root_le[..] != btcspv::extract_merkle_root_le(self.raw) {
            return Err(SPVError::WrongMerkleRoot);
        }
        if self.merkle_root_le[..] != utils::reverse_endianness(&self.merkle_root.to_vec())[..] {
            return Err(SPVError::NonMatchingMerkleRoots);
        }
        if self.prevhash_le[..] != btcspv::extract_prev_block_hash_le(self.raw) {
            return Err(SPVError::WrongPrevHash);
        }
        if self.prevhash_le[..] != utils::reverse_endianness(&self.prevhash.to_vec())[..] {
            return Err(SPVError::NonMatchingPrevhashes);
        }
        Ok(())
    }
}

impl PartialEq for BitcoinHeader {
    /// Compares two Bitcoin headers
    ///
    /// # Arguments
    ///
    /// * `self` - The Bitcoin header
    /// * ` other` - The second Bitcoin header
    fn eq(&self, other: &Self) -> bool {
        (self.raw[..] == other.raw[..]
            && self.hash == other.hash
            && self.hash_le == other.hash_le
            && self.height == other.height
            && self.prevhash == other.prevhash
            && self.prevhash_le == other.prevhash_le
            && self.merkle_root == other.merkle_root
            && self.merkle_root_le == other.merkle_root_le)
    }
}

impl Eq for BitcoinHeader {}

#[cfg_attr(tarpaulin, skip)]
impl fmt::Debug for BitcoinHeader {
    /// Formats the bitcoin header for readability
    ///
    /// # Arguments
    ///
    /// * `self` - The Bitcoin header
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Header (height {:?}:\t{})",
            self.height,
            utils::serialize_hex(&self.raw[..])
        )
    }
}

#[cfg_attr(tarpaulin, skip)]
impl fmt::Display for BitcoinHeader {
    /// Formats the bitcoin header for readability
    ///
    /// # Arguments
    ///
    /// * `self` - The Bitcoin header
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Header (height {:?}:\t{})",
            self.height,
            utils::serialize_hex(&self.raw[..])
        )
    }
}

/// SPVProof is an SPV inclusion proof for a confirmed transaction.
#[derive(PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct SPVProof {
    /// The 4-byte LE-encoded version number. Currently always 1 or 2.
    #[serde(with = "internal_ser::vec_ser")]
    pub version: RawBytes,
    /// The transaction input vector, length-prefixed.
    #[serde(with = "internal_ser::vec_ser")]
    pub vin: RawBytes,
    /// The transaction output vector, length-prefixed.
    #[serde(with = "internal_ser::vec_ser")]
    pub vout: RawBytes,
    /// The transaction input vector, length-prefixed.
    #[serde(with = "internal_ser::vec_ser")]
    pub locktime: RawBytes,
    /// The tx id
    #[serde(with = "internal_ser::digest_ser")]
    pub tx_id: Hash256Digest,
    /// The tx id in LE
    #[serde(with = "internal_ser::digest_ser")]
    pub tx_id_le: Hash256Digest,
    /// The transaction index
    pub index: u32,
    /// The confirming Bitcoin header
    pub confirming_header: BitcoinHeader,
    /// The intermediate nodes (digests between leaf and root)
    #[serde(with = "internal_ser::vec_ser")]
    pub intermediate_nodes: RawBytes,
}

impl SPVProof {
    /// Checks validity of an entire SPV Proof
    ///
    /// # Arguments
    ///
    /// * `self` - The SPV Proof
    ///
    /// # Errors
    ///
    /// * Errors if any of the SPV Proof elements are invalid.
    pub fn validate(&self) -> Result<(), SPVError> {
        if !btcspv::validate_vin(&self.vin) {
            return Err(SPVError::InvalidVin);
        }

        if !btcspv::validate_vout(&self.vout) {
            return Err(SPVError::InvalidVout);
        }

        let tx_id =
            validatespv::calculate_txid(&self.version, &self.vin, &self.vout, &self.locktime);
        if tx_id[..] != self.tx_id_le[..] {
            return Err(SPVError::WrongTxID);
        }

        self.confirming_header.validate()?;

        if !validatespv::prove(
            tx_id,
            self.confirming_header.merkle_root_le,
            &self.intermediate_nodes,
            self.index as u64,
        ) {
            return Err(SPVError::BadMerkleProof);
        }
        Ok(())
    }
}

#[cfg_attr(tarpaulin, skip)]
impl fmt::Debug for SPVProof {
    /// Formats the SPV Proof for readability
    ///
    /// # Arguments
    ///
    /// * `self` - The SPV Proof
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "\nSPVProof (\n\ttx_id:\t{}\n\tindex:\t{}\n\th:\t",
            utils::serialize_hex(&self.tx_id[..]),
            self.index
        )?;
        self.confirming_header.fmt(f)?;
        write!(
            f,
            "\n\tproof:\t{})\n",
            utils::serialize_hex(&self.intermediate_nodes[..])
        )
    }
}

#[cfg_attr(tarpaulin, skip)]
impl fmt::Display for SPVProof {
    /// Formats the SPVProof for readability
    ///
    /// # Arguments
    ///
    /// * `self` - The SPV Proof
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "\nSPVProof (\n\ttx_id:\t{}\n\tindex:\t{}\n\th:\t",
            utils::serialize_hex(&self.tx_id[..]),
            self.index
        )?;
        self.confirming_header.fmt(f)?;
        write!(
            f,
            "\n\tproof:\t{})\n",
            utils::serialize_hex(&self.intermediate_nodes[..])
        )
    }
}

mod internal_ser {
    use super::{Hash256Digest, RawHeader};
    use crate::utils;
    use serde::{Deserialize, Deserializer, Serializer};

    pub mod vec_ser {
        use super::*;

        pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
        where
            D: Deserializer<'de>,
        {
            let s: &str = Deserialize::deserialize(deserializer)?;
            let result = utils::deserialize_hex(s);
            match result {
                Ok(v) => Ok(v),
                Err(e) => Err(serde::de::Error::custom(e.to_string())),
            }
        }

        pub fn serialize<S>(d: &[u8], serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let s: &str = &utils::serialize_hex(&d[..]);
            serializer.serialize_str(s)
        }
    }

    pub mod digest_ser {
        use super::*;

        pub fn deserialize<'de, D>(deserializer: D) -> Result<Hash256Digest, D::Error>
        where
            D: Deserializer<'de>,
        {
            let s: &str = Deserialize::deserialize(deserializer)?;
            let mut digest: Hash256Digest = Default::default();
            let result = utils::deserialize_hex(s);

            let deser: Vec<u8>;
            match result {
                Ok(v) => deser = v,
                Err(e) => return Err(serde::de::Error::custom(e.to_string())),
            }
            if deser.len() != 32 {
                let err_string: String = format!("Expected 32 bytes, got {:?} bytes", deser.len());
                return Err(serde::de::Error::custom(err_string));
            }
            digest.copy_from_slice(&deser);
            Ok(digest)
        }

        pub fn serialize<S>(d: &Hash256Digest, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let s: &str = &utils::serialize_hex(&d[..]);
            serializer.serialize_str(s)
        }
    }

    pub mod raw_ser {
        use super::*;

        pub fn deserialize<'de, D>(deserializer: D) -> Result<RawHeader, D::Error>
        where
            D: Deserializer<'de>,
        {
            let s: &str = Deserialize::deserialize(deserializer)?;
            let mut header: RawHeader = [0; 80];

            let result = utils::deserialize_hex(s);

            let deser: Vec<u8>;
            match result {
                Ok(v) => deser = v,
                Err(e) => return Err(serde::de::Error::custom(e.to_string())),
            }
            if deser.len() != 80 {
                let err_string: String = format!("Expected 80 bytes, got {:?} bytes", deser.len());
                return Err(serde::de::Error::custom(err_string));
            }
            header.copy_from_slice(&deser);
            Ok(header)
        }

        pub fn serialize<S>(d: &RawHeader, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let s: &str = &utils::serialize_hex(&d[..]);
            serializer.serialize_str(s)
        }
    }
}

#[cfg(test)]
#[cfg_attr(tarpaulin, skip)]
mod tests {
    use serde_json;
    use std::fs::File;
    use std::io::Read;
    use std::panic;

    use super::*;
    use crate::utils::test_utils;

    #[derive(Debug, Deserialize)]
    struct InvalidHeadersCases {
        header: BitcoinHeader,
        e: String,
    }

    #[derive(Debug, Deserialize)]
    struct InvalidProofsCases {
        proof: SPVProof,
        e: String,
    }

    #[allow(non_snake_case)]
    #[derive(Debug, Deserialize)]
    struct TestCases {
        valid: Vec<String>,
        badHeaders: Vec<InvalidHeadersCases>,
        badSPVProofs: Vec<InvalidProofsCases>,
        errBadHexBytes: String,
        errBadHexHash256: String,
        errBadLenHash256: String,
        errBadHexRawHeader: String,
        errBadLenRawHeader: String,
    }

    fn setup() -> TestCases {
        let mut file = File::open("../testProofs.json").unwrap();
        let mut data = String::new();
        file.read_to_string(&mut data).unwrap();

        let cases: TestCases = serde_json::from_str(&data).unwrap();
        cases
    }

    fn run_test<T>(test: T) -> ()
    where
        T: FnOnce(&TestCases) -> () + panic::UnwindSafe,
    {
        let cases = setup();

        let result = panic::catch_unwind(|| test(&cases));

        assert!(result.is_ok())
    }

    #[test]
    fn it_can_deser_and_reser_proofs() {
        run_test(|cases| {
            let valid_proofs = &cases.valid;
            for case in valid_proofs {
                let proof: SPVProof = serde_json::from_str(&case).unwrap();
                let re_stringed = serde_json::to_string(&proof).unwrap();
                let re_proofed: SPVProof = serde_json::from_str(&re_stringed).unwrap();
                assert_eq!(re_proofed, proof);
            }
        })
    }

    #[test]
    fn it_errors_on_bad_hex_bytes() {
        run_test(|cases| {
            let invalid_proof = &cases.errBadHexBytes;
            let proof: serde_json::Result<SPVProof> = serde_json::from_str(invalid_proof);
            let expected = "Invalid character \'Q\' at position";
            match proof {
                Ok(_) => assert!(false, "Expected error"),
                Err(v) => assert!(v.to_string().contains(expected)),
            }
        })
    }

    #[test]
    fn it_errors_on_bad_hash256_bytes() {
        run_test(|cases| {
            let invalid_proof = &cases.errBadHexHash256;
            let proof: serde_json::Result<SPVProof> = serde_json::from_str(invalid_proof);
            let expected = "Invalid character \'R\' at position";
            match proof {
                Ok(_) => assert!(false, "Expected error"),
                Err(v) => assert!(v.to_string().contains(expected)),
            }
        })
    }

    #[test]
    fn it_errors_on_bad_hash256_len() {
        run_test(|cases| {
            let invalid_proof = &cases.errBadLenHash256;
            let proof: serde_json::Result<SPVProof> = serde_json::from_str(invalid_proof);
            let expected = "Expected 32 bytes, got 31 bytes";
            match proof {
                Ok(_) => assert!(false, "Expected error"),
                Err(v) => assert!(v.to_string().contains(expected)),
            }
        })
    }

    #[test]
    fn it_errors_on_bad_header_bytes() {
        run_test(|cases| {
            let invalid_proof = &cases.errBadHexRawHeader;
            let proof: serde_json::Result<SPVProof> = serde_json::from_str(invalid_proof);
            let expected = "Invalid character \'S\' at position";
            match proof {
                Ok(_) => assert!(false, "Expected error"),
                Err(v) => assert!(v.to_string().contains(expected)),
            }
        })
    }

    #[test]
    fn it_errors_on_bad_header_len() {
        run_test(|cases| {
            let invalid_proof = &cases.errBadLenRawHeader;
            let proof: serde_json::Result<SPVProof> = serde_json::from_str(invalid_proof);
            let expected = "Expected 80 bytes, got 79 bytes";
            match proof {
                Ok(_) => assert!(false, "Expected error"),
                Err(v) => assert!(v.to_string().contains(expected)),
            }
        })
    }

    #[test]
    fn it_validates_bitcoin_header_objects() {
        run_test(|cases| {
            let valid: SPVProof = serde_json::from_str(&cases.valid[0]).unwrap();
            let header = valid.confirming_header;
            let result = header.validate();
            result.unwrap(); // panics if there's an error

            let invalid = &cases.badHeaders;
            for i in invalid {
                let res = i.header.validate();
                let expected = test_utils::match_string_to_err(&i.e);
                match res {
                    Ok(_) => assert!(false, "Expected an error"),
                    Err(e) => assert_eq!(e, expected),
                }
            }
        })
    }

    #[test]
    fn it_validates_spv_proof_objects() {
        run_test(|cases| {
            let valid: SPVProof = serde_json::from_str(&cases.valid[0]).unwrap();
            let result = valid.validate();
            result.unwrap(); // panics if there's an error

            let invalid = &cases.badSPVProofs;
            for i in invalid {
                let res = i.proof.validate();
                let expected = test_utils::match_string_to_err(&i.e);
                match res {
                    Ok(_) => assert!(false, "Expected an error"),
                    Err(e) => assert_eq!(e, expected),
                }
            }
        })
    }
}
