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

fn save_last_key(tx: &Transaction, last_key: &str) -> Result<()> {
    tx.execute(
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

fn get_total_saved_utxos(conn: &Connection) -> Result<i64> {
    let mut stmt = conn.prepare("SELECT COUNT(*) FROM utxos")?;
    let total_saved_utxos: i64 = stmt.query_row([], |row| row.get(0)).unwrap_or(0);
    Ok(total_saved_utxos)
}

fn copy_data(src_conn: &Connection, dest_conn: &mut Connection) -> Result<()> {
    let mut src_stmt = src_conn.prepare("SELECT height, address, txid, vout, value, scriptPubKey FROM utxos")?;
    let utxo_iter = src_stmt.query_map([], |row| {
        Ok(Utxo {
            height: row.get(0)?,
            address: row.get(1)?,
            txid: row.get(2)?,
            vout: row.get(3)?,
            value: row.get(4)?,
            scriptPubKey: row.get(5)?,
        })
    })?;

    let tx = dest_conn.transaction()?;
    for utxo in utxo_iter {
        let utxo = utxo?;
        tx.execute(
            "INSERT OR IGNORE INTO utxos (height, address, txid, vout, value, scriptPubKey) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![utxo.height, utxo.address, utxo.txid, utxo.vout, utxo.value, utxo.scriptPubKey],
        )?;
    }
    tx.commit()?;
    Ok(())
}

fn main() -> Result<()> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("Failed to build HTTP client");

    let db_path = "/mnt3/utxos_sqlite/utxos.db";
    let mut conn = Connection::open(":memory:")?;
    conn.execute("PRAGMA synchronous = OFF", [])?;
    conn.execute("PRAGMA journal_mode = MEMORY", [])?;
    conn.execute(
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
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS progress (
            id INTEGER PRIMARY KEY,
            last_key TEXT
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_utxos_height_txid_vout ON utxos (height, txid, vout)",
        [],
    )?;

    let mut url = "http://localhost:8081/proxy/all_utxos?limit=1000".to_string();
    if let Some(last_key) = get_last_key(&conn)? {
        url = format!("http://localhost:8081/proxy/all_utxos?limit=1000&startkey={}", last_key);
    }

    let mut total_saved_utxos = match get_total_saved_utxos(&conn) {
        Ok(count) => count,
        Err(_) => 0,
    };

    loop {
        println!("Fetching UTXOs from URL: {}", url);
        let response = fetch_utxos(&client, &url);

        if response.is_err() {
            println!("Request failed: {}. Retrying...", response.unwrap_err());
            std::thread::sleep(Duration::from_secs(30));
            continue;
        }

        let response = response.unwrap();
        println!("Fetched {} UTXOs", response.utxos.len());

        let tx = conn.transaction()?;
        let saved_count = save_utxos(&tx, &response.utxos)?;
        if let Some(last_key) = &response.last_key {
            save_last_key(&tx, last_key)?;
        }
        tx.commit()?;

        total_saved_utxos += saved_count as i64;
        println!("Saved {} UTXOs in this batch, total UTXOs saved: {}", saved_count, total_saved_utxos);

        if response.utxos.len() < 1000 {
            println!("Fetched less than limit, stopping.");
            break;
        }

        if let Some(last_key) = response.last_key {
            url = format!("http://localhost:8081/proxy/all_utxos?limit=1000&startkey={}", last_key);
        } else {
            println!("No last_key provided, stopping.");
            break;
        }
    }

    // Export the in-memory database to a file
    let mut disk_conn = Connection::open(db_path)?;
    copy_data(&conn, &mut disk_conn)?;

    println!("Total UTXOs saved: {}", total_saved_utxos);

    Ok(())
}
