
use sha2::{Sha256, Digest};
use crate::types::H256Le;
use primitive_types::H256;


/// Computes Bitcoin's double SHA256 hash over a LE byte encoded input
/// 
/// # Arguments
/// * data: LE bytes encoded input
/// 
/// # Returns
/// * The double SHA256 hash encoded as LE bytes from data
pub fn sha256d(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::default();
    hasher.input(bytes);
    let digest = hasher.result();

    let mut second_hasher = Sha256::default();
    second_hasher.input(digest);

    let mut ret = [0; 32];
    ret.copy_from_slice(&second_hasher.result()[..]);
    ret
}

/// Concatenates and hashes two inputs for merkle proving.
///
/// # Arguments
///
/// * `a` - The first hash
/// * `b` - The second hash
pub fn hash256_merkle_step(a: &[u8], b: &[u8]) -> H256Le {
    let mut res: Vec<u8> = vec![];
    res.extend(a);
    res.extend(b);
    H256Le::from_bytes_le(&sha256d(&res))
}


/// Reverses endianness of the value
/// ```
/// let bytes = bitcoin::utils::reverse_endianness(&[1, 2, 3]);
/// assert_eq!(&bytes, &[3, 2, 1])
/// ```
pub fn reverse_endianness(bytes: &[u8]) -> Vec<u8> {
    let mut vec = Vec::from(bytes);
    vec.reverse();
    vec
}


// FIXME: maybe use sp_core sha2_256?
pub fn sha256d_be(bytes: &[u8]) -> H256 {
    return H256::from_slice(&sha256d(bytes)[..]);
}

pub fn sha256d_le(bytes: &[u8]) -> H256Le {
    return H256Le::from_bytes_le(&sha256d(bytes));
}