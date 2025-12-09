// rust_core/src/rules.rs
// Rules engine for Magic: The Gathering game state assessment in Rust.
// This module checks for rule violations and state-based actions (SBAs).
// This file contains pure functions. They take data in and return a verdict.
// They do not talk to a database or the internet; they just compute "Magic Physics."

use crate::models::{GameState, GameAction, Permanent, Phase, Card, CardType};

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
                    rulings.push(Self::check_cast_timing(state, card));
                },
                // Add Ability activation checks here later
                _ => {}
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{CardType, Color, Permanent, GameState, Phase};

    // Helper to create a dummy card for Board State
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
            is_active_player: true,
            phase: Phase::Main1, // UPDATED: Use Enum
            battlefield: vec![urza_1, urza_2], // <--- Violation!
            stack: vec![],
            lands_played: 0,        // UPDATED: Added field
            pending_action: None,   // UPDATED: Added field
        };

        // 2. Execution: Ask the Judge
        let rulings = Judge::assess_state(&state);

        // 3. Assertion: Logic check
        match &rulings[0] {
            Ruling::StateBasedAction(msg) => {
                assert!(msg.contains("Legend Rule Violation"));
            }
            _ => panic!("Expected Legend Rule violation, got {:?}", rulings),
        }
    }

    #[test]
    fn test_no_trigger_for_different_legends() {
        let urza = mock_legend("Urza, Lord High Artificer", "uuid-1");
        let thassa = mock_legend("Thassa, Deep-Dwelling", "uuid-3");

        let state = GameState {
            active_player: "Player 1".to_string(),
            is_active_player: true,
            phase: Phase::Main1,
            battlefield: vec![urza, thassa],
            stack: vec![],
            lands_played: 0,
            pending_action: None,
        };

        let rulings = Judge::assess_state(&state);
        assert!(matches!(rulings[0], Ruling::Legal));
    }

    #[test]
    fn test_cannot_play_land_on_stack() {
        // New test to verify the merged Action logic works
        let land_card = Card { 
            name: "Mountain".into(), 
            type_line: vec![CardType::Land], 
            mana_cost: None 
        };

        let state = GameState {
            active_player: "Player 1".to_string(),
            is_active_player: true,
            phase: Phase::Main1,
            battlefield: vec![], 
            stack: vec!["Lightning Bolt".to_string()], // Stack is NOT empty!
            lands_played: 0,
            pending_action: Some(GameAction::PlayLand(land_card)),
        };

        let rulings = Judge::assess_state(&state);
        
        match &rulings[0] {
            Ruling::Illegal(reason) => assert!(reason.contains("stack")),
            _ => panic!("Should be illegal"),
        }
    }
}