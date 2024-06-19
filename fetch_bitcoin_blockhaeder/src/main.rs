use reqwest::blocking::Client;
use serde::Deserialize;
use std::fs::{File, OpenOptions};
use csv::{Writer, ReaderBuilder};
use std::thread;
use std::time::{Duration, Instant};

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

#[derive(Deserialize)]
struct BlockHeader {
    hash: String,
    height: u32,
    merkleroot: String,
    #[allow(dead_code)]
    previousblockhash: Option<String>,
    nextblockhash: Option<String>,
}

const URL: &str = "http://regtest.exactsat.io:18443/";
const USERNAME: &str = "test";
const PASSWORD: &str = "test";

fn get_block_hash(client: &Client, block_height: u32) -> String {
    for _ in 0..5 {
        match client
            .post(URL)
            .basic_auth(USERNAME, Some(PASSWORD))
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

fn get_block_header(client: &Client, block_hash: &str) -> BlockHeader {
    for _ in 0..5 {
        match client
            .post(URL)
            .basic_auth(USERNAME, Some(PASSWORD))
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

fn read_last_block_height(output_file: &str) -> Option<(u32, String)> {
    let file = File::open(output_file).ok()?;
    let mut rdr = ReaderBuilder::new().has_headers(true).from_reader(file);
    let last_record = rdr.records().last()?.ok()?;
    let height: u32 = last_record.get(0)?.parse().ok()?;
    let hash: String = last_record.get(2)?.to_string();
    Some((height, hash))
}

fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.as_secs();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

fn main() {
    let client = Client::new();
    let output_file = "block_headers.csv";

    let mut wtr = Writer::from_writer(OpenOptions::new().append(true).create(true).open(output_file).expect("Unable to create file"));

    let (last_height, last_hash) = read_last_block_height(output_file).unwrap_or((0, get_block_hash(&client, 0)));
    let mut current_height = last_height;
    let mut current_block_hash = last_hash.clone();

    if last_height == 0 {
        wtr.write_record(&["height", "merkleroot", "hash"]).expect("Failed to write header");
    } else {
        // Move to the next block height and hash to avoid duplicating the last record
        current_height += 1;
        current_block_hash = get_block_hash(&client, current_height);
    }

    let mut batch_count = 0;
    let start_time = Instant::now();
    let total_blocks = 840000 - current_height;

    while current_height < 840000 {
        let block_header = get_block_header(&client, &current_block_hash);
        
        wtr.write_record(&[
            block_header.height.to_string(),
            block_header.merkleroot.clone(),
            block_header.hash.clone(),
        ]).expect("Failed to write record");

        batch_count += 1;
        current_height += 1;

        if batch_count >= 20 {
            wtr.flush().expect("Failed to flush CSV writer");
            batch_count = 0;
        }

        match block_header.nextblockhash {
            Some(hash) => current_block_hash = hash,
            None => break,  // No more blocks
        }

        // Calculate and display ETA
        let elapsed_time = start_time.elapsed();
        let blocks_processed = current_height - last_height;
        let avg_time_per_block = elapsed_time / blocks_processed as u32;
        let remaining_blocks = total_blocks - blocks_processed;
        let eta = avg_time_per_block * remaining_blocks as u32;

        println!(
            "Processed block height: {} (ETA: {})",
            current_height,
            format_duration(eta)
        );

        // Limit the rate of API calls to avoid hitting rate limits
        thread::sleep(Duration::from_millis(100));
    }

    wtr.flush().expect("Failed to flush CSV writer");
    println!("Block headers saved to {}", output_file);
}
