use primitive_types::{H256};

use bitcoin_spv::btcspv;


/// Computes Bitcoin's double SHA256 hash over the given input
/// 
/// # Arguments
/// * data: bytes encoded input
/// 
/// # Returns
/// * The double SHA256 hash encodes as a bytes from data
pub fn sha256d(data: &[u8]) -> H256{
    let hash = &btcspv::hash256(data)[..];
    H256::from_slice(hash)
}


