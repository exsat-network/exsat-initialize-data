package main

import (
	"fmt"
	"log"

	"github.com/syndtr/goleveldb/leveldb"
	"github.com/syndtr/goleveldb/leveldb/opt"
	"github.com/syndtr/goleveldb/leveldb/util"
)

func main() {
    // Change this to the path of your LevelDB directory
    dbPath := "/mnt/electrumx/db/utxo"

    // Open the LevelDB database
    db, err := leveldb.OpenFile(dbPath, &opt.Options{
        ReadOnly: true,
    })
    if err != nil {
        log.Fatalf("Failed to open LevelDB: %v", err)
    }
    defer db.Close()

    // Create an iterator to read the database
    iter := db.NewIterator(&util.Range{}, nil)
    defer iter.Release()

    count := 0
    // Iterate over the database
    for iter.Next() {
        key := iter.Key()
        value := iter.Value()

        fmt.Printf("Key: %x\n", key)
        fmt.Printf("Value: %x\n", value)
        count++

        // Break after reading 100 entries to avoid overwhelming output (remove this for full iteration)
        if count >= 100 {
            break
        }
    }

    // Check for errors during iteration
    if err := iter.Error(); err != nil {
        log.Fatalf("Iterator error: %v", err)
    }

    fmt.Printf("Read %d entries from LevelDB\n", count)
}
