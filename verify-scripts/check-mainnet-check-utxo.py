import csv
import asyncio
import aiohttp
import logging
import json
import os
import time
from collections import defaultdict
from aiohttp import ClientSession, TCPConnector
from asyncio import Semaphore

 
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')

INPUT_CSV = "data_main.csv"
OUTPUT_CSV = "cleaned_data_main.csv"
CHECKPOINT_FILE = "checkpoint-data.json"
API_URLS = [
    "https://rpc-us.exsat.network/v1/chain/get_table_rows",
    "https://as-node.defibox.xyz/v1/chain/get_table_rows"
]
BATCH_SIZE = 1000
MAX_CONCURRENT_REQUESTS = 20
WRITE_BUFFER_SIZE = 10000
CHECKPOINT_INTERVAL = 10000
RATE_LIMIT = 5
PROGRESS_UPDATE_INTERVAL = 10   

semaphore = Semaphore(MAX_CONCURRENT_REQUESTS)
rate_limiter = Semaphore(RATE_LIMIT)

async def fetch_missing_data(session, api_url, id, retries=3):
    payload = {
        "json": True,
        "code": "utxomng.xsat",
        "scope": "utxomng.xsat",
        "table": "utxos",
        "lower_bound": str(id),
        "upper_bound": str(id),
        "limit": 1,
        "reverse": False,
        "show_payer": False
    }
    
    for attempt in range(retries):
        try:
            async with semaphore, rate_limiter:
                async with session.post(api_url, json=payload, timeout=30) as response:
                    if response.status == 200:
                        data = await response.json()
                        if data['rows'] and 'data' in data['rows'][0]:
                            row = data['rows'][0]['data']
                            return [row['id'], row['txid'], row['index'], row['scriptpubkey'], row['value']]
                    elif response.status == 429:  # Too Many Requests
                        wait_time = int(response.headers.get('Retry-After', 60))
                        logging.warning(f"Rate limit exceeded. Waiting for {wait_time} seconds.")
                        await asyncio.sleep(wait_time)
                    else:
                        logging.warning(f"Received status {response.status} for ID {id}")
            await asyncio.sleep(1)   
        except Exception as e:
            logging.error(f"Error fetching data for ID {id}: {e}")
            if attempt < retries - 1:
                await asyncio.sleep(2 ** attempt)   
            else:
                logging.error(f"Failed to fetch data for ID {id} after {retries} attempts")
    return None

async def process_batch(session, api_url, batch, missing_ids):
    tasks = []
    for id in missing_ids:
        task = asyncio.ensure_future(fetch_missing_data(session, api_url, id))
        tasks.append(task)
    
    results = await asyncio.gather(*tasks, return_exceptions=True)
    return [result for result in results if result is not None and not isinstance(result, Exception)]

def save_checkpoint(last_processed_id, total_processed):
    with open(CHECKPOINT_FILE, 'w') as f:
        json.dump({
            "last_processed_id": last_processed_id,
            "total_processed": total_processed
        }, f)

def load_checkpoint():
    if os.path.exists(CHECKPOINT_FILE):
        with open(CHECKPOINT_FILE, 'r') as f:
            return json.load(f)
    return None

def print_progress_bar(iteration, total, prefix='', suffix='', decimals=1, length=50, fill='█', print_end="\r"):
    """
    Call in a loop to create terminal progress bar
    @params:
        iteration   - Required  : current iteration (Int)
        total       - Required  : total iterations (Int)
        prefix      - Optional  : prefix string (Str)
        suffix      - Optional  : suffix string (Str)
        decimals    - Optional  : positive number of decimals in percent complete (Int)
        length      - Optional  : character length of bar (Int)
        fill        - Optional  : bar fill character (Str)
        print_end   - Optional  : end character (e.g. "\r", "\r\n") (Str)
    """
    percent = ("{0:." + str(decimals) + "f}").format(100 * (iteration / float(total)))
    filled_length = int(length * iteration // total)
    bar = fill * filled_length + '-' * (length - filled_length)
    print(f'\r{prefix} |{bar}| {percent}% {suffix}', end=print_end)
    if iteration == total: 
        print()

async def main():
    start_time = time.time()
    api_index = 0
    write_buffer = []
    
    checkpoint = load_checkpoint()
    last_processed_id = checkpoint["last_processed_id"] if checkpoint else 0
    total_processed = checkpoint["total_processed"] if checkpoint else 0

    connector = TCPConnector(limit_per_host=MAX_CONCURRENT_REQUESTS)
    async with ClientSession(connector=connector) as session:
        with open(INPUT_CSV, 'r') as infile, open(OUTPUT_CSV, 'a' if checkpoint else 'w', newline='') as outfile:
            reader = csv.reader(infile)
            writer = csv.writer(outfile)
            
            if not checkpoint:
                header = next(reader)
                writer.writerow(header)
            else:
                
                for _ in range(total_processed):
                    next(reader, None)
            
            batch = []
            id_set = set()
            last_progress_update = time.time()
            
          
            total_rows = sum(1 for _ in reader)
            infile.seek(0)
            next(reader)   
            
            for row in reader:
                current_id = int(row[0])
                if current_id <= last_processed_id:
                    continue

                batch.append(row)
                id_set.add(current_id)
                
                 
                if time.time() - last_progress_update > PROGRESS_UPDATE_INTERVAL:
                    logging.info(f"Current ID: {current_id}, Processed: {total_processed}, Batch size: {len(batch)}")
                    print_progress_bar(total_processed, total_rows, prefix='Progress:', suffix='Complete', length=50)
                    last_progress_update = time.time()
                
                if len(batch) >= BATCH_SIZE:
                    min_id, max_id = int(batch[0][0]), int(batch[-1][0])
                    missing_ids = set(range(min_id, max_id + 1)) - id_set
                    
                    if missing_ids:
                        api_url = API_URLS[api_index]
                        api_index = (api_index + 1) % len(API_URLS)
                        logging.info(f"Fetching missing data for IDs {min(missing_ids)} to {max(missing_ids)}")
                        fetched_data = await process_batch(session, api_url, batch, missing_ids)
                        batch.extend(fetched_data)
                    
                    batch.sort(key=lambda x: int(x[0]))
                    write_buffer.extend(batch)
                    
                    if len(write_buffer) >= WRITE_BUFFER_SIZE:
                        writer.writerows(write_buffer)
                        total_processed += len(write_buffer)
                        last_processed_id = int(write_buffer[-1][0])
                        logging.info(f"Processed {total_processed} records. Last ID: {last_processed_id}")
                        write_buffer = []

                        if total_processed % CHECKPOINT_INTERVAL == 0:
                            save_checkpoint(last_processed_id, total_processed)
                    
                    batch = []
                    id_set = set()
            
         
            if batch:
                min_id, max_id = int(batch[0][0]), int(batch[-1][0])
                missing_ids = set(range(min_id, max_id + 1)) - id_set
                if missing_ids:
                    api_url = API_URLS[api_index]
                    logging.info(f"Fetching final missing data for IDs {min(missing_ids)} to {max(missing_ids)}")
                    fetched_data = await process_batch(session, api_url, batch, missing_ids)
                    batch.extend(fetched_data)
                batch.sort(key=lambda x: int(x[0]))
                write_buffer.extend(batch)
            
            if write_buffer:
                writer.writerows(write_buffer)
                total_processed += len(write_buffer)
                last_processed_id = int(write_buffer[-1][0])
    
  
    save_checkpoint(last_processed_id, total_processed)
    
    end_time = time.time()
    logging.info(f"Processing completed. Total records processed: {total_processed}")
    logging.info(f"Total time taken: {end_time - start_time:.2f} seconds")
    print_progress_bar(total_processed, total_rows, prefix='Progress:', suffix='Complete', length=50)

if __name__ == "__main__":
    asyncio.run(main())
