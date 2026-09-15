import re

def update_lib():
    with open('rust_core/src/lib.rs', 'r') as f:
        content = f.read()

    # Remove state.run_sba_loop(); from apply_action, resolve_stack_top, pass_priority_endpoint, resolve_combat_damage_endpoint
    content = re.sub(r'^\s*state\.run_sba_loop\(\);\n', '', content, flags=re.MULTILINE)
    
    # Change Judge::resolve_combat_damage to judge.resolve_combat_damage
    content = content.replace('let message = match Judge::resolve_combat_damage(&mut state) {', 
                              'let judge = Judge::default_engine();\n    let message = match judge.resolve_combat_damage(&mut state) {')

    with open('rust_core/src/lib.rs', 'w') as f:
        f.write(content)

def update_rules():
    with open('rust_core/src/rules.rs', 'r') as f:
        content = f.read()

    # Add enforce_sbas_and_triggers right after default_engine
    if 'pub fn enforce_sbas_and_triggers' not in content:
        insert_idx = content.find('pub fn assess_state')
        if insert_idx != -1:
            snippet = """    /// Enforces CR 117.5: State-Based Actions and Triggered Abilities before priority.
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
    fn put_waiting_triggers_on_stack(&self, _state: &mut GameState) -> bool {
        // TODO: Scan for met trigger conditions and push them to the stack.
        // Return true if at least one trigger was added.
        false
    }

    """
            content = content[:insert_idx] + snippet + content[insert_idx:]

    # apply_action: replace lines 206-208
    content = re.sub(
        r'// 3\. Cleanup\n\s*state\.pending_action = None;\n\s*Ok\("Action successfully applied\."\.to_string\(\)\)',
        r'// 3. Cleanup\n        state.pending_action = None;\n        self.enforce_sbas_and_triggers(state);\n        state.priority_player = state.active_player.clone();\n        state.consecutive_passes = 0;\n        Ok("Action successfully applied.".to_string())',
        content
    )

    # resolve_top: replace sba_msgs and effect_msgs.extend
    content = re.sub(
        r'// CR 117\.5: Enforce SBAs immediately after ANY spell resolves.*?effect_msgs\.extend\(sba_msgs\);',
        r'self.enforce_sbas_and_triggers(state);\n        state.priority_player = state.active_player.clone();\n        state.consecutive_passes = 0;',
        content,
        flags=re.DOTALL
    )

    # resolve_combat_damage: signature and end
    content = content.replace('pub fn resolve_combat_damage(state: &mut GameState) -> Result<String, String>', 'pub fn resolve_combat_damage(&self, state: &mut GameState) -> Result<String, String>')
    content = re.sub(
        r'Ok\("No combat damage was dealt\."\.to_string\(\)\)\n\s*\} else \{\n\s*Ok\(effect_msgs\.join\(" "\)\)\n\s*\}',
        r'self.enforce_sbas_and_triggers(state);\n        state.priority_player = state.active_player.clone();\n        state.consecutive_passes = 0;\n\n        if effect_msgs.is_empty() {\n            Ok("No combat damage was dealt.".to_string())\n        } else {\n            Ok(effect_msgs.join(" "))\n        }',
        content
    )

    # advance_step: replace priority section
    content = re.sub(
        r'state\.priority_player = state\.active_player\.clone\(\); // AP gets priority first\n\s*state\.mana_pool\.clear\(\); // Mana empties as steps/phases end',
        r'state.mana_pool.clear(); // Mana empties as steps/phases end\n        \n        self.enforce_sbas_and_triggers(state);\n        state.priority_player = state.active_player.clone(); // AP gets priority first\n        state.consecutive_passes = 0;',
        content
    )

    # pass_priority: remove extra priority setting
    content = re.sub(
        r'let res = self\.resolve_top\(state\);\n\s*state\.priority_player = state\.active_player\.clone\(\); // AP gets priority after resolution\n\s*return res;',
        r'return self.resolve_top(state);',
        content
    )

    with open('rust_core/src/rules.rs', 'w') as f:
        f.write(content)

update_lib()
update_rules()
