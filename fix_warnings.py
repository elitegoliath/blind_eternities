with open('rust_core/src/models.rs', 'r') as f:
    text = f.read()
text = text.replace('pub fn emit_event(&mut self, event: crate::events::GameEvent) {',
                    '#[allow(dead_code)]\n    pub fn emit_event(&mut self, _event: crate::events::GameEvent) {')
text = text.replace('let mut new_triggers = Vec::new();', 'let new_triggers = Vec::new();')
with open('rust_core/src/models.rs', 'w') as f:
    f.write(text)

with open('rust_core/src/events.rs', 'r') as f:
    text = f.read()
text = text.replace('pub enum GameEvent', '#[allow(dead_code)]\npub enum GameEvent')
with open('rust_core/src/events.rs', 'w') as f:
    f.write(text)
