use rusqlite::{Connection, Result as RusqliteResult};
use bitcoin::blockdata::block::BlockHeader as BitcoinBlockHeader;
use crate::block_header::BlockHeader;
use std::time::Instant;
use bitcoincore_rpc::{Auth, Client, RpcApi};

// Define RPC connection details
const RPC_URL: &str = "http://regtest.exactsat.io:18443/";
const RPC_USER: &str = "test";
const RPC_PASSWORD: &str = "test";

// Function to get block header information from the local Bitcoin node
fn get_block_header_from_node(client: &Client, block_height: u64) -> BitcoinBlockHeader {
    let block_hash = client.get_block_hash(block_height).expect("Failed to get block hash");
    let block_header = client.get_block_header(&block_hash).expect("Failed to get block header");

    BitcoinBlockHeader {
        version: block_header.version,
        prev_blockhash: block_header.prev_blockhash,
        merkle_root: block_header.merkle_root,
        time: block_header.time,
        bits: block_header.bits,
        nonce: block_header.nonce,
    }
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
    let client = Client::new(RPC_URL, Auth::UserPass(RPC_USER.to_string(), RPC_PASSWORD.to_string())).expect("Failed to create RPC client");

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
        let remote_header = get_block_header_from_node(&client, height);

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
fn compare_block_headers(local: &BlockHeader, remote: &BitcoinBlockHeader) -> Option<String> {
    if local.merkleroot != remote.merkle_root.to_string() {
        return Some(format!("Merkle root differs: local={}, remote={}", local.merkleroot, remote.merkle_root));
    }
    if !compare_previous_block_hash(local.previousblockhash.as_deref(), Some(&remote.prev_blockhash.to_string())) {
        return Some(format!("Previous block hash differs: local={:?}, remote={}", local.previousblockhash, remote.prev_blockhash));
    }
    if local.time != remote.time as u64 {
        return Some(format!("Time differs: local={}, remote={}", local.time, remote.time));
    }
    if local.bits != format!("{:x}", remote.bits) {
        return Some(format!("Bits differ: local={}, remote={:x}", local.bits, remote.bits));
    }
    if local.nonce != remote.nonce {
        return Some(format!("Nonce differs: local={}, remote={}", local.nonce, remote.nonce));
    }
    if local.version != remote.version {
        return Some(format!("version differs: local={}, remote={}", local.nonce, remote.nonce));
    }
    None
}

// Function to compare previous block hashes, treating None and all-zero hash as equivalent
fn compare_previous_block_hash(local: Option<&str>, remote: Option<&str>) -> bool {
    let all_zero_hash = "0000000000000000000000000000000000000000000000000000000000000000";
    match (local, remote) {
        (None, Some(r)) if r == all_zero_hash => true,
        (Some(l), Some(r)) if l == r => true,
        _ => false,
    }
}

// Function to format duration
fn format_duration(duration: std::time::Duration) -> String {
    let total_seconds = duration.as_secs();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}
