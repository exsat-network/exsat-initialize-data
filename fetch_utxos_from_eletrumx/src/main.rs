use reqwest::Client;
use serde::{Deserialize, Serialize};
use clickhouse::{Client as CHClient, Row};
use std::time::Duration;
use log::{info, error};
use thiserror::Error;
use tokio::time::sleep;

#[derive(Debug, Serialize, Deserialize, Row)]
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

#[derive(Debug, Row, Deserialize)]
struct LastKeyRow {
    last_key: String,
}

#[derive(Error, Debug)]
enum AppError {
    #[error("Reqwest error: {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("Clickhouse error: {0}")]
    ClickhouseError(#[from] clickhouse::error::Error),
    #[error("Other error: {0}")]
    Other(String),
}

async fn fetch_utxos(client: &Client, url: &str) -> Result<ApiResponse, AppError> {
    let response = client.get(url).send().await?.json::<ApiResponse>().await?;
    Ok(response)
}

async fn save_utxos(ch_client: &CHClient, utxos: &[Utxo]) -> Result<(), AppError> {
    let mut insert = ch_client.insert("blockchain.utxos")?;
    for utxo in utxos {
        insert.write(utxo).await?;
    }
    insert.end().await?;
    Ok(())
}

async fn save_last_key(ch_client: &CHClient, last_key: &str) -> Result<(), AppError> {
    ch_client.query("INSERT INTO blockchain.progress (id, last_key) VALUES (1, ?)")
        .bind(last_key)
        .execute().await?;
    Ok(())
}

async fn get_last_key(ch_client: &CHClient) -> Result<Option<String>, AppError> {
    let mut cursor = ch_client
        .query("SELECT last_key FROM blockchain.progress WHERE id = 1")
        .fetch::<LastKeyRow>()?;
    
    if let Some(row) = cursor.next().await? {
        Ok(Some(row.last_key))
    } else {
        Ok(None)
    }
}

async fn setup_database(ch_client: &CHClient) -> Result<(), AppError> {
    ch_client.query("CREATE DATABASE IF NOT EXISTS blockchain").execute().await?;
    ch_client.query("CREATE TABLE IF NOT EXISTS blockchain.utxos (
            height Int64,
            address Nullable(String),
            txid String,
            vout Int64,
            value Int64,
            scriptPubKey String
        ) ENGINE = MergeTree()
        ORDER BY (height, txid, vout)").execute().await?;
    ch_client.query("CREATE TABLE IF NOT EXISTS blockchain.progress (
            id Int32,
            last_key String
        ) ENGINE = TinyLog").execute().await?;
    Ok(())
}
#[tokio::main]
async fn main() -> Result<(), AppError> {
    env_logger::init();

    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("Failed to build HTTP client");

    let ch_client = CHClient::default().with_url("http://localhost:8123").with_user("default")
    .with_password("exsat");;

    setup_database(&ch_client).await?;

    let mut url = "http://localhost:8081/proxy/all_utxos?limit=1000".to_string();
    if let Some(last_key) = get_last_key(&ch_client).await? {
        url = format!("http://localhost:8081/proxy/all_utxos?limit=1000&startkey={}", last_key);
    }

    let mut total_saved_utxos = 0;
    let mut retry_count = 0;
    const MAX_RETRIES: u32 = 5;

    loop {
        println!("Fetching UTXOs from URL: {}", url);
        match fetch_utxos(&client, &url).await {
            Ok(response) => {
                println!("Fetched {} UTXOs", response.utxos.len());
                save_utxos(&ch_client, &response.utxos).await?;
                total_saved_utxos += response.utxos.len();
                println!("Saved {} UTXOs in this batch, total UTXOs saved: {}", response.utxos.len(), total_saved_utxos);

                retry_count = 0;

                if response.utxos.len() < 1000 {
                    println!("Fetched less than limit, stopping.");
                    break;
                }

                if let Some(last_key) = response.last_key {
                    save_last_key(&ch_client, &last_key).await?;
                    url = format!("http://localhost:8081/proxy/all_utxos?limit=1000&startkey={}", last_key);
                } else {
                    println!("No last_key provided, stopping.");
                    break;
                }
            }
            Err(e) => {
                error!("Request failed: {}. Retrying...", e);
                retry_count += 1;
                if retry_count >= MAX_RETRIES {
                    return Err(AppError::Other("Max retries reached".to_string()));
                }
                 sleep(Duration::from_secs(30)).await;
            }
        }
    }

    println!("Total UTXOs saved: {}", total_saved_utxos);

    Ok(())
}
