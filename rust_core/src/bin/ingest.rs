// rust_core/src/bin/ingest.rs

use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;
use flate2::read::GzDecoder;

// --- 1. Define the Data Structure ---
#[derive(Debug, Serialize, Deserialize)]
struct ScryfallCard {
    id: String,
    name: String,
    #[serde(default)]
    mana_cost: String,
    #[serde(default)]
    type_line: String,
    #[serde(default)]
    oracle_text: String,
    set_type: String, 
    legalities: Legalities,
}

#[derive(Debug, Serialize, Deserialize)]
struct Legalities {
    commander: String, 
    vintage: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(">>> Initializing Scryfall Ingestion...");

    let client = Client::builder()
        .user_agent("BlindEternitiesEngine/1.0")
        .build()?;

    // --- Step 1: Get the Download Link ---
    println!(">>> Fetching metadata...");
    
    // Using the modern underscore format for the endpoint
    let response = client
        .get("https://api.scryfall.com/bulk-data/oracle_cards")
        .send()
        .await?;

    if !response.status().is_success() {
        panic!("Scryfall API Error: {} - check your connection or User-Agent.", response.status());
    }

    let bulk_meta: Value = response.json().await?;

    // THE FIX: Use the new JSONL download URI key
    let download_uri = bulk_meta["jsonl_download_uri"]
        .as_str()
        .expect("Failed to find 'jsonl_download_uri'. API response format changed.");
    
    println!(">>> Target acquired: {}", download_uri);

    // --- Step 2: Download Stream to Disk ---
    let temp_file_path = "scryfall_raw.jsonl.gz";
    
    if Path::new(temp_file_path).exists() {
        println!(">>> Temp file exists. Skipping download (delete '{}' to force refresh).", temp_file_path);
    } else {
        println!(">>> Downloading gzipped JSONL stream...");
        let response = client.get(download_uri).send().await?;
        let mut file = File::create(temp_file_path)?;
        let mut stream = response.bytes_stream();

        while let Some(item) = stream.next().await {
            let chunk = item?;
            file.write_all(&chunk)?;
        }
        println!(">>> Download complete.");
    }

    // --- Step 3: Stream-Parse and Filter ---
    println!(">>> Decompressing, Parsing and Filtering...");
    
    let file = File::open("scryfall_raw.jsonl.gz")?;
    
    // Automatically decompress the gzip stream on the fly
    let gz = GzDecoder::new(file);
    let reader = BufReader::new(gz);
    
    let output_file = File::create("processed_cards.jsonl")?;
    let mut writer = BufWriter::new(output_file);
    
    let mut valid_cards = 0;
    let mut skipped_cards = 0;

    // Because it's JSONL, we can just read it line by line directly!
    for line_result in reader.lines() {
        let line = line_result?;
        if line.trim().is_empty() { continue; }

        // Attempt to parse the line into our ScryfallCard struct
        if let Ok(card) = serde_json::from_str::<ScryfallCard>(&line) {
            
            // FILTER: Remove Un-sets, Tokens, etc.
            if card.set_type == "funny" || card.set_type == "token" || card.set_type == "memorabilia" {
                skipped_cards += 1;
                continue;
            }

            serde_json::to_writer(&mut writer, &card)?;
            writer.write_all(b"\n")?;
            
            valid_cards += 1;
            if valid_cards % 5000 == 0 {
                print!("\rProcessed: {} | Skipped: {}", valid_cards, skipped_cards);
                let _ = std::io::stdout().flush();
            }
        }
    }

    println!("\n>>> Ingestion Complete.");
    println!(">>> Database ready: {} valid cards saved.", valid_cards);
    
    Ok(())
}