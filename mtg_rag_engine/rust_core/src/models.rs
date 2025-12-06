// mtg_rag_engine/rust_core/src/models.rs
// Models for Magic: The Gathering game state representation in Rust.
// This is where Rust shines. We don't use strings for phases or colors; we use Enums.
// This makes "illegal states" unrepresentable. If you try to create a card with
// the color "Purple," the code won't even compile (or deserialize).

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

// 1. Strict Enums for Game Concepts
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub enum Color {
    White, Blue, Black, Red, Green, Colorless
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum CardType {
    Artifact, Creature, Enchantment, Instant, Land, Planeswalker, Sorcery
}

// 2. The "Card" Object
// This represents a permanent on the battlefield
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Permanent {
    pub id: String,          // Unique ID (UUID)
    pub name: String,
    pub oracle_text: String,
    pub mana_value: u32,
    pub types: Vec<CardType>,
    pub colors: Vec<Color>,
    pub is_legendary: bool,
    pub controller: String,  // "Player" or "Opponent"
    
    // Mutable state tracked by the engine
    #[serde(default)]
    pub is_tapped: bool,
    #[serde(default)]
    pub damage_marked: u32,
}

// 3. The "Board State" Snapshot
// Python sends this entire object every time it asks a question
#[derive(Debug, Serialize, Deserialize)]
pub struct GameState {
    pub active_player: String,
    pub phase: String, // You could make this an Enum too (Main1, Combat, etc.)
    pub battlefield: Vec<Permanent>,
    pub stack: Vec<String>, // Simplified for now
}