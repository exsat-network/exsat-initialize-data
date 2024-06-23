package main

import (
	"encoding/binary"
	"encoding/hex"
	"fmt"
	"log"

	"github.com/syndtr/goleveldb/leveldb"
)

func main() {
    db, err := leveldb.OpenFile("/mnt/electrumx/db/utxo", nil)
    if err != nil {
        log.Fatal(err)
    }
    defer db.Close()

    iter := db.NewIterator(nil, nil)
    for iter.Next() {
        key := iter.Key()
        value := iter.Value()

        txID, txPos, err := decodeKey(key)
        if err != nil {
            log.Printf("Failed to decode key: %v", err)
            continue
        }

        txNum, height, amount, err := decodeValue(value)
        if err != nil {
            log.Printf("Failed to decode value: %v", err)
            continue
        }

        fmt.Printf("TXID: %s, Vout: %d, TXNum: %d, Height: %d, Value: %d\n", txID, txPos, txNum, height, amount)
    }
    iter.Release()
    err = iter.Error()
    if err != nil {
        log.Fatal(err)
    }
}

func decodeKey(key []byte) (string, uint32, error) {
    if len(key) != 36 {
        return "", 0, fmt.Errorf("unexpected key length: %d", len(key))
    }

    txID := hex.EncodeToString(key[:32])
    txPos := binary.BigEndian.Uint32(key[32:])

    return txID, txPos, nil
}

func decodeValue(value []byte) (uint64, uint32, uint64, error) {
    if len(value) != 12 {
        return 0, 0, 0, fmt.Errorf("unexpected value length: %d", len(value))
    }

    txNum := binary.BigEndian.Uint64(value[:8])
    height := binary.BigEndian.Uint32(value[8:12])
    amount := binary.BigEndian.Uint64(value[12:20])

    return txNum, height, amount, nil
}

