use bitcoin::types::{H256Le, RawBlockHeader};
use flate2::read::GzDecoder;
use serde::Deserialize;
use std::{
    fs,
    io::{BufReader, Read},
    path::PathBuf,
};

/// Bitcoin blocks and transaction from mainnet taken from blockstream.com
const PATH_MAINNET_BLOCKS_AND_TRANSACTIONS: &str = "./tests/data/bitcoin-testdata.gzip";

/// Bitcoin core fork testdata from testnet3: https://raw.githubusercontent.com/bitcoin/bitcoin/d6a59166a1879c1dd5b3a301847961f4b3f17742/test/functional/data/blockheader_testnet3.hex
/// The headers data is taken from testnet3 for early blocks from genesis until the first checkpoint. There are
/// two headers with valid POW at height 1 and 2, forking off from genesis. They are indicated by the FORK_PREFIX.
const PATH_TESTNET_FORKS: &str = "./tests/data/blockheader_testnet3.hex";
const FORK_PREFIX: &str = "fork:";
const NUM_FORK_HEADERS: u16 = 549;

const ERR_FILE_NOT_FOUND: &str = "Testdata not found. Please run the python script under the scripts folder to obtain bitcoin blocks and transactions.";
const ERR_JSON_FORMAT: &str = "JSON was not well-formatted";
const ERR_INVALID_HEADER: &str = "Invalid raw header";
const ERR_INVALID_PROOF: &str = "Invalid Merkle proof";

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
        RawBlockHeader::from_hex(&self.raw_header).expect(ERR_INVALID_HEADER)
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
        hex::decode(&self.raw_merkle_proof).expect(ERR_INVALID_PROOF)
    }
}

fn read_data(data: &str) -> String {
    let path_str = String::from(data);
    let path = PathBuf::from(&path_str);
    let abs_path = fs::canonicalize(&path).unwrap();
    let debug_help = abs_path.as_path().to_str().unwrap();

    let error_message = "\n".to_owned() + ERR_FILE_NOT_FOUND + "\n" + debug_help;

    match data {
        f if f.ends_with("gzip") => {
            let f = fs::File::open(&path_str).expect(&error_message);
            let b = BufReader::new(f);
            let mut d = GzDecoder::new(b);
            let mut s = String::new();
            d.read_to_string(&mut s).unwrap();
            s
        }
        _ => fs::read_to_string(&path_str).expect(&error_message),
    }
}

pub fn get_bitcoin_testdata() -> Vec<Block> {
    let data = read_data(PATH_MAINNET_BLOCKS_AND_TRANSACTIONS);

    let test_data: Vec<Block> = serde_json::from_str(&data).expect(ERR_JSON_FORMAT);

    assert!(test_data.windows(2).all(|b| b[0].height + 1 == b[1].height));

    test_data
}

pub fn get_fork_testdata() -> Vec<RawBlockHeader> {
    let data = read_data(PATH_TESTNET_FORKS);

    let test_data: Vec<RawBlockHeader> = data
        .lines()
        .filter_map(|s| RawBlockHeader::from_hex(s.strip_prefix(FORK_PREFIX).unwrap_or(s)).ok())
        .collect();

    assert!(test_data.len() == NUM_FORK_HEADERS as usize);

    test_data
}
