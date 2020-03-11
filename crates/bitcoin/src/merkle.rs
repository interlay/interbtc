use primitive_types::H256;

use crate::parser;
use crate::types::BlockHeader;

/// Struct to store the content of a merkle proof
pub struct MerkleProof {
    pub block_header: BlockHeader,
    pub transactions_count: u32,
    pub hashes: Vec<H256>,
    pub flag_bits: Vec<u8>,
}

/// Parses a merkle proof as produced by the bitcoin client gettxoutproof
///
/// Block header (80 bytes)
/// Number of transactions in the block (unsigned int, 4 bytes, little endian)
/// Number of hashes (varint, 1 - 3 bytes)
/// Hashes (N * 32 bytes, little endian)
/// Number of bytes of flag bits (varint, 1 - 3 bytes)
/// Flag bits (little endian)
///
/// See: https://bitqa.app/questions/how-to-decode-merkle-transaction-proof-that-bitcoin-sv-software-provides
///
/// # Arguments
///
/// * `merkle_proof` - Raw bytes of the merkle proof
pub fn parse_proof(merkle_proof: &[u8]) -> MerkleProof {
    let header = parser::parse_block_header(parser::header_from_bytes(&merkle_proof[0..80]));
    let mut transactions_count: [u8; 4] = Default::default();
    transactions_count.copy_from_slice(&merkle_proof[80..84]);
    let (bytes_consumed, hashes_count) = parser::parse_varint(&merkle_proof[84..87]);
    let mut current_index = bytes_consumed + 84;

    let mut hashes = Vec::new();
    for _ in 0..hashes_count {
        let raw_hash = &merkle_proof[current_index..current_index + 32];
        let raw_hash_be = bitcoin_spv::utils::reverse_endianness(raw_hash);
        let hash = H256::from_slice(&raw_hash_be);
        hashes.push(hash);
        current_index += 32;
    }

    let (bytes_consumed, flag_bits_count) =
        parser::parse_varint(&merkle_proof[current_index..current_index + 3]);
    current_index += bytes_consumed;

    let mut flag_bits = Vec::new();

    for i in 0..flag_bits_count {
        flag_bits.push(merkle_proof[current_index + i as usize]);
    }

    MerkleProof {
        block_header: header,
        transactions_count: u32::from_le_bytes(transactions_count),
        hashes: hashes,
        flag_bits: flag_bits,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use bitcoin_spv::utils::deserialize_hex;
    use primitive_types::H256;
    use std::str::FromStr;

    // curl -s -H 'content-type: application/json' http://satoshi.doc.ic.ac.uk:8332 -d '{
    //   "jsonrpc": "1.0",
    //   "id": "test",
    //   "method": "gettxoutproof",
    //   "params": [["3bdb32c04e10b6c399bd3657ef8b0300649189e90d7cb79c4f997dea8fb532cb"],
    //              "0000000000000000007962066dcd6675830883516bcf40047d42740a85eb2919"] }'
    // }'
    // block: https://www.blockchain.com/btc/block/0000000000000000007962066dcd6675830883516bcf40047d42740a85eb2919

    const PROOF_HEX: &str = "00000020ecf348128755dbeea5deb8eddf64566d9d4e59bc65d485000000000000000000901f0d92a66ee7dcefd02fa282ca63ce85288bab628253da31ef259b24abe8a0470a385a45960018e8d672f8a90a00000dcb32b58fea7d994f9cb77c0de989916400038bef5736bd99c3b6104ec032db3b29b1faeca50468e861cb635cb0e63edaac5d4568351cb4aeba5f04ce7b9347444069f77938daed9a1e177f6d77135b4d2d7db987f0293de3ca380811b88fac6eb448b42467764071a2e702107ef82259600f32e3a6e007f560a0242a5be2e4bf08429fb158a09b8d0ac301368f3d4aac125d1c0c0e378bc0a3f00b90d267a0d6833dcb15ae984f7ab69297af19558fd40aabe327d6d447090cec2c530469aa91319e0b532d22251c5814b7b962b3c437c9afe3943bf7bb03f5fcb95229d6676800dd4bdee0254c069ccb728d3516fe1e27578b31d70695e3e35483da448f3a951273e018de7f2a8f657064b013c6ede75c74bbd7f98fdae1c2ac6789ee7b21a791aa29d60e89fff2d1d2b1ada50aa9f59f403823c8c58bb092dc58dc09b28158ca15447da9c3bedb0b160f3fe1668d5a27716e27661bcb75ddbf3468f5c76b7bed1004c6b4df4da2ce80b831a7c260b515e6355e1c306373d2233e8de6fda3674ed95d17a01a1f64b27ba88c3676024fbf8d5dd962ffc4d5e9f3b1700763ab8804ff1f0000";

    #[test]
    fn test_parse_proof() {
        let raw_proof = deserialize_hex(&PROOF_HEX[..]).unwrap();
        let proof = parse_proof(&raw_proof);
        let expected_merkle =
            H256::from_str("a0e8ab249b25ef31da538262ab8b2885ce63ca82a22fd0efdce76ea6920d1f90")
                .unwrap();
        assert_eq!(proof.block_header.merkle_root, expected_merkle);
        assert_eq!(proof.transactions_count, 2729);
        assert_eq!(proof.hashes.len(), 13);
        // NOTE: following hash is in big endian
        let expected_hash =
            H256::from_str("3bdb32c04e10b6c399bd3657ef8b0300649189e90d7cb79c4f997dea8fb532cb")
                .unwrap();
        assert_eq!(proof.hashes[0], expected_hash);
        assert_eq!(proof.flag_bits.len(), 4);
    }
}
