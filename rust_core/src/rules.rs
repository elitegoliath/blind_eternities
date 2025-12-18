// rust_core/src/rules.rs
// Rules engine for Magic: The Gathering game state assessment in Rust.
// This module checks for rule violations and state-based actions (SBAs).
// This file contains pure functions. They take data in and return a verdict.
// They do not talk to a database or the internet; they just compute "Magic Physics."

use crate::models::{GameState, GameAction, Permanent, Phase, Card, CardType, ManaPool};

// Result Enum: What happened?
#[derive(Debug)]
pub enum Ruling {
    Legal,
    Illegal(String), // Reason why
    StateBasedAction(String), // "Legend Rule applies"
}

pub struct Judge;

impl Judge {
    /// The Main Loop: Checks for any violations or triggers
    pub fn assess_state(state: &GameState) -> Vec<Ruling> {
        let mut rulings = Vec::new();

        // 1. Check State-Based Actions (SBAs)
        // These happen automatically, regardless of player intent.
        if let Some(sba) = Self::check_legend_rule(&state.battlefield) {
            rulings.push(sba);
        }
        
        // 2. Check Player Actions
        // "Can I actually do this thing I'm trying to do?"
        if let Some(action) = &state.pending_action {
            match action {
                GameAction::PlayLand(card) => {
                    rulings.push(Self::check_land_drop(state, card));
                },
                GameAction::CastSpell(card) => {
                    // 1. Check Timing
                    let timing = Self::check_cast_timing(state, card);
                    if let Ruling::Illegal(_) = timing {
                        rulings.push(timing);
                    } else {
                        // 2. Check Mana (Only if timing is okay)
                        rulings.push(Self::check_mana_cost(state, card));
                    }
                },
                GameAction::ActivateAbility { .. } => {}
            }
        }

        // If no errors were found, default to Legal
        if rulings.is_empty() {
            vec![Ruling::Legal]
        } else {
            rulings
        }
    }

    /// Internal Logic: The "Legend Rule" (CR 704.5j)
    fn check_legend_rule(permanents: &[Permanent]) -> Option<Ruling> {
        // Simple O(N^2) check: Do two legendary perms share a name and controller?
        for (i, p1) in permanents.iter().enumerate() {
            if !p1.is_legendary { continue; }
            
            for (j, p2) in permanents.iter().enumerate() {
                if i == j { continue; } // Don't compare self
                
                if p2.is_legendary 
                    && p1.name == p2.name 
                    && p1.controller == p2.controller 
                {
                    return Some(Ruling::StateBasedAction(format!(
                        "Legend Rule Violation: Two instances of '{}' controlled by '{}'", 
                        p1.name, p1.controller
                    )));
                }
            }
        }
        None
    }

    /// Internal Logic: Playing a Land (CR 305)
    fn check_land_drop(state: &GameState, card: &Card) -> Ruling {
        if !card.type_line.contains(&CardType::Land) {
            return Ruling::Illegal(format!("{} is not a Land.", card.name));
        }
        if !state.is_active_player {
            return Ruling::Illegal("Cannot play lands on opponent's turn.".to_string());
        }
        if !state.stack.is_empty() {
            return Ruling::Illegal("Cannot play lands while spells are on the stack.".to_string());
        }
        match state.phase {
            Phase::Main1 | Phase::Main2 => {},
            _ => return Ruling::Illegal("Lands can only be played during Main Phases.".to_string())
        }
        if state.lands_played >= 1 {
            return Ruling::Illegal("Land limit for turn reached.".to_string());
        }
        Ruling::Legal
    }

    /// Internal Logic: Casting a Spell (Timing Rules)
    fn check_cast_timing(state: &GameState, card: &Card) -> Ruling {
        let is_instant_speed = card.type_line.contains(&CardType::Instant) 
                            || card.type_line.contains(&CardType::Unknown);

        if is_instant_speed {
            return Ruling::Legal;
        }

        // Sorcery Speed Checks
        if !state.is_active_player {
            return Ruling::Illegal("Cannot cast sorcery-speed spells on opponent's turn.".to_string());
        }
        if !state.stack.is_empty() {
            return Ruling::Illegal("Cannot cast sorcery-speed spells while stack is not empty.".to_string());
        }
        match state.phase {
            Phase::Main1 | Phase::Main2 => Ruling::Legal,
            _ => Ruling::Illegal("Sorcery-speed spells can only be cast during Main Phases.".to_string())
        }
    }

    /// Internal Logic: Casting a Spell (Mana Cost Rules)
    fn check_mana_cost(state: &GameState, card: &Card) -> Ruling {
        // 1. Parse the cost string (e.g., "{1}{U}{U}")
        let (required_generic, required_pool) = match Self::parse_mana_cost(&card.mana_cost) {
            Ok(res) => res,
            Err(e) => return Ruling::Illegal(format!("Invalid Mana Cost: {}", e)),
        };

        // 2. Clone the player's pool so we can simulate spending it
        let mut available = state.mana_pool.clone();

        // 3. Pay Colored Costs First
        // We saturate_sub to avoid underflow panics, then check if we had enough.
        if available.white < required_pool.white { return Ruling::Illegal("Not enough White mana.".into()); }
        available.white -= required_pool.white;

        if available.blue < required_pool.blue { return Ruling::Illegal("Not enough Blue mana.".into()); }
        available.blue -= required_pool.blue;

        if available.black < required_pool.black { return Ruling::Illegal("Not enough Black mana.".into()); }
        available.black -= required_pool.black;

        if available.red < required_pool.red { return Ruling::Illegal("Not enough Red mana.".into()); }
        available.red -= required_pool.red;

        if available.green < required_pool.green { return Ruling::Illegal("Not enough Green mana.".into()); }
        available.green -= required_pool.green;

        if available.colorless < required_pool.colorless { return Ruling::Illegal("Not enough Colorless mana.".into()); }
        available.colorless -= required_pool.colorless;

        // 4. Pay Generic Cost
        // Sum up EVERYTHING remaining in the pool
        let remaining_total = available.total_available();

        if remaining_total < required_generic {
            return Ruling::Illegal(format!(
                "Not enough generic mana. Need {}, have {}.", 
                required_generic, remaining_total
            ));
        }

        Ruling::Legal
    }

    /// Helper: Parses "{3}{U}{B}" into (3, ManaPool { blue: 1, black: 1 ... })
    fn parse_mana_cost(cost_str: &str) -> Result<(u32, ManaPool), String> {
        let mut generic_total = 0;
        let mut pool = ManaPool::default();

        if cost_str.is_empty() {
            return Ok((0, pool));
        }

        // Split by '}' to handle tokens. string is like "{3}{U}" -> ["{3", "{U", ""]
        // This is a naive parser but works for standard formatting.
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
                num_str => {
                    // Try parsing as generic number
                    if let Ok(num) = num_str.parse::<u32>() {
                        generic_total += num;
                    } else {
                        // Handle Hybrid later? For now, fail.
                        return Err(format!("Unknown symbol '{}'", content));
                    }
                }
            }
        }

        Ok((generic_total, pool))
    }
}

// --- TESTS ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mana_parsing_and_payment() {
        // Scenario: Cost is "{1}{U}{U}" (Cryptic Command logic)
        // Player has: {U}:3
        
        let card = Card {
            name: "Counterspell".into(),
            type_line: vec![CardType::Instant],
            mana_cost: "{1}{U}{U}".into(),
        };

        let mut pool = ManaPool::default();
        pool.blue = 3; // Total 3 mana, all blue.

        let state = GameState {
            active_player: "Hero".into(),
            is_active_player: true,
            phase: Phase::Main1,
            battlefield: vec![],
            stack: vec![],
            lands_played: 0,
            mana_pool: pool, // <--- 3 Blue Available
            pending_action: Some(GameAction::CastSpell(card)),
        };

        // Should be Legal:
        // Cost: 2 Blue (paid), 1 Generic (paid with remaining 1 Blue)
        let rulings = Judge::assess_state(&state);
        assert!(matches!(rulings[0], Ruling::Legal));
    }

    #[test]
    fn test_mana_insufficient_generic() {
        // Scenario: Cost "{4}"
        // Player has: {R}: 3
        let card = Card {
            name: "Golem".into(),
            type_line: vec![CardType::Artifact],
            mana_cost: "{4}".into(),
        };

        let mut pool = ManaPool::default();
        pool.red = 3; 

        let state = GameState {
            active_player: "Hero".into(),
            is_active_player: true,
            phase: Phase::Main1,
            battlefield: vec![],
            stack: vec![],
            lands_played: 0,
            mana_pool: pool,
            pending_action: Some(GameAction::CastSpell(card)),
        };

        let rulings = Judge::assess_state(&state);
        match &rulings[0] {
            Ruling::Illegal(msg) => assert!(msg.contains("Not enough generic")),
            _ => panic!("Should fail generic check"),
        }
    }

    #[test]
    fn test_mana_insufficient_color() {
        // Scenario: Cost "{U}"
        // Player has: {R}: 10
        let card = Card {
            name: "Unsummon".into(),
            type_line: vec![CardType::Instant],
            mana_cost: "{U}".into(),
        };

        let mut pool = ManaPool::default();
        pool.red = 10; 

        let state = GameState {
            active_player: "Hero".into(),
            is_active_player: true,
            phase: Phase::Main1,
            battlefield: vec![],
            stack: vec![],
            lands_played: 0,
            mana_pool: pool,
            pending_action: Some(GameAction::CastSpell(card)),
        };

        let rulings = Judge::assess_state(&state);
        match &rulings[0] {
            Ruling::Illegal(msg) => assert!(msg.contains("Not enough Blue")),
            _ => panic!("Should fail color check"),
        }
    }
}