use bitcoin::types::{H256Le, RawBlockHeader};
use serde::Deserialize;
use std::{fs, path::PathBuf};

const ERR_FILE_NOT_FOUND: &str = "Testdata not found. Please run the python script under the parachain/scripts folder to obtain bitcoin blocks and transactions.";
const ERR_JSON_FORMAT: &str = "JSON was not well-formatted";

#[derive(Clone, Debug, Deserialize)]
pub struct Block {
    pub height: u32,
    hash: String,
    raw_header: String,
    pub test_txs: Vec<Transaction>,
}

impl Block {
    pub fn get_block_hash(&self) -> H256Le {
        H256Le::from_hex_be(&self.hash)
    }

    pub fn get_raw_header(&self) -> RawBlockHeader {
        RawBlockHeader::from_hex(&self.raw_header).expect("invalid raw header")
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Transaction {
    txid: String,
    raw_merkle_proof: String,
}

impl Transaction {
    pub fn get_txid(&self) -> H256Le {
        H256Le::from_hex_be(&self.txid)
    }

    pub fn get_raw_merkle_proof(&self) -> Vec<u8> {
        hex::decode(&self.raw_merkle_proof).expect("Error parsing merkle proof")
    }
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
