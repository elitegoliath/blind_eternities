// rust_core/src/rules.rs
// Rules engine for Magic: The Gathering game state assessment in Rust.
// This module checks for rule violations and state-based actions (SBAs).
// This file contains pure functions. They take data in and return a verdict.
// They do not talk to a database or the internet; they just compute "Magic Physics."

use std::collections::HashMap;

use crate::models::{Card, CardType, GameAction, GameState, ManaPool, Permanent, Phase, RulesConfig, Ruling};

pub struct Judge;

impl Judge {
    /// The Main Loop: Checks for any violations or triggers
    pub fn assess_state(state: &GameState) -> Vec<Ruling> {
        let mut rulings = Vec::new();
        
        // Check Player Actions
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

    /// Helper: Does a player have Hexproof or Shroud?
    fn player_has_protection(state: &GameState, target_player: &str, source_controller: &str) -> bool {
        state.battlefield.iter().any(|perm| {
            // Did this player's permanent grant them protection?
            if perm.controller == target_player {
                let text = perm.oracle_text.to_lowercase();
                let has_shroud = text.contains("you have shroud");
                let has_hexproof = text.contains("you have hexproof");
                
                if has_shroud { return true; }
                if has_hexproof && target_player != source_controller { return true; }
            }
            false
        })
    }

    /// Re-evaluates targets upon resolution (CR 608.2b)
    /// Returns true if at least ONE target is still legal.
    fn are_targets_still_legal(state: &GameState, controller: &str, targets: &[crate::models::Target]) -> bool {
        if targets.is_empty() { return true; } // Spells without targets always resolve

        let mut legal_count = 0;

        for target in targets {
            match target {
                crate::models::Target::Permanent(id) => {
                    // Still on the battlefield?
                    if let Some(perm) = state.battlefield.iter().find(|p| p.id == *id) {
                        // Still lacking Shroud/Hexproof?
                        let text = perm.oracle_text.to_lowercase();
                        let shroud = text.contains("shroud");
                        let hexproof = text.contains("hexproof") && perm.controller != controller;

                        if !shroud && !hexproof {
                            legal_count += 1;
                        }
                    }
                },
                crate::models::Target::StackObject(id) => {
                    if state.stack.iter().any(|obj| obj.id == *id) {
                        legal_count += 1;
                    }
                },
                crate::models::Target::Player(name) => {
                    if !Self::player_has_protection(state, name, controller) {
                        legal_count += 1;
                    }
                },
                _ => { legal_count += 1; }
            }
        }

        legal_count > 0
    }

    /// Pops the top of the stack and resolves it
    pub fn resolve_top(state: &mut GameState) -> Result<String, String> {
        let top = state.stack.pop().ok_or("The stack is already empty.")?;

        if !Self::are_targets_still_legal(state, &top.controller, &top.targets) {
            return Ok(format!("Spell '{}' fizzled because all targets became illegal.", top.card.name));
        }

        let mut effect_msgs = Vec::new();
        let is_permanent = top.card.type_line.iter().any(|t| 
            matches!(t, CardType::Creature | CardType::Artifact | CardType::Enchantment | CardType::Planeswalker)
        );

        if is_permanent {
            let perm = Permanent::from_card(&top.card, top.controller.clone(), state.battlefield.len());
            state.battlefield.push(perm);
            effect_msgs.push(format!("{} entered the battlefield.", top.card.name));
        } else {
            // Instant or Sorcery
            for effect in &top.card.effects {
                match effect {
                    crate::models::Effect::DealDamage { amount } => {
                        for target in &top.targets {
                            if let crate::models::Target::Permanent(id) = target {
                                if let Some(perm) = state.battlefield.iter_mut().find(|p| p.id == *id) {
                                    perm.damage_marked += amount;
                                    effect_msgs.push(format!("Dealt {} damage to {}.", amount, perm.name));
                                }
                            }
                        }
                    }
                }
            }
        }

        // CR 117.5: Enforce SBAs immediately after ANY spell resolves (Permanent or Spell)
        let sba_msgs = Self::enforce_sbas(state);
        effect_msgs.extend(sba_msgs);

        Ok(format!("{} resolved. {}", top.card.name, effect_msgs.join(" ")))
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
                    if Self::player_has_protection(state, name, source_controller) {
                        return Some(Ruling::Illegal(format!("Invalid target: '{}' has Hexproof or Shroud.", name)));
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

    /// Actively sweeps the board and removes permanents that violate state (CR 704)
    pub fn enforce_sbas(state: &mut GameState) -> Vec<String> {
        let mut messages = Vec::new();
        let mut seen_legends: std::collections::HashMap<(String, String), usize> = std::collections::HashMap::new();
        let mut i = 0;

        while i < state.battlefield.len() {
            let mut should_remove = false;
            let perm = &state.battlefield[i].clone(); // Clone for safe reading
            
            // 1. Lethal Damage (AUTOMATIC EXECUTION)
            let is_creature = perm.types.contains(&CardType::Creature);
            if is_creature && (perm.toughness <= 0 || perm.damage_marked >= perm.toughness as u32) {
                messages.push(format!("{} was destroyed by state-based actions.", perm.name));
                should_remove = true;
            }

            // 2. The Legend Rule (CHOICE REQUIRED)
            if !should_remove && state.rules_config.legend_rule_enabled && perm.types.contains(&CardType::Legendary) {
                let scope_key = if state.rules_config.legend_scope == "controller" {
                    (perm.name.clone(), perm.controller.clone())
                } else {
                    (perm.name.clone(), "global".to_string())
                };

                let count = seen_legends.entry(scope_key).or_insert(0);
                *count += 1;

                if *count > state.rules_config.legend_max_allowed {
                    // DO NOT DELETE IT. Just scream at the LLM to ask the player.
                    messages.push(format!("ACTION REQUIRED: Legend Rule violation for {}. Player must choose which one to keep and put the rest into the graveyard.", perm.name));
                }
            }

            // Execute Removal (Only for Automatic SBAs like damage)
            if should_remove {
                state.battlefield.remove(i);
            } else {
                i += 1;
            }
        }
        
        messages
    }
}
