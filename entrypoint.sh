#!/bin/bash
# entrypoint.sh: Startup logic for the Blind Eternities Python container

# Enable strict error checking
set -e

# Define paths
DATA_DIR="${LANCEDB_URI:-/app/data/lancedb}"
RAW_FILE="/app/scryfall_raw.json"
JSONL_FILE="/app/processed_cards.jsonl"

echo ">>> Checking vector database status at $DATA_DIR..."

# Check if LanceDB directory exists and is NOT empty
if [ ! -d "$DATA_DIR" ] || [ -z "$(ls -A "$DATA_DIR")" ]; then
    echo ">>> Database missing or empty. Starting ingestion pipeline..."

    # 1. Run Scryfall Ingestion (Downloads and filters data)
    echo ">>> Executing ingest binary..."
    cd /app/rust_core
    cargo run --release --bin ingest

    # 2. Run Vector Indexing (Embeds and builds LanceDB)
    echo ">>> Executing index binary..."
    cargo run --release --bin index
    cd /app

    # Optional: Clean up intermediate files to save space
    # rm -f "$RAW_FILE" "$JSONL_FILE"
    echo ">>> Ingestion pipeline complete."
else
    echo ">>> Vector database found at $DATA_DIR. Skipping ingestion."
fi

# Hand over control to the main CMD (starting the python agent)
# This replaces the shell process with the Python process
echo ">>> Booting Python Agent..."
exec "$@"