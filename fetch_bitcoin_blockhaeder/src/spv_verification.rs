use rusqlite::{Connection, Result as RusqliteResult};
use crate::{types::BlockHeader, utils::get_env_var};
use std::time::Instant;
use reqwest::blocking::Client;
use crate::utils::{ get_block_hash, get_block_header};


// Function to get block header information from the local Bitcoin node
fn get_block_header_from_node(client: &Client, block_height: u64) -> BlockHeader {
    
    let block_hash = get_block_hash(&client, block_height, get_env_var("VERIFY_URL"), get_env_var("VERIFY_USERNAME"), get_env_var("VERIFY_PASSWORD"));
    let block_header = get_block_header(&client, &block_hash, get_env_var("VERIFY_URL"), get_env_var("VERIFY_USERNAME"), get_env_var("VERIFY_PASSWORD"));

    block_header
}

// Function to get block header information from the local database
pub fn get_block_header_by_height(conn: &Connection, height: u64) -> RusqliteResult<BlockHeader> {
    let mut stmt = conn.prepare("SELECT hash, height, version,  previousblockhash, nextblockhash, merkleroot, time, bits, nonce, difficulty FROM block_headers WHERE height = ?1")?;
    let block_header_iter = stmt.query_map([height], |row| {
        Ok(BlockHeader {
            hash: row.get(0)?,
            height: row.get(1)?,
            version: row.get(2)?,
            previousblockhash: row.get(3)?,
            nextblockhash: row.get(4)?,
            merkleroot: row.get(5)?,
            time: row.get(6)?,
            bits: row.get(7)?,
            nonce: row.get(8)?,
            difficulty: row.get(9)?,
        })
    })?;

    for block_header in block_header_iter {
        return block_header;
    }
    Err(rusqlite::Error::QueryReturnedNoRows)
}

// Function to perform SPV verification
pub fn perform_spv_verification(conn: &Connection) {
    // Configure connection to the local Bitcoin RPC node
    let client = Client::new();

    // Prepare and execute SQL statement to get block heights from the local database
    let mut stmt = conn.prepare("SELECT height FROM block_headers ORDER BY height").expect("Failed to prepare statement");
    let mut rows = stmt.query([]).expect("Failed to query rows");

    let start_time = Instant::now();
    let mut processed_blocks = 0;
    let mut total_blocks = 0;

    // Count total blocks
    while rows.next().expect("Failed to fetch row").is_some() {
        total_blocks += 1;
    }

    // Reset rows iterator
    let mut stmt = conn.prepare("SELECT height FROM block_headers ORDER BY height").expect("Failed to prepare statement");
    let mut rows = stmt.query([]).expect("Failed to query rows");

    // Iterate over block heights and compare local and remote block headers
    while let Some(row) = rows.next().expect("Failed to fetch row") {
        let height: u64 = row.get(0).expect("Failed to get height");
        let local_header = get_block_header_by_height(conn, height).expect("Failed to get block header");
        let remote_header: BlockHeader = get_block_header_from_node(&client, height);

        //   println!("remote_header {:?}", remote_header);
        // Compare local and remote block headers
        if let Some(difference) = compare_block_headers(&local_header, &remote_header) {
            println!("Mismatch found at block height {}: {}", height, difference);
            return;
        }

        processed_blocks += 1;

        // Display progress
        let elapsed_time = start_time.elapsed();
        let avg_time_per_block = elapsed_time / processed_blocks;
        let remaining_blocks = total_blocks - processed_blocks;
        let eta = avg_time_per_block * remaining_blocks;

        println!(
            "Processed block height: {} (Progress: {}/{}, ETA: {})",
            height, processed_blocks, total_blocks, format_duration(eta)
        );
    }

    println!("All local block headers match remote data~");
}

// Function to compare local and remote block headers and display the differing field
fn compare_block_headers(local: &BlockHeader, remote: &BlockHeader) -> Option<String> {
    if local.merkleroot != remote.merkleroot.to_string() {
        return Some(format!("Merkle root differs: local={}, remote={}", local.merkleroot, remote.merkleroot));
    }
    if local.previousblockhash !=  remote.previousblockhash {
        return Some(format!("Previous block hash differs: local={:?}, remote={:?}", local.previousblockhash, remote.previousblockhash));
    }
    if local.time != remote.time as u64 {
        return Some(format!("Time differs: local={}, remote={}", local.time, remote.time));
    }
    if local.bits !=  remote.bits {
        return Some(format!("Bits differ: local={}, remote={}", local.bits, remote.bits));
    }
    if local.nonce != remote.nonce {
        return Some(format!("Nonce differs: local={}, remote={}", local.nonce, remote.nonce));
    }
    if local.version != remote.version {
        return Some(format!("version differs: local={}, remote={}", local.nonce, remote.nonce));
    }
    None
}


// Function to format duration
fn format_duration(duration: std::time::Duration) -> String {
    let total_seconds = duration.as_secs();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}
