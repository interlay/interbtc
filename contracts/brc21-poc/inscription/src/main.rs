mod rpc;
mod brc21;

use rpc::{
    ord_wallet_create,
    ord_wallet_receive,
    ord_wallet_inscribe,
    ord_wallet_send,
    cli_generate_to_address
};
use rpc::Inscription;
use brc21::Brc21;

const TOKEN: &str = "INTR";
const SOURCE: &str = "INTERLAY";


fn main() {
    // Create an Ord wallet
    println!("Creating wallet...");
    ord_wallet_create();

    // Get an address to receive funds
    println!("Getting address...");
    let address = ord_wallet_receive();
    println!("{}", address);

    // Mint funds to the address
    println!("Minting funds...");
    cli_generate_to_address(&address, 101);

    // Inscribe a mint transaction for a BRC21
    println!("Inscribing mint transaction...");
    let brc21 = Brc21::new_mint(
        TOKEN,
        200,
        SOURCE
    );
    let inscription = ord_wallet_inscribe(brc21);
    println!("{}", inscription.to_string());
    // Genreate blocks to complete mint
    cli_generate_to_address(&address, 5);
}