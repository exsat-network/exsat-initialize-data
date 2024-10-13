import requests
import json
import csv
import time
import logging
import argparse
import os
from concurrent.futures import ThreadPoolExecutor, as_completed

 
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')

API_URLS = [
    "https://rpc-us.exsat.network/v1/chain/get_table_rows",
    "https://as-node.defibox.xyz/v1/chain/get_table_rows"
]
OUTPUT_FILE = "data_main.csv"
CHECKPOINT_FILE = "checkpoint-main.json"
BATCH_SIZE = 1000
MAX_RETRIES = 5
INITIAL_RETRY_DELAY = 1
MAX_RETRY_DELAY = 60
RANGE_SIZE = 1000000   
MAX_ID = 176944794   

def fetch_data(api_url, lower_bound, upper_bound, retry_count=0):
    payload = {
        "json": True,
        "code": "utxomng.xsat",
        "scope": "utxomng.xsat",
        "table": "utxos",
        "lower_bound": str(lower_bound),
        "upper_bound": str(upper_bound),
        "index_position": 1,
        "key_type": "",
        "limit": str(BATCH_SIZE),
        "reverse": False,
        "show_payer": False
    }

    try:
        response = requests.post(api_url, json=payload, timeout=30)
        response.raise_for_status()
        return response.json()
    except requests.RequestException as e:
        if retry_count < MAX_RETRIES:
            delay = min(INITIAL_RETRY_DELAY * (2 ** retry_count), MAX_RETRY_DELAY)
            logging.warning(f"Request failed for {api_url}. Retrying in {delay:.2f} seconds... (Attempt {retry_count + 1}/{MAX_RETRIES})")
            time.sleep(delay)
            return fetch_data(api_url, lower_bound, upper_bound, retry_count + 1)
        else:
            logging.error(f"Failed to fetch data from {api_url} after {MAX_RETRIES} attempts: {e}")
            return None

def process_batch(api_url, lower_bound, upper_bound):
    response_data = fetch_data(api_url, lower_bound, upper_bound)
    if not response_data:
        return None, lower_bound, False

    rows = []
    for item in response_data.get('rows', []):
        data = item.get('data', item)
        rows.append({
            'id': int(data['id']),
            'txid': data['txid'],
            'index': int(data['index']),
            'scriptpubkey': data['scriptpubkey'],
            'value': int(data['value'])
        })

    more = response_data.get('more', False)
    next_key = response_data.get('next_key', '')

    if next_key == '':
        logging.warning(f"Empty next_key received for range {lower_bound}-{upper_bound}. Using upper_bound as next_key.")
        next_key = str(upper_bound)

    try:
        next_key = int(next_key)
    except ValueError:
        logging.error(f"Invalid next_key received: '{next_key}'. Using upper_bound as next_key.")
        next_key = upper_bound

    return rows, next_key, more

def save_checkpoint(ranges, total_processed):
    with open(CHECKPOINT_FILE, 'w') as f:
        json.dump({"ranges": ranges, "total_processed": total_processed}, f)

def load_checkpoint():
    try:
        with open(CHECKPOINT_FILE, 'r') as f:
            return json.load(f)
    except FileNotFoundError:
        return None

def main(start_id=None, max_records=None, continue_on_error=False):
    checkpoint = load_checkpoint() if start_id is None else None

    if checkpoint:
        ranges = checkpoint["ranges"]
        total_processed = checkpoint["total_processed"]
        logging.info(f"Resuming from checkpoint. Starting at ranges: {ranges}")
    else:
        start_id = int(start_id or 1)
        ranges = [(start_id + i * RANGE_SIZE, min(start_id + (i + 1) * RANGE_SIZE - 1, MAX_ID)) for i in range(len(API_URLS))]
        total_processed = 0

    start_time = time.time()
    more = True

    file_exists = os.path.exists(OUTPUT_FILE) and os.path.getsize(OUTPUT_FILE) > 0

    with open(OUTPUT_FILE, 'a', newline='') as csvfile:
        writer = csv.writer(csvfile)

        if not file_exists:
            writer.writerow(['id', 'txid', 'index', 'scriptpubkey', 'value'])

        with ThreadPoolExecutor(max_workers=len(API_URLS)) as executor:
            while more and (max_records is None or total_processed < max_records):
                futures = []
                for i, api_url in enumerate(API_URLS):
                    lower, upper = ranges[i]
                    if lower > MAX_ID:
                        continue
                    futures.append(executor.submit(process_batch, api_url, lower, upper))

                all_rows = []
                for future in as_completed(futures):
                    try:
                        result = future.result()
                        if result[0] is None:
                            if not continue_on_error:
                                logging.error("Critical error occurred. Stopping the process.")
                                return
                            continue

                        rows, next_key, batch_more = result
                        all_rows.extend(rows)

                        api_index = futures.index(future)
                        ranges[api_index] = (next_key, ranges[api_index][1])

                        if not batch_more or next_key > ranges[api_index][1]:
                            # Move to the next range
                            new_lower = ranges[api_index][1] + 1
                            new_upper = min(new_lower + RANGE_SIZE - 1, MAX_ID)
                            ranges[api_index] = (new_lower, new_upper)

                        more = more or batch_more

                    except Exception as e:
                        logging.error(f"Error processing batch: {e}")
                        if not continue_on_error:
                            raise

                sorted_rows = sorted(all_rows, key=lambda x: x['id'])
                writer.writerows([[row['id'], row['txid'], row['index'], row['scriptpubkey'], row['value']] for row in sorted_rows])
                total_processed += len(sorted_rows)
                csvfile.flush()

                save_checkpoint(ranges, total_processed)

                if max_records and total_processed >= max_records:
                    break

                logging.info(f"Processed {total_processed} unique records. Current ranges: {ranges}")
                time.sleep(0.1)  

               
                if all(lower > MAX_ID for lower, _ in ranges):
                    logging.info(f"Reached maximum ID of {MAX_ID}. Stopping the process.")
                    break

    end_time = time.time()
    logging.info(f"Finished processing {total_processed} unique records in {end_time - start_time:.2f} seconds")

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Fetch spring table data")
    parser.add_argument("--start_id", help="Starting ID for data fetch")
    parser.add_argument("--max_records", type=int, help="Maximum number of records to fetch")
    parser.add_argument("--continue_on_error", action="store_true", help="Continue fetching data even if some requests fail")
    args = parser.parse_args()

    main(args.start_id, args.max_records, args.continue_on_error)
