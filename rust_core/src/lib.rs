// rust_core/src/lib.rs
// Rust core library for Magic: The Gathering rules engine.
// This library exposes a single function to Python via PyO3.
// The function takes a JSON string representing the game state
// and returns a JSON string with the rulings.
// This connects the wires. It deserializes the JSON string from Python,
// hands it to the Judge in rules.rs, converts the Ruling enum back
// to a string, and returns it.

use pyo3::prelude::*;
use serde_json::json;
use std::sync::{OnceLock, Mutex};

// Imports for the Librarian (Search)
use arrow_array::{RecordBatch, StringArray};
use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};
use lancedb::connect;
use lancedb::query::{ExecutableQuery, QueryBase}; // Import the trait for .limit()
use futures::TryStreamExt;
use tokio::runtime::Runtime; // Import Runtime

mod models;
mod rules;

use models::GameState;
use rules::{Judge, Ruling};

// --- SINGLETONS ---

// 1. Embedding Model (Heavy: ~30MB)
static MODEL: OnceLock<Mutex<TextEmbedding>> = OnceLock::new();

fn get_model() -> &'static Mutex<TextEmbedding> {
    MODEL.get_or_init(|| {
        Mutex::new(TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::AllMiniLML6V2)
                .with_show_download_progress(false)
        ).expect("Failed to load Embedding Model"))
    })
}

// 2. Tokio Runtime (Heavy: Thread Pool)
// OPTIMIZATION: Created once, reused for all async calls.
static RUNTIME: OnceLock<Runtime> = OnceLock::new();

fn get_runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        Runtime::new().expect("Failed to create Tokio Runtime")
    })
}

// --- HELPER FUNCTIONS ---

// OPTIMIZATION: Safer Arrow Column Extraction
// This replaces the .unwrap() chains. It returns a Result so we can handle
// schema errors without crashing the Python interpreter.
fn get_string_column<'a>(batch: &'a RecordBatch, col_name: &str) -> Result<&'a StringArray, String> {
    let col = batch.column_by_name(col_name)
        .ok_or_else(|| format!("Column '{}' not found in LanceDB results", col_name))?;

    col.as_any()
        .downcast_ref::<StringArray>()
        .ok_or_else(|| format!("Column '{}' is not a StringArray (Type Mismatch)", col_name))
}


// --- THE JUDGE (Rule Engine) ---
#[pyfunction]
fn check_board_state(json_payload: String) -> PyResult<String> {
    let state: GameState = match serde_json::from_str(&json_payload) {
        Ok(s) => s,
        Err(e) => return Ok(json!({
            "status": "error",
            "message": format!("JSON Parse Error: {}", e)
        }).to_string()),
    };

    let rulings = Judge::assess_state(&state);

    let response = rulings.iter().map(|r| match r {
        Ruling::Legal => json!({ "status": "legal" }),
        Ruling::Illegal(reason) => json!({ "status": "illegal", "reason": reason }),
        Ruling::StateBasedAction(action) => json!({ "status": "sba_trigger", "action": action }),
    }).collect::<Vec<_>>();

    Ok(serde_json::to_string(&response).unwrap())
}

// --- THE LIBRARIAN (Vector Search) ---
#[pyfunction]
fn search_cards(query: String, limit: Option<usize>, where_clause: Option<String>) -> PyResult<String> {
    let limit = limit.unwrap_or(5);

    // 1. Generate Embedding
    let model = get_model();
    let mut model_lock = model.lock().unwrap();
    let query_embedding = model_lock.embed(vec![query], None)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
    
    let query_vector = query_embedding[0].clone();

    // 2. Run Async Search using Global Runtime
    let rt = get_runtime(); // <--- OPTIMIZATION: Use the static runtime
    
    let results_json = rt.block_on(async {
        // Connect
        let uri = "data/lancedb";
        let db = connect(uri).execute().await
            .map_err(|e| format!("DB Connection Failed: {}", e))?;
        
        let table = db.open_table("cards").execute().await
            .map_err(|e| format!("Table Open Failed: {}", e))?;

        // Initialize Query Builder
        let mut query_builder = table.query()
            .nearest_to(query_vector)
            .map_err(|e| format!("Invalid Query Vector: {}", e))?;

        // Apply Hybrid Filter
        if let Some(sql) = where_clause {
            // "filter" accepts standard SQL strings like "type_line LIKE '%Creature%'"
            query_builder = query_builder.only_if(sql);
        }

        let results = query_builder
            .limit(limit)
            .execute()
            .await
            .map_err(|e| format!("Query Execution Failed: {}", e))?
            .try_collect::<Vec<_>>()
            .await
            .map_err(|e| format!("Stream Collection Failed: {}", e))?;

        // Extract Data
        let mut found_cards = Vec::new();

        for batch in results {
            // OPTIMIZATION: Use the safer helper function with '?'
            // If the schema is wrong, this returns Err(String) immediately
            let names = get_string_column(&batch, "name")?;
            let texts = get_string_column(&batch, "oracle_text")?;
            let types = get_string_column(&batch, "type_line")?;

            for i in 0..batch.num_rows() {
                found_cards.push(json!({
                    "name": names.value(i),
                    "type": types.value(i),
                    "text": texts.value(i)
                }));
            }
        }

        Ok::<String, String>(serde_json::to_string(&found_cards).unwrap())
    });

    match results_json {
        Ok(json_str) => Ok(json_str),
        Err(err_msg) => Ok(json!({ "status": "error", "message": err_msg }).to_string())
    }
}

#[pymodule]
fn mtg_logic_core(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(check_board_state, m)?)?;
    m.add_function(wrap_pyfunction!(search_cards, m)?)?;
    Ok(())
}
