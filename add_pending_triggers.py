import re
with open('rust_core/src/models.rs', 'r') as f:
    content = f.read()

# Add pending_triggers to GameState
if "pending_triggers" not in content:
    content = content.replace('    pub continuous_effects: Vec<ContinuousEffect>,',
                              '    pub continuous_effects: Vec<ContinuousEffect>,\n\n    #[serde(default)]\n    pub pending_triggers: Vec<crate::triggers::PendingTrigger>,')

# Add emit_event to GameState impl
if "pub fn emit_event" not in content:
    impl_insert = """
    /// Emits a game event, triggering permanents to place effects into the pending_triggers queue
    pub fn emit_event(&mut self, event: crate::events::GameEvent) {
        // TODO: Scan battlefield for permanents with matching triggers and push to pending_triggers.
        // For now, this is a scaffold.
        let mut new_triggers = Vec::new();
        // for perm in &self.battlefield {
        //     // check perm.triggers ...
        // }
        self.pending_triggers.extend(new_triggers);
    }
"""
    content = content.replace('    pub fn get_mana_pool(&self, player: &str) -> ManaPool {',
                              impl_insert + '\n    pub fn get_mana_pool(&self, player: &str) -> ManaPool {')

with open('rust_core/src/models.rs', 'w') as f:
    f.write(content)
