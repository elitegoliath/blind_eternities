// mtg_rag_engine/rust_core/src/lib.rs
// Rust core library for Magic: The Gathering rules engine.
// This library exposes a single function to Python via PyO3.
// The function takes a JSON string representing the game state
// and returns a JSON string with the rulings.
// This connects the wires. It deserializes the JSON string from Python,
// hands it to the Judge in rules.rs, converts the Ruling enum back
// to a string, and returns it.
use pyo3::prelude::*;
use pyo3::wrap_pyfunction;
use serde_json::json;

mod models;
mod rules;

use models::GameState;
use rules::{Judge, Ruling};

/// The single entry point exposed to Python
#[pyfunction]
fn check_board_state(json_payload: String) -> PyResult<String> {
    // 1. Parse JSON (Python -> Rust)
    let state: GameState = match serde_json::from_str(&json_payload) {
        Ok(s) => s,
        Err(e) => return Ok(json!({
            "status": "error",
            "message": format!("JSON Parse Error: {}", e)
        }).to_string()),
    };

    // 2. Run Logic (Pure Rust)
    let rulings = Judge::assess_state(&state);

    // 3. Format Response (Rust -> Python)
    // We transform the Rust Enums into a friendly JSON response
    let response = rulings.iter().map(|r| match r {
        Ruling::Legal => json!({ "status": "legal" }),
        Ruling::Illegal(reason) => json!({ "status": "illegal", "reason": reason }),
        Ruling::StateBasedAction(action) => json!({ "status": "sba_trigger", "action": action }),
    }).collect::<Vec<_>>();

    // Return the list of rulings
    Ok(serde_json::to_string(&response).unwrap())
}

#[pymodule]
fn mtg_logic_core(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(check_board_state, m)?)?;
    Ok(())
}
