import re

with open('rust_core/src/rules.rs', 'r') as f:
    content = f.read()

# Replace early returns in Destroy
destroy_orig = """                                if state.battlefield.len() < original_len {
                                    return Ok(format!(
                                        "{} resolved. Destroyed {}.",
                                        top.card.name, target_name
                                    ));
                                }"""
destroy_new = """                                if state.battlefield.len() < original_len {
                                    effect_msgs.push(format!("Destroyed {}.", target_name));
                                }"""
content = content.replace(destroy_orig, destroy_new)

# Replace early return in DrawCards
draw_orig = """                        return Ok(format!(
                            "{} resolved. Player draws {} card(s).",
                            top.card.name, amount
                        ));"""
draw_new = """                        effect_msgs.push(format!("Player draws {} card(s).", amount));"""
content = content.replace(draw_orig, draw_new)

# Replace early returns in Counter
counter_orig = """                                if state.stack.len() < original_len {
                                    return Ok(format!(
                                        "{} resolved. Countered {}.",
                                        top.card.name, countered_name
                                    ));
                                } else {
                                    return Ok(format!("{} resolved, but its target was no longer on the stack (Fizzled).", top.card.name));
                                }"""
counter_new = """                                if state.stack.len() < original_len {
                                    effect_msgs.push(format!("Countered {}.", countered_name));
                                } else {
                                    effect_msgs.push(format!("Its target was no longer on the stack (Fizzled)."));
                                }"""
content = content.replace(counter_orig, counter_new)

with open('rust_core/src/rules.rs', 'w') as f:
    f.write(content)
