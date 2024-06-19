mod block_header;
mod spv_verification;
mod utils;
mod types;

use block_header::index_block_headers;
use spv_verification::perform_spv_verification;
use utils::create_db_connection;
use std::io;


const MAX_BLOCK_HEIGHT: u32 = 840000;

fn main() {
    let conn: rusqlite::Connection = create_db_connection("block_headers.db").expect("Failed to create DB connection");

    println!("Select an option:");
    println!("1. Index block headers");
    println!("2. Perform SPV verification");
    
    let mut choice = String::new();
    io::stdin().read_line(&mut choice).expect("Failed to read line");
    let choice = choice.trim().parse::<u32>().expect("Please enter a number");

    match choice {
        1 => {
            index_block_headers(&conn, MAX_BLOCK_HEIGHT).expect("Failed to index block headers");
        },
        2 => {
            perform_spv_verification(&conn);
        },
        _ => println!("Invalid choice"),
    }
}
