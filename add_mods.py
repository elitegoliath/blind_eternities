import re
with open('rust_core/src/lib.rs', 'r') as f:
    text = f.read()

text = text.replace('mod models;\nmod rules;', 'pub mod events;\npub mod triggers;\nmod models;\nmod rules;')
with open('rust_core/src/lib.rs', 'w') as f:
    f.write(text)
