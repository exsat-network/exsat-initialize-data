mod block_header;
mod spv_verification;

use block_header::{index_block_headers, create_db_connection};
use spv_verification::verify_transaction_spv;
use std::io;

const MAX_BLOCK_HEIGHT: u32 = 840000;

fn main() {
    println!("Select an option:");
    println!("1. Index block headers");
    println!("2. Perform SPV verification");
    
    let mut choice = String::new();
    io::stdin().read_line(&mut choice).expect("Failed to read line");
    let choice = choice.trim().parse::<u32>().expect("Please enter a number");

    match choice {
        1 => {
            let conn = create_db_connection("block_headers.db").expect("Failed to create DB connection");
            index_block_headers(&conn, MAX_BLOCK_HEIGHT).expect("Failed to index block headers");
        },
        2 => {
            let conn = create_db_connection("block_headers.db").expect("Failed to create DB connection");
            // Example usage: replace these with actual values
            let txid = "0f9188f13cb7b2c71f2a335e3a4fc328bf5beb436012afca590b1a11466e2206";
            let block_height = 0;
            let merkle_block_data: Vec<u8> = vec![]; // Fill this with your Merkle block data
            let merkle_block: bitcoin::util::merkleblock::MerkleBlock = bitcoin::consensus::encode::deserialize(&merkle_block_data).expect("Failed to deserialize Merkle block");
            let is_valid = verify_transaction_spv(&conn, txid, block_height, &merkle_block);
            println!("Transaction SPV verification result: {}", is_valid);
        },
        _ => println!("Invalid choice"),
    }
}
