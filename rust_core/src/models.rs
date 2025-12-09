// rust_core/src/models.rs
// Models for Magic: The Gathering game state representation in Rust.
// This is where Rust shines. We don't use strings for phases or colors; we use Enums.
// This makes "illegal states" unrepresentable. If you try to create a card with
// the color "Purple," the code won't even compile (or deserialize).

use serde::{Deserialize, Serialize};

// --- ENUMS ---

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub enum Color {
    White, Blue, Black, Red, Green, Colorless
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum CardType {
    Artifact, 
    Creature, 
    Enchantment, 
    Instant, 
    Land, 
    Planeswalker, 
    Sorcery, 
    Battle,  // Added for completeness
    Unknown  // Safety fallback
}

// Replaces "String" phases with strict logical steps
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub enum Phase {
    Untap,
    Upkeep,
    Draw,
    #[serde(rename = "Main Phase 1")]
    Main1,
    #[serde(rename = "Combat")]
    Combat,
    #[serde(rename = "Main Phase 2")]
    Main2,
    End,
}

// --- STRUCTS ---

// 1. The "Card" (In Hand / On Stack)
// Used when the player attempts an action. It doesn't have board state like 'tapped'.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Card {
    pub name: String,
    pub type_line: Vec<CardType>,
    pub mana_cost: Option<String>,
}

// 2. The "Permanent" (On Battlefield)
// Your original struct, kept exactly as is for board tracking.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Permanent {
    pub id: String,
    pub name: String,
    pub oracle_text: String,
    pub mana_value: u32,
    pub types: Vec<CardType>,
    pub colors: Vec<Color>,
    pub is_legendary: bool,
    pub controller: String,
    
    #[serde(default)]
    pub is_tapped: bool,
    #[serde(default)]
    pub damage_marked: u32,
}

// --- THE STATE CONTAINER ---

#[derive(Debug, Serialize, Deserialize)]
pub struct GameState {
    pub active_player: String, // "Player" or "Opponent"
    pub is_active_player: bool, // Helper bool: Is it actually MY turn?
    
    pub phase: Phase,          // STRICT ENUM now
    
    pub battlefield: Vec<Permanent>,
    pub stack: Vec<String>,    // We can check .len() on this
    
    #[serde(default)] 
    pub lands_played: u8,      // Crucial for Land Logic
    
    // The "Request": What is the user trying to do?
    pub pending_action: Option<GameAction>, 
}

// --- ACTIONS ---

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", content = "payload")]
pub enum GameAction {
    CastSpell(Card),
    PlayLand(Card),
    ActivateAbility { source_id: String, ability_index: u32 },
}
