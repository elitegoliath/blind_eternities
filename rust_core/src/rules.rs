// rust_core/src/rules.rs
// Rules engine for Magic: The Gathering game state assessment in Rust.
// This module checks for rule violations and state-based actions (SBAs).
// This file contains pure functions. They take data in and return a verdict.
// They do not talk to a database or the internet; they just compute "Magic Physics."

use crate::models::{GameState, Permanent};

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
        if let Some(sba) = Self::check_legend_rule(&state.battlefield) {
            rulings.push(sba);
        }
        
        // 2. Check Toughness (Lethal Damage)
        // (Implementation placeholder)
        
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{CardType, Color, Permanent, GameState};

    // Helper to create a dummy card
    fn mock_legend(name: &str, id: &str) -> Permanent {
        Permanent {
            id: id.to_string(),
            name: name.to_string(),
            oracle_text: "".to_string(),
            mana_value: 2,
            types: vec![CardType::Creature],
            colors: vec![Color::Blue],
            is_legendary: true,
            controller: "Player 1".to_string(),
            is_tapped: false,
            damage_marked: 0,
        }
    }

    #[test]
    fn test_legend_rule_triggers() {
        // 1. Setup: Create a board with TWO "Urza, Lord High Artificer"
        let urza_1 = mock_legend("Urza, Lord High Artificer", "uuid-1");
        let urza_2 = mock_legend("Urza, Lord High Artificer", "uuid-2");
        
        let state = GameState {
            active_player: "Player 1".to_string(),
            phase: "Main Phase".to_string(),
            battlefield: vec![urza_1, urza_2], // <--- Violation!
            stack: vec![],
        };

        // 2. Execution: Ask the Judge
        let rulings = Judge::assess_state(&state);

        // 3. Assertion: Logic check
        match &rulings[0] {
            Ruling::StateBasedAction(msg) => {
                assert!(msg.contains("Legend Rule Violation"));
                println!("Test Passed: Judge correctly flagged the Legend Rule.");
            }
            _ => panic!("Expected Legend Rule violation, got {:?}", rulings),
        }
    }

    #[test]
    fn test_no_trigger_for_different_legends() {
        // 1. Setup: Urza and Thassa (Different names, no violation)
        let urza = mock_legend("Urza, Lord High Artificer", "uuid-1");
        let thassa = mock_legend("Thassa, Deep-Dwelling", "uuid-3");

        let state = GameState {
            active_player: "Player 1".to_string(),
            phase: "Main Phase".to_string(),
            battlefield: vec![urza, thassa],
            stack: vec![],
        };

        let rulings = Judge::assess_state(&state);

        // 2. Assertion: Should be legal
        assert!(matches!(rulings[0], Ruling::Legal));
    }
}
