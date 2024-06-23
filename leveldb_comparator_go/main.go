package main

import (
	"encoding/binary"
	"encoding/hex"
	"flag"
	"fmt"
	"log"
	"os"
	"path/filepath"

	"github.com/syndtr/goleveldb/leveldb"
	"github.com/syndtr/goleveldb/leveldb/util"
)

// UTXO represents an unspent transaction output
type UTXO struct {
	TxNum  uint64
	TxPos  uint32
	TxHash string
	Height int
	Value  uint64
}

// DB represents the LevelDB database and metadata
type DB struct {
	utxoDB     *leveldb.DB
	txCounts   []uint64
	dbHeight   int
	hashesFile *LogicalFile
}

// LogicalFile represents a logical file split across several files
type LogicalFile struct {
	filenameFmt string
	fileSize    int
}

// Read reads data from the logical file starting at offset start and up to size bytes
func (lf *LogicalFile) Read(start int, size int) []byte {
	if size == -1 {
		size = 1 << 30
	}
	buf := make([]byte, size)
	readBytes := 0
	for size > 0 {
		filename := fmt.Sprintf(lf.filenameFmt, start/lf.fileSize)
		offset := start % lf.fileSize
		file, err := os.Open(filename)
		if err != nil {
			break
		}
		file.Seek(int64(offset), 0)
		n, _ := file.Read(buf[readBytes:])
		file.Close()
		if n == 0 {
			break
		}
		readBytes += n
		start += n
		size -= n
	}
	return buf[:readBytes]
}

// FSTxHash returns the transaction hash and height for the given transaction number
func (db *DB) FSTxHash(txNum uint64) (string, int) {
	txHeight := bisectRight(db.txCounts, txNum)
	if txHeight > db.dbHeight {
		return "", txHeight
	}
	txHash := db.hashesFile.Read(int(txNum*32), 32)
	return hex.EncodeToString(txHash), txHeight
}

// BisectRight finds the position to insert x in a to maintain sorted order
func bisectRight(a []uint64, x uint64) int {
	for i, v := range a {
		if x < v {
			return i
		}
	}
	return len(a)
}

// UnpackLEUint32 unpacks a little-endian uint32 from a byte slice
func unpackLEUint32(b []byte) uint32 {
	return binary.LittleEndian.Uint32(b)
}

// UnpackLEUint64 unpacks a little-endian uint64 from a byte slice
func unpackLEUint64(b []byte) uint64 {
	return binary.LittleEndian.Uint64(b)
}

// PageableUTXOs returns UTXOs starting from lastKey with a limit
func (db *DB) PageableUTXOs(lastKey string, limit int) (string, []UTXO, error) {
	var lastDbKey string
	var utxos []UTXO

	txnumPadding := make([]byte, 8-4)
	var iterator = db.utxoDB.NewIterator(nil, nil)

	if lastKey != "" {
		startKey, err := hex.DecodeString(lastKey)
		if err != nil {
			return "", nil, err
		}
		iterator = db.utxoDB.NewIterator(&util.Range{Start: startKey}, nil)
	} else {
		iterator = db.utxoDB.NewIterator(nil, nil)
	}
	defer iterator.Release()

	for iterator.Next() {
		dbKey := iterator.Key()
		dbValue := iterator.Value()

		txoutIdx := unpackLEUint32(dbKey[len(dbKey)-8 : len(dbKey)-4])
		txNum := unpackLEUint64(append(dbKey[len(dbKey)-4:], txnumPadding...))
		value := unpackLEUint64(dbValue)

		txHash, height := db.FSTxHash(txNum)
		if txHash == "" {
			continue
		}

		utxo := UTXO{
			TxNum:  txNum,
			TxPos:  txoutIdx,
			TxHash: txHash,
			Height: height,
			Value:  value,
		}
		utxos = append(utxos, utxo)
		lastDbKey = hex.EncodeToString(dbKey)
		if len(utxos) == limit {
			break
		}
	}

	if err := iterator.Error(); err != nil {
		return "", nil, err
	}

	return lastDbKey, utxos, nil
}

// ReadTxCounts reads transaction counts from the logical file
func readTxCounts(logicalFile *LogicalFile) ([]uint64, int, error) {
	data := logicalFile.Read(0, -1)
	if len(data)%8 != 0 {
		return nil, 0, fmt.Errorf("invalid txcounts file size")
	}

	counts := make([]uint64, len(data)/8)
	for i := range counts {
		counts[i] = binary.LittleEndian.Uint64(data[i*8:])
	}

	dbHeight := len(counts) - 1
	return counts, dbHeight, nil
}

func main() {
	dbPath := flag.String("dbpath", "", "Path to the LevelDB database")
	flag.Parse()

	if *dbPath == "" {
		log.Fatal("Database path is required")
	}

	db := &DB{
		hashesFile: &LogicalFile{
			filenameFmt: filepath.Join(*dbPath, "meta/hashes%04d"),
			fileSize:    16000000,
		},
	}
	var err error
	db.utxoDB, err = leveldb.OpenFile(filepath.Join(*dbPath, "utxo"), nil)
	if err != nil {
		log.Fatal(err)
	}
	defer db.utxoDB.Close()

	txCountsFile := &LogicalFile{
		filenameFmt: filepath.Join(*dbPath, "meta/txcounts%02d"),
		fileSize:    2000000,
	}

	db.txCounts, db.dbHeight, err = readTxCounts(txCountsFile)
	if err != nil {
		log.Fatal(err)
	}

	lastKey, utxos, err := db.PageableUTXOs("", 10)
	if err != nil {
		log.Fatal(err)
	}

	fmt.Printf("Last Key: %s\n", lastKey)
	for _, utxo := range utxos {
		fmt.Printf("UTXO: %+v\n", utxo)
	}
}
