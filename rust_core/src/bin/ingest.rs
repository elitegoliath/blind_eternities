// rust_core/src/bin/ingest.rs

use futures_util::StreamExt;
use reqwest::Client;
use serde::{de::{SeqAccess, Visitor}, Deserialize, Serialize, Deserializer};
use serde_json::Value;
use std::fmt;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

// --- 1. Define the Data Structure ---
// We only keep fields relevant to gameplay rules.
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
    
    // "set_type" helps us filter out Un-sets (joke cards)
    set_type: String, 
    
    legalities: Legalities,
}

#[derive(Debug, Serialize, Deserialize)]
struct Legalities {
    // We check this to ensure we don't index illegal cards if we don't want to
    commander: String, 
    vintage: String,
}

// --- 1. The Stream Logic ---
// We define a custom Visitor that processes elements one by one as they are parsed.
struct CardArrayVisitor<'a> {
    writer: &'a mut BufWriter<File>,
    valid_count: &'a mut usize,
    skipped_count: &'a mut usize,
}

impl<'a, 'de> Visitor<'de> for CardArrayVisitor<'a> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a JSON array of Scryfall cards")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        // This loop pulls one object at a time from the stream
        while let Some(card) = seq.next_element::<ScryfallCard>()? {
            
            // FILTER: Remove Un-sets, Tokens, etc.
            if card.set_type == "funny" || card.set_type == "token" || card.set_type == "memorabilia" {
                *self.skipped_count += 1;
                continue;
            }

            // Write to JSONL
            serde_json::to_writer(&mut *self.writer, &card).map_err(serde::de::Error::custom)?;
            self.writer.write_all(b"\n").map_err(serde::de::Error::custom)?;
            
            *self.valid_count += 1;
            if *self.valid_count % 5000 == 0 {
                print!("\rProcessed: {} | Skipped: {}", self.valid_count, self.skipped_count);
                let _ = std::io::stdout().flush();
            }
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(">>> Initializing Scryfall Ingestion...");

    // Create a Client with a User-Agent (Mandatory for Scryfall)
    let client = Client::builder()
        .user_agent("BlindEternitiesEngine/1.0")
        .build()?;

    // --- Step 1: Get the Download Link ---
    println!(">>> Fetching metadata...");
    
    let response = client
        .get("https://api.scryfall.com/bulk-data/oracle-cards")
        .send()
        .await?;

    // Check for HTTP errors (403/404/429) before parsing
    if !response.status().is_success() {
        panic!("Scryfall API Error: {} - check your connection or User-Agent.", response.status());
    }

    let bulk_meta: Value = response.json().await?;

    // Debug print to see what we actually got if it fails again
    // println!("DEBUG API Response: {:?}", bulk_meta);

    let download_uri = bulk_meta["download_uri"]
        .as_str()
        .expect("Failed to find 'download_uri'. API response format may have changed.");
    
    println!(">>> Target acquired: {}", download_uri);

    // --- Step 2: Download Stream to Disk ---
    // We stream this to a temp file because it's too big to hold in RAM safely.
    let temp_file_path = "scryfall_raw.json";
    
    if Path::new(temp_file_path).exists() {
        println!(">>> Temp file exists. Skipping download (delete '{}' to force refresh).", temp_file_path);
    } else {
        println!(">>> Downloading raw JSON stream (approx 200MB)...");
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
    println!(">>> Parsing and Filtering...");
    
    let file = File::open("scryfall_raw.json")?;
    let reader = std::io::BufReader::new(file);
    let mut output_file = File::create("processed_cards.jsonl")?;
    let mut writer = BufWriter::new(output_file);
    
    let mut valid_cards = 0;
    let mut skipped_cards = 0;

    // We manually construct the Deserializer
    let mut deserializer = serde_json::Deserializer::from_reader(reader);

    // We tell Serde: "Expect a Sequence (Array), and give me control inside the loop"
    deserializer.deserialize_seq(CardArrayVisitor {
        writer: &mut writer,
        valid_count: &mut valid_cards,
        skipped_count: &mut skipped_cards,
    })?;

    println!("\n>>> Ingestion Complete.");
    println!(">>> Database ready: {} valid cards saved.", valid_cards);
    
    Ok(())
}
