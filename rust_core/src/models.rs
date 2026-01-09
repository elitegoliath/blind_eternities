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
    Battle,
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

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum Ruling {
    Legal,
    Illegal(String), // The reason why it's illegal
    StateBasedAction(String), // e.g. "Legend Rule"
}

// --- MANA SYSTEM ---

#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq)]
pub struct ManaPool {
    #[serde(default)] pub white: u32,
    #[serde(default)] pub blue: u32,
    #[serde(default)] pub black: u32,
    #[serde(default)] pub red: u32,
    #[serde(default)] pub green: u32,
    #[serde(default)] pub colorless: u32,
}

impl ManaPool {
    pub fn total_available(&self) -> u32 {
        self.white + self.blue + self.black + self.red + self.green + self.colorless
    }
    
    /// Parses "{1}{U}{U}" into (generic_needed, specific_pool)
    pub fn from_cost_string(cost_str: &str) -> Result<(u32, ManaPool), String> {
        let mut generic_total = 0;
        let mut pool = ManaPool::default();

        if cost_str.is_empty() { return Ok((0, pool)); }

        let tokens = cost_str.split('}').filter(|s| !s.is_empty());

        for token in tokens {
            let content = token.trim_start_matches('{');
            match content {
                "W" => pool.white += 1,
                "U" => pool.blue += 1,
                "B" => pool.black += 1,
                "R" => pool.red += 1,
                "G" => pool.green += 1,
                "C" => pool.colorless += 1,
                "X" => {}, // Handle X spells as 0 for base cost?
                num_str => {
                    if let Ok(num) = num_str.parse::<u32>() {
                        generic_total += num;
                    } else {
                        return Err(format!("Unknown symbol '{}'", content));
                    }
                }
            }
        }
        Ok((generic_total, pool))
    }

    /// Attempts to deduct the cost from self. Returns true if successful (mutates), false if insufficient.
    pub fn pay(&mut self, cost: &ManaPool, generic_cost: u32) -> bool {
        // 1. Check strict colors
        if self.white < cost.white || self.blue < cost.blue || self.black < cost.black ||
           self.red < cost.red || self.green < cost.green || self.colorless < cost.colorless {
            return false;
        }

        // 2. Deduct strict colors
        self.white -= cost.white;
        self.blue -= cost.blue;
        self.black -= cost.black;
        self.red -= cost.red;
        self.green -= cost.green;
        self.colorless -= cost.colorless;

        // 3. Fail Fast if we don't have enough total mana left
        if self.total_available() < generic_cost {
             return false;
        }

        // 4. Deduct generic from whatever is largest/remaining (Simplified: just subtract total) (Greedy Algorithm)
        // In a real engine, we'd ask the user WHICH mana to spend. 
        // For this prototype, we just subtract from the pool greedily.
        let mut remaining_to_pay = generic_cost;
        
        // Helper closure to drain a color
        let mut drain = |pool_amt: &mut u32| {
            if remaining_to_pay > 0 && *pool_amt > 0 {
                let take = (*pool_amt).min(remaining_to_pay);
                *pool_amt -= take;
                remaining_to_pay -= take;
            }
        };

        // Drain colorless first, then WUBRG
        drain(&mut self.colorless);
        drain(&mut self.red);
        drain(&mut self.green);
        drain(&mut self.black);
        drain(&mut self.blue);
        drain(&mut self.white);

        remaining_to_pay == 0
    }
}

// --- STRUCTS ---

// 1. The "Card" (In Hand / On Stack)
// Used when the player attempts an action. It doesn't have board state like 'tapped'.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Card {
    pub name: String,
    pub type_line: Vec<CardType>,
    #[serde(default)] 
    pub mana_cost: String,
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

impl Permanent {
    // Helper to turn a Card into a Permanent
    pub fn from_card(card: &Card, controller: String, id_suffix: usize) -> Self {
        Permanent {
            id: format!("{}-{}", card.name, id_suffix), // Simple ID generation
            name: card.name.clone(),
            oracle_text: "".to_string(), // We don't have text on Card struct yet
            mana_value: 0, // Need to calculate from mana_cost parsing (skip for now)
            types: card.type_line.clone(),
            colors: vec![], // Need to parse colors from cost (skip for now)
            is_legendary: false, // Need this info on Card (skip for now)
            controller,
            is_tapped: false,
            damage_marked: 0
        }
    }
}

// --- THE STATE CONTAINER ---

#[derive(Debug, Serialize, Deserialize)]
pub struct GameState {
    pub active_player: String, // "Player" or "Opponent"
    pub is_active_player: bool, // Helper bool: Is it actually MY turn?
    pub phase: Phase,
    pub battlefield: Vec<Permanent>,
    pub stack: Vec<String>,    // We can check .len() on this
    pub lands_played: u8,      // Crucial for Land Logic
    
    #[serde(default)] 
    pub mana_pool: ManaPool,   // The floating mana available to pay costs
    pub pending_action: Option<GameAction>, // The "Request": What is the user trying to do?
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", content = "payload")]
pub enum GameAction {
    CastSpell(Card),
    PlayLand(Card),
    ActivateAbility { source_id: String, ability_index: u32 },
}
