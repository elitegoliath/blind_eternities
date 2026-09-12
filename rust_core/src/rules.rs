// rust_core/src/rules.rs
// Rules engine for Magic: The Gathering game state assessment in Rust.
// This module checks for rule violations and state-based actions (SBAs).
// This file contains pure functions. They take data in and return a verdict.
// They do not talk to a database or the internet; they just compute "Magic Physics."

use crate::models::{
    Card, CardType, Effect, GameAction, GameState, ManaPool, Permanent, Phase, Ruling, Target,
};

pub trait RuleValidator: Send + Sync {
    fn assess_action(&self, _state: &GameState, _action: &GameAction) -> Vec<Ruling> {
        Vec::new()
    }
    fn enforce_sbas(&self, _state: &mut GameState) -> Vec<String> {
        Vec::new()
    }
}

pub struct DefaultActionValidator;
impl RuleValidator for DefaultActionValidator {
    fn assess_action(&self, state: &GameState, action: &GameAction) -> Vec<Ruling> {
        let mut rulings = Vec::new();
        match action {
            GameAction::PlayLand(card) => {
                rulings.push(Judge::check_land_drop(state, card));
            }
            GameAction::CastSpell { card, targets } => {
                if let Some(target_violation) =
                    Judge::check_targets(state, &state.active_player, targets)
                {
                    rulings.push(target_violation);
                } else {
                    let timing = Judge::check_cast_timing(state, card);
                    if let Ruling::Illegal(_) = timing {
                        rulings.push(timing);
                    } else {
                        rulings.push(Judge::check_mana_cost(state, card));
                    }
                }
            }
            GameAction::ActivateAbility { targets, .. } => {
                if let Some(target_violation) =
                    Judge::check_targets(state, &state.active_player, targets)
                {
                    rulings.push(target_violation);
                }
            }
            GameAction::DeclareAttackers { attackers: _ } => {}
            GameAction::DeclareBlockers { blockers: _ } => {}
            GameAction::Custom(_) => {}
        }
        rulings
    }
}

pub struct DefaultSBAValidator;
impl RuleValidator for DefaultSBAValidator {
    fn enforce_sbas(&self, state: &mut GameState) -> Vec<String> {
        Judge::default_enforce_sbas(state)
    }
}

pub struct Judge {
    pub validators: Vec<Box<dyn RuleValidator>>,
}

impl Judge {
    pub fn new(validators: Vec<Box<dyn RuleValidator>>) -> Self {
        Self { validators }
    }

    pub fn default_engine() -> Self {
        Self {
            validators: vec![
                Box::new(DefaultActionValidator),
                Box::new(DefaultSBAValidator),
            ],
        }
    }
    /// The Main Loop: Checks for any violations or triggers
    pub fn assess_state(&self, state: &GameState) -> Vec<Ruling> {
        let mut rulings = Vec::new();
        if let Some(action) = &state.pending_action {
            for v in &self.validators {
                rulings.extend(v.assess_action(state, action));
            }
        }
        
        let mut final_rulings = Vec::new();
        for r in rulings {
            if r != Ruling::Legal {
                final_rulings.push(r);
            }
        }
        
        if final_rulings.is_empty() {
            vec![Ruling::Legal]
        } else {
            final_rulings
        }
    }

    /// Validation + Execution
    /// Returns Ok(NewState) or Err(Reason)
    pub fn apply_action(&self, state: &mut GameState) -> Result<(), String> {
        // 1. Verify Legality First
        let rulings = self.assess_state(state);
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
                        state.battlefield.len(),
                    );
                    state.battlefield.push(perm);
                }
                GameAction::CastSpell { card, targets } => {
                    // Calculate Cost again
                    let (generic, cost_pool) =
                        crate::models::ManaPool::from_cost_string(&card.mana_cost)
                            .map_err(|e| e)?;

                    // Pay Mana (Mutates Pool)
                    if !state.mana_pool.pay(&cost_pool, generic) {
                        return Err(
                            "CRITICAL: Mana validation passed but payment failed.".to_string()
                        );
                    }

                    // Move to Stack as a fully realized StackObject
                    let stack_id = format!(
                        "spell-{}-{}",
                        card.name.replace(" ", "").to_lowercase(),
                        state.stack.len()
                    );

                    let spell = crate::models::StackObject {
                        id: stack_id,
                        card: card.clone(),
                        controller: state.active_player.clone(),
                        targets: targets.clone(),
                        source_id: None,
                    };

                    state.stack.push(spell);
                }
                GameAction::ActivateAbility {
                    source_id,
                    ability_index,
                    targets,
                } => {
                    if let Some(perm) = state.battlefield.iter().find(|p| p.id == *source_id) {
                        let ability_card = crate::models::Card {
                            name: format!("Ability {} of {}", ability_index, perm.name),
                            type_line: vec![crate::models::CardType::Unknown],
                            mana_cost: "".to_string(),
                            oracle_text: "Activated Ability".to_string(),
                            effects: vec![],
                        };

                        let stack_id = format!("ability-{}-{}", source_id, state.stack.len());
                        let ability_obj = crate::models::StackObject {
                            id: stack_id,
                            card: ability_card,
                            controller: state.active_player.clone(),
                            targets: targets.clone(),
                            source_id: Some(source_id.clone()),
                        };
                        state.stack.push(ability_obj);
                    } else {
                        return Err(format!("Permanent with id '{}' not found.", source_id));
                    }
                }
                GameAction::DeclareAttackers { attackers } => {
                    state.attackers = attackers.clone();
                }
                GameAction::DeclareBlockers { blockers } => {
                    state.blockers = blockers.clone();
                }
                GameAction::Custom(_) => {}
            }
        }


        // 3. Cleanup
        state.pending_action = None;
        Ok(())
    }

    /// Helper: Does a player have Hexproof or Shroud?
    fn player_has_protection(
        state: &GameState,
        target_player: &str,
        source_controller: &str,
    ) -> bool {
        state.battlefield.iter().any(|perm| {
            // Did this player's permanent grant them protection?
            if perm.controller == target_player {
                let text = perm.oracle_text.to_lowercase();
                let has_shroud = text.contains("you have shroud");
                let has_hexproof = text.contains("you have hexproof");

                if has_shroud {
                    return true;
                }
                if has_hexproof && target_player != source_controller {
                    return true;
                }
            }
            false
        })
    }

    /// Re-evaluates targets upon resolution (CR 608.2b)
    /// Returns true if at least ONE target is still legal.
    fn are_targets_still_legal(
        state: &GameState,
        controller: &str,
        targets: &[crate::models::Target],
    ) -> bool {
        if targets.is_empty() {
            return true;
        } // Spells without targets always resolve

        let mut legal_count = 0;

        for target in targets {
            match target {
                Target::Permanent(id) => {
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
                }
                Target::StackObject(id) => {
                    if state.stack.iter().any(|obj| obj.id == *id) {
                        legal_count += 1;
                    }
                }
                Target::Player(name) => {
                    if !Self::player_has_protection(state, name, controller) {
                        legal_count += 1;
                    }
                }
                _ => {
                    legal_count += 1;
                }
            }
        }

        legal_count > 0
    }

    /// Pops the top of the stack and resolves it
    pub fn resolve_top(&self, state: &mut GameState) -> Result<String, String> {
        let top = state.stack.pop().ok_or("The stack is already empty.")?;

        if !Self::are_targets_still_legal(state, &top.controller, &top.targets) {
            return Ok(format!(
                "Spell '{}' fizzled because all targets became illegal.",
                top.card.name
            ));
        }

        let mut effect_msgs = Vec::new();
        let is_permanent = top.card.type_line.iter().any(|t| {
            matches!(
                t,
                CardType::Creature
                    | CardType::Artifact
                    | CardType::Enchantment
                    | CardType::Planeswalker
            )
        });

        if is_permanent {
            let perm =
                Permanent::from_card(&top.card, top.controller.clone(), state.battlefield.len());
            state.battlefield.push(perm);
            effect_msgs.push(format!("{} entered the battlefield.", top.card.name));
        } else {
            // Instant or Sorcery
            for effect in &top.card.effects {
                match effect {
                    Effect::DealDamage { amount } => {
                        for target in &top.targets {
                            if let Target::Permanent(id) = target {
                                if let Some(perm) =
                                    state.battlefield.iter_mut().find(|p| p.id == *id)
                                {
                                    perm.damage_marked += amount;
                                    effect_msgs
                                        .push(format!("Dealt {} damage to {}.", amount, perm.name));
                                }
                            }
                        }
                    }
                    Effect::Destroy => {
                        // Direct removal. Target is wiped from the battlefield immediately.
                        if let Some(target) = top.targets.first() {
                            // Match the tuple variant which only contains the ID string
                            if let Target::Permanent(target_id) = target {
                                let original_len = state.battlefield.len();

                                // Quickly grab the name of the permanent before we delete it (for the return message)
                                let target_name = state
                                    .battlefield
                                    .iter()
                                    .find(|p| p.id == *target_id)
                                    .map(|p| p.name.clone())
                                    .unwrap_or_else(|| "permanent".to_string());

                                // Retain everything that does NOT match the target_id
                                state.battlefield.retain(|p| p.id != *target_id);

                                if state.battlefield.len() < original_len {
                                    return Ok(format!(
                                        "{} resolved. Destroyed {}.",
                                        top.card.name, target_name
                                    ));
                                }
                            }
                        }
                    }
                    Effect::DrawCards { amount } => {
                        // TODO: Future architecture will pop off the deck array and push to the hand array.
                        // For now, we just validate the action and report the game state change.
                        return Ok(format!(
                            "{} resolved. Player draws {} card(s).",
                            top.card.name, amount
                        ));
                    }
                    Effect::Counter => {
                        if let Some(target) = top.targets.first() {
                            // Match the StackObject tuple variant which holds the spell's ID string
                            if let Target::StackObject(target_id) = target {
                                let original_len = state.stack.len();

                                // Grab the name of the spell we are countering for the return message
                                let countered_name = state
                                    .stack
                                    .iter()
                                    .find(|spell| spell.id == *target_id)
                                    .map(|spell| spell.card.name.clone())
                                    .unwrap_or_else(|| "spell".to_string());

                                // Retain everything on the stack that does NOT match the target_id
                                state.stack.retain(|spell| spell.id != *target_id);

                                if state.stack.len() < original_len {
                                    return Ok(format!(
                                        "{} resolved. Countered {}.",
                                        top.card.name, countered_name
                                    ));
                                } else {
                                    return Ok(format!("{} resolved, but its target was no longer on the stack (Fizzled).", top.card.name));
                                }
                            }
                        }
                    }
                    Effect::Custom(_) => {}
                }
            }
            // Put Instant/Sorcery in graveyard
            state.graveyard.push(top.card.clone());
        }

        // CR 117.5: Enforce SBAs immediately after ANY spell resolves (Permanent or Spell)
        let sba_msgs = self.enforce_sbas(state);
        effect_msgs.extend(sba_msgs);

        Ok(format!(
            "{} resolved. {}",
            top.card.name,
            effect_msgs.join(" ")
        ))
    }

    /// Internal Logic: Parameterized Land Drops
    fn check_land_drop(state: &GameState, card: &Card) -> Ruling {
        if !card.type_line.contains(&CardType::Land) {
            return Ruling::Illegal("Not a Land".into());
        }
        if !state.is_active_player {
            return Ruling::Illegal("Not your turn".into());
        }
        if !state.stack.is_empty() {
            return Ruling::Illegal("Stack not empty".into());
        }

        // Read the limit from config
        if state.lands_played >= state.rules_config.max_lands_per_turn {
            return Ruling::Illegal(format!(
                "Land limit of {} reached",
                state.rules_config.max_lands_per_turn
            ));
        }

        match state.phase {
            Phase::Main1 | Phase::Main2 => Ruling::Legal,
            _ => Ruling::Illegal("Wrong Phase".into()),
        }
    }

    /// Internal Logic: Casting a Spell (Timing Rules)
    fn check_cast_timing(state: &GameState, card: &Card) -> Ruling {
        let is_instant = card.type_line.contains(&CardType::Instant);
        if is_instant {
            return Ruling::Legal;
        }
        // Sorcery Speed Checks
        if !state.is_active_player {
            return Ruling::Illegal("Not your turn".into());
        }
        if !state.stack.is_empty() {
            return Ruling::Illegal("Stack not empty".into());
        }
        match state.phase {
            Phase::Main1 | Phase::Main2 => Ruling::Legal,
            _ => Ruling::Illegal("Wrong Phase".into()),
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
    fn check_targets(
        state: &GameState,
        source_controller: &str,
        targets: &[crate::models::Target],
    ) -> Option<Ruling> {
        for target in targets {
            match target {
                Target::Permanent(id) => {
                    // 1. Does it exist?
                    let target_perm = state.battlefield.iter().find(|p| p.id == *id);

                    if let Some(perm) = target_perm {
                        // 2. Check Targeting Restrictions (Shroud & Hexproof)
                        let text = perm.oracle_text.to_lowercase();

                        if text.contains("shroud") {
                            return Some(Ruling::Illegal(format!(
                                "Invalid target: '{}' has Shroud and cannot be targeted.",
                                perm.name
                            )));
                        }

                        if text.contains("hexproof") && perm.controller != source_controller {
                            return Some(Ruling::Illegal(format!(
                                "Invalid target: '{}' has Hexproof and cannot be targeted by spells controlled by an opponent.", perm.name
                            )));
                        }

                        // Future: Check "Protection from [Color]" here
                    } else {
                        return Some(Ruling::Illegal(format!(
                            "Target permanent ID '{}' not found on the battlefield.",
                            id
                        )));
                    }
                }
                Target::StackObject(id) => {
                    // Used for Counterspells, Forks, etc.
                    if !state.stack.iter().any(|obj| obj.id == *id) {
                        return Some(Ruling::Illegal(format!(
                            "Target spell ID '{}' not found on the stack.",
                            id
                        )));
                    }
                }
                Target::Player(name) => {
                    // Verify the player exists (for now, hardcoded string check)
                    if name != "Player" && name != "Opponent" {
                        return Some(Ruling::Illegal(format!(
                            "Invalid player target: '{}'.",
                            name
                        )));
                    }
                    if Self::player_has_protection(state, name, source_controller) {
                        return Some(Ruling::Illegal(format!(
                            "Invalid target: '{}' has Hexproof or Shroud.",
                            name
                        )));
                    }
                }
                Target::ZoneCard(_) => {
                    // Future: Graveyard or Exile targets (e.g., Reanimate)
                }
                Target::Custom(_) => {}
            }
        }

        // If we looped through all targets and found no violations, it's clean.
        None
    }

    /// Actively sweeps the board and removes permanents that violate state (CR 704)
    pub fn resolve_combat_damage(state: &mut GameState) -> Result<String, String> {
        let mut effect_msgs = Vec::new();
        
        // Quick copy of combatants to avoid borrow checker issues when mutating damage
        let attackers = state.attackers.clone();
        let blockers_map = state.blockers.clone();
        
        for attacker_id in &attackers {
            // Find attacker
            let attacker_power = if let Some(a) = state.battlefield.iter().find(|p| &p.id == attacker_id) {
                a.power
            } else {
                continue; // Attacker died or vanished
            };
            
            let blockers = blockers_map.get(attacker_id);
            
            if let Some(blks) = blockers {
                if !blks.is_empty() {
                    // Blocked!
                    // Attacker deals damage to blockers. We distribute greedily for now.
                    let mut remaining_power = attacker_power;
                    
                    for blocker_id in blks {
                        let mut blocker_power = 0;
                        if let Some(b) = state.battlefield.iter_mut().find(|p| &p.id == blocker_id) {
                            blocker_power = b.power;
                            if remaining_power > 0 {
                                // Assign damage up to toughness or remaining power
                                let lethal = b.toughness - b.damage_marked as i32;
                                let damage_to_deal = if lethal > 0 { 
                                    remaining_power.min(lethal) 
                                } else { 
                                    remaining_power 
                                };
                                
                                b.damage_marked += damage_to_deal as u32;
                                remaining_power -= damage_to_deal;
                            }
                        }
                        
                        // Blocker deals damage back
                        if blocker_power > 0 {
                            if let Some(a) = state.battlefield.iter_mut().find(|p| &p.id == attacker_id) {
                                a.damage_marked += blocker_power as u32;
                            }
                        }
                    }
                    effect_msgs.push(format!("Combat damage resolved for {}.", attacker_id));
                    continue;
                }
            }
            
            // Unblocked! Deals damage to opponent
            // Retrieve controller to figure out who is defending
            let mut attacker_name = "".to_string();
            let mut controller = "".to_string();
            if let Some(a) = state.battlefield.iter().find(|p| &p.id == attacker_id) {
                attacker_name = a.name.clone();
                controller = a.controller.clone();
            }
            
            let defending_player = if controller == "Player" { "Opponent" } else { "Player" };
            
            if let Some(life) = state.life_totals.get_mut(defending_player) {
                *life -= attacker_power;
                effect_msgs.push(format!("{} dealt {} damage to {}.", attacker_name, attacker_power, defending_player));
            }
        }
        
        state.attackers.clear();
        state.blockers.clear();
        
        if effect_msgs.is_empty() {
            Ok("No combat damage was dealt.".to_string())
        } else {
            Ok(effect_msgs.join(" "))
        }
    }

    /// Actively sweeps the board and removes permanents that violate state (CR 704)
    pub fn default_enforce_sbas(state: &mut GameState) -> Vec<String> {
        let mut messages = Vec::new();
        let mut seen_legends: std::collections::HashMap<(String, String), usize> =
            std::collections::HashMap::new();
        let mut i = 0;

        while i < state.battlefield.len() {
            let mut should_remove = false;
            let perm = &state.battlefield[i].clone();

            let is_creature = perm.types.contains(&CardType::Creature);
            if is_creature && (perm.toughness <= 0 || perm.damage_marked >= perm.toughness as u32) {
                messages.push(format!(
                    "{} was destroyed by state-based actions.",
                    perm.name
                ));
                should_remove = true;
            }

            if !should_remove
                && state.rules_config.legend_rule_enabled
                && perm.types.contains(&CardType::Legendary)
            {
                let scope_key = if state.rules_config.legend_scope == "controller" {
                    (perm.name.clone(), perm.controller.clone())
                } else {
                    (perm.name.clone(), "global".to_string())
                };

                let count = seen_legends.entry(scope_key).or_insert(0);
                *count += 1;

                if *count > state.rules_config.legend_max_allowed {
                    messages.push(format!("ACTION REQUIRED: Legend Rule violation for {}. Player must choose which one to keep and put the rest into the graveyard.", perm.name));
                }
            }

            if should_remove {
                state.battlefield.remove(i);
            } else {
                i += 1;
            }
        }
        messages
    }

    pub fn enforce_sbas(&self, state: &mut GameState) -> Vec<String> {
        let mut msgs = Vec::new();
        for v in &self.validators {
            msgs.extend(v.enforce_sbas(state));
        }
        msgs
    }

    // This function manages the priority sequence
    pub fn pass_priority(&self, state: &mut GameState) -> Result<String, String> {
        // Increment the counter every time a player passes
        state.consecutive_passes += 1;

        // Assuming a 2-player game: if both players pass in succession
        if state.consecutive_passes >= 2 {
            state.consecutive_passes = 0; // Reset for the next interaction

            if !state.stack.is_empty() {
                // CR 117.4: If all players pass, resolve the top spell on the stack
                return self.resolve_top(state);
            } else {
                // CR 117.4: If the stack is empty and all players pass, advance the phase
                // (Phase transition logic will be built out later)
                return Ok("Stack is empty. Proceeding to next phase/step.".to_string());
            }
        }

        Ok("Priority passed to the next player.".to_string())
    }
}
