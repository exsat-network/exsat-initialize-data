# exSat Initialize Data

This repository contains tools and scripts for initializing exSat with UTXO data, fetching block header data, and verifying data. It includes methods for retrieving UTXO data from a local RocksDB, fetching Bitcoin block headers from a node, and comparing data between different sources.

## Overview

- **Comparator**: Compares two RocksDB directories and calculates the MD5 checksums of their files.
- **Block Header Fetcher**: Retrieves Bitcoin block headers and saves them to a CSV file.

## Prerequisites

Before running the programs, ensure you have Rust installed on your system. You can install Rust using the following command:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

## Block Headers Data < 840000

> [Sqlite Database in S3](https://s3.amazonaws.com/exsat.initialize.data/block_headers_lt_840000_sqlite.zip)


## Setup Clickhouse from docker
1. Change the volume mapping to your localhost disk and create some folders
```
       - /mnt3/clickhouse:/var/lib/clickhouse
       mkdir -p /mnt3/clickhouse/logs
       mkdir -p /mnt3/clickhouse/tmp
       mkdir -p /mnt3/clickhouse/user_files
       mkdir -p /mnt3/clickhouse/format_schemas

```
2. Run the docker compose file
```
docker-compose up -d
```
3. Enter the clickhouse client 
docker exec -it  clickhouse /bin/bash

clickhouse-client

USE blockchain;

SELECT * FROM utxos LIMIT 1;

SELECT SUM(value) FROM (SELECT value FROM utxos LIMIT 1000000);

SELECT  COUNT(*) AS total_unique_rows  FROM ( SELECT uniqExact(tuple(height, txid, vout)) AS total_unique_rows FROM blockchain.utxos ) AS subquery

```