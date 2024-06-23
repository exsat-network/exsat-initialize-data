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
    scriptPubKeyHex: String,
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
        let result = conn.execute(
            "INSERT OR IGNORE INTO utxos (height, address, txid, vout, value, scriptPubKeyHex) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![utxo.height, utxo.address, utxo.txid, utxo.vout, utxo.value, utxo.scriptPubKeyHex],
        );

        match result {
            Ok(rows_affected) => {
                if rows_affected > 0 {
                    count += rows_affected;
                }
            }
            Err(err) => {
                println!("Failed to save UTXO: {:?}, error: {:?}", utxo, err);
            }
        }
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

fn main() -> Result<()> {
    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("Failed to build HTTP client");

    let db_path = "/mnt3/utxos_sqlite/utxos.db";
    let conn = Connection::open(db_path)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS utxos (
            height INTEGER,
            address TEXT,
            txid TEXT,
            vout INTEGER,
            value INTEGER,
            scriptPubKeyHex TEXT,
            UNIQUE(txid, vout)
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

    let mut url = "http://localhost:8080/proxy/all_utxos?limit=100".to_string();
    if let Some(last_key) = get_last_key(&conn)? {
        url = format!("http://localhost:8080/proxy/all_utxos?limit=100&startkey={}", last_key);
    }

    let mut total_saved_utxos = 0;

    loop {
        println!("Fetching UTXOs from URL: {}", url);
        let response = fetch_utxos(&client, &url);

        if response.is_err() {
            println!("Request failed: {}. Retrying...", response.unwrap_err());
            std::thread::sleep(Duration::from_secs(5));
            continue;
        }

        let response = response.unwrap();
        println!("Fetched {} UTXOs", response.utxos.len());
        let saved_count = save_utxos(&conn, &response.utxos)?;
        total_saved_utxos += saved_count;
        println!("Saved {} UTXOs in this batch, total UTXOs saved: {}", saved_count, total_saved_utxos);

        if response.utxos.len() < 100 {
            println!("Fetched less than limit, stopping.");
            break;
        }

        if let Some(last_key) = response.last_key {
            save_last_key(&conn, &last_key)?;
            url = format!("http://localhost:8080/proxy/all_utxos?limit=100&startkey={}", last_key);
        } else {
            println!("No last_key provided, stopping.");
            break;
        }
    }

    println!("Total UTXOs saved: {}", total_saved_utxos);

    Ok(())
}
