use leveldb::database::Database;
use leveldb::iterator::Iterable;
use leveldb::options::{Options, ReadOptions};
use std::path::Path;

fn main() {
    // Ensure the LevelDB database path is provided correctly
    let db_path = "/mnt/electrumx/db/utxo";
    let path = Path::new(db_path);

    // Open the LevelDB database
    let mut options = Options::new();
    options.create_if_missing = false;

    match Database::<i32>::open(path, options) {
        Ok(db) => {
            let read_opts = ReadOptions::new();
            let iter = db.iter(read_opts);

            // Iterate through the LevelDB entries and process them
            for (key, value) in iter {
                println!("{:?} -> {:?}", key, value);
                // Add your processing logic here
            }
        }
        Err(e) => {
            eprintln!("Failed to open LevelDB database: {}", e);
        }
    }
}
