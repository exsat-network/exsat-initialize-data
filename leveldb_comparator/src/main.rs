use rocksdb::{DB, Options, DBWithThreadMode, SingleThreaded, ReadOptions};
use md5::Context;
use std::path::Path;
use std::collections::HashMap;
use std::io::{self, Write};

// Function to calculate the MD5 hash of a single RocksDB file and print a few entries
fn calculate_file_md5(db: &DBWithThreadMode<SingleThreaded>, print_entries: bool) -> String {
    let mut hasher = Context::new();
    let mut roptions = ReadOptions::default();
    roptions.fill_cache(false);
    let iter = db.iterator_opt(rocksdb::IteratorMode::Start, roptions);
    let mut count = 0;

    for item in iter {
        match item {
            Ok((key, value)) => {
                hasher.consume(&key);
                hasher.consume(&value);

                if print_entries && count < 5 {
                    println!("Key: {:?}, Value: {:?}", key, value);
                    count += 1;
                }
            },
            Err(e) => eprintln!("Error iterating over the database: {}", e),
        }
    }
    format!("{:x}", hasher.compute())
}

// Function to calculate the MD5 hashes of all RocksDB files in a directory
fn calculate_directory_md5(directory_path: &Path) -> Result<HashMap<String, String>, String> {
    let mut file_hashes = HashMap::new();
    let options = Options::default();
    let db = DBWithThreadMode::<SingleThreaded>::open_for_read_only(&options, directory_path, false)
        .map_err(|e| format!("Failed to open RocksDB at {:?}: {}", directory_path, e))?;
    let file_hash = calculate_file_md5(&db, true); // Enable printing of entries
    file_hashes.insert("rocksdb_db".to_string(), file_hash);
    Ok(file_hashes)
}

// Function to read input from the user
fn read_input(prompt: &str) -> String {
    print!("{}", prompt);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

// Main function to calculate and save hashes
fn main() {
    let dir_path = read_input("Enter the directory path to RocksDB files: ");
    let dir = Path::new(&dir_path);

    if !dir.exists() {
        println!("The provided directory does not exist.");
        return;
    }

    match calculate_directory_md5(dir) {
        Ok(hashes) => {
            // Example: print hashes
            for (filename, hash) in &hashes {
                println!("File: {}, MD5: {}", filename, hash);
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }
}
