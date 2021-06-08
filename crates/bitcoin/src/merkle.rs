#[cfg(test)]
extern crate mocktopus;

#[cfg(test)]
use mocktopus::macros::mockable;

use crate::{
    parser::BytesParser,
    types::{BlockHeader, CompactUint, H256Le},
    utils::hash256_merkle_step,
    Error,
};
use sp_std::prelude::*;

// Values taken from https://github.com/bitcoin/bitcoin/blob/78dae8caccd82cfbfd76557f1fb7d7557c7b5edb/src/consensus/consensus.h
const MAX_BLOCK_WEIGHT: u32 = 4_000_000;
const WITNESS_SCALE_FACTOR: u32 = 4;
const MIN_TRANSACTION_WEIGHT: u32 = WITNESS_SCALE_FACTOR * 60;
const MAX_TRANSACTIONS_IN_PROOF: u32 = MAX_BLOCK_WEIGHT / MIN_TRANSACTION_WEIGHT;

/// Stores the content of a merkle tree
#[derive(Clone)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct MerkleTree;

/// Stores the content of a merkle proof
#[derive(Clone)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct MerkleProof {
    pub block_header: BlockHeader,
    pub flag_bits: Vec<bool>,
    pub transactions_count: u32,
    pub hashes: Vec<H256Le>,
}

struct MerkleProofTraversal {
    bits_used: usize,
    hashes_used: usize,
    merkle_position: Option<u32>,
    hash_position: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProofResult {
    pub extracted_root: H256Le,
    pub transaction_hash: H256Le,
    pub transaction_position: u32,
}

impl MerkleTree {
    pub fn compute_width(transactions_count: u32, height: u32) -> u32 {
        (transactions_count + (1 << height) - 1) >> height
    }

    pub fn compute_height(transactions_count: u32) -> u32 {
        let mut height = 0;
        while Self::compute_width(transactions_count, height) > 1 {
            height += 1;
        }
        height
    }

    pub fn compute_root(index: u32, height: u32, transactions_count: u32, hashes: &[H256Le]) -> Result<H256Le, Error> {
        if height == 0 {
            Ok(hashes[index as usize])
        } else {
            let left = Self::compute_root(
                index.checked_mul(2).ok_or(Error::ArithmeticOverflow)?,
                height.checked_sub(1).ok_or(Error::ArithmeticUnderflow)?,
                transactions_count,
                hashes,
            )?;
            let right_index = index
                .checked_mul(2)
                .ok_or(Error::ArithmeticOverflow)?
                .checked_add(1)
                .ok_or(Error::ArithmeticOverflow)?;
            let right = if right_index < Self::compute_width(transactions_count, height - 1) {
                Self::compute_root(
                    right_index,
                    height.checked_sub(1).ok_or(Error::ArithmeticUnderflow)?,
                    transactions_count,
                    hashes,
                )?
            } else {
                left
            };
            Ok(hash256_merkle_step(&left.to_bytes_le(), &right.to_bytes_le()))
        }
    }
}

#[cfg_attr(test, mockable)]
impl MerkleProof {
    /// Returns the width of the partial merkle tree
    pub fn compute_partial_tree_width(&self, height: u32) -> u32 {
        MerkleTree::compute_width(self.transactions_count, height)
    }

    /// Returns the height of the partial merkle tree
    pub fn compute_partial_tree_height(&self) -> u32 {
        MerkleTree::compute_height(self.transactions_count)
    }

    pub fn compute_merkle_root(&self, index: u32, height: u32, tx_ids: &[H256Le]) -> Result<H256Le, Error> {
        MerkleTree::compute_root(index, height, self.transactions_count, &tx_ids.to_vec())
    }

    /// Performs a depth-first traversal of the partial merkle tree
    /// and returns the computed merkle root
    fn traverse_and_extract(
        &self,
        height: u32,
        pos: u32,
        traversal: &mut MerkleProofTraversal,
    ) -> Result<H256Le, Error> {
        // this code is ported from the official Bitcoin client:
        // https://github.com/bitcoin/bitcoin/blob/99813a9745fe10a58bedd7a4cb721faf14f907a4/src/merkleblock.cpp
        let parent_of_hash = *self.flag_bits.get(traversal.bits_used).ok_or(Error::EndOfFile)?;
        traversal.bits_used = traversal.bits_used.checked_add(1).ok_or(Error::ArithmeticOverflow)?;

        if height == 0 || !parent_of_hash {
            if traversal.hashes_used >= self.hashes.len() {
                return Err(Error::MalformedMerkleProof);
            }
            let hash = self.hashes[traversal.hashes_used];
            if height == 0 && parent_of_hash {
                traversal.merkle_position = Some(pos);
                traversal.hash_position = Some(traversal.hashes_used);
            }
            traversal.hashes_used = traversal.hashes_used.checked_add(1).ok_or(Error::ArithmeticOverflow)?;
            return Ok(hash);
        }

        let next_height = height.checked_sub(1).ok_or(Error::ArithmeticUnderflow)?;
        let left_index = pos.checked_mul(2).ok_or(Error::ArithmeticOverflow)?;
        let right_index = left_index.checked_add(1).ok_or(Error::ArithmeticOverflow)?;

        let left = self.traverse_and_extract(next_height, left_index, traversal)?;
        let right = if right_index < self.compute_partial_tree_width(next_height) {
            self.traverse_and_extract(next_height, right_index, traversal)?
        } else {
            left
        };

        let hashed_bytes = hash256_merkle_step(&left.to_bytes_le(), &right.to_bytes_le());
        Ok(hashed_bytes)
    }

    /// Computes the merkle root of the proof partial merkle tree
    pub fn verify_proof(&self) -> Result<ProofResult, Error> {
        let mut traversal = MerkleProofTraversal {
            bits_used: 0,
            hashes_used: 0,
            merkle_position: None,
            hash_position: None,
        };

        // fail if no transactions
        if self.transactions_count == 0 {
            return Err(Error::MalformedMerkleProof);
        }

        // fail if too many transactions
        if self.transactions_count > MAX_TRANSACTIONS_IN_PROOF {
            return Err(Error::MalformedMerkleProof);
        }

        // fail if not at least one bit per hash
        if self.flag_bits.len() < self.hashes.len() {
            return Err(Error::MalformedMerkleProof);
        }

        let root = self.traverse_and_extract(self.compute_partial_tree_height(), 0, &mut traversal)?;
        let merkle_position = traversal.merkle_position.ok_or(Error::InvalidMerkleProof)?;
        let hash_position = traversal.hash_position.ok_or(Error::InvalidMerkleProof)?;

        // fail if all hashes are not used
        if traversal.hashes_used != self.hashes.len() {
            return Err(Error::MalformedMerkleProof);
        }

        // fail if all bits are not used
        if traversal
            .bits_used
            .checked_add(7)
            .ok_or(Error::ArithmeticOverflow)?
            .checked_div(8)
            .ok_or(Error::ArithmeticUnderflow)?
            != self
                .flag_bits
                .len()
                .checked_add(7)
                .ok_or(Error::ArithmeticOverflow)?
                .checked_div(8)
                .ok_or(Error::ArithmeticUnderflow)?
        {
            return Err(Error::MalformedMerkleProof);
        }

        Ok(ProofResult {
            extracted_root: root,
            transaction_hash: self.hashes[hash_position],
            transaction_position: merkle_position,
        })
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
    /// See: <https://bitqa.app/questions/how-to-decode-merkle-transaction-proof-that-bitcoin-sv-software-provides>
    ///
    /// # Arguments
    ///
    /// * `merkle_proof` - Raw bytes of the merkle proof
    pub fn parse(merkle_proof: &[u8]) -> Result<MerkleProof, Error> {
        let mut proof_parser = BytesParser::new(merkle_proof);
        let block_header = proof_parser.parse()?;
        let transactions_count = proof_parser.parse()?;

        let hashes_count: CompactUint = proof_parser.parse()?;
        let mut hashes = Vec::<H256Le>::new();
        for _ in 0..hashes_count.value {
            hashes.push(proof_parser.parse()?);
        }

        let flag_bits_count: CompactUint = proof_parser.parse()?;
        let mut flag_bits = Vec::new();
        for _ in 0..flag_bits_count.value {
            flag_bits.extend(proof_parser.parse::<Vec<bool>>()?);
        }

        Ok(MerkleProof {
            block_header,
            flag_bits,
            transactions_count,
            hashes,
        })
    }

    pub(crate) fn traverse_and_build(
        &mut self,
        height: u32,
        pos: u32,
        tx_ids: &[H256Le],
        matches: &[bool],
    ) -> Result<(), Error> {
        let mut parent_of_match = false;
        let mut p = pos << height;
        while p < (pos + 1) << height && p < self.transactions_count {
            parent_of_match |= matches[p as usize];
            p += 1;
        }

        self.flag_bits.push(parent_of_match);

        if height == 0 || !parent_of_match {
            let hash = self.compute_merkle_root(pos, height, tx_ids)?;
            self.hashes.push(hash);
        } else {
            let next_height = height.checked_sub(1).ok_or(Error::ArithmeticUnderflow)?;
            let left_index = pos.checked_mul(2).ok_or(Error::ArithmeticOverflow)?;
            let right_index = left_index.checked_add(1).ok_or(Error::ArithmeticOverflow)?;

            self.traverse_and_build(next_height, left_index, tx_ids, matches)?;
            if right_index < self.compute_partial_tree_width(next_height) {
                self.traverse_and_build(next_height, right_index, tx_ids, matches)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use mocktopus::mocking::*;
    use sp_core::H256;
    use sp_std::str::FromStr;

    // curl -s -H 'content-type: application/json' http://satoshi.doc.ic.ac.uk:8332 -d '{
    //   "jsonrpc": "1.0",
    //   "id": "test",
    //   "method": "gettxoutproof",
    //   "params": [["61a05151711e4716f31f7a3bb956d1b030c4d92093b843fa2e771b95564f0704"],
    //              "0000000000000000007962066dcd6675830883516bcf40047d42740a85eb2919"]
    // }'
    // block: https://www.blockchain.com/btc/block/0000000000000000007962066dcd6675830883516bcf40047d42740a85eb2919

    const PROOF_HEX: &str = "00000020ecf348128755dbeea5deb8eddf64566d9d4e59bc65d485000000000000000000901f0d92a66ee7dcefd02fa282ca63ce85288bab628253da31ef259b24abe8a0470a385a45960018e8d672f8a90a00000d0bdabada1fb6e3cef7f5c6e234621e3230a2f54efc1cba0b16375d9980ecbc023cbef3ba8d8632ea220927ec8f95190b30769eb35d87618f210382c9445f192504074f56951b772efa43b89320d9c430b0d156b93b7a1ff316471e715151a0619a39392657f25289eb713168818bd5b37476f1bc59b166deaa736d8a58756f9d7ce2aef46d8004c5fe3293d883838f87b5f1da03839878895b71530e9ff89338bb6d4578b3c3135ff3e8671f9a64d43b22e14c2893e8271cecd420f11d2359307403bb1f3128885b3912336045269ef909d64576b93e816fa522c8c027fe408700dd4bdee0254c069ccb728d3516fe1e27578b31d70695e3e35483da448f3a951273e018de7f2a8f657064b013c6ede75c74bbd7f98fdae1c2ac6789ee7b21a791aa29d60e89fff2d1d2b1ada50aa9f59f403823c8c58bb092dc58dc09b28158ca15447da9c3bedb0b160f3fe1668d5a27716e27661bcb75ddbf3468f5c76b7bed1004c6b4df4da2ce80b831a7c260b515e6355e1c306373d2233e8de6fda3674ed95d17a01a1f64b27ba88c3676024fbf8d5dd962ffc4d5e9f3b1700763ab88047f7d0000";

    fn sample_valid_proof_result() -> ProofResult {
        let tx_id = H256Le::from_bytes_le(
            &hex::decode("c8589f304d3b9df1d4d8b3d15eb6edaaa2af9d796e9d9ace12b31f293705c5e9".to_owned()).unwrap(),
        );
        let merkle_root = H256Le::from_bytes_le(
            &hex::decode("90d079ef103a8b7d3d9315126468f78b456690ba6628d1dcd5a16c9990fbe11e".to_owned()).unwrap(),
        );
        ProofResult {
            extracted_root: merkle_root,
            transaction_hash: tx_id,
            transaction_position: 0,
        }
    }

    #[test]
    fn test_mock_verify_proof() {
        let mock_proof_result = sample_valid_proof_result();

        let proof = MerkleProof::parse(&hex::decode(PROOF_HEX).unwrap()).unwrap();
        MerkleProof::verify_proof.mock_safe(move |_| MockResult::Return(Ok(mock_proof_result)));

        let res = MerkleProof::verify_proof(&proof).unwrap();
        assert_eq!(res, mock_proof_result);
    }

    #[test]
    fn test_parse_proof() {
        let raw_proof = hex::decode(PROOF_HEX).unwrap();
        let proof = MerkleProof::parse(&raw_proof).unwrap();
        let expected_merkle_root =
            H256::from_str("a0e8ab249b25ef31da538262ab8b2885ce63ca82a22fd0efdce76ea6920d1f90").unwrap();
        assert_eq!(proof.block_header.merkle_root, expected_merkle_root);
        assert_eq!(proof.transactions_count, 2729);
        assert_eq!(proof.hashes.len(), 13);
        // NOTE: following hash is in big endian
        let expected_hash = H256Le::from_hex_be("02bcec80995d37160bba1cfc4ef5a230321e6234e2c6f5f7cee3b61fdabada0b");
        assert_eq!(proof.hashes[0], expected_hash);
        assert_eq!(proof.flag_bits.len(), 4 * 8);
    }

    #[test]
    fn test_parse_proof_testnet() {
        let raw_proof = hex::decode("00000020b0b3d77b97015b519553423c96642b33ca534c50ecefd133640000000000000029a0a725684aeca24af83e3ba0a3e3ee56adfdf032d19e5acba6d0a262e1580ca354915fd4c8001ac42a7b3a1000000005df41db041b26536b5b7fd7aeea4ea6bdb64f7039e4a566b1fa138a07ed2d3705932955c94ee4755abec003054128b10e0fbcf8dedbbc6236e23286843f1f82a018dc7f5f6fba31aa618fab4acad7df5a5046b6383595798758d30d68c731a14043a50d7cb8560d771fad70c5e52f6d7df26df13ca457655afca2cbab2e3b135c0383525b28fca31296c809641205962eb353fb88a9f3602e98a93b1e9ffd469b023d00").unwrap();
        let proof = MerkleProof::parse(&raw_proof).unwrap();
        let expected_block_header =
            H256Le::from_hex_be("000000000000002e59ed7b899b3f0f83c48d0548309a8fb7693297e3937fe1d3");

        assert_eq!(proof.block_header.hash, expected_block_header);
    }

    #[test]
    fn test_compute_tree_width() {
        let proof = MerkleProof::parse(&hex::decode(PROOF_HEX).unwrap()).unwrap();
        assert_eq!(proof.compute_partial_tree_width(0), proof.transactions_count);
        assert_eq!(proof.compute_partial_tree_width(1), proof.transactions_count / 2 + 1);
        assert_eq!(proof.compute_partial_tree_width(12), 1);
    }

    #[test]
    fn test_compute_merkle_proof_tree_height() {
        let proof = MerkleProof::parse(&hex::decode(PROOF_HEX).unwrap()).unwrap();
        assert_eq!(proof.compute_partial_tree_height(), 12);
    }

    #[test]
    fn test_extract_hash() {
        let proof = MerkleProof::parse(&hex::decode(PROOF_HEX).unwrap()).unwrap();
        let merkle_root = H256Le::from_bytes_le(&proof.block_header.merkle_root.to_bytes_le());
        let result = proof.verify_proof().unwrap();
        assert_eq!(result.extracted_root, merkle_root);
        assert_eq!(result.transaction_position, 48);
        let expected_tx_hash = H256Le::from_hex_be("61a05151711e4716f31f7a3bb956d1b030c4d92093b843fa2e771b95564f0704");
        assert_eq!(result.transaction_hash, expected_tx_hash);
    }

    #[test]
    fn test_parse_regtest_merkle_proof_succeeds() {
        let raw_merkle_proof_hex = "0000002031a3479e5062e200279af822d816d02cab347bc3719726541c4fd5edfc3ffd7d680b2710119c752e5fb1b963ad2ee3539f6b3fe0e9b054e681734b631e92c2faf449ca5fffff7f20000000000300000003f0d6a860c811b45bbbe4f0401f26e2fafc40e50bb03782025c0ef82768703d3de263ed560faac245c73725f295eb653268bca3387f9e03b18ca6ab242ce6c54b5625d63322e74c0aa94c794cbf065858bddc5b8ea178fbb0549956149a7d4686010b";
        let raw_merkle_proof = hex::decode(&raw_merkle_proof_hex).unwrap();
        MerkleProof::parse(&raw_merkle_proof).unwrap();
    }
}
