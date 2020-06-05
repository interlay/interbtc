use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

const ERR_FILE_NOT_FOUND: &'static str = "Testdata not found. Please run the python script under the parachain/scripts folder to obtain bitcoin blocks and transactions.";
const ERR_JSON_FORMAT: &'static str = "JSON was not well-formatted";

#[derive(Clone, Debug, Deserialize)]
pub struct Block {
    pub height: u32,
    pub hash: String,
    pub raw_header: String,
    pub test_txs: Vec<Transaction>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Transaction {
    pub txid: String,
    pub raw_merkle_proof: String,
}

pub fn get_bitcoin_testdata() -> Vec<Block> {
    let path_str = String::from("./tests/data/bitcoin-testdata.json");
    let path = PathBuf::from(&path_str);
    let abs_path = fs::canonicalize(&path).unwrap();
    let debug_help = abs_path.as_path().to_str().unwrap();

    let error_message = "\n".to_owned() + ERR_FILE_NOT_FOUND + "\n" + debug_help;

    let data = fs::read_to_string(&path_str).expect(&error_message);

    let test_data: Vec<Block> = serde_json::from_str(&data).expect(ERR_JSON_FORMAT);

    test_data
}
