#!/bin/bash

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Check if required commands are available
for cmd in cleos sha256sum jq xxd; do
    if ! command_exists $cmd; then
        echo "Error: $cmd is not installed or not in PATH"
        exit 1
    fi
done

# Function to get contract hashes
get_contract_hashes() {
    local account=$1
    local api_endpoint=$2

    echo "Processing contract: $account"

    # Get ABI
    abi_json=$(cleos -u $api_endpoint get abi $account)
    if [ $? -ne 0 ]; then
        echo "Error: Failed to get ABI for $account"
        return 1
    fi

    # Extract ABI from JSON, convert to hex, then calculate hash
    abi_hash=$(echo $abi_json | jq -r '.abi' | jq -c . | tr -d '[:space:]' | sha256sum | awk '{print $1}')

    # Get code hash
    code_hash=$(cleos -u $api_endpoint get code $account | grep "code hash" | awk '{print $3}')

    echo "  ABI Hash: $abi_hash"
    echo "  Code Hash: $code_hash"
    echo "-------------------"
}

# Main script
if [ "$#" -lt 2 ]; then
    echo "Usage: $0 <api_endpoint> <contract_account1> [contract_account2 ...]"
    echo "Example: $0 https://eos.greymass.com eosio.token eosio.system"
    exit 1
fi

api_endpoint=$1
shift  # Remove the first argument (API endpoint) from the list

# Process each contract account
for contract in "$@"; do
    get_contract_hashes $contract $api_endpoint
done