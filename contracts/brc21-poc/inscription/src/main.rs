use serde_json::Value;
use std::process::Command;

fn main() {
    // Create an Ord wallet
    println!("Creating wallet...");
    let output = Command::new("ord")
        .arg("--regtest")
        .arg("wallet")
        .arg("create")
        .output()
        .expect("Failed to execute command");
    println!("{}", String::from_utf8_lossy(&output.stdout));

    // Get an address to receive funds
    println!("Getting address...");
    let address = get_ord_ddress();
    println!("{}", address);

    // Mint funds to the address
    println!("Minting funds...");
    generate_to_address(&address, 10);

    // Inscribe a mint transaction for a BRC21
    println!("Inscribing mint transaction...");
    let fee_rate = "1.0"; // Replace with your desired fee rate
    let output = Command::new("ord")
        .arg("--regtest")
        .arg("wallet")
        .arg("inscribe")
        .arg("--fee-rate")
        .arg(fee_rate)
        .arg(token)
        .output()
        .expect("Failed to execute command");
    println!("{}", String::from_utf8_lossy(&output.stdout));
}