// rust_core/src/rules.rs
// Rules engine for Magic: The Gathering game state assessment in Rust.
// This module checks for rule violations and state-based actions (SBAs).
// This file contains pure functions. They take data in and return a verdict.
// They do not talk to a database or the internet; they just compute "Magic Physics."

use crate::models::{GameState, GameAction, Card, CardType, Ruling, ManaPool, Permanent, Phase};

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

    /// Validation + Execution
    /// Returns Ok(NewState) or Err(Reason)
    pub fn apply_action(state: &mut GameState) -> Result<(), String> {
        // 1. Verify Legality First
        let rulings = Self::assess_state(state);
        for r in rulings {
            if let Ruling::Illegal(reason) = r {
                return Err(reason);
            }
        }

        // 2. Execute Action (If we are here, it's legal)
        if let Some(action) = &state.pending_action {
            match action {
                GameAction::PlayLand(card) => {
                    // Update Limits
                    state.lands_played += 1;
                    
                    // Create Permanent
                    let perm = Permanent::from_card(
                        card, 
                        state.active_player.clone(), 
                        state.battlefield.len()
                    );
                    state.battlefield.push(perm);
                },
                GameAction::CastSpell(card) => {
                    // Calculate Cost again
                    let (generic, cost_pool) = ManaPool::from_cost_string(&card.mana_cost)
                        .map_err(|e| e)?; // Should catch in validation, but safe unwrap here
                    
                    // Pay Mana (Mutates Pool)
                    if !state.mana_pool.pay(&cost_pool, generic) {
                        return Err("CRITICAL: Mana validation passed but payment failed.".to_string());
                    }

                    // Move to Stack
                    // For now, we just push the name string. 
                    // In real engine, we'd push a SpellObject.
                    state.stack.push(card.name.clone());
                },
                _ => {}
            }
        }

        // 3. Cleanup
        state.pending_action = None;
        Ok(())
    }

    /// Internal Logic: The "Legend Rule" (CR 704.5j)
    fn check_legend_rule(permanents: &[Permanent]) -> Option<Ruling> {
        // Simple O(N^2) check: Do two legendary perms share a name and controller?
        for (i, p1) in permanents.iter().enumerate() {
            if !p1.is_legendary { continue; }
            for (j, p2) in permanents.iter().enumerate() {
                if i == j { continue; } // Don't compare self
                if p2.is_legendary && p1.name == p2.name && p1.controller == p2.controller {
                    return Some(Ruling::StateBasedAction(format!("Legend Rule: {}", p1.name)));
                }
            }
        }
        None
    }

    /// Internal Logic: Playing a Land (CR 305)
    fn check_land_drop(state: &GameState, card: &Card) -> Ruling {
        if !card.type_line.contains(&CardType::Land) { return Ruling::Illegal("Not a Land".into()); }
        if !state.is_active_player { return Ruling::Illegal("Not your turn".into()); }
        if !state.stack.is_empty() { return Ruling::Illegal("Stack not empty".into()); }
        if state.lands_played >= 1 { return Ruling::Illegal("Land limit reached".into()); }
        match state.phase {
            Phase::Main1 | Phase::Main2 => Ruling::Legal,
            _ => Ruling::Illegal("Wrong Phase".into())
        }
    }

    /// Internal Logic: Casting a Spell (Timing Rules)
    fn check_cast_timing(state: &GameState, card: &Card) -> Ruling {
        let is_instant = card.type_line.contains(&CardType::Instant);
        if is_instant { return Ruling::Legal; }
        // Sorcery Speed Checks
        if !state.is_active_player { return Ruling::Illegal("Not your turn".into()); }
        if !state.stack.is_empty() { return Ruling::Illegal("Stack not empty".into()); }
        match state.phase {
            Phase::Main1 | Phase::Main2 => Ruling::Legal,
            _ => Ruling::Illegal("Wrong Phase".into())
        }
    }

    /// Internal Logic: Casting a Spell (Mana Cost Rules)
    fn check_mana_cost(state: &GameState, card: &Card) -> Ruling {
        // Use Model Parser
        let (required_generic, required_pool) = match ManaPool::from_cost_string(&card.mana_cost) {
            Ok(res) => res,
            Err(e) => return Ruling::Illegal(format!("Invalid Cost: {}", e)),
        };

        // Simulate Payment
        let mut temp_pool = state.mana_pool.clone();
        if temp_pool.pay(&required_pool, required_generic) {
            Ruling::Legal
        } else {
            Ruling::Illegal("Insufficient Mana".to_string())
        }
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