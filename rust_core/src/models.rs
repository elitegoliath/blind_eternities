// rust_core/src/models.rs
// Models for Magic: The Gathering game state representation in Rust.
// This is where Rust shines. We don't use strings for phases or colors; we use Enums.
// This makes "illegal states" unrepresentable. If you try to create a card with
// the color "Purple," the code won't even compile (or deserialize).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// --- ENUMS ---

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum TriggerCondition {
    EntersBattlefield,
    #[serde(untagged)]
    Custom(serde_json::Value),
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(tag = "type")]
pub enum Ability {
    Triggered {
        condition: TriggerCondition,
        effect: Effect,
    },
    #[serde(untagged)]
    Custom(serde_json::Value),
}

// Define the Effect Enum
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
#[derive(PartialEq)]
pub enum Effect {
    DealDamage {
        amount: u32,
    },
    Destroy, // No extra fields needed, it just targets
    DrawCards {
        amount: u32,
    },
    Counter,
    #[serde(untagged)]
    Custom(serde_json::Value),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub enum Color {
    White,
    Blue,
    Black,
    Red,
    Green,
    Colorless,
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
    Unknown, // Safety fallback
    #[serde(untagged)]
    Custom(serde_json::Value),
}

// Replaces "String" phases with strict logical steps
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub enum Phase {
    Beginning,
    PreCombatMain,
    Combat,
    PostCombatMain,
    Ending,
    #[serde(untagged)]
    Custom(serde_json::Value),
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub enum Step {
    Untap,
    Upkeep,
    Draw,
    BeginCombat,
    DeclareAttackers,
    DeclareBlockers,
    CombatDamage,
    EndCombat,
    End,
    Cleanup,
    #[serde(untagged)]
    Custom(serde_json::Value),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum Ruling {
    Legal,
    Illegal(crate::errors::EngineError), // The reason why it's illegal
    StateBasedAction(String),            // e.g. "Legend Rule"
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type", content = "payload")]
pub enum GameAction {
    CastSpell {
        card: Card,
        #[serde(default)]
        targets: Vec<Target>,
        #[serde(default)]
        targeting_requirements: Vec<TargetRequirement>,
    },
    PlayLand(Card),
    ActivateAbility {
        source_id: String,
        ability_index: u32,
        #[serde(default)]
        targets: Vec<Target>,
        #[serde(default)]
        targeting_requirements: Vec<TargetRequirement>,
    },
    DeclareAttackers {
        attackers: Vec<String>,
    },
    DeclareBlockers {
        blockers: HashMap<String, Vec<String>>,
    },
    PassPriority,
    #[serde(untagged)]
    Custom(serde_json::Value),
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(tag = "type")]
pub enum TargetRequirement {
    Any,
    Permanent {
        #[serde(default)]
        types: Vec<CardType>,
    },
    Spell,
    Player,
    #[serde(untagged)]
    Custom(serde_json::Value),
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(tag = "type", content = "id")]
pub enum Target {
    Permanent(String),   // Points to the `id` of a Permanent on the battlefield
    Player(String),      // "Player" or "Opponent"
    StackObject(String), // Points to the `id` of a spell currently on the stack
    ZoneCard(String),    // Points to a card in a Graveyard or Exile
    #[serde(untagged)]
    Custom(serde_json::Value),
}

// The "Stack" object
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StackObject {
    #[serde(default = "generate_fallback_id")]
    pub id: String, // Every spell needs a UUID so it can be targeted by Counterspells
    pub card: Card, // The base card data (or ability dummy card)
    pub controller: String,
    #[serde(default)]
    pub targets: Vec<Target>, // The things the spell is pointing at
    #[serde(default)]
    pub targeting_requirements: Vec<TargetRequirement>,
    #[serde(default)]
    pub source_id: Option<String>, // If it's an ability, points to the permanent
}

// The "Mana" system
#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq)]
pub struct ManaPool {
    #[serde(default)]
    pub white: u32,
    #[serde(default)]
    pub blue: u32,
    #[serde(default)]
    pub black: u32,
    #[serde(default)]
    pub red: u32,
    #[serde(default)]
    pub green: u32,
    #[serde(default)]
    pub colorless: u32,
}

impl ManaPool {
    pub fn total_available(&self) -> u32 {
        self.white + self.blue + self.black + self.red + self.green + self.colorless
    }

    /// Parses "{1}{U}{U}" into (generic_needed, specific_pool)
    pub fn from_cost_string(cost_str: &str) -> Result<(u32, ManaPool), String> {
        let mut generic_total = 0;
        let mut pool = ManaPool::default();

        if cost_str.is_empty() {
            return Ok((0, pool));
        }

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
                "X" => {} // Handle X spells as 0 for base cost?
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
        if self.white < cost.white
            || self.blue < cost.blue
            || self.black < cost.black
            || self.red < cost.red
            || self.green < cost.green
            || self.colorless < cost.colorless
        {
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
    pub oracle_text: String,
    #[serde(default)]
    pub effects: Vec<Effect>, // The LLM will populate this!
}

// The "Permanent" (On Battlefield)
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Permanent {
    #[serde(default = "generate_fallback_id")] // Custom fallback for unique IDs
    pub id: String,
    pub name: String,

    #[serde(default)]
    pub oracle_text: String,
    #[serde(default)]
    pub mana_value: u32,
    #[serde(default)]
    pub is_legendary: bool,

    #[serde(default = "default_controller")] // Sane default controller
    pub controller: String,

    #[serde(default)]
    pub is_tapped: bool,
    #[serde(default)]
    pub damage_marked: u32,

    #[serde(default)]
    pub counters: HashMap<String, u32>,

    #[serde(default)]
    pub base_characteristics: Characteristics,

    #[serde(default)]
    pub current_characteristics: Characteristics,
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
        let chars = Characteristics {
            types: card.type_line.clone(),
            colors: vec![],
            abilities: vec![],
            power: 0,
            toughness: 0,
        };

        Permanent {
            id: format!("{}-{}", card.name, id_suffix),
            name: card.name.clone(),
            oracle_text: "".to_string(),
            mana_value: 0,
            is_legendary: false,
            controller,
            is_tapped: false,
            damage_marked: 0,
            counters: HashMap::new(),
            base_characteristics: chars.clone(),
            current_characteristics: chars,
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
fn default_true() -> bool {
    true
}
fn default_life_totals() -> HashMap<String, i32> {
    let mut m = HashMap::new();
    m.insert("Player".to_string(), 20);
    m.insert("Opponent".to_string(), 20);
    m
}
fn default_legend_max() -> usize {
    1
}
fn default_scope() -> String {
    "controller".to_string()
}
fn default_land_limit() -> u8 {
    1
}

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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Layer {
    OneCopiableValues = 10,
    TwoControlChanging = 20,
    ThreeTextChanging = 30,
    FourTypeChanging = 40,
    FiveColorChanging = 50,
    SixAbilityAddingRemoving = 60,
    SevenAPowerToughnessCDA = 70,
    SevenBPowerToughnessSet = 71,
    SevenCPowerToughnessModify = 72,
    SevenDPowerToughnessCounters = 73,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(tag = "type")]
pub enum Modifier {
    AddSubtype { types: Vec<CardType> },
    SetPowerToughness { power: i32, toughness: i32 },
    ModifyPowerToughness { power: i32, toughness: i32 },
    AddAbility { ability: Ability },
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
pub struct Characteristics {
    #[serde(default)]
    pub types: Vec<CardType>,
    #[serde(default)]
    pub colors: Vec<Color>,
    #[serde(default)]
    pub abilities: Vec<Ability>,
    #[serde(default)]
    pub power: i32,
    #[serde(default)]
    pub toughness: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContinuousEffect {
    pub source_id: String,
    pub timestamp: u64,
    pub layer: Layer,
    pub modifier: Modifier,
    #[serde(default)]
    pub targets: Vec<String>,
}

// The "State Container"
// --- THE STATE CONTAINER ---

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameState {
    pub active_player: String, // "Player" or "Opponent"

    #[serde(default = "default_player")]
    pub priority_player: String,

    #[serde(default = "default_turn")]
    pub turn_number: u32,

    pub phase: Phase,
    pub step: Option<Step>,

    pub stack: Vec<StackObject>,
    pub battlefield: Vec<Permanent>,

    #[serde(default)]
    pub graveyard: Vec<Card>,

    #[serde(default)]
    pub exile: Vec<Card>,

    #[serde(default)]
    pub hand: HashMap<String, Vec<Card>>,

    #[serde(default = "default_life_totals")]
    pub life_totals: HashMap<String, i32>,

    #[serde(default)]
    pub mana_pool: HashMap<String, ManaPool>, // player_id -> mana pool

    #[serde(default)]
    pub continuous_effects: Vec<ContinuousEffect>,

    #[serde(default)]
    pub pending_triggers: Vec<crate::triggers::PendingTrigger>,

    pub pending_action: Option<GameAction>,

    // Internal engine state variables (skipped by Python if missing)
    #[serde(default)]
    pub attackers: Vec<String>,
    #[serde(default)]
    pub blockers: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub lands_played: u8,
    #[serde(default)]
    pub consecutive_passes: u8,
    #[serde(default)]
    pub rules_config: RulesConfig,
}

fn default_player() -> String {
    "Player".to_string()
}
fn default_turn() -> u32 {
    1
}

impl GameState {
    pub fn get_player_perspective(&self, player_id: &str) -> GameState {
        let mut sanitized = self.clone();
        for (pid, cards) in sanitized.hand.iter_mut() {
            if pid != player_id {
                for card in cards.iter_mut() {
                    card.name = "Hidden Card".to_string();
                    card.type_line = vec![];
                    card.mana_cost = "".to_string();
                    card.oracle_text = "".to_string();
                    card.effects = vec![];
                }
            }
        }
        sanitized
    }
    /// Sweeps the board for State-Based Actions.

    /// Emits a game event, triggering permanents to place effects into the pending_triggers queue
    pub fn emit_event(&mut self, event: crate::events::GameEvent) {
        let mut new_triggers = Vec::new();
        match &event {
            crate::events::GameEvent::ZoneChange {
                object_id, to_zone, ..
            } => {
                if to_zone == "Battlefield" {
                    if let Some(perm) = self.battlefield.iter().find(|p| &p.id == object_id) {
                        for ability in &perm.current_characteristics.abilities {
                            if let Ability::Triggered { condition, effect } = ability {
                                if *condition == TriggerCondition::EntersBattlefield {
                                    new_triggers.push(crate::triggers::PendingTrigger {
                                        source_id: perm.id.clone(),
                                        controller: perm.controller.clone(),
                                        effect: effect.clone(),
                                        required_targets: 0,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        self.pending_triggers.extend(new_triggers);
    }

    pub fn get_mana_pool(&self, player: &str) -> ManaPool {
        self.mana_pool.get(player).cloned().unwrap_or_default()
    }

    pub fn get_mana_pool_mut(&mut self, player: &str) -> &mut ManaPool {
        self.mana_pool.entry(player.to_string()).or_default()
    }

    /// Returns true if any actions were taken (meaning we need to loop and check again).
    pub fn check_state_based_actions(&mut self) -> bool {
        let original_count = self.battlefield.len();

        let mut to_graveyard = Vec::new();

        // `retain` keeps only the elements where the closure returns true.
        // If it returns false, the permanent is destroyed/put into the graveyard.
        self.battlefield.retain(|permanent| {
            // Since toughness is i32, we can safely check if it is 0 or less.
            let zero_or_less_toughness = permanent.current_characteristics.toughness <= 0
                && permanent
                    .current_characteristics
                    .types
                    .contains(&CardType::Creature);

            // We need to cast damage_marked to i32 for the comparison.
            let lethal_damage = (permanent.damage_marked as i32)
                >= permanent.current_characteristics.toughness
                && permanent
                    .current_characteristics
                    .types
                    .contains(&CardType::Creature);

            // Planeswalker legality
            let zero_loyalty = permanent
                .current_characteristics
                .types
                .contains(&CardType::Planeswalker)
                && *permanent.counters.get("Loyalty").unwrap_or(&0) == 0;

            if lethal_damage || zero_or_less_toughness || zero_loyalty {
                // Return false to drop the permanent from the vector (send to graveyard)

                // Reconstruct a base card for the graveyard
                to_graveyard.push(Card {
                    name: permanent.name.clone(),
                    type_line: permanent.current_characteristics.types.clone(),
                    mana_cost: "".to_string(), // Incomplete reconstruction for graveyard right now, but functional
                    oracle_text: permanent.oracle_text.clone(),
                    effects: vec![],
                });
                return false;
            }

            true // Keep the permanent alive
        });

        self.graveyard.extend(to_graveyard);

        // If the length changed, an SBA occurred.
        self.battlefield.len() < original_count
    }

    /// The MTG Rules dictate that SBAs loop until the board is completely clean.
    pub fn run_sba_loop(&mut self) {
        self.recalculate_stats();
        while self.check_state_based_actions() {
            // Loop runs until check_state_based_actions() returns false.
        }
    }

    /// Layer 7 Calculation
    pub fn recalculate_stats(&mut self) {
        // 1. Reset all permanents to base
        for perm in &mut self.battlefield {
            perm.current_characteristics = perm.base_characteristics.clone();
        }

        // 2. Gather active effects
        let mut effects = self.continuous_effects.clone();
        effects.sort_by(|a, b| match a.layer.cmp(&b.layer) {
            std::cmp::Ordering::Equal => a.timestamp.cmp(&b.timestamp),
            other => other,
        });

        // 3. Apply effects
        for effect in effects {
            for perm in &mut self.battlefield {
                if effect.targets.is_empty() || effect.targets.contains(&perm.id) {
                    match &effect.modifier {
                        Modifier::AddSubtype { types } => {
                            for ct in types {
                                if !perm.current_characteristics.types.contains(ct) {
                                    perm.current_characteristics.types.push(ct.clone());
                                }
                            }
                        }
                        Modifier::SetPowerToughness { power, toughness } => {
                            perm.current_characteristics.power = *power;
                            perm.current_characteristics.toughness = *toughness;
                        }
                        Modifier::ModifyPowerToughness { power, toughness } => {
                            perm.current_characteristics.power += *power;
                            perm.current_characteristics.toughness += *toughness;
                        }
                        Modifier::AddAbility { ability } => {
                            perm.current_characteristics.abilities.push(ability.clone());
                        }
                    }
                }
            }
        }

        // 4. Layer 7d: Counters
        for perm in &mut self.battlefield {
            let plus = *perm.counters.get("+1/+1").unwrap_or(&0) as i32;
            let minus = *perm.counters.get("-1/-1").unwrap_or(&0) as i32;
            perm.current_characteristics.power += plus - minus;
            perm.current_characteristics.toughness += plus - minus;
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EngineResponse {
    pub success: bool,
    pub state: Option<GameState>,
    pub message: Option<String>,
    pub error: Option<crate::errors::EngineError>,
    pub logs: Vec<String>,
}
