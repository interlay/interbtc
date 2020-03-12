extern crate bitcoin;

use bitcoin::merkle::MerkleProof;
use bitcoin_spv::utils::deserialize_hex;

const PROOF_HEX: &str = "010000006fd2c5a8fac33dbe89bb2a2947a73eed2afc3b1d4f886942df08000000000000b152eca4364850f3424c7ac2b337d606c5ca0a3f96f1554f8db33d2f6f130bbed325a04e4b6d0b1a85790e6b0a000000038d9d737b484e96eed701c4b3728aea80aa7f2a7f57125790ed9998f9050a1bef90e03319ddc9d48da38ab39b2f37c0a5af5afc736f6ff2a9d8b8653e0feb308d84251842a4c0f0e188e1c2bf643ec37a1402dd86a25a9ab5004633467d16e313013d";

fn main() {
    let raw_proof = deserialize_hex(&PROOF_HEX[..]).unwrap();
    let proof = MerkleProof::parse(&raw_proof);
    let computed_merkle_root = proof.compute_merkle_root();
    println!(
        "proof: tx count = {}, hash count = {}, tree height = {}, merkle root = {:?}, hashes count = {}, flags={:?}, computed merkle root = {}",
        proof.transactions_count,
        proof.hashes.len(),
        proof.compute_tree_height(),
        proof.block_header.merkle_root,
        proof.hashes.len(),
        proof.flag_bits,
        computed_merkle_root
    );
}
