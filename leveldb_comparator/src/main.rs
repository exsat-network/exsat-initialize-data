use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use rusqlite::{params, Connection, Result};
use rusqlite::OptionalExtension;
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize)]
struct Utxo {
    height: i64,
    address: String,
    txid: String,
    vout: i64,
    value: i64,
    scriptPubKeyHex: String, // 添加新的字段
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiResponse {
    last_key: Option<String>,
    utxos: Vec<Utxo>,
}

fn fetch_utxos(client: &Client, url: &str) -> Result<ApiResponse, reqwest::Error> {
    let response = client.get(url).send()?.json::<ApiResponse>()?;
    Ok(response)
}

fn save_utxos(conn: &Connection, utxos: &[Utxo]) -> Result<usize> {
    let mut count = 0;
    for utxo in utxos {
        let rows_affected = conn.execute(
            "INSERT OR IGNORE INTO utxos (height, address, txid, vout, value, scriptPubKeyHex) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![utxo.height, utxo.address, utxo.txid, utxo.vout, utxo.value, utxo.scriptPubKeyHex],
        )?;
        count += rows_affected;
    }
    Ok(count)
}

fn save_last_key(conn: &Connection, last_key: &str) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO progress (id, last_key) VALUES (1, ?1)",
        params![last_key],
    )?;
    Ok(())
}

fn get_last_key(conn: &Connection) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT last_key FROM progress WHERE id = 1")?;
    let last_key: Option<String> = stmt.query_row([], |row| row.get(0)).optional()?;
    Ok(last_key)
}

fn get_utxo_count(conn: &Connection) -> Result<i64> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM utxos", [], |row| row.get(0))?;
    Ok(count)
}

fn main() -> Result<()> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("Failed to build HTTP client");

    let db_path = "utxos.db";
    let conn = Connection::open(db_path)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS utxos (
            height INTEGER,
            address TEXT,
            txid TEXT UNIQUE,
            vout INTEGER,
            value INTEGER,
            scriptPubKeyHex TEXT
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS progress (
            id INTEGER PRIMARY KEY,
            last_key TEXT
        )",
        [],
    )?;

    let mut url = "http://localhost:8080/proxy/all_utxos?limit=1000".to_string();
    if let Some(last_key) = get_last_key(&conn)? {
        url = format!("http://localhost:8080/proxy/all_utxos?limit=1000&last_key={}", last_key);
    }

    loop {
        match fetch_utxos(&client, &url) {
            Ok(response) => {
                let saved_count = save_utxos(&conn, &response.utxos)?;
                let total_count = get_utxo_count(&conn)?;
                println!("Saved {} UTXOs, total UTXOs saved: {}", saved_count, total_count);

                if response.utxos.len() < 1000 {
                    println!("Fetched less than limit, stopping.");
                    break;
                }

                if let Some(last_key) = response.last_key {
                    save_last_key(&conn, &last_key)?;
                    url = format!("http://localhost:8080/proxy/all_utxos?limit=1000&last_key={}", last_key);
                } else {
                    println!("No last_key provided, stopping.");
                    break;
                }
            }
            Err(e) => {
                println!("Request failed: {}. Retrying...", e);
                std::thread::sleep(Duration::from_secs(5));
            }
        }
    }

    Ok(())
}
