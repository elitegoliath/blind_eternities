with open('rust_core/src/rules.rs', 'r') as f:
    content = f.read()

# Fix the duplicate if block
bad_block = """        if effect_msgs.is_empty() {
            self.enforce_sbas_and_triggers(state);
        state.priority_player = state.active_player.clone();
        state.consecutive_passes = 0;

        if effect_msgs.is_empty() {
            Ok("No combat damage was dealt.".to_string())
        } else {
            Ok(effect_msgs.join(" "))
        }"""

good_block = """        self.enforce_sbas_and_triggers(state);
        state.priority_player = state.active_player.clone();
        state.consecutive_passes = 0;

        if effect_msgs.is_empty() {
            Ok("No combat damage was dealt.".to_string())
        } else {
            Ok(effect_msgs.join(" "))
        }"""

content = content.replace(bad_block, good_block)

with open('rust_core/src/rules.rs', 'w') as f:
    f.write(content)
