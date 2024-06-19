use reqwest::blocking::Client;
use serde::Deserialize;
use rusqlite::{params, Connection, Result};
use std::thread;
use std::time::{Duration, Instant};
use crate::utils::get_env_var;

#[derive(Deserialize)]
struct GetBlockHashResponse {
    result: String,
    #[allow(dead_code)]
    error: Option<serde_json::Value>,
    #[allow(dead_code)]
    id: String,
}

#[derive(Deserialize)]
struct GetBlockHeaderResponse {
    result: BlockHeader,
    #[allow(dead_code)]
    error: Option<serde_json::Value>,
    #[allow(dead_code)]
    id: String,
}

#[derive(Debug, Deserialize)]
pub struct BlockHeader {
    pub hash: String,
    pub height: u32,
    pub version: i32,
    pub previousblockhash: Option<String>,
    pub nextblockhash: Option<String>,
    pub merkleroot: String,
    pub time: u64,
    pub bits: String,
    pub nonce: u32,
    pub difficulty: f64,
}



pub fn create_db_connection(db_path: &str) -> Result<Connection> {
    let conn = Connection::open(db_path)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS block_headers (
            hash TEXT PRIMARY KEY,
            height INTEGER,
            version INTEGER,
            previousblockhash TEXT,
            nextblockhash TEXT,
            merkleroot TEXT,
            time INTEGER,
            bits TEXT,
            nonce INTEGER,
            difficulty REAL
        )",
        [],
    )?;
    Ok(conn)
}

pub fn get_block_hash(client: &Client, block_height: u32) -> String {
   
    for _ in 0..5 {
        match client
            .post(get_env_var("SOURCE_URL"))
            .basic_auth(get_env_var("SOURCE_USERNAME"), Some(get_env_var("SOURCE_PASSWORD")))
            .header("Content-Type", "application/json")
            .body(serde_json::json!({
                "jsonrpc": "1.0",
                "id": "1",
                "method": "getblockhash",
                "params": [block_height]
            }).to_string())
            .send()
            .and_then(|r| r.text())
        {
            Ok(response) => {
                let response: GetBlockHashResponse = serde_json::from_str(&response).expect("Failed to parse JSON");
                return response.result;
            }
            Err(e) => {
                eprintln!("Error fetching block hash: {}, retrying in 5 seconds...", e);
                thread::sleep(Duration::from_secs(5));
            }
        }
    }
    panic!("Failed to fetch block hash after 5 attempts");
}

pub fn get_block_header(client: &Client, block_hash: &str) -> BlockHeader {
 
    for _ in 0..5 {
        match client
            .post(get_env_var("SOURCE_URL"))
            .basic_auth(get_env_var("SOURCE_USERNAME"), Some(get_env_var("SOURCE_PASSWORD")))
            .header("Content-Type", "application/json")
            .body(serde_json::json!({
                "jsonrpc": "1.0",
                "id": "1",
                "method": "getblockheader",
                "params": [block_hash]
            }).to_string())
            .send()
            .and_then(|r| r.text())
        {
            Ok(response) => {
                let response: GetBlockHeaderResponse = serde_json::from_str(&response).expect("Failed to parse JSON");
                return response.result;
            }
            Err(e) => {
                eprintln!("Error fetching block header: {}, retrying in 5 seconds...", e);
                thread::sleep(Duration::from_secs(5));
            }
        }
    }
    panic!("Failed to fetch block header after 5 attempts");
}

fn save_block_header(conn: &Connection, block_header: &BlockHeader) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO block_headers (hash, height, version, previousblockhash, nextblockhash, merkleroot, time, bits, nonce, difficulty) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            block_header.hash,
            block_header.height,
            block_header.version,
            block_header.previousblockhash,
            block_header.nextblockhash,
            block_header.merkleroot,
            block_header.time,
            block_header.bits,
            block_header.nonce,
            block_header.difficulty
        ],
    )?;
    Ok(())
}

fn get_last_indexed_height(conn: &Connection) -> Result<u32> {
    let mut stmt = conn.prepare("SELECT MAX(height) FROM block_headers")?;
    let height: u32 = stmt.query_row([], |row| row.get(0)).unwrap_or(0);
    Ok(height)
}

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
