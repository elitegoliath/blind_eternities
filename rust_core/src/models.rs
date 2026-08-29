// rust_core/src/models.rs
// Models for Magic: The Gathering game state representation in Rust.
// This is where Rust shines. We don't use strings for phases or colors; we use Enums.
// This makes "illegal states" unrepresentable. If you try to create a card with
// the color "Purple," the code won't even compile (or deserialize).

use serde::{Deserialize, Serialize};

// --- ENUMS ---

// Define the Effect Enum
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum Effect {
    DealDamage { amount: u32 },
    // Future effects go here: DrawCards { amount: u32 }, GainLife { amount: u32 }, etc.
}

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
    Legendary, 
    Basic,
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

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type", content = "payload")]
pub enum GameAction {
    CastSpell {
        card: Card,
        #[serde(default)]
        targets: Vec<Target>
    },
    PlayLand(Card),
    ActivateAbility { 
        source_id: String,
        ability_index: u32,
        #[serde(default)]
        targets: Vec<Target>
    },
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(tag = "type", content = "id")]
pub enum Target {
    Permanent(String),    // Points to the `id` of a Permanent on the battlefield
    Player(String),       // "Player" or "Opponent"
    StackObject(String),  // Points to the `id` of a spell currently on the stack
    ZoneCard(String)      // Points to a card in a Graveyard or Exile
}

// The "Stack" object
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StackObject {
    #[serde(default = "generate_fallback_id")]
    pub id: String,           // Every spell needs a UUID so it can be targeted by Counterspells
    pub card: Card,           // The base card data
    pub controller: String,
    #[serde(default)]
    pub targets: Vec<Target>  // The things the spell is pointing at
}

// The "Mana" system
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
        // For this prototype, just subtract from the pool greedily.
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

// The "Card" (In Hand / On Stack)
// Used when the player attempts an action. It doesn't have board state like 'tapped'.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Card {
    pub name: String,
    pub type_line: Vec<CardType>,
    #[serde(default)] 
    pub mana_cost: String,
    #[serde(default)]
    pub effects: Vec<Effect>, // The LLM will populate this!
}

// The "Permanent" (On Battlefield)
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Permanent {
    #[serde(default = "generate_fallback_id")] // Custom fallback for unique IDs
    pub id: String,
    pub name: String,

    #[serde(default)] pub oracle_text: String,
    #[serde(default)] pub mana_value: u32,
    #[serde(default, alias = "type_line")] pub types: Vec<CardType>,
    #[serde(default)] pub colors: Vec<Color>,
    #[serde(default)] pub is_legendary: bool,
    
    #[serde(default = "default_controller")] // Sane default controller
    pub controller: String,
    
    #[serde(default)] pub is_tapped: bool,
    #[serde(default)] pub damage_marked: u32,

    #[serde(default)] pub power: i32,
    #[serde(default)] pub toughness: i32,
}

fn generate_fallback_id() -> String {
    // Falls back to a random-ish identifier if the LLM leaves it blank
    format!("auto-{}", zone_version_rand()) 
}

fn default_controller() -> String {
    "Player".to_string()
}

fn zone_version_rand() -> u16 {
    // Quick pseudo-random number for basic structural integrity
    let p = &0 as *const i32 as usize;
    (p & 0xFFFF) as u16
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
            damage_marked: 0,
            power: 0,
            toughness: 0
        }
    }
}

// --- RULES CONFIG FOR GAME STATE ---

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RulesConfig {
    #[serde(default = "default_true")]
    pub legend_rule_enabled: bool,
    #[serde(default = "default_legend_max")]
    pub legend_max_allowed: usize,
    #[serde(default = "default_scope")]
    pub legend_scope: String, // "controller" or "global"
    #[serde(default = "default_land_limit")]
    pub max_lands_per_turn: u8,
}

// Sane defaults so you don't have to update Python immediately
fn default_true() -> bool { true }
fn default_legend_max() -> usize { 1 }
fn default_scope() -> String { "controller".to_string() }
fn default_land_limit() -> u8 { 1 }

impl Default for RulesConfig {
    fn default() -> Self {
        Self {
            legend_rule_enabled: true,
            legend_max_allowed: 1,
            legend_scope: "controller".to_string(),
            max_lands_per_turn: 1,
        }
    }
}

// --- THE STATE CONTAINER ---

#[derive(Debug, Serialize, Deserialize)]
pub struct GameState {
    pub active_player: String,  // "Player" or "Opponent"
    pub is_active_player: bool, // Helper bool: Is it actually MY turn?
    pub phase: Phase,
    pub battlefield: Vec<Permanent>,
    pub stack: Vec<StackObject>, 
    pub lands_played: u8,       // Crucial for Land Logic
    
    #[serde(default)] 
    pub mana_pool: ManaPool,    // The floating mana available to pay costs
    pub pending_action: Option<GameAction>, // The "Request": What is the user trying to do?

    #[serde(default)] // Fallback to defaults if Python omits it
    pub rules_config: RulesConfig,
}

impl GameState {
    /// Sweeps the board for State-Based Actions. 
    /// Returns true if any actions were taken (meaning we need to loop and check again).
    pub fn check_state_based_actions(&mut self) -> bool {
        let original_count = self.battlefield.len();
        
        // `retain` keeps only the elements where the closure returns true.
        // If it returns false, the permanent is destroyed/put into the graveyard.
        self.battlefield.retain(|permanent| {
            // Since toughness is i32, we can safely check if it is 0 or less.
            let zero_or_less_toughness = permanent.toughness <= 0;

            // We need to cast damage_marked to i32 for the comparison.
            let lethal_damage = (permanent.damage_marked as i32) >= permanent.toughness;
            
            if lethal_damage || zero_or_less_toughness {
                // Return false to drop the permanent from the vector (send to graveyard)
                return false; 
            }
            
            true // Keep the permanent alive
        });

        // If the length changed, an SBA occurred.
        self.battlefield.len() < original_count
    }

    /// The MTG Rules dictate that SBAs loop until the board is completely clean.
    pub fn run_sba_loop(&mut self) {
        while self.check_state_based_actions() {
            // Loop runs until check_state_based_actions() returns false.
            // This handles domino effects (e.g., an anthem creature dies, 
            // lowering toughness of other creatures, causing them to die on the next pass).
        }
    }
}
