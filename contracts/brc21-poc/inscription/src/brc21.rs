use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};

const PROTOCOL: &str = "brc-21";

#[derive(Serialize, Deserialize)]
enum Operation {
    Mint,
    Transfer,
    Redeem,
}

#[derive(Serialize, Deserialize)]
pub struct Brc21 {
    protocol: String,
    operation: Operation,
    ticker: String,
    amount: u128,
    source: Option<String>,
    dest: Option<String>,
    acc: Option<String>,
}

impl Brc21 {
    pub fn new_mint(ticker: &str, amount: u128, source: &str) -> Self {
        Brc21 {
            protocol: PROTOCOL.to_string(),
            operation: Operation::Mint,
            ticker: ticker.to_string(),
            amount,
            source: Some(source.to_string()),
            dest: None,
            acc: None,
        }
    }

    pub fn new_transfer(ticker: &str, amount: u128) -> Self {
        Brc21 {
            protocol: PROTOCOL.to_string(),
            operation: Operation::Transfer,
            ticker: ticker.to_string(),
            amount,
            source: None,
            dest: None,
            acc: None,
        }
    }

    pub fn new_redeem(ticker: &str, amount: u128, dest: &str, acc: &str) -> Self {
        Brc21 {
            protocol: PROTOCOL.to_string(),
            operation: Operation::Redeem,
            ticker: ticker.to_string(),
            amount,
            source: None,
            dest: Some(dest.to_string()),
            acc: Some(acc.to_string()),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn from_json(json: &str) -> Self {
        serde_json::from_str(json).unwrap()
    }

    pub fn write_to_file(&self, filename: &str) {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(filename)
            .unwrap();
        let json = self.to_json();
        file.write_all(json.as_bytes()).unwrap();
    }

    pub fn read_from_file(filename: &str) -> Self {
        let mut file = File::open(filename).unwrap();
        let mut json = String::new();
        file.read_to_string(&mut json).unwrap();
        Self::from_json(&json)
    }
}