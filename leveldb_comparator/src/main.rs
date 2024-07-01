use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use rusqlite::{params, Connection, Result, Transaction};
use rusqlite::OptionalExtension;
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Utxo {
    height: i64,
    #[serde(default)]
    address: Option<String>,
    txid: String,
    vout: i64,
    value: i64,
    scriptPubKey: String,
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

fn save_utxos(tx: &Transaction, utxos: &[Utxo]) -> Result<usize> {
    let mut count = 0;
    for utxo in utxos {
        let result = tx.execute(
            "INSERT OR IGNORE INTO utxos (height, address, txid, vout, value, scriptPubKey) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![utxo.height, utxo.address, utxo.txid, utxo.vout, utxo.value, utxo.scriptPubKey],
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

fn save_last_key(conn: &mut Connection, last_key: &str) -> Result<()> {
    match conn.execute(
        "INSERT OR REPLACE INTO progress (id, last_key) VALUES (1, ?1)",
        params![last_key],
    ) {
        Ok(_) => Ok(()),
        Err(err) => {
            println!("Failed to save last_key: {}, error: {:?}", last_key, err);
            Err(err)
        }
    }
}

fn get_last_key(conn: &Connection) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT last_key FROM progress WHERE id = 1")?;
    match stmt.query_row([], |row| row.get(0)).optional() {
        Ok(last_key) => Ok(last_key),
        Err(err) => {
            println!("Failed to get last_key, error: {:?}", err);
            Err(err)
        }
    }
}

fn get_total_saved_utxos(conn: &Connection) -> Result<i64> {
    let mut stmt = conn.prepare("SELECT COUNT(*) FROM utxos")?;
    match stmt.query_row([], |row| row.get(0)) {
        Ok(total_saved_utxos) => Ok(total_saved_utxos),
        Err(err) => {
            println!("Failed to get total_saved_utxos, error: {:?}", err);
            Err(err)
        }
    }
}

fn main() -> Result<()> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("Failed to build HTTP client");

    let db_path = "/mnt3/utxos_sqlite/utxos.db";
    let mut conn = Connection::open(db_path)?;
    conn.execute("PRAGMA synchronous = OFF", [])?;
    conn.execute("PRAGMA journal_mode = WAL", [])?;
    
    match conn.execute(
        "CREATE TABLE IF NOT EXISTS utxos (
            height INTEGER,
            address TEXT,
            txid TEXT,
            vout INTEGER,
            value INTEGER,
            scriptPubKey TEXT,
            UNIQUE(txid, vout)
        )",
        [],
    ) {
        Ok(_) => println!("Table utxos created or already exists."),
        Err(err) => {
            println!("Failed to create table utxos, error: {:?}", err);
            return Err(err);
        }
    }
    
    match conn.execute(
        "CREATE TABLE IF NOT EXISTS progress (
            id INTEGER PRIMARY KEY,
            last_key TEXT
        )",
        [],
    ) {
        Ok(_) => println!("Table progress created or already exists."),
        Err(err) => {
            println!("Failed to create table progress, error: {:?}", err);
            return Err(err);
        }
    }
    
    match conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_utxos_height_txid_vout ON utxos (height, txid, vout)",
        [],
    ) {
        Ok(_) => println!("Index idx_utxos_height_txid_vout created or already exists."),
        Err(err) => {
            println!("Failed to create index idx_utxos_height_txid_vout, error: {:?}", err);
            return Err(err);
        }
    }

    let mut url = "http://localhost:8081/proxy/all_utxos?limit=1000".to_string();
    if let Some(last_key) = get_last_key(&conn)? {
        url = format!("http://localhost:8081/proxy/all_utxos?limit=1000&startkey={}", last_key);
    }

    let mut total_saved_utxos = match get_total_saved_utxos(&conn) {
        Ok(count) => count,
        Err(_) => 0,
    };

    let batch_size = 10_000;
    let mut utxo_buffer: Vec<Utxo> = Vec::with_capacity(batch_size);

    loop {
        println!("Fetching UTXOs from URL: {}", url);
        let response = fetch_utxos(&client, &url);

        if response.is_err() {
            println!("Request failed: {}. Retrying...", response.unwrap_err());
            std::thread::sleep(Duration::from_secs(30));
            continue;
        }

        let mut response = response.unwrap();
        println!("Fetched {} UTXOs", response.utxos.len());
        utxo_buffer.extend(response.utxos.clone());

        if utxo_buffer.len() >= batch_size {
            let tx = conn.transaction()?;
            let saved_count = save_utxos(&tx, &utxo_buffer)?;
            tx.commit()?;
            total_saved_utxos += saved_count as i64;
            println!("Saved {} UTXOs in this batch, total UTXOs saved: {}", saved_count, total_saved_utxos);
            utxo_buffer.clear();
        }

        if response.utxos.len() < 1000 {
            println!("Fetched less than limit, stopping.");
            break;
        }

        if let Some(last_key) = response.last_key {
            save_last_key(&mut conn, &last_key)?;
            url = format!("http://localhost:8081/proxy/all_utxos?limit=1000&startkey={}", last_key);
        } else {
            println!("No last_key provided, stopping.");
            break;
        }
    }

    // Save any remaining UTXOs
    if !utxo_buffer.is_empty() {
        let tx = conn.transaction()?;
        let saved_count = save_utxos(&tx, &utxo_buffer)?;
        tx.commit()?;
        total_saved_utxos += saved_count as i64;
        println!("Saved {} remaining UTXOs, total UTXOs saved: {}", saved_count, total_saved_utxos);
    }

    println!("Total UTXOs saved: {}", total_saved_utxos);

    Ok(())
}
