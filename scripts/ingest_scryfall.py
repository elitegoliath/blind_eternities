import os
import json
import time
import sqlite3
import requests

import gzip

DATA_DIR = "data"
RAW_DIR = os.path.join(DATA_DIR, "raw")
DB_PATH = os.path.join(DATA_DIR, "mtg_database.sqlite")
SCRYFALL_BULK_URL = "https://api.scryfall.com/bulk-data"
HEADERS = {
    "User-Agent": "BlindEternities/1.0",
    "Accept": "application/json"
}

def ensure_directories():
    os.makedirs(RAW_DIR, exist_ok=True)

def download_if_needed(target_type):
    # 1. Fetch bulk data manifest
    print(f"Fetching bulk data manifest for {target_type}...")
    resp = requests.get(SCRYFALL_BULK_URL, headers=HEADERS)
    resp.raise_for_status()
    bulk_data = resp.json()["data"]
    
    # 2. Find target
    target_info = next((item for item in bulk_data if item["type"] == target_type), None)
    if not target_info:
        raise ValueError(f"Could not find bulk data type: {target_type}")
        
    download_uri = target_info.get("jsonl_download_uri")
    if not download_uri:
        download_uri = target_info.get("download_uri") # Fallback
    local_path = os.path.join(RAW_DIR, f"{target_type}.jsonl.gz")
    
    # 3. Check cache
    if os.path.exists(local_path):
        file_age = time.time() - os.path.getmtime(local_path)
        if file_age < 86400: # 24 hours
            print(f"Cached {target_type} is less than 24 hours old. Skipping download.")
            return local_path
            
    # 4. Download
    print(f"Downloading {target_type} from {download_uri}...")
    # stream it to avoid large memory spikes
    with requests.get(download_uri, headers=HEADERS, stream=True) as r:
        r.raise_for_status()
        with open(local_path, 'wb') as f:
            for chunk in r.iter_content(chunk_size=8192):
                f.write(chunk)
                
    print(f"Downloaded {target_type} successfully.")
    return local_path

def ingest_to_sqlite(cards_path, rulings_path):
    print(f"Connecting to SQLite database at {DB_PATH}...")
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()
    
    # Create Tables
    cursor.execute('''
        CREATE TABLE IF NOT EXISTS cards (
            id TEXT PRIMARY KEY,
            oracle_id TEXT,
            name TEXT,
            type_line TEXT,
            mana_cost TEXT,
            oracle_text TEXT,
            power TEXT,
            toughness TEXT,
            cmc REAL
        )
    ''')
    
    cursor.execute('''
        CREATE TABLE IF NOT EXISTS rulings (
            oracle_id TEXT,
            date TEXT,
            text TEXT,
            FOREIGN KEY(oracle_id) REFERENCES cards(oracle_id)
        )
    ''')
    
    # Clear existing data for a fresh ingest
    cursor.execute('DELETE FROM cards')
    cursor.execute('DELETE FROM rulings')
    
    # Ingest Cards
    print("Ingesting Oracle Cards...")
    card_tuples = []
    with gzip.open(cards_path, 'rt', encoding='utf-8') as f:
        for line in f:
            c = json.loads(line)
            card_tuples.append((
                c.get('id'),
                c.get('oracle_id'),
                c.get('name'),
                c.get('type_line'),
                c.get('mana_cost'),
                c.get('oracle_text'),
                c.get('power'),
                c.get('toughness'),
                c.get('cmc', 0.0)
            ))
        
    cursor.executemany('''
        INSERT INTO cards (id, oracle_id, name, type_line, mana_cost, oracle_text, power, toughness, cmc)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
    ''', card_tuples)
    
    # Ingest Rulings
    print("Ingesting Rulings...")
    ruling_tuples = []
    with gzip.open(rulings_path, 'rt', encoding='utf-8') as f:
        for line in f:
            r = json.loads(line)
            ruling_tuples.append((
                r.get('oracle_id'),
                r.get('published_at'),
                r.get('comment')
            ))
        
    cursor.executemany('''
        INSERT INTO rulings (oracle_id, date, text)
        VALUES (?, ?, ?)
    ''', ruling_tuples)
    
    # Create indices for fast RAG lookups
    print("Creating indices...")
    cursor.execute('CREATE INDEX IF NOT EXISTS idx_cards_name ON cards(name)')
    cursor.execute('CREATE INDEX IF NOT EXISTS idx_rulings_oracle_id ON rulings(oracle_id)')
    
    conn.commit()
    conn.close()
    print("Ingestion complete!")

def test_ingestion():
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()
    
    test_card = "Humility"
    print(f"\n--- Testing Query for: {test_card} ---")
    
    cursor.execute("SELECT id, oracle_id, name, type_line, oracle_text FROM cards WHERE name = ?", (test_card,))
    card = cursor.fetchone()
    
    if not card:
        print(f"Card {test_card} not found!")
        return
        
    print(f"Name: {card[2]}")
    print(f"Type: {card[3]}")
    print(f"Text: {card[4]}")
    
    oracle_id = card[1]
    cursor.execute("SELECT date, text FROM rulings WHERE oracle_id = ? ORDER BY date ASC", (oracle_id,))
    rulings = cursor.fetchall()
    
    print(f"\nRulings ({len(rulings)}):")
    for date, text in rulings:
        print(f"[{date}] {text}")
        
    conn.close()

if __name__ == "__main__":
    ensure_directories()
    try:
        cards_path = download_if_needed("oracle_cards")
        rulings_path = download_if_needed("rulings")
        ingest_to_sqlite(cards_path, rulings_path)
        test_ingestion()
    except Exception as e:
        print(f"Error during ingestion: {e}")