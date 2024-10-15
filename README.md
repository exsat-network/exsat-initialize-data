# exSat Initialize Data

This repository contains tools and scripts for initializing exSat with UTXO data, fetching block header data, and verifying data. It includes methods for retrieving UTXO data from a local RocksDB, fetching Bitcoin block headers from a node, and comparing data between different sources.

## Overview

- **Fetch UTXOs From ElectrumX**: Fetch UTXOs from ElectrumX and save in Clickhouse.
- **Block Header Fetcher**: Retrieves Bitcoin block headers and saves them to a CSV file.



## Set up the btc fullnode

1. Setup btc fullnode by [script](./setup-bitcoin-fullnode.sh).

    The following are the minimum configuration requirements for running a BTC full node: (The configuration requirements are not high, mainly disk)

    CPU: 4+ cores

    RAM: 8 GB memory

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

exit

sudo -s

then 

./setup-bitcoin-fullnode.sh

# select 2 run_btc_full_node

```

## Set up the mainnet btc prune node

1. Setup btc mainnet prunenode by [script](./setup-bitcoin-prunenode.sh).

    The following are the minimum configuration requirements for running a BTC prune node: (The configuration requirements are not high, mainly disk)

    CPU: 4+ cores

    RAM: 8 GB memory

    SSD: 50G

    System: Ubuntu 22.04

    Traffic bandwidth: 50M bandwidth and above

    Task time: 5 minutes;


```

sudo -s
chmod +x ./setup-bitcoin-prunenode.sh

./setup-bitcoin-prunenode.sh

# select 1 install_btc_prune_node

exit

sudo -s

then 

./setup-bitcoin-prunenode.sh

# select 2 run_btc_prune_node

```
## Prerequisites
r4.2xlarge aws

CPU: 4 GHz | 4+ cores

RAM: 128 GB memory

SSD: 50G

Disk: 1.5TB or more

System: Ubuntu 22.04


## Set up the RUST Environment
Before running the programs, ensure you have Rust installed on your system. You can install Rust using the following command:

1. Setup rust env by [script](./setup-rust.sh).


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
1. Run the [fullnode](https://github.com/exsat-network/bitcoin) and make it sync to 839999.
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

## #4 Verify Process

Folks can pull all the initial data from the Spring mainnet before exSat mainnet launch. 
1. [Pull UTXO data on the Spring mainnet](./verify-scripts/fetch-mainnet-utxo.py)
2. [Pull Block Header data on the Spring mainnet](./verify-scripts/fetch-mainnet-blocks.py)
3. [Pull UTXO data from the Bitcoin node](./fetch_utxos_from_eletrumx/)
4. [Pull Block Header data from the Bitcoin node](./fetch_bitcoin_blockhaeder/)
5. Data comparison

You can set a [spring node](https://github.com/eosnetworkfoundation/evm-public-docs/tree/taokayan-exsat-doc/deployment_for_exSat#create-a-256gb-swap-and-240gb-tmpfs-system-to-hold-the-native-blockchain-state) to import the snapshot if the exSat mainnet is launched, because the UTXO data will change afterward. 

1. Before you run the node, you should change the data-dir/config.ini and comment the peers' info:
```yaml
# 180GB chain-base size, using swap & tmpfs
chain-state-db-size-mb = 184320
access-control-allow-credentials = false

allowed-connection = any
p2p-listen-endpoint = 0.0.0.0:9876
p2p-max-nodes-per-host = 10
http-server-address = 0.0.0.0:8888
state-history-endpoint = 0.0.0.0:8999

trace-history = true
chain-state-history = false
http-max-response-time-ms = 1000

# add/remove p2p peers if necessary
#p2p-peer-address=xx.xx.xx.xx:9882
#p2p-peer-address=xx.xx.xx.xx:9876
#p2p-peer-address=xx.xx.xx.xx:9876

max-transaction-time = 499
read-only-read-window-time-us = 1000000
transaction-retry-max-storage-size-gb = 1

# Plugin(s) to enable, may be specified multiple times
plugin = eosio::producer_plugin
plugin = eosio::chain_api_plugin
plugin = eosio::http_plugin
plugin = eosio::producer_api_plugin
plugin = eosio::state_history_plugin
plugin = eosio::net_plugin
plugin = eosio::net_api_plugin
plugin = eosio::db_size_api_plugin
```

2. Modify these config in [fetch-mainnet-blocks](verify-scripts/fetch-mainnet-blocks.py) &  [ fetch-mainnet-utxo](verify-scripts/fetch-mainnet-utxo.py)
from 
```javascript
API_URLS = [
    "https://rpc-us.exsat.network/v1/chain/get_table_rows",
    "https://as-node.defibox.xyz/v1/chain/get_table_rows"
]
```

to 
```javascript
API_URLS = [
    "http://127.0.0.1:8888/v1/chain/get_table_rows",
]
```
