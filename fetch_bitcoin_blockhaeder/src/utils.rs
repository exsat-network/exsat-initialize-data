use std::{env, thread, time::Duration};
use dotenv::dotenv;
use rusqlite::{params, Connection, Result};
use crate::types::{BlockHeader, GetBlockHashResponse, GetBlockHeaderResponse};
use reqwest::blocking::Client;

pub fn get_env_var(key: &str) -> String {
    dotenv().ok();
    env::var(key).expect(&format!("Environment variable {} not found", key))
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
            difficulty REAL,
            chainwork TEXT
        )",
        [],
    )?;
      conn.execute(
        "CREATE TABLE IF NOT EXISTS spv_progress (
            height INTEGER PRIMARY KEY
        )",
        [],
    )?;
    Ok(conn)
}

pub fn get_block_hash(client: &Client, block_height: u64, url: String, username: String, password: String) -> String {
   
    for _ in 0..5 {
        match client
            .post(url.clone())
            .basic_auth(username.clone(), Some(password.clone()))
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

pub fn get_block_header(client: &Client, block_hash: &str, url: String, username: String, password: String) -> BlockHeader {
 
    for _ in 0..5 {
        match client
             .post(url.clone())
            .basic_auth(username.clone(), Some(password.clone()))
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

pub fn save_block_header(conn: &Connection, block_header: &BlockHeader) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO block_headers (hash, height, version, previousblockhash, nextblockhash, merkleroot, time, bits, nonce, difficulty, chainwork) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
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
            block_header.difficulty,
            block_header.chainwork,
        ],
    )?;
    Ok(())
}

pub fn get_last_indexed_height(conn: &Connection) -> Result<u64> {
    let mut stmt = conn.prepare("SELECT MAX(height) FROM block_headers")?;
    let height: u64 = stmt.query_row([], |row| row.get(0)).unwrap_or(0);
    Ok(height)
}