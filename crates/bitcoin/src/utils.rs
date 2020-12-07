use primitive_types::{H256, U256};
use sha2::{Digest, Sha256};
use sp_std::{prelude::*, vec};

use crate::types::H256Le;

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

/// Returns the (ceiled) log base 2 of the value
/// ```
/// assert_eq!(bitcoin::utils::log2(4), 2);
/// assert_eq!(bitcoin::utils::log2(5), 3);
/// assert_eq!(bitcoin::utils::log2(8), 3);
/// assert_eq!(bitcoin::utils::log2(256), 8);
/// assert_eq!(bitcoin::utils::log2(257), 9);
/// assert_eq!(bitcoin::utils::log2(65536), 16);
/// assert_eq!(bitcoin::utils::log2(65537), 17);
/// ```
pub fn log2(value: u64) -> u32 {
    let mut current = value - 1;
    let mut result: u32 = 0;
    while current > 0 {
        current >>= 1;
        result += 1;
    }
    result
}

/// Returns the (ceiled) log base 256 of the value
/// ```
/// assert_eq!(bitcoin::utils::log256(&256u32.into()), 1);
/// assert_eq!(bitcoin::utils::log256(&257u32.into()), 2);
/// assert_eq!(bitcoin::utils::log256(&65536u32.into()), 2);
/// assert_eq!(bitcoin::utils::log256(&65537u32.into()), 3);
/// ```
pub fn log256(value: &U256) -> u8 {
    let mut current = value - 1;
    let mut result: u8 = 0;
    while current > 0.into() {
        current >>= 8;
        result += 1;
    }
    result
}

pub fn sha256d_be(bytes: &[u8]) -> H256 {
    H256::from_slice(&sha256d(bytes)[..])
}

pub fn sha256d_le(bytes: &[u8]) -> H256Le {
    H256Le::from_bytes_le(&sha256d(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log256() {
        let value = U256::from_dec_str("680733321990486529407107157001552378184394215934016880640")
            .unwrap();
        let result = log256(&value);
        assert_eq!(result, 24);
    }

    #[test]
    fn test_sha256d() {
        assert_eq!(
            [
                97, 244, 23, 55, 79, 68, 0, 180, 125, 202, 225, 168, 244, 2, 212, 244, 218, 207,
                69, 90, 4, 66, 160, 106, 164, 85, 164, 71, 176, 212, 225, 112
            ],
            sha256d(b"Hello World!")
        );
    }
}
