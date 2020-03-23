use primitive_types::{H256};

use crate::types::{Error};

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


pub fn check_p2pkh_flag(raw_output: &[u8]) -> bool {
    let tag = &raw_output[8..11];
    return tag == [0x19, 0x76, 0xa9];
}

pub fn check_opreturn_flag(raw_output: &[u8]) -> bool {
    return raw_output[9] == 0x6a;
}


pub fn extract_value(raw_output: &[u8]) -> u64 {
    return btcspv::extract_value(raw_output);
}

pub fn extract_address_hash(output_script: &[u8]) -> Result<Vec<u8>, Error> {

    let script_len = output_script.len();
    
    // Witness
    if output_script[0] == 0 {
        if script_len < 2 {
            return Err(Error::MalformedWitnessOutput);
        }
        if output_script[1] == (script_len - 2) as u8 {
            return Ok(output_script[2..].to_vec());
        } else {
            return Err(Error::MalformedWitnessOutput);
        }
    }

    // P2PKH
    // 25 bytes
    // Format:
    // 0x76 (OP_DUP) - 0xa9 (OP_HASH160) - 0x14 (20 bytes len) - <20 bytes pubkey hash> - 0x88 (OP_EQUALVERIFY) - 0xac (OP_CHECKSIG)
    if script_len == 25 && output_script[0..2] == [0x76, 0xa9, 0x14] {
        if output_script[script_len - 2..] != [0x88, 0xac] {
            return Err(Error::MalformedP2PKHOutput);
        }
        return Ok(output_script[3..script_len-2].to_vec());
    }

    // P2SH
    // 23 bytes
    // Format: 
    // 0xa9 (OP_HASH160) - 0x14 (20 bytes hash) - <20 bytes script hash> - 0x87 (OP_EQUAL)
    if script_len == 23 && output_script[0..1] == [0xa9, 0x14] {
        if output_script[script_len-1] as u8 != 0x87 {
            return Err(Error::MalformedP2SHOutput)
        }
        return Ok(output_script[1..(script_len-1)].to_vec())
    }
    return Err(Error::UnsupportedOutputFormat)
}

pub fn extract_op_return_data(raw_output: &[u8]) -> Result<Vec<u8>, Error> {
    match btcspv::extract_op_return_data(raw_output) {
        Ok(opreturn) => Ok(opreturn),
        Err(err) => Err(Error::MalformedOpReturnOutput)
    }
}