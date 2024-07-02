# exSat Initialize Data

This repository contains tools and scripts for initializing exSat with UTXO data, fetching block header data, and verifying data. It includes methods for retrieving UTXO data from a local RocksDB, fetching Bitcoin block headers from a node, and comparing data between different sources.

## Overview

- **Fetch UTXOs From ElectrumX**: Fetch UTXOs from ElectrumX and save in Clickhouse.
- **Block Header Fetcher**: Retrieves Bitcoin block headers and saves them to a CSV file.

## Prerequisites
CPU >= 4
RAM >= 32 GiB
Disk >= 1.5T

Before running the programs, ensure you have Rust installed on your system. You can install Rust using the following command:

1. Setup btc fullnode by [script](./setup-bitcoin-fullnode.sh).


2. Setup rust env by [script](./setup-rust.sh).

## #1 Block Headers Data < 840000

1. Run the fullnode and make it sync.
2. Enter the `fetch_bitcoin_blockheader`.
3. `cargo run`

4. Finally you'll get the result.

> [Sqlite Database in S3](https://s3.amazonaws.com/exsat.initialize.data/block_headers_lt_840000_sqlite.zip)

```shell
md5sum block_headers_lt_840000_sqlite.zip
e849ee5c80eefee3061b267bc317a142  block_headers_lt_840000_sqlite.zip
```

## #2 UTXOs Data < 840000

1. Run the fullnode and make it sync.
2. git clone https://github.com/exsat-network/electrumx.git
3. Run the electrumx manually or by Docker file. Please make sure you set the endblock to 839999.
4. Move data from electrumx ot Clickhouse.

### Setup Clickhouse from docker
0. Create swap if your RAM is not enough 
```
sudo fallocate -l 10G /mnt3/swapfile
sudo chmod 600 /mnt3/swapfile
sudo mkswap /mnt3/swapfile
sudo swapon /mnt3/swapfile

echo '/mnt3/swapfile none swap sw 0 0' | sudo tee -a /etc/fstab
```

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
│         176960293 │
└───────────────────┘





SET max_memory_usage = 40000000000; -- Set this to 40GB or any other appropriate value

SELECT SUM(value) AS total_value
FROM (
    SELECT
        any(value) AS value
    FROM blockchain.utxos
    GROUP BY
        height,
        txid,
        vout
) AS unique_combinations;
 -- query sums the value of these unique combinations.

```