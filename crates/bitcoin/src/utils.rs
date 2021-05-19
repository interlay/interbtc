use sha2::{Digest, Sha256};
use sp_core::{H256, U256};
use sp_std::{prelude::*, vec};

use crate::types::H256Le;

// the _SIZE constants describe the size in number of bytes of various parts of transactions.
// Since bytes in the witnesses cost only 1/4th of the cost to transmit, the so called virtual
// size, or vsize, can be fractional. In order to be able to work with integer math, we also
// use  weight, which is 4 times the virtual size. See https://en.bitcoin.it/wiki/Weight_units
// for more detail.
const P2PKH_IN_WEIGHT: u32 = 148 * 4;
const P2PKH_OUT_SIZE: u32 = 34;
const P2SH_OUT_SIZE: u32 = 32;
const P2WPKH_OUT_SIZE: u32 = 31;
const PUBKEY_SIZE: u32 = 33;
const SIGNATURE_SIZE: u32 = 72;
const OP_RETURN_OUT_SIZE: u32 = 34;
const P2WPKH_IN_WEIGHT: u32 = 271; // 67.75 * 4;

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

pub enum InputType {
    P2PKH,
    P2SH { num_signatures: u32, num_pubkeys: u32 },
    P2WPKHv0,
}

pub struct TransactionInputMetadata {
    pub script_type: InputType,
    pub count: u32,
}

pub struct TransactionOutputMetadata {
    pub num_p2pkh: u32,
    pub num_p2sh: u32,
    pub num_p2wpkh: u32,
    pub num_op_return: u32,
}

const fn script_length_size(length: u32) -> u32 {
    if length < 75 {
        1
    } else if length <= 255 {
        2
    } else if length <= 65535 {
        3
    } else {
        5
    }
}

const fn var_int_size(length: u32) -> u32 {
    if length < 253 {
        1
    } else if length < 65535 {
        3
    } else if length < 4294967295 {
        5
    } else {
        9
    }
}

const fn transaction_header_weight(input_type: InputType, input_count: u32, output_count: u32) -> u32 {
    let extra_witness_weight = match input_type {
        InputType::P2PKH | InputType::P2SH { .. } => 0,
        InputType::P2WPKHv0 => var_int_size(input_count) + 2, // sigwit marker, flag & witness element count
    };

    let header_bytes = 4 // nVersion
        + var_int_size(input_count) // number of inputs
        + var_int_size(output_count) // number of outputs
        + 4; // nLockTime

    header_bytes * 4 + extra_witness_weight
}

/// Bytes in the witnesses cost only 1/4th of the cost to transmit. In order to calculate the cost
/// in fixed point math, this function calculates the weight instead of the virtual size, which is
/// equal to the virtual size multiplied by 4.
/// This code is based on https://github.com/jlopp/bitcoin-transaction-size-calculator
const fn transaction_weight(input: TransactionInputMetadata, output: TransactionOutputMetadata) -> u32 {
    let input_weight = input.count
        * match input.script_type {
            InputType::P2PKH => P2PKH_IN_WEIGHT,
            InputType::P2WPKHv0 => P2WPKH_IN_WEIGHT,
            InputType::P2SH {
                num_signatures,
                num_pubkeys,
            } => {
                let redeem_script_size = 1              // OP_M
                    + num_pubkeys * (1 + PUBKEY_SIZE)   // OP_PUSH33 <pubkey>
                    + 1                                 // OP_N
                    + 1; // OP_CHECKMULTISIG
                let script_sig_size = 1                     // size(0)
                    + num_signatures * (1 + SIGNATURE_SIZE) // size(SIGNATURE_SIZE) + signature
                    + script_length_size(redeem_script_size)
                    + redeem_script_size;
                let input_size = 32 + 4 + var_int_size(script_sig_size) + script_sig_size + 4;

                input_size * 4
            }
        };

    let output_weight = 4
        * (output.num_p2pkh * P2PKH_OUT_SIZE
            + output.num_p2sh * P2SH_OUT_SIZE
            + output.num_p2wpkh * P2WPKH_OUT_SIZE
            + output.num_op_return * OP_RETURN_OUT_SIZE);

    let output_count = output.num_op_return + output.num_p2pkh + output.num_p2sh + output.num_p2wpkh;
    let header_weight = transaction_header_weight(input.script_type, input.count, output_count);

    input_weight + output_weight + header_weight
}

pub const fn virtual_transaction_size(input: TransactionInputMetadata, output: TransactionOutputMetadata) -> u32 {
    let weight = transaction_weight(input, output);
    (weight + 3) / 4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log256() {
        let value = U256::from_dec_str("680733321990486529407107157001552378184394215934016880640").unwrap();
        let result = log256(&value);
        assert_eq!(result, 24);
    }

    #[test]
    fn test_sha256d() {
        assert_eq!(
            [
                97, 244, 23, 55, 79, 68, 0, 180, 125, 202, 225, 168, 244, 2, 212, 244, 218, 207, 69, 90, 4, 66, 160,
                106, 164, 85, 164, 71, 176, 212, 225, 112
            ],
            sha256d(b"Hello World!")
        );
    }

    #[test]
    fn test_transaction_weight() {
        assert_eq!(
            transaction_weight(
                TransactionInputMetadata {
                    count: 2,
                    script_type: InputType::P2PKH
                },
                TransactionOutputMetadata {
                    num_op_return: 1,
                    num_p2pkh: 3,
                    num_p2sh: 4,
                    num_p2wpkh: 5
                }
            ),
            2764 + 4 * OP_RETURN_OUT_SIZE
        );

        assert_eq!(
            transaction_weight(
                TransactionInputMetadata {
                    count: 1,
                    script_type: InputType::P2PKH
                },
                TransactionOutputMetadata {
                    num_op_return: 0,
                    num_p2pkh: 1,
                    num_p2sh: 0,
                    num_p2wpkh: 0
                }
            ),
            768
        );

        assert_eq!(
            transaction_weight(
                TransactionInputMetadata {
                    count: 3,
                    script_type: InputType::P2SH {
                        num_pubkeys: 9,
                        num_signatures: 7
                    }
                },
                TransactionOutputMetadata {
                    num_op_return: 1,
                    num_p2pkh: 5,
                    num_p2sh: 3,
                    num_p2wpkh: 4
                }
            ),
            12004 + OP_RETURN_OUT_SIZE * 4
        );

        assert_eq!(
            transaction_weight(
                TransactionInputMetadata {
                    count: 3,
                    script_type: InputType::P2WPKHv0
                },
                TransactionOutputMetadata {
                    num_op_return: 1,
                    num_p2pkh: 5,
                    num_p2sh: 3,
                    num_p2wpkh: 4
                }
            ),
            2416 + OP_RETURN_OUT_SIZE * 4
        );
    }

    #[test]
    fn test_virtual_transaction_size() {
        assert_eq!(
            virtual_transaction_size(
                TransactionInputMetadata {
                    count: 2,
                    script_type: InputType::P2PKH
                },
                TransactionOutputMetadata {
                    num_op_return: 1,
                    num_p2pkh: 2,
                    num_p2sh: 0,
                    num_p2wpkh: 0,
                }
            ),
            374 + OP_RETURN_OUT_SIZE
        );
    }
}
