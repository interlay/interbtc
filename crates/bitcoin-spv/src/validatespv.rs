extern crate num_bigint as bigint;

use bigint::BigUint;

use crate::btcspv;
use crate::types::{Hash256Digest, RawHeader, SPVError};


/// Evaluates a Bitcoin merkle inclusion proof.
/// Note that `index` is not a reliable indicator of location within a block.
///
/// # Arguments
///
/// * `txid` - The txid (LE)
/// * `merkle_root` - The merkle root (as in the block header)
/// * `intermediate_nodes` - The proof's intermediate nodes (digests between leaf and root)
/// * `index` - The leaf's index in the tree (0-indexed)
pub fn prove(
    txid: Hash256Digest,
    merkle_root: Hash256Digest,
    intermediate_nodes: &[u8],
    index: u64,
) -> bool {
    if txid == merkle_root && index == 0 && intermediate_nodes.is_empty() {
        return true;
    }
    let mut proof: Vec<u8> = vec![];
    proof.extend(&txid);
    proof.extend(intermediate_nodes);
    proof.extend(&merkle_root);

    btcspv::verify_hash256_merkle(&proof, index)
}

/// Hashes transaction to get txid.
///
/// # Arguments
///
/// * `version` - 4-bytes version
/// * `vin` - Raw bytes length-prefixed input vector
/// * `vout` - Raw bytes length-prefixed output vector
/// * `locktime` - 4-byte tx locktime
pub fn calculate_txid(version: &[u8], vin: &[u8], vout: &[u8], locktime: &[u8]) -> Hash256Digest {
    let mut tx: Vec<u8> = vec![];
    tx.extend(version);
    tx.extend(vin);
    tx.extend(vout);
    tx.extend(locktime);
    btcspv::hash256(&tx)
}

/// Checks validity of header work.
///
/// # Arguments
///
/// * `digest` - The digest
/// * `target` - The target threshold
pub fn validate_header_work(digest: Hash256Digest, target: &BigUint) -> bool {
    let empty: Hash256Digest = Default::default();

    if digest == empty {
        return false;
    }

    BigUint::from_bytes_le(&digest[..]) < *target
}

/// Checks validity of header chain.
///
/// # Arguments
///
/// * `header` - The raw bytes header
/// * `prev_hash` - The previous header's digest
pub fn validate_header_prev_hash(header: RawHeader, prev_hash: Hash256Digest) -> bool {
    let actual = btcspv::extract_prev_block_hash_le(header);
    actual == prev_hash
}

/// Checks validity of header chain.
/// Compares the hash of each header to the prevHash in the next header.
///
/// # Arguments
///
/// * `headers` - Raw byte array of header chain
///
/// # Errors
///
/// * Errors if header chain is the wrong length, chain is invalid or insufficient work
pub fn validate_header_chain(headers: &[u8]) -> Result<BigUint, SPVError> {
    if headers.len() % 80 != 0 {
        return Err(SPVError::WrongLengthHeader);
    }

    let mut digest: Hash256Digest = Default::default();
    let mut total_difficulty = BigUint::from(0 as u8);

    for i in 0..headers.len() / 80 {
        let start = i * 80;
        let mut header: RawHeader = [0; 80];
        header.copy_from_slice(&headers[start..start + 80]);

        if i != 0 && !validate_header_prev_hash(header, digest) {
            return Err(SPVError::InvalidChain);
        }

        let target = btcspv::extract_target(header);
        digest.copy_from_slice(&btcspv::hash256(&header));
        if !validate_header_work(digest, &target) {
            return Err(SPVError::InsufficientWork);
        }
        total_difficulty += btcspv::calculate_difficulty(&target);
    }
    Ok(total_difficulty)
}

#[cfg(test)]
#[cfg_attr(tarpaulin, skip)]
mod tests {

    use super::*;
    use crate::utils::*;

    #[test]
    fn it_verifies_merkle_inclusion_proofs() {
        test_utils::run_test(|fixtures| {
            let test_cases = test_utils::get_test_cases("prove", &fixtures);
            for case in test_cases {
                let inputs = case.input.as_object().unwrap();

                let mut txid: Hash256Digest = Default::default();
                let id = force_deserialize_hex(inputs.get("txIdLE").unwrap().as_str().unwrap());
                txid.copy_from_slice(&id);

                let mut merkle_root: Hash256Digest = Default::default();
                let root =
                    force_deserialize_hex(inputs.get("merkleRootLE").unwrap().as_str().unwrap());
                merkle_root.copy_from_slice(&root);

                let proof = force_deserialize_hex(inputs.get("proof").unwrap().as_str().unwrap());
                let index = inputs.get("index").unwrap().as_u64().unwrap() as u64;

                let expected = case.output.as_bool().unwrap();
                assert_eq!(prove(txid, merkle_root, &proof, index), expected);
            }
        })
    }

    #[test]
    fn it_calculates_transaction_ids() {
        test_utils::run_test(|fixtures| {
            let test_cases = test_utils::get_test_cases("calculateTxId", &fixtures);
            for case in test_cases {
                let inputs = case.input.as_object().unwrap();
                let version =
                    force_deserialize_hex(inputs.get("version").unwrap().as_str().unwrap());
                let vin = force_deserialize_hex(inputs.get("vin").unwrap().as_str().unwrap());
                let vout = force_deserialize_hex(inputs.get("vout").unwrap().as_str().unwrap());
                let locktime =
                    force_deserialize_hex(inputs.get("locktime").unwrap().as_str().unwrap());
                let mut expected: Hash256Digest = Default::default();
                expected.copy_from_slice(&force_deserialize_hex(case.output.as_str().unwrap()));

                assert_eq!(calculate_txid(&version, &vin, &vout, &locktime), expected);
            }
        })
    }

    #[test]
    fn it_checks_header_work() {
        test_utils::run_test(|fixtures| {
            let test_cases = test_utils::get_test_cases("validateHeaderWork", &fixtures);
            for case in test_cases {
                let inputs = case.input.as_object().unwrap();

                let mut digest: Hash256Digest = Default::default();
                digest.copy_from_slice(&force_deserialize_hex(
                    inputs.get("digest").unwrap().as_str().unwrap(),
                ));

                let t = inputs.get("target").unwrap();
                let target = match t.is_u64() {
                    true => BigUint::from(t.as_u64().unwrap()),
                    false => BigUint::from_bytes_be(&force_deserialize_hex(t.as_str().unwrap())),
                };

                let expected = case.output.as_bool().unwrap();
                assert_eq!(validate_header_work(digest, &target), expected);
            }
        })
    }

    #[test]
    fn it_checks_header_prev_hash() {
        test_utils::run_test(|fixtures| {
            let test_cases = test_utils::get_test_cases("validateHeaderPrevHash", &fixtures);
            for case in test_cases {
                let inputs = case.input.as_object().unwrap();

                let mut prev_hash: Hash256Digest = Default::default();
                prev_hash.copy_from_slice(&force_deserialize_hex(
                    inputs.get("prevHash").unwrap().as_str().unwrap(),
                ));

                let mut header: RawHeader = [0; 80];
                header.copy_from_slice(&force_deserialize_hex(
                    inputs.get("header").unwrap().as_str().unwrap(),
                ));

                let expected = case.output.as_bool().unwrap();
                assert_eq!(validate_header_prev_hash(header, prev_hash), expected);
            }
        })
    }

    #[test]
    fn it_validates_header_chains() {
        test_utils::run_test(|fixtures| {
            let test_cases = test_utils::get_test_cases("validateHeaderChain", &fixtures);
            for case in test_cases {
                let input = force_deserialize_hex(case.input.as_str().unwrap());
                let output = case.output.as_u64().unwrap();
                let expected = BigUint::from(output);
                assert_eq!(validate_header_chain(&input).unwrap(), expected);
            }
        })
    }

    #[test]
    fn it_errors_while_validating_header_chains() {
        test_utils::run_test(|fixtures| {
            let test_cases = test_utils::get_test_cases("validateHeaderChainError", &fixtures);
            for case in test_cases {
                let input = force_deserialize_hex(case.input.as_str().unwrap());
                let expected =
                    test_utils::match_string_to_err(case.error_message.as_str().unwrap());
                match validate_header_chain(&input) {
                    Ok(_) => assert!(false, "expected an error"),
                    Err(v) => assert_eq!(v, expected),
                }
            }
        })
    }

    #[test]
    fn it_extracts_difficulty_from_headers() {
        test_utils::run_test(|fixtures| {
            let test_cases = test_utils::get_test_cases("retargetAlgorithm", &fixtures);
            for case in test_cases {
                let headers = test_utils::get_headers(&case.input);
                for header in headers {
                    assert_eq!(btcspv::extract_difficulty(header.raw), header.difficulty);
                }
            }
        })
    }
}
