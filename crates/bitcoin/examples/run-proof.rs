extern crate bitcoin;

use bitcoin::merkle::MerkleProof;

// Proving that the transaction
// 8d30eb0f3e65b8d8a9f26f6f73fc5aafa5c0372f9bb38aa38dd4c9dd1933e090
// is included in block 0000000000000a3290f20e75860d505ce0e948a1d1d846bec7e39015d242884b
// https://www.blockchain.com/btc/block/150000
// curl -s -d '{"jsonrpc": "1.0", "id":"test", "method": "gettxoutproof", "params": [["8d30eb0f3e65b8d8a9f26f6f73fc5aafa5c0372f9bb38aa38dd4c9dd1933e090"], "0000000000000a3290f20e75860d505ce0e948a1d1d846bec7e39015d242884b"] }' http://satoshi.doc.ic.ac.uk:8332

const PROOF_HEX: &str = "010000006fd2c5a8fac33dbe89bb2a2947a73eed2afc3b1d4f886942df08000000000000b152eca4364850f3424c7ac2b337d606c5ca0a3f96f1554f8db33d2f6f130bbed325a04e4b6d0b1a85790e6b0a000000038d9d737b484e96eed701c4b3728aea80aa7f2a7f57125790ed9998f9050a1bef90e03319ddc9d48da38ab39b2f37c0a5af5afc736f6ff2a9d8b8653e0feb308d84251842a4c0f0e188e1c2bf643ec37a1402dd86a25a9ab5004633467d16e313013d";

fn main() {
    let raw_proof = hex::decode(PROOF_HEX).unwrap();
    let proof = MerkleProof::parse(&raw_proof).unwrap();
    let result = proof.verify_proof().unwrap();
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
