#!/bin/bash

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Check if required commands are available
if ! command_exists cleos; then
    echo "Error: cleos is not installed or not in PATH"
    exit 1
fi

if ! command_exists sha256sum; then
    echo "Error: sha256sum is not installed"
    exit 1
fi

# Function to get contract hashes
get_contract_hashes() {
    local account=$1
    local api_endpoint=$2

    # Get ABI
    abi_json=$(cleos -u $api_endpoint get abi $account)
    if [ $? -ne 0 ]; then
        echo "Error: Failed to get ABI for $account"
        return 1
    fi

    # Extract ABI from JSON and calculate hash
    abi_hash=$(echo $abi_json | jq -r '.abi' | jq -c . | sha256sum | awk '{print $1}')

    # Get code hash
    code_hash=$(cleos -u $api_endpoint get code $account | grep "code hash" | awk '{print $3}')

    echo "Contract: $account"
    echo "ABI Hash: $abi_hash"
    echo "Code Hash: $code_hash"
}

# Main script
if [ "$#" -ne 2 ]; then
    echo "Usage: $0 <contract_account> <api_endpoint>"
    echo "Example: $0 eosio.token https://rpc.xxx.com"
    exit 1
fi

contract_account=$1
api_endpoint=$2

get_contract_hashes $contract_account $api_endpoint