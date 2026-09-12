import re

with open("python_agent/tools.py", "r") as f:
    content = f.read()

# Fix _normalize_permanent
old_normalize = '''def _normalize_permanent(item: Any) -> dict:
    """Ensures each battlefield entry matches Rust's expected Permanent schema."""
    if isinstance(item, str):
        return {
            "id": f"perm-{hash(item) % 10000}",
            "name": item,
            "controller": "Opponent",
            "power": 0,
            "toughness": 1,
            "card": {
                "name": item,
                "type_line": ["Creature"],
                "mana_cost": "",
                "oracle_text": ""
            },
            "damage_marked": 0
        }
    
    if isinstance(item, dict):
        if "card" in item and isinstance(item["card"], dict):
            return item
        
        name = item.get("name", "Unknown")
        
        # Catch LLM using "types" instead of "type_line"
        type_line = item.get("type_line") or item.get("types") or ["Creature"]
        if isinstance(type_line, str):
            type_line = [type_line]
            
        return {
            "id": str(item.get("id", f"perm-{hash(name) % 10000}")),
            "name": name,
            "controller": item.get("controller", "Opponent"),
            "damage_marked": int(item.get("damage_marked") or 0),
            "power": int(item.get("power") or 0),
            "toughness": int(item.get("toughness") or 1),
            "card": {
                "name": name,
                "type_line": type_line,
                "mana_cost": item.get("mana_cost", ""),
                "oracle_text": item.get("oracle_text", "")
            }
        }
    return item'''

new_normalize = '''def _normalize_permanent(item: Any) -> dict:
    """Ensures each battlefield entry matches Rust's expected Permanent schema."""
    if isinstance(item, str):
        return {
            "id": f"perm-{hash(item) % 10000}",
            "name": item,
            "controller": "Opponent",
            "damage_marked": 0,
            "power": 0,
            "toughness": 1,
            "types": ["Creature"],
            "oracle_text": "",
            "mana_value": 0,
            "colors": [],
            "is_legendary": False,
            "is_tapped": False
        }
    
    if isinstance(item, dict):
        # We don't return item blindly if it has 'card', we translate it.
        name = item.get("name", "Unknown")
        card = item.get("card", item)
        
        type_line = card.get("type_line") or item.get("types") or ["Creature"]
        if isinstance(type_line, str):
            type_line = [type_line]
            
        return {
            "id": str(item.get("id", f"perm-{hash(name) % 10000}")),
            "name": name,
            "controller": item.get("controller", "Opponent"),
            "damage_marked": int(item.get("damage_marked") or 0),
            "power": int(item.get("power") or 0),
            "toughness": int(item.get("toughness") or 1),
            "types": type_line,
            "oracle_text": card.get("oracle_text", ""),
            "mana_value": int(item.get("mana_value", 0)),
            "colors": item.get("colors", []),
            "is_legendary": "Legendary" in type_line,
            "is_tapped": bool(item.get("is_tapped", False))
        }
    return item'''

content = content.replace(old_normalize, new_normalize)

# Fix cast_spell payload wrapper
old_cast_spell = '''    # 2. Construct the Action Object
    action_payload = {
        "type": "CastSpell",
        "card": {
            "name": card_name,
            "type_line": type_line,
            "mana_cost": mana_cost,
            "oracle_text": oracle_text,
            "effects": parsed_effects
        },
        "targets": parsed_targets
    }'''

new_cast_spell = '''    # 2. Construct the Action Object
    action_payload = {
        "type": "CastSpell",
        "payload": {
            "card": {
                "name": card_name,
                "type_line": type_line,
                "mana_cost": mana_cost,
                "oracle_text": oracle_text,
                "effects": parsed_effects
            },
            "targets": parsed_targets
        }
    }'''

content = content.replace(old_cast_spell, new_cast_spell)

# Fix activate_ability payload wrapper
old_activate = '''    action_payload = {
        "type": "ActivateAbility",
        "source_id": source_id,
        "ability_index": ability_index,
        "targets": parsed_targets
    }'''

new_activate = '''    action_payload = {
        "type": "ActivateAbility",
        "payload": {
            "source_id": source_id,
            "ability_index": ability_index,
            "targets": parsed_targets
        }
    }'''

content = content.replace(old_activate, new_activate)

# Fix declare_attackers payload wrapper
old_attackers = '''    action_payload = {
        "type": "DeclareAttackers",
        "attackers": parsed_attackers
    }'''

new_attackers = '''    action_payload = {
        "type": "DeclareAttackers",
        "payload": {
            "attackers": parsed_attackers
        }
    }'''

content = content.replace(old_attackers, new_attackers)

# Fix declare_blockers payload wrapper
old_blockers = '''    action_payload = {
        "type": "DeclareBlockers",
        "blockers": parsed_blockers
    }'''

new_blockers = '''    action_payload = {
        "type": "DeclareBlockers",
        "payload": {
            "blockers": parsed_blockers
        }
    }'''

content = content.replace(old_blockers, new_blockers)


with open("python_agent/tools.py", "w") as f:
    f.write(content)
