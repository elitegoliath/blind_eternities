// rust_core/src/rules.rs
// Rules engine for Magic: The Gathering game state assessment in Rust.
// This module checks for rule violations and state-based actions (SBAs).
// This file contains pure functions. They take data in and return a verdict.
// They do not talk to a database or the internet; they just compute "Magic Physics."

use crate::models::{
    Card, CardType, Effect, GameAction, GameState, ManaPool, Permanent, Phase, Ruling, Step, Target,
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
            GameAction::CastSpell {
                card,
                targets,
                targeting_requirements,
            } => {
                if let Some(target_violation) = Judge::check_targets(
                    state,
                    &state.active_player,
                    targets,
                    targeting_requirements,
                ) {
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
            GameAction::ActivateAbility {
                targets,
                targeting_requirements,
                ..
            } => {
                if let Some(target_violation) = Judge::check_targets(
                    state,
                    &state.active_player,
                    targets,
                    targeting_requirements,
                ) {
                    rulings.push(target_violation);
                }
            }
            GameAction::DeclareAttackers { attackers: _ } => {}
            GameAction::DeclareBlockers { blockers: _ } => {}
            GameAction::PassPriority => {}
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
    #[allow(dead_code)]
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
    /// Enforces CR 117.5: State-Based Actions and Triggered Abilities before priority.
    pub fn enforce_sbas_and_triggers(&self, state: &mut GameState) {
        loop {
            // 2. Execute engine-level SBAs (e.g. lethal damage, 0 toughness)
            state.run_sba_loop();

            // Execute validator-based SBAs (e.g. Legend Rule)
            let _ = self.enforce_sbas(state);

            // 3. (Scaffold for later) Check for waiting Triggered Abilities
            let triggers_added = self.put_waiting_triggers_on_stack(state);

            // 4. If triggers were placed on the stack, SBAs must be checked AGAIN
            if !triggers_added {
                break;
            }
        }
    }

    /// Helper for placing waiting triggers onto the stack (CR 603)
    fn put_waiting_triggers_on_stack(&self, state: &mut GameState) -> bool {
        if state.pending_triggers.is_empty() {
            return false;
        }

        let mut ap_triggers = Vec::new();
        let mut nap_triggers = Vec::new();

        for trigger in state.pending_triggers.drain(..) {
            if trigger.controller == state.active_player {
                ap_triggers.push(trigger);
            } else {
                nap_triggers.push(trigger);
            }
        }

        // Active Player triggers on stack first (resolve last)
        // TODO: Handle simultaneous triggers controlled by the same player (let them choose order)
        for (i, trigger) in ap_triggers.into_iter().enumerate() {
            let card_dummy = crate::models::Card {
                name: format!("Ability from {}", trigger.source_id),
                mana_cost: "".to_string(),
                type_line: vec![],
                oracle_text: "".to_string(),
                effects: vec![trigger.effect],
            };
            state.stack.push(crate::models::StackObject {
                id: format!("trigger-{}-{}", trigger.source_id, i),
                card: card_dummy,
                controller: trigger.controller,
                targets: vec![],
                targeting_requirements: vec![],
                source_id: Some(trigger.source_id),
            });
        }

        // Non-Active Player triggers on stack next (resolve first)
        for (i, trigger) in nap_triggers.into_iter().enumerate() {
            let card_dummy = crate::models::Card {
                name: format!("Ability from {}", trigger.source_id),
                mana_cost: "".to_string(),
                type_line: vec![],
                oracle_text: "".to_string(),
                effects: vec![trigger.effect],
            };
            state.stack.push(crate::models::StackObject {
                id: format!("trigger-nap-{}-{}", trigger.source_id, i),
                card: card_dummy,
                controller: trigger.controller,
                targets: vec![],
                targeting_requirements: vec![],
                source_id: Some(trigger.source_id),
            });
        }

        true
    }

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
    pub fn apply_action(
        &self,
        state: &mut GameState,
    ) -> Result<String, crate::errors::EngineError> {
        // 1. Verify Legality First
        let rulings = self.assess_state(state);
        for r in rulings {
            if let Ruling::Illegal(reason) = r {
                return Err(reason.clone());
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
                    let perm_id = perm.id.clone();
                    state.battlefield.push(perm);

                    state.emit_event(crate::events::GameEvent::ZoneChange {
                        object_id: perm_id,
                        from_zone: "Hand".to_string(),
                        to_zone: "Battlefield".to_string(),
                    });
                }
                GameAction::CastSpell {
                    card,
                    targets,
                    targeting_requirements,
                } => {
                    // Calculate Cost again
                    let (generic, cost_pool) =
                        crate::models::ManaPool::from_cost_string(&card.mana_cost)
                            .map_err(|e| crate::errors::EngineError::Custom(e))?;

                    // Pay Mana (Mutates Pool)
                    let active_player = state.active_player.clone();
                    if !state
                        .get_mana_pool_mut(&active_player)
                        .pay(&cost_pool, generic)
                    {
                        return Err(crate::errors::EngineError::Custom(
                            "CRITICAL: Mana validation passed but payment failed.".to_string(),
                        ));
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
                        targeting_requirements: targeting_requirements.clone(),
                        source_id: None,
                    };

                    state.stack.push(spell);
                }
                GameAction::ActivateAbility {
                    source_id,
                    ability_index,
                    targets,
                    targeting_requirements,
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
                            targeting_requirements: targeting_requirements.clone(),
                            source_id: Some(source_id.clone()),
                        };
                        state.stack.push(ability_obj);
                    } else {
                        return Err(crate::errors::EngineError::Custom(format!(
                            "Permanent with id '{}' not found.",
                            source_id
                        )));
                    }
                }
                GameAction::DeclareAttackers { attackers } => {
                    state.attackers = attackers.clone();
                }
                GameAction::DeclareBlockers { blockers } => {
                    state.blockers = blockers.clone();
                }
                GameAction::PassPriority => {
                    return self.pass_priority(state);
                }
                GameAction::Custom(_) => {}
            }
        }

        // 3. Cleanup
        state.pending_action = None;
        self.enforce_sbas_and_triggers(state);
        state.priority_player = state.active_player.clone();
        state.consecutive_passes = 0;
        Ok("Action successfully applied.".to_string())
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
    /// Pops the top of the stack and resolves it
    pub fn resolve_top(&self, state: &mut GameState) -> Result<String, crate::errors::EngineError> {
        let top = state.stack.pop().ok_or_else(|| {
            crate::errors::EngineError::StackEmpty("The stack is already empty.".to_string())
        })?;

        // CR 608.2b - Target Re-validation (The Fizzle Rule)
        if !top.targets.is_empty() {
            let mut any_legal = false;
            for (idx, target) in top.targets.iter().enumerate() {
                let single_target = [target.clone()];
                let single_req = if top.targeting_requirements.len() > idx {
                    [top.targeting_requirements[idx].clone()]
                } else {
                    [crate::models::TargetRequirement::Any]
                };

                if Judge::check_targets(state, &top.controller, &single_target, &single_req)
                    .is_none()
                {
                    any_legal = true;
                }
            }

            if !any_legal {
                state.graveyard.push(top.card.clone());
                return Ok(format!(
                    "{} fizzled because all targets were illegal.",
                    top.card.name
                ));
            }
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
            let perm_id = perm.id.clone();
            state.battlefield.push(perm);

            state.emit_event(crate::events::GameEvent::ZoneChange {
                object_id: perm_id,
                from_zone: "Stack".to_string(),
                to_zone: "Battlefield".to_string(),
            });

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
                                    effect_msgs.push(format!("Destroyed {}.", target_name));
                                }
                            }
                        }
                    }
                    Effect::DrawCards { amount } => {
                        // TODO: Future architecture will pop off the deck array and push to the hand array.
                        // For now, we just validate the action and report the game state change.
                        effect_msgs.push(format!("Player draws {} card(s).", amount));
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
                                    effect_msgs.push(format!("Countered {}.", countered_name));
                                } else {
                                    effect_msgs.push(format!(
                                        "Its target was no longer on the stack (Fizzled)."
                                    ));
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

        self.enforce_sbas_and_triggers(state);
        state.priority_player = state.active_player.clone();
        state.consecutive_passes = 0;

        Ok(format!(
            "{} resolved. {}",
            top.card.name,
            effect_msgs.join(" ")
        ))
    }

    /// Internal Logic: Parameterized Land Drops
    fn check_land_drop(state: &GameState, card: &Card) -> Ruling {
        if !card.type_line.contains(&CardType::Land) {
            return Ruling::Illegal(crate::errors::EngineError::OutOfPhase(
                "Not a Land".to_string(),
            ));
        }
        if state.priority_player != state.active_player {
            return Ruling::Illegal(crate::errors::EngineError::OutOfPhase(
                "Not your turn".to_string(),
            ));
        }
        if !state.stack.is_empty() {
            return Ruling::Illegal(crate::errors::EngineError::OutOfPhase(
                "Stack not empty".to_string(),
            ));
        }

        // Read the limit from config
        if state.lands_played >= state.rules_config.max_lands_per_turn {
            return Ruling::Illegal(crate::errors::EngineError::IllegalTarget(format!(
                "Land limit of {} reached",
                state.rules_config.max_lands_per_turn
            )));
        }

        match state.phase {
            Phase::PreCombatMain | Phase::PostCombatMain => Ruling::Legal,
            _ => Ruling::Illegal(crate::errors::EngineError::OutOfPhase(
                "Wrong Phase".to_string(),
            )),
        }
    }

    /// Internal Logic: Casting a Spell (Timing Rules)
    fn check_cast_timing(state: &GameState, card: &Card) -> Ruling {
        let is_instant = card.type_line.contains(&CardType::Instant);
        // Note: checking flash would go here too
        if is_instant {
            return Ruling::Legal;
        }
        // Sorcery Speed Checks
        if state.priority_player != state.active_player {
            return Ruling::Illegal(crate::errors::EngineError::OutOfPhase(
                "Not your turn".to_string(),
            ));
        }
        if !state.stack.is_empty() {
            return Ruling::Illegal(crate::errors::EngineError::OutOfPhase(
                "Stack not empty".to_string(),
            ));
        }
        match state.phase {
            Phase::PreCombatMain | Phase::PostCombatMain => Ruling::Legal,
            _ => Ruling::Illegal(crate::errors::EngineError::OutOfPhase(
                "Wrong Phase".to_string(),
            )),
        }
    }

    /// Internal Logic: Casting a Spell (Mana Cost Rules)
    fn check_mana_cost(state: &GameState, card: &Card) -> Ruling {
        // Use Model Parser
        let (required_generic, required_pool) = match ManaPool::from_cost_string(&card.mana_cost) {
            Ok(res) => res,
            Err(e) => {
                return Ruling::Illegal(crate::errors::EngineError::IllegalTarget(format!(
                    "Invalid Cost: {}",
                    e
                )))
            }
        };

        // Simulate Payment
        let mut temp_pool = state.get_mana_pool(&state.active_player);
        if temp_pool.pay(&required_pool, required_generic) {
            Ruling::Legal
        } else {
            Ruling::Illegal(crate::errors::EngineError::InsufficientMana(
                "Insufficient Mana".to_string(),
            ))
        }
    }

    /// Internal Logic: Target Legality (CR 601.2c)
    fn check_targets(
        state: &GameState,
        source_controller: &str,
        targets: &[crate::models::Target],
        targeting_requirements: &[crate::models::TargetRequirement],
    ) -> Option<Ruling> {
        for (idx, target) in targets.iter().enumerate() {
            let req = targeting_requirements
                .get(idx)
                .unwrap_or(&crate::models::TargetRequirement::Any);

            match target {
                Target::Permanent(id) => {
                    let target_perm = state.battlefield.iter().find(|p| p.id == *id);

                    if let Some(perm) = target_perm {
                        match req {
                            crate::models::TargetRequirement::Permanent { types } => {
                                if !types.is_empty() && !types.iter().any(|t| perm.current_characteristics.types.contains(t)) {
                                    return Some(Ruling::Illegal(crate::errors::EngineError::IllegalTarget(format!(
                                        "Invalid target: '{}' does not match the required permanent types.", perm.name
                                    ))));
                                }
                            }
                            crate::models::TargetRequirement::Any => {}
                            _ => return Some(Ruling::Illegal(crate::errors::EngineError::InsufficientMana("Invalid target: must be a spell or player, but a permanent was chosen.".to_string()))),
                        }

                        let text = perm.oracle_text.to_lowercase();
                        if text.contains("shroud") {
                            return Some(Ruling::Illegal(
                                crate::errors::EngineError::IllegalTarget(format!(
                                    "Invalid target: '{}' has Shroud and cannot be targeted.",
                                    perm.name
                                )),
                            ));
                        }
                        if text.contains("hexproof") && perm.controller != source_controller {
                            return Some(Ruling::Illegal(crate::errors::EngineError::IllegalTarget(format!("Invalid target: '{}' has Hexproof and cannot be targeted by spells controlled by an opponent.", perm.name))));
                        }
                        if Self::player_has_protection(state, &perm.name, source_controller) {
                            return Some(Ruling::Illegal(
                                crate::errors::EngineError::IllegalTarget(format!(
                                    "Invalid target: '{}' has Hexproof or Shroud.",
                                    perm.name
                                )),
                            ));
                        }
                    } else {
                        return Some(Ruling::Illegal(crate::errors::EngineError::IllegalTarget(
                            format!("Target permanent ID '{}' not found on the battlefield.", id),
                        )));
                    }
                }
                Target::StackObject(id) => {
                    match req {
                        crate::models::TargetRequirement::Spell | crate::models::TargetRequirement::Any => {}
                        _ => return Some(Ruling::Illegal(crate::errors::EngineError::InsufficientMana("Invalid target: must be a permanent or player, but a spell on the stack was chosen.".to_string()))),
                    }
                    if !state.stack.iter().any(|obj| obj.id == *id) {
                        return Some(Ruling::Illegal(crate::errors::EngineError::IllegalTarget(
                            format!("Target spell ID '{}' not found on the stack.", id),
                        )));
                    }
                }
                Target::Player(name) => {
                    match req {
                        crate::models::TargetRequirement::Player | crate::models::TargetRequirement::Any => {}
                        _ => return Some(Ruling::Illegal(crate::errors::EngineError::InsufficientMana("Invalid target: must be a permanent or spell, but a player was chosen.".to_string()))),
                    }
                    if name != "Player" && name != "Opponent" {
                        return Some(Ruling::Illegal(crate::errors::EngineError::IllegalTarget(
                            format!("Invalid player target: '{}'.", name),
                        )));
                    }
                }
                Target::ZoneCard(_) => {}
                Target::Custom(_) => {}
            }
        }
        None
    }

    /// Actively sweeps the board and removes permanents that violate state (CR 704)
    pub fn resolve_combat_damage(
        &self,
        state: &mut GameState,
    ) -> Result<String, crate::errors::EngineError> {
        let mut effect_msgs = Vec::new();

        // Quick copy of combatants to avoid borrow checker issues when mutating damage
        let attackers = state.attackers.clone();
        let blockers_map = state.blockers.clone();

        for attacker_id in &attackers {
            // Find attacker
            let attacker_power =
                if let Some(a) = state.battlefield.iter().find(|p| &p.id == attacker_id) {
                    a.current_characteristics.power
                } else {
                    continue; // Attacker died or vanished
                };

            let attacker_damage = std::cmp::max(0, attacker_power);

            let blockers = blockers_map.get(attacker_id);

            if let Some(blks) = blockers {
                if !blks.is_empty() {
                    // Blocked!
                    // Attacker deals damage to blockers. We distribute greedily for now.
                    let mut remaining_power = attacker_damage;

                    for blocker_id in blks {
                        let mut blocker_power = 0;
                        if let Some(b) = state.battlefield.iter_mut().find(|p| &p.id == blocker_id)
                        {
                            blocker_power = b.current_characteristics.power;
                            if remaining_power > 0 {
                                // Assign damage up to toughness or remaining power
                                let lethal =
                                    b.current_characteristics.toughness - b.damage_marked as i32;
                                let damage_to_deal = if lethal > 0 {
                                    remaining_power.min(lethal)
                                } else {
                                    0
                                };

                                b.damage_marked += damage_to_deal as u32;
                                remaining_power -= damage_to_deal;
                            }
                        }

                        // Blocker deals damage back
                        let blocker_damage = std::cmp::max(0, blocker_power);
                        if blocker_damage > 0 {
                            if let Some(a) =
                                state.battlefield.iter_mut().find(|p| &p.id == attacker_id)
                            {
                                a.damage_marked += blocker_damage as u32;
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

            let defending_player = if controller == "Player" {
                "Opponent"
            } else {
                "Player"
            };

            if attacker_damage > 0 {
                if let Some(life) = state.life_totals.get_mut(defending_player) {
                    *life -= attacker_damage;
                    effect_msgs.push(format!(
                        "{} dealt {} damage to {}.",
                        attacker_name, attacker_damage, defending_player
                    ));
                }
            }
        }

        state.attackers.clear();
        state.blockers.clear();

        self.enforce_sbas_and_triggers(state);
        state.priority_player = state.active_player.clone();
        state.consecutive_passes = 0;

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

            let is_creature = perm
                .current_characteristics
                .types
                .contains(&CardType::Creature);
            if is_creature
                && (perm.current_characteristics.toughness <= 0
                    || perm.damage_marked >= perm.current_characteristics.toughness as u32)
            {
                messages.push(format!(
                    "{} was destroyed by state-based actions.",
                    perm.name
                ));
                should_remove = true;
            }

            if !should_remove
                && state.rules_config.legend_rule_enabled
                && perm
                    .current_characteristics
                    .types
                    .contains(&CardType::Legendary)
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
    pub fn advance_step(
        &self,
        state: &mut GameState,
    ) -> Result<String, crate::errors::EngineError> {
        let (next_phase, next_step) = match (&state.phase, &state.step) {
            (Phase::Beginning, Some(Step::Untap)) => (Phase::Beginning, Step::Upkeep),
            (Phase::Beginning, Some(Step::Upkeep)) => (Phase::Beginning, Step::Draw),
            (Phase::Beginning, Some(Step::Draw)) => (Phase::PreCombatMain, Step::BeginCombat), // Actually it goes to Main 1, but we don't have a Main 1 step, we just have Phase::PreCombatMain. Let's use None for main phases.
            (Phase::Beginning, _) => (Phase::PreCombatMain, Step::BeginCombat),

            (Phase::PreCombatMain, _) => (Phase::Combat, Step::BeginCombat),

            (Phase::Combat, Some(Step::BeginCombat)) => (Phase::Combat, Step::DeclareAttackers),
            (Phase::Combat, Some(Step::DeclareAttackers)) => (Phase::Combat, Step::DeclareBlockers),
            (Phase::Combat, Some(Step::DeclareBlockers)) => (Phase::Combat, Step::CombatDamage),
            (Phase::Combat, Some(Step::CombatDamage)) => (Phase::Combat, Step::EndCombat),
            (Phase::Combat, Some(Step::EndCombat)) => (Phase::PostCombatMain, Step::End), // dummy step
            (Phase::Combat, _) => (Phase::PostCombatMain, Step::End),

            (Phase::PostCombatMain, _) => (Phase::Ending, Step::End),

            (Phase::Ending, Some(Step::End)) => (Phase::Ending, Step::Cleanup),
            (Phase::Ending, Some(Step::Cleanup)) => {
                // Next turn
                state.turn_number += 1;
                // We'll flip active player between "Player" and "Opponent" for a generic 2-player game
                if state.active_player == "Player" {
                    state.active_player = "Opponent".to_string();
                } else {
                    state.active_player = "Player".to_string();
                }
                (Phase::Beginning, Step::Untap)
            }
            (Phase::Ending, _) => {
                // Fallback
                (Phase::Beginning, Step::Untap)
            }

            (Phase::Custom(_), _) => (Phase::Beginning, Step::Untap),
        };

        let msg = format!("Advanced to {:?} {:?}", next_phase, next_step);
        state.phase = next_phase;
        state.step = Some(next_step);
        state.mana_pool.clear(); // Mana empties as steps/phases end

        self.enforce_sbas_and_triggers(state);
        state.priority_player = state.active_player.clone(); // AP gets priority first
        state.consecutive_passes = 0;

        Ok(msg)
    }

    pub fn pass_priority(
        &self,
        state: &mut GameState,
    ) -> Result<String, crate::errors::EngineError> {
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
                return self.advance_step(state);
            }
        }

        // Pass priority to the other player (Assume 2 players)
        if state.priority_player == "Player" {
            state.priority_player = "Opponent".to_string();
        } else {
            state.priority_player = "Player".to_string();
        }

        Ok(format!("Priority passed to {}.", state.priority_player))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::GameEvent;
    use crate::models::{Ability, CardType, Effect, GameState, Permanent, Phase, TriggerCondition};

    #[test]
    fn test_etb_trigger_lifecycle() {
        let mut state = GameState {
            active_player: "Player".to_string(),
            priority_player: "Player".to_string(),
            turn_number: 1,
            phase: Phase::PreCombatMain,
            step: None,
            stack: vec![],
            battlefield: vec![],
            graveyard: vec![],
            exile: vec![],
            hand: std::collections::HashMap::new(),
            life_totals: std::collections::HashMap::new(),
            mana_pool: std::collections::HashMap::new(),
            continuous_effects: vec![],
            pending_action: None,
            attackers: vec![],
            blockers: std::collections::HashMap::new(),
            lands_played: 0,
            consecutive_passes: 0,
            rules_config: crate::models::RulesConfig::default(),
            pending_triggers: vec![],
        };

        // Setup: Add a permanent with an ETB ability
        let perm = Permanent {
            id: "test-perm-1".to_string(),
            name: "Elvish Visionary".to_string(),
            oracle_text: "When Elvish Visionary enters the battlefield, draw a card.".to_string(),
            mana_value: 2,
            is_legendary: false,
            controller: "Player".to_string(),
            is_tapped: false,
            damage_marked: 0,
            counters: std::collections::HashMap::new(),
            base_characteristics: crate::models::Characteristics {
                types: vec![CardType::Creature],
                colors: vec![],
                abilities: vec![Ability::Triggered {
                    condition: TriggerCondition::EntersBattlefield,
                    effect: Effect::DrawCards { amount: 1 },
                }],
                power: 1,
                toughness: 1,
            },
            current_characteristics: crate::models::Characteristics {
                types: vec![CardType::Creature],
                colors: vec![],
                abilities: vec![Ability::Triggered {
                    condition: TriggerCondition::EntersBattlefield,
                    effect: Effect::DrawCards { amount: 1 },
                }],
                power: 1,
                toughness: 1,
            },
        };
        state.battlefield.push(perm);

        // Action 1: Simulate the ZoneChange event
        state.emit_event(GameEvent::ZoneChange {
            object_id: "test-perm-1".to_string(),
            from_zone: "Stack".to_string(),
            to_zone: "Battlefield".to_string(),
        });

        // Assert 1
        assert_eq!(state.pending_triggers.len(), 1);

        // Action 2: Run the engine SBA/Trigger loop
        let judge = Judge::default_engine();
        judge.enforce_sbas_and_triggers(&mut state);

        // Assert 2
        assert_eq!(state.pending_triggers.len(), 0);
        assert_eq!(state.stack.len(), 1);

        let stack_top = &state.stack[0];
        assert_eq!(stack_top.controller, "Player");
        assert_eq!(stack_top.source_id, Some("test-perm-1".to_string()));
    }
}
