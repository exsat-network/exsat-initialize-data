use reqwest::blocking::Client;
use rusqlite::{Connection, Result};
use std::thread;
use std::time::{Duration, Instant};
use crate::utils::{get_last_indexed_height, get_block_hash, get_block_header, save_block_header};



pub fn index_block_headers(conn: &Connection, max_block_height: u32) -> Result<()> {
    let client = Client::new();
    let mut current_height = get_last_indexed_height(conn)?;
    let mut current_block_hash = if current_height > 0 {
        get_block_hash(&client, current_height)
    } else {
        get_block_hash(&client, 0)  // Start from the genesis block
    };

    let start_time = Instant::now();

    while current_height <= max_block_height {
        let block_header = get_block_header(&client, &current_block_hash);
        save_block_header(conn, &block_header)?;

        if let Some(nextblockhash) = block_header.nextblockhash.clone() {
            current_block_hash = nextblockhash;
            current_height += 1;  // Increment height based on successful retrieval
        } else {
            break;  // Exit if no next block
        }

        let elapsed_time = start_time.elapsed();
        let eta = calculate_eta(elapsed_time, current_height, max_block_height);
        println!("Processed block height: {} (ETA: {})", current_height, eta);

        thread::sleep(Duration::from_millis(100));  // Throttle requests
    }

    println!("Block headers saved to database.");
    Ok(())
}

fn calculate_eta(elapsed_time: Duration, current_height: u32, max_block_height: u32) -> String {
    let total_blocks = max_block_height - current_height;
    let avg_time_per_block = elapsed_time / current_height as u32;
    let eta = avg_time_per_block * total_blocks as u32;
    format_duration(eta)
}

fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.as_secs();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}
