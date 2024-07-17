import os
import time
import requests
import json
from concurrent.futures import ThreadPoolExecutor, as_completed

# Configuration for Bitcoin Core RPC
RPC_USER = 'exSat'
RPC_PASSWORD = 'exSat123'
RPC_PORT = '8332'
RPC_HOST = '127.0.0.1'
RPC_URL = f'http://{RPC_USER}:{RPC_PASSWORD}@{RPC_HOST}:{RPC_PORT}'

CACHE_FILE = "block_height_cache.txt"
HEADERS = {'content-type': 'application/json'}
MAX_WORKERS = 10  # Number of concurrent threads

def rpc_request(method, params=None):
    payload = json.dumps({
        "method": method,
        "params": params or [],
        "jsonrpc": "2.0",
        "id": 0,
    })
    response = requests.post(RPC_URL, headers=HEADERS, data=payload)
    response_json = response.json()
    if 'error' in response_json and response_json['error'] is not None:
        raise Exception(response_json['error'])
    return response_json['result']

def get_block_hash(block_height):
    return rpc_request('getblockhash', [block_height])

def get_block(block_hash):
    return rpc_request('getblock', [block_hash, 2])

def get_raw_transaction(txid):
    return rpc_request('getrawtransaction', [txid, True])

def calculate_miner_fee_for_tx(tx):
    input_sum = sum(get_raw_transaction(vin['txid'])['vout'][vin['vout']]['value'] for vin in tx['vin'])
    output_sum = sum(vout['value'] for vout in tx['vout'])
    fee = input_sum - output_sum
    return fee

def calculate_miner_fee(block_data):
    total_fee = 0
    transactions = [tx for tx in block_data['tx'] if 'coinbase' not in tx['vin'][0]]

    with ThreadPoolExecutor(max_workers=MAX_WORKERS) as executor:
        futures = [executor.submit(calculate_miner_fee_for_tx, tx) for tx in transactions]
        for future in as_completed(futures):
            total_fee += future.result()

    return total_fee

def read_cache():
    if os.path.exists(CACHE_FILE):
        with open(CACHE_FILE, 'r') as file:
            last_block = file.readline().strip()
            if last_block.isdigit():
                return int(last_block)
    return 0

def write_cache(block_height):
    with open(CACHE_FILE, 'w') as file:
        file.write(str(block_height))

def main():
    start_block = read_cache()
    end_block = 839999
    total_fee_all_blocks = 0

    for block_height in range(start_block, end_block + 1):
        try:
            block_hash = get_block_hash(block_height)
            block_data = get_block(block_hash)
            total_fee = calculate_miner_fee(block_data)
            total_fee_all_blocks += total_fee
            print(f"Block {block_height}: {total_fee} BTC")
            
            # Write the current block height to cache
            write_cache(block_height)

        except Exception as e:
            print(f"Error fetching data for block {block_height}: {e}")
            break

    print(f"Total miner fee for blocks {start_block} to {end_block}: {total_fee_all_blocks} BTC")

if __name__ == "__main__":
    main()
