extern crate bitcoin;
extern crate hex;

const RAW_TRANSACTION: &str = "020000000001010000000000000000000000000000000000000000000000000000000000000000ffffffff0502cb000101ffffffff02400606950000000017a91466c7060feb882664ae62ffad0051fe843e318e85870000000000000000266a24aa21a9ede5c17d15b8b1fa2811b7e6da66ffa5e1aaa05922c69068bf90cd585b95bb46750120000000000000000000000000000000000000000000000000000000000000000000000000";

use bitcoin::parser::parse_transaction;

fn main() {
    let raw_tx = hex::decode(RAW_TRANSACTION).unwrap();
    let tx = parse_transaction(&raw_tx).unwrap();
    println!("ouptut 1 script {}", tx.outputs[0].script.as_hex());
    println!("ouptut 2 script {}", tx.outputs[1].script.as_hex());
}
