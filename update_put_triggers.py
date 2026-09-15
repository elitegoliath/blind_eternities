import re
with open('rust_core/src/rules.rs', 'r') as f:
    content = f.read()

new_fn = """    fn put_waiting_triggers_on_stack(&self, state: &mut GameState) -> bool {
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
                source_id: Some(trigger.source_id),
            });
        }

        true
    }"""

# regex replace
content = re.sub(
    r'    fn put_waiting_triggers_on_stack\(&self, state: &mut GameState\) -> bool \{\n\s*// TODO.*?\n\s*// Return true.*?\n\s*false\n\s*\}',
    new_fn,
    content,
    flags=re.DOTALL
)
# Note: Since the prompt is:
# fn put_waiting_triggers_on_stack(&self, _state: &mut GameState) -> bool {
content = re.sub(
    r'    fn put_waiting_triggers_on_stack\(&self, _?state: &mut GameState\) -> bool \{.*?\n    \}',
    new_fn,
    content,
    flags=re.DOTALL
)

with open('rust_core/src/rules.rs', 'w') as f:
    f.write(content)
