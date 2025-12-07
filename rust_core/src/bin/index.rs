// LanceDB Vector Indexer for MTG Cards using FastEmbed v5.x
// Updated for LanceDB 0.22+ API changes

use arrow_array::{FixedSizeListArray, Float32Array, RecordBatch, RecordBatchIterator, StringArray};
use arrow_schema::{DataType, Field, Schema};
use arrow_array::types::Float32Type;
use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};
use lancedb::connect;
use serde::Deserialize;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::sync::Arc;

#[derive(Debug, Deserialize)]
struct CardJson {
    name: String,
    oracle_text: String,
    type_line: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(">>> Initializing Vector Indexer (Modern Stack)...");

    // 1. Setup Embedding Model (FastEmbed v5.x)
    // The API changed from a struct literal to a builder pattern
    let mut model = TextEmbedding::try_new(
        InitOptions::new(EmbeddingModel::AllMiniLML6V2)
            .with_show_download_progress(true)
    )?;

    // 2. Connect to LanceDB
    // 0.22+ uses 'execute()' pattern for connections
    let uri = "data/lancedb";
    let db = connect(uri).execute().await?;
    
    // 3. Read Data
    println!(">>> Reading processed_cards.jsonl...");
    let file = File::open("processed_cards.jsonl")?;
    let reader = BufReader::new(file);

    let mut names = Vec::new();
    let mut texts = Vec::new(); // For display
    let mut types = Vec::new();
    let mut embeddings = Vec::new();

    let mut count = 0;
    
    for line in reader.lines() {
        let line = line?;
        // Robustness: Ignore empty lines or parse errors
        if let Ok(card) = serde_json::from_str::<CardJson>(&line) {
            // Skip cards with no text to save space
            if card.oracle_text.is_empty() { continue; }

            names.push(card.name.clone());
            texts.push(card.oracle_text.clone());
            types.push(card.type_line.clone());

            // Combine fields for richer semantic search
            let combined_text = format!("{} - {} \n {}", card.name, card.type_line, card.oracle_text);
            
            // Generate Vector
            let vector = model.embed(vec![combined_text], None)?;
            embeddings.push(vector[0].clone());

            count += 1;
            if count % 100 == 0 {
                print!("\rIndexing: {} cards...", count);
            }
            
            // Limit for testing (remove this line for full import)
            if count >= 1000 { break; } 
        }
    }

    println!("\n>>> Converting to Arrow format...");

    // 4. Create Arrow Schema & Batch
    // We use lancedb::arrow types to ensure version compatibility
    let schema = Arc::new(Schema::new(vec![
        Field::new("name", DataType::Utf8, false),
        Field::new("type_line", DataType::Utf8, false),
        Field::new("oracle_text", DataType::Utf8, false),
        Field::new("vector", DataType::FixedSizeList(
            Arc::new(Field::new("item", DataType::Float32, true)),
            384 // Dimension size for MiniLM
        ), false),
    ]));

    let total_rows = names.len();
    
    // Flatten embeddings for the FixedSizeListArray
    let flattened_embeddings: Vec<f32> = embeddings.into_iter().flatten().collect();
    
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(StringArray::from(names)),
            Arc::new(StringArray::from(types)),
            Arc::new(StringArray::from(texts)),
            Arc::new(FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                // We reconstruct the list array from the flattened data
                (0..total_rows).map(|i| {
                    Some(flattened_embeddings[i*384..(i+1)*384].to_vec().into_iter().map(Some))
                }),
                384
            )),
        ],
    )?;

    // 5. Write to DB
    // LanceDB 0.22 expects an Iterator of batches, not a single batch
    let batches = RecordBatchIterator::new(vec![Ok(batch)], schema.clone());

    println!(">>> Writing to LanceDB...");
    
    // 'create_table' now returns a builder, we call execute()
    db.create_table("cards", batches)
        .execute()
        .await?;

    println!(">>> Indexing Complete. Table 'cards' created at ./data/lancedb");
    Ok(())
}
