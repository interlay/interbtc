use std::process::Command;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::brc21::Brc21;

const FEE_RATE : &str = "1";

#[derive(Serialize, Deserialize)]
pub struct Inscription {
    commit: String,
    inscription: String,
    reveal: String,
    fees: u128,
}

impl Inscription {
    pub fn to_string(&self) -> String {
        format!(
            "Commit: {}\nInscription: {}\nReveal: {}\nFees: {}",
            self.commit,
            self.inscription,
            self.reveal,
            self.fees
        )
    }
}

pub fn ord_wallet_create() {
    let output = Command::new("ord")
        .arg("--regtest")
        .arg("wallet")
        .arg("create")
        .output()
        .expect("Failed to execute command");
    println!("{}", String::from_utf8_lossy(&output.stdout));
}

pub fn ord_wallet_receive() -> String {
    let output = Command::new("ord")
        .arg("--regtest")
        .arg("wallet")
        .arg("receive")
        .output()
        .expect("Failed to execute command");
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout)).unwrap();
    json["address"].as_str().unwrap().to_string()
}

/// Inscribe either a mint, transfer, or a redeem transaction
pub fn ord_wallet_inscribe(brc21: Brc21) -> Inscription {
    // Construct the filename as a temporary file
    let file = format!("/tmp/{}.json", Uuid::new_v4());
    brc21.write_to_file(&file);
    let output = Command::new("ord")
        .arg("--regtest")
        .arg("wallet")
        .arg("inscribe")
        .arg("--fee-rate")
        .arg(FEE_RATE)
        .arg(file)
        .output()
        .expect("Failed to execute command");
    println!("{}", String::from_utf8_lossy(&output.stderr));
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout)).unwrap();
    Inscription {
        commit: json["commit"].as_str().unwrap().to_string(),
        inscription: json["inscription"].as_str().unwrap().to_string(),
        reveal: json["reveal"].as_str().unwrap().to_string(),
        fees: json["fees"].as_u64().unwrap() as u128,
    }
}

/// Transfer the inscribed transfer to another account
pub fn ord_wallet_send(address: &str, inscription_id: &str) -> String {
    let output = Command::new("ord")
        .arg("--regtest")
        .arg("wallet")
        .arg("send")
        .arg("--fee-rate")
        .arg(FEE_RATE)
        .arg(address)
        .arg(inscription_id)
        .output()
        .expect("Failed to execute command");
    String::from_utf8_lossy(&output.stdout).to_string()
}

pub fn cli_generate_to_address(address: &str, num_blocks: u32) {
    let output = Command::new("bitcoin-cli")
        .arg("-regtest")
        .arg("generatetoaddress")
        .arg(num_blocks.to_string())
        .arg(address)
        .output()
        .expect("Failed to execute command");
    println!("{}", String::from_utf8_lossy(&output.stdout));
}