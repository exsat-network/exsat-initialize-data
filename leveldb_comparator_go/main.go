package main

import (
	"encoding/binary"
	"encoding/hex"
	"fmt"
	"log"

	"github.com/syndtr/goleveldb/leveldb"
)

func main() {
	dbPath := "/mnt/electrumx/db/utxo" // Update this path to your LevelDB directory
	db, err := leveldb.OpenFile(dbPath, nil)
	if err != nil {
		log.Fatalf("Failed to open LevelDB: %v", err)
	}
	defer db.Close()

	iter := db.NewIterator(nil, nil)
	for iter.Next() {
		key := iter.Key()
		value := iter.Value()

		// Decode the key
		if len(key) != 12 {
			fmt.Printf("Unexpected key length: %d\n", len(key))
			continue
		}

		prefix := key[0]
		blockHeight := binary.BigEndian.Uint32(key[1:5])
		txPos := binary.BigEndian.Uint32(key[5:9])
		outputIndex := binary.BigEndian.Uint32(key[9:12])

		fmt.Printf("Key:\n  Prefix: 0x%x\n  Block Height: %d\n  Tx Position: %d\n  Output Index: %d\n", prefix, blockHeight, txPos, outputIndex)

		// Decode the value
		if len(value) < 9 {
			fmt.Printf("Unexpected value length: %d\n", len(value))
			continue
		}

		txValue := binary.LittleEndian.Uint64(value[:8])
		scriptLen := value[8]
		scriptPubKey := value[9 : 9+scriptLen]

		fmt.Printf("Value:\n  Tx Value (satoshis): %d\n  Script Length: %d\n  ScriptPubKey: %s\n\n", txValue, scriptLen, hex.EncodeToString(scriptPubKey))
	}

	iter.Release()
	if err := iter.Error(); err != nil {
		log.Fatalf("Iterator error: %v", err)
	}
}
