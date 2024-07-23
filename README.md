# exSat Initialize Data

This repository contains tools and scripts for initializing exSat with UTXO data, fetching block header data, and verifying data. It includes methods for retrieving UTXO data from a local RocksDB, fetching Bitcoin block headers from a node, and comparing data between different sources.

## Overview

- **Fetch UTXOs From ElectrumX**: Fetch UTXOs from ElectrumX and save in Clickhouse.
- **Block Header Fetcher**: Retrieves Bitcoin block headers and saves them to a CSV file.



Set up the btc fullnode

1. Setup btc fullnode by [script](./setup-bitcoin-fullnode.sh).

    The following are the minimum configuration requirements for running a BTC full node: (The configuration requirements are not high, mainly disk)

    CPU: 2 GHz | 2+ cores

    RAM: 2 GB memory

    SSD: 50G

    Disk: 1TB or more

    System: Ubuntu 22.04

    Traffic bandwidth: 50M bandwidth and above, full node uses more than 200 GB per month

    Task time: 5 minutes;


```
# replace setup-bitcoin-fullnode.sh to setup-bitcoin-testnet3.sh if you want to set up the testnet node.

sudo -s
chmod +x ./setup-bitcoin-fullnode.sh

./setup-bitcoin-fullnode.sh

# select 1 install_btc_full_node

then 

./setup-bitcoin-fullnode.sh

# select 2 run_btc_full_node

```

Before running the programs, ensure you have Rust installed on your system. You can install Rust using the following command:

## Prerequisites
r4.2xlarge aws
CPU >= 4
RAM >= 64 GiB
SWAP >= 64 GiB
Disk >= 1.5T

2. Setup rust env by [script](./setup-rust.sh).

## #1 Block Headers Data < 840000

1. Run the fullnode and make it sync.
2. Enter the `fetch_bitcoin_blockheader`.
3. `cargo run`
4. Finally you'll get the result `block_headers.db`.
5. convert to csv 

```
sqlite3 ./block_headers.db
sqlite> .headers on
sqlite> .mode csv
sqlite> .output block_headers.csv
sqlite> select * from `block_headers`;
sqlite> .quit

zip block_headers_lt_840000.csv.zip ./block_headers.csv
```

> [block header data in S3](https://s3.amazonaws.com/exsat.initialize.data/block_headers_lt_840000.csv.zip)

```shell
sha256sum block_headers_lt_840000.csv.zip
601c86ff3f50783d00d5a93c78bdb2d96ef1e0a5327a4dcfbea9209ec54a1d84  block_headers_lt_840000.csv.zip
```

## #2 UTXOs Data < 840000 (Electrumx)

1. Run the fullnode and make it sync.
2. git clone https://github.com/exsat-network/electrumx.git
3. Run the electrumx manually or by Docker file. Please make sure you set the endblock to 839999.
4. Move data from electrumx ot Clickhouse.

### Setup Clickhouse from docker

1. Change the volume mapping to your localhost disk and create some folders
```shell
       - /mnt3/clickhouse:/var/lib/clickhouse
       mkdir -p /mnt3/clickhouse/logs
       mkdir -p /mnt3/clickhouse/tmp
       mkdir -p /mnt3/clickhouse/user_files
       mkdir -p /mnt3/clickhouse/format_schemas
```
1. Run the docker compose file
```shell
docker-compose up -d
```

1. Run `cargo run` in the `fetch_utxos_from_eletrumx`
2. The moving will be done in about 15hrs.
3. Enter the clickhouse client & check the data.
```shell
docker exec -it  clickhouse /bin/bash

clickhouse-client
```

4. query in clickhouse
```sql
USE blockchain;


SELECT * FROM utxos LIMIT 1;

SET max_memory_usage = 20000000000; -- Set this to 20GB or any other appropriate value

SELECT uniqExact((height, txid, vout)) AS total_unique_rows
FROM blockchain.utxos; --  to count the unique rows based on a combination of height, txid, and vout. 

Query id: 68ab206b-9bc8-4de9-b822-c9a89b2ca86a

┌─total_unique_rows─┐
│         176944794 │
└───────────────────┘


SET max_memory_usage = 60000000000; -- Set this to 40GB or any other appropriate value

SELECT SUM(value) AS total_value
FROM (
    SELECT
        any(value) AS value
    FROM blockchain.utxos
    WHERE address IS NOT NULL
    GROUP BY
        height,
        txid,
        vout
) AS unique_combinations;
 -- query sums the value of these unique combinations.

Query id: 67b76c6d-0b0d-4b39-b048-447618f9b30f

┌──────total_value─┐
│ 1968728049271483 │
└──────────────────┘




-- create and import addresses & balance data from csv(parse from https://github.com/gcarq/rusty-blockparser)
CREATE TABLE IF NOT EXISTS blockchain.addresses (
    address Nullable(String),
    balance UInt64
) ENGINE = MergeTree() ORDER BY address

clickhouse-client -h localhost --query="INSERT INTO blockchain.addresses FORMAT CSVWithNames" --format_csv_delimiter=";" < /var/lib/clickhouse/balances-0-839999.csv


-- dedup
SET max_memory_usage = 60000000000;
CREATE TABLE IF NOT EXISTS blockchain.deduped_utxos
(
    id UInt64,
    height Int64,
    address Nullable(String),
    txid String,
    vout Int64,
    value Int64,
    scriptPubKey String
) ENGINE = MergeTree()
ORDER BY id;

INSERT INTO blockchain.deduped_utxos
SELECT 
    rowNumberInAllBlocks() as id, 
    height, 
    address, 
    txid, 
    vout, 
    value, 
    scriptPubKey
FROM 
(
    SELECT DISTINCT height, txid, vout, address, value, scriptPubKey
    FROM blockchain.utxos
)

clickhouse-client --query="INSERT INTO blockchain.addresses FORMAT CSVWithNames" --format_csv_delimiter=";" < ./balances-0-839999.csv

-- Ctrl + D

```

## #3 UTXOs Data < 840000 ([bitcoin-utxo-dump](https://github.com/exsat-network/bitcoin-utxo-dump))
1. Run the [fullnode](https://github.com/exsat-network/bitcoin) and make it sync to 83999.
2. git clone https://github.com/exsat-network/bitcoin-utxo-dump
3. Run the `bitcoin-utxo-dump`.

```
Total UTXOs: 176944794
Total BTC:   19687280.49271483
Script Types:
 p2pkh        51294486
 p2sh         21198713
 p2ms         1692228
 p2wpkh       57255394
 p2wsh        1536300
 p2tr         43901619
 non-standard 20665
 p2pk         45389
```