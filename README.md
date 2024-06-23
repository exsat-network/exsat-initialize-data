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

```
go mod tidy
```