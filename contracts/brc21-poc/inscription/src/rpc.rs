use std::process::Command;

const FEE_RATE : &str = "1";

struct Inscription {
    commit: String,
    inscription: String,
    reveal: String,
    fees: u128,
}

fn ord_wallet_receive() -> String {
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
fn ord_wallet_inscribe(brc21: Brc21) -> Inscription {
    // Construct the filename as a temporary file
    let filename = format!("/tmp/{}.json", Uuid::new_v4().to_string());
    brc21.write_to_file(filename);
    let output = Command::new("ord")
        .arg("--regtest")
        .arg("wallet")
        .arg("inscribe")
        .arg("--fee-rate")
        .arg(FEE_RATE)
        .arg(filename)
        .output()
        .expect("Failed to execute command");
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout)).unwrap();
    Inscription {
        commit: json["commit"].as_str().unwrap().to_string(),
        inscription: json["inscription"].as_str().unwrap().to_string(),
        reveal: json["reveal"].as_str().unwrap().to_string(),
        fees: json["fees"].as_u64().unwrap() as u128,
    }
}

/// Transfer the inscribed transfer to another account
fn ord_wallet_send(address: &str, inscription_id: &str) -> String {
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

fn cli_generate_to_address(address: &str, num_blocks: u32) {
    let output = Command::new("bitcoin-cli")
        .arg("-regtest")
        .arg("generatetoaddress")
        .arg(num_blocks.to_string())
        .arg(address)
        .output()
        .expect("Failed to execute command");
    println!("{}", String::from_utf8_lossy(&output.stdout));
}