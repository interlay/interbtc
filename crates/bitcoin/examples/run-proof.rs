extern crate bitcoin;

use bitcoin::{
    merkle::{MerkleProof, PartialTransactionProof},
    parser::parse_transaction,
};

// Proving that the transaction
// 8d30eb0f3e65b8d8a9f26f6f73fc5aafa5c0372f9bb38aa38dd4c9dd1933e090
// is included in block 0000000000000a3290f20e75860d505ce0e948a1d1d846bec7e39015d242884b
// https://www.blockchain.com/btc/block/150000
// curl -s -d '{"jsonrpc": "1.0", "id":"test", "method": "gettxoutproof", "params": [["8d30eb0f3e65b8d8a9f26f6f73fc5aafa5c0372f9bb38aa38dd4c9dd1933e090"], "0000000000000a3290f20e75860d505ce0e948a1d1d846bec7e39015d242884b"] }' http://satoshi.doc.ic.ac.uk:8332

const PROOF_HEX: &str = "010000006fd2c5a8fac33dbe89bb2a2947a73eed2afc3b1d4f886942df08000000000000b152eca4364850f3424c7ac2b337d606c5ca0a3f96f1554f8db33d2f6f130bbed325a04e4b6d0b1a85790e6b0a000000038d9d737b484e96eed701c4b3728aea80aa7f2a7f57125790ed9998f9050a1bef90e03319ddc9d48da38ab39b2f37c0a5af5afc736f6ff2a9d8b8653e0feb308d84251842a4c0f0e188e1c2bf643ec37a1402dd86a25a9ab5004633467d16e313013d";

fn main() {
    let raw_proof = hex::decode(PROOF_HEX).unwrap();
    let proof = MerkleProof::parse(&raw_proof).unwrap();
    let tx_hex = "010000000168a59c95a89ed5e9af00e90a7823156b02b7811000c63170bb2440d8db6a1869000000008a473044022050c32cf6cd888178268701a636b189dc3f026ee3ebd230fd77018e54044aac77022055aa7fa73c524dd4f0be02694683a21eb03d5d2f2c519d7dc7110b742c417517014104aa5c77986a87b93b03d949013e629601b6dbdbd5fc09f3bef9263b64b3c38d79d443fafa2fbf422a203fe433adf6e071f3172a53747739ce72c640fe7e514981ffffffff0140420f00000000001976a91449cf380abdb86449efc694988bf0f447739f73cd88ac00000000";
    let raw_tx = hex::decode(tx_hex).unwrap();
    let transaction = parse_transaction(&raw_tx).unwrap();

    let unchecked_proof = PartialTransactionProof {
        transaction,
        tx_encoded_len: raw_tx.len() as u32,
        merkle_proof: proof.clone(),
    };

    let result = unchecked_proof.verify_proof().unwrap();
    println!(
        "proof: transactions count = {}, hash count = {}, tree height = {},\nmerkle root = {:?}, hashes count = {}, flags={:?},\ncomputed merkle root = {}, position = {}",
        proof.transactions_count,
        proof.hashes.len(),
        proof.compute_partial_tree_height(),
        proof.block_header.merkle_root,
        proof.hashes.len(),
        proof.flag_bits,
        result.extracted_root,
        result.transaction_position
    );
}
