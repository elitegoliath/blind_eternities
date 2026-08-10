// rust_core/src/rules.rs
// Rules engine for Magic: The Gathering game state assessment in Rust.
// This module checks for rule violations and state-based actions (SBAs).
// This file contains pure functions. They take data in and return a verdict.
// They do not talk to a database or the internet; they just compute "Magic Physics."

use crate::models::{Card, CardType, GameAction, GameState, ManaPool, Permanent, Phase, RulesConfig, Ruling};

pub struct Judge;

impl Judge {
    /// The Main Loop: Checks for any violations or triggers
    pub fn assess_state(state: &GameState) -> Vec<Ruling> {
        let mut rulings = Vec::new();

        // 1. Check State-Based Actions (SBAs)
        // These happen automatically, regardless of player intent.
        if let Some(sba) = Self::check_legend_rule(&state.battlefield, &state.rules_config) {
            rulings.push(sba);
        }
        
        // 2. Check Player Actions
        // "Can I actually do this thing I'm trying to do?"
        if let Some(action) = &state.pending_action {
            match action {
                GameAction::PlayLand(card) => {
                    rulings.push(Self::check_land_drop(state, card));
                },
                GameAction::CastSpell { card, targets } => {
                    // 1. Check Targets First!
                    if let Some(target_violation) = Self::check_targets(state, &state.active_player, targets) {
                        rulings.push(target_violation);
                    } else {
                        // 2. Check Timing (Only if targets are valid)
                        let timing = Self::check_cast_timing(state, card);
                        if let Ruling::Illegal(_) = timing {
                            rulings.push(timing);
                        } else {
                            // 3. Check Mana 
                            rulings.push(Self::check_mana_cost(state, card));
                        }
                    }
                },
                GameAction::ActivateAbility { source_id, ability_index, targets } => {
                    if let Some(target_violation) = Self::check_targets(state, &state.active_player, targets) {
                        rulings.push(target_violation);
                    }
                    // Future: Implement ability cost and timing checks
                }
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

        // We clone the action so we can mutate the state without borrow checker fights
        let pending = state.pending_action.clone();

        // 2. Execute Action (If we are here, it's legal)
        if let Some(action) = pending {
            match action {
                GameAction::PlayLand(card) => {
                    // Update Limits
                    state.lands_played += 1;
                    
                    // Create Permanent
                    let perm = Permanent::from_card(
                        &card, 
                        state.active_player.clone(), 
                        state.battlefield.len()
                    );
                    state.battlefield.push(perm);
                },
                GameAction::CastSpell { card, targets } => {
                    // Calculate Cost again
                    let (generic, cost_pool) = crate::models::ManaPool::from_cost_string(&card.mana_cost)
                        .map_err(|e| e)?; 
                    
                    // Pay Mana (Mutates Pool)
                    if !state.mana_pool.pay(&cost_pool, generic) {
                        return Err("CRITICAL: Mana validation passed but payment failed.".to_string());
                    }

                    // Move to Stack as a fully realized StackObject
                    let stack_id = format!("spell-{}-{}", card.name.replace(" ", "").to_lowercase(), state.stack.len());
                    
                    let spell = crate::models::StackObject {
                        id: stack_id,
                        card,
                        controller: state.active_player.clone(),
                        targets,
                    };
                    
                    state.stack.push(spell);
                },
                GameAction::ActivateAbility { source_id, ability_index, targets } => {
                    // Future: Pay ability costs and put a StackObject (Ability) on the stack
                }
            }
        }

        // 3. Cleanup
        state.pending_action = None;
        Ok(())
    }

    /// Internal Logic: The parameterized "Legend Rule"
    fn check_legend_rule(permanents: &[Permanent], config: &RulesConfig) -> Option<Ruling> {
        if !config.legend_rule_enabled { return None; }

        for (i, p1) in permanents.iter().enumerate() {
            if !p1.types.contains(&CardType::Legendary) { continue; } 
            
            let mut match_count = 1;
            
            for (j, p2) in permanents.iter().enumerate() {
                if i == j { continue; } 
                
                // Read the scope from config
                let scope_match = if config.legend_scope == "controller" {
                    p1.controller == p2.controller
                } else {
                    true // The old global rule
                };

                if p2.types.contains(&CardType::Legendary) && p1.name == p2.name && scope_match {
                    match_count += 1;
                }
            }

            // Read the limit from config
            if match_count > config.legend_max_allowed {
                return Some(Ruling::StateBasedAction(format!("Legend Rule: {}", p1.name)));
            }
        }
        None
    }

    /// Internal Logic: Parameterized Land Drops
    fn check_land_drop(state: &GameState, card: &Card) -> Ruling {
        if !card.type_line.contains(&CardType::Land) { return Ruling::Illegal("Not a Land".into()); }
        if !state.is_active_player { return Ruling::Illegal("Not your turn".into()); }
        if !state.stack.is_empty() { return Ruling::Illegal("Stack not empty".into()); }
        
        // Read the limit from config
        if state.lands_played >= state.rules_config.max_lands_per_turn { 
            return Ruling::Illegal(format!("Land limit of {} reached", state.rules_config.max_lands_per_turn)); 
        }
        
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

    /// Internal Logic: Target Legality (CR 601.2c)
    fn check_targets(state: &GameState, source_controller: &str, targets: &[crate::models::Target]) -> Option<Ruling> {
        for target in targets {
            match target {
                crate::models::Target::Permanent(id) => {
                    // 1. Does it exist?
                    let target_perm = state.battlefield.iter().find(|p| p.id == *id);
                    
                    if let Some(perm) = target_perm {
                        // 2. Check Targeting Restrictions (Shroud & Hexproof)
                        let text = perm.oracle_text.to_lowercase();
                        
                        if text.contains("shroud") {
                            return Some(Ruling::Illegal(format!(
                                "Invalid target: '{}' has Shroud and cannot be targeted.", perm.name
                            )));
                        }
                        
                        if text.contains("hexproof") && perm.controller != source_controller {
                            return Some(Ruling::Illegal(format!(
                                "Invalid target: '{}' has Hexproof and cannot be targeted by spells controlled by an opponent.", perm.name
                            )));
                        }
                        
                        // Future: Check "Protection from [Color]" here
                    } else {
                        return Some(Ruling::Illegal(format!("Target permanent ID '{}' not found on the battlefield.", id)));
                    }
                },
                crate::models::Target::StackObject(id) => {
                    // Used for Counterspells, Forks, etc.
                    if !state.stack.iter().any(|obj| obj.id == *id) {
                        return Some(Ruling::Illegal(format!("Target spell ID '{}' not found on the stack.", id)));
                    }
                },
                crate::models::Target::Player(name) => {
                    // Verify the player exists (for now, hardcoded string check)
                    if name != "Player" && name != "Opponent" {
                        return Some(Ruling::Illegal(format!("Invalid player target: '{}'.", name)));
                    }
                },
                crate::models::Target::ZoneCard(_) => {
                    // Future: Graveyard or Exile targets (e.g., Reanimate)
                }
            }
        }
        
        // If we looped through all targets and found no violations, it's clean.
        None
    }
}
