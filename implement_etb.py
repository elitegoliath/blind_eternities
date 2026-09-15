import re

# 1. Update events.rs
with open('rust_core/src/events.rs', 'r') as f:
    events_code = f.read()
events_code = events_code.replace('card_name: String,', 'object_id: String,')
with open('rust_core/src/events.rs', 'w') as f:
    f.write(events_code)

# 2. Update models.rs
with open('rust_core/src/models.rs', 'r') as f:
    models_code = f.read()

enums_code = """#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum TriggerCondition {
    EntersBattlefield,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Ability {
    Triggered {
        condition: TriggerCondition,
        effect: Effect,
    },
}

// Define the Effect Enum"""
models_code = models_code.replace('// Define the Effect Enum', enums_code)

models_code = models_code.replace(
    'pub counters: std::collections::HashMap<String, u32>,',
    'pub counters: std::collections::HashMap<String, u32>,\n    #[serde(default)]\n    pub abilities: Vec<Ability>,'
)

models_code = models_code.replace(
    'counters: HashMap::new(),\n        }',
    'counters: HashMap::new(),\n            abilities: Vec::new(),\n        }'
)

pattern = r'(?:#\[allow\(dead_code\)\]\n\s*)?pub fn emit_event\(&mut self.*?self\.pending_triggers\.extend\(new_triggers\);\n\s*\}'
new_emit = """pub fn emit_event(&mut self, event: crate::events::GameEvent) {
        let mut new_triggers = Vec::new();
        match &event {
            crate::events::GameEvent::ZoneChange { object_id, to_zone, .. } => {
                if to_zone == "Battlefield" {
                    if let Some(perm) = self.battlefield.iter().find(|p| &p.id == object_id) {
                        for ability in &perm.abilities {
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
    }"""
models_code = re.sub(pattern, new_emit, models_code, flags=re.DOTALL)

with open('rust_core/src/models.rs', 'w') as f:
    f.write(models_code)

# 3. Update rules.rs
with open('rust_core/src/rules.rs', 'r') as f:
    rules_code = f.read()

if "mod tests {" not in rules_code:
    test_code = """

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{GameState, Permanent, Phase, Effect, Ability, TriggerCondition, CardType};
    use crate::events::GameEvent;

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
            types: vec![CardType::Creature],
            colors: vec![],
            is_legendary: false,
            controller: "Player".to_string(),
            is_tapped: false,
            damage_marked: 0,
            power: 1,
            toughness: 1,
            base_power: 1,
            base_toughness: 1,
            counters: std::collections::HashMap::new(),
            abilities: vec![
                Ability::Triggered {
                    condition: TriggerCondition::EntersBattlefield,
                    effect: Effect::DrawCards { amount: 1 },
                }
            ],
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
"""
    rules_code += test_code
    with open('rust_core/src/rules.rs', 'w') as f:
        f.write(rules_code)
