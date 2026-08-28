# python_agent/tools.py
# This file is the "Translation Layer."
# It takes a fuzzy request from the LLM and turns it into a
# strict function call to the compiled Rust binary.

import lancedb
from fastembed import TextEmbedding
import json
import ast
from typing import Any, Optional
from langchain_core.tools import tool
import mtg_logic_core  # type: ignore # <--- This is the compiled Rust code!

# Initialize the models outside the function so they stay hot in memory
print("[DEBUG] 📚 Loading FastEmbed Model and LanceDB...")
embed_model = TextEmbedding(model_name="sentence-transformers/all-MiniLM-L6-v2")
db = lancedb.connect("/app/data/lancedb")  # Maps to the Docker volume mount


def _lookup_card_direct(card_name: str) -> Optional[dict]:
    """Helper to query LanceDB directly for fallback resolution."""
    try:
        table = db.open_table("cards")
        query_vector = list(embed_model.embed([card_name]))[0]
        results = table.search(query_vector).limit(1).to_list()
        if results:
            card = results[0]
            # Ensure type_line is a list
            raw_type = card.get("type_line", "")
            type_list = raw_type.split() if isinstance(raw_type, str) else raw_type
            return {
                "name": card.get("name", card_name),
                "type_line": type_list,
                "mana_cost": card.get("mana_cost", ""),
                "oracle_text": card.get("oracle_text", "")
            }
    except Exception as e:
        print(f"[DEBUG] ⚠️ Fallback card lookup failed: {e}")
    return None


def _normalize_permanent(item: Any) -> dict:
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
    return item


def _normalize_stack_item(item: Any) -> dict:
    """Ensures each stack entry matches Rust's expected StackObject schema."""
    if isinstance(item, str):
        return {
            "id": f"spell-{hash(item) % 10000}",
            "controller": "Player",
            "targets": [],
            "card": {
                "name": item,
                "type_line": ["Instant"],
                "effects": []
            }
        }
    
    if isinstance(item, dict):
        if "card" in item and isinstance(item["card"], dict):
            return item
        
        name = item.get("name", "Unknown")
        type_line = item.get("type_line", ["Instant"])
        if isinstance(type_line, str):
            type_line = [type_line]
            
        return {
            "id": str(item.get("id", f"spell-{hash(name) % 10000}")),
            "controller": item.get("controller", "Player"),
            "targets": item.get("targets", []),
            "card": {
                "name": name,
                "type_line": type_line,
                "effects": item.get("effects", [])
            }
        }
    return item


def _normalize_mana_pool(raw_pool: Any) -> dict:
    """Translates loose LLM mana dictionaries into strict Rust struct formatting."""
    # Baseline pool using exact Rust field names
    normalized = {
        "white": 0, 
        "blue": 0, 
        "black": 0, 
        "red": 0, 
        "green": 0, 
        "colorless": 0
    }
    
    # Catch stringified dicts
    if isinstance(raw_pool, str):
        try:
            raw_pool = json.loads(raw_pool.replace("'", '"'))
        except (ValueError, SyntaxError):
            pass

    if not isinstance(raw_pool, dict):
        return normalized
        
    # Map EVERYTHING to the full Rust struct field names
    color_map = {
        "w": "white", "white": "white", 
        "u": "blue",  "blue": "blue", 
        "b": "black", "black": "black", 
        "r": "red",   "red": "red", 
        "g": "green", "green": "green", 
        "c": "colorless", "colorless": "colorless"
    }
    
    for key, value in raw_pool.items():
        standard_key = color_map.get(str(key).lower().strip())
        if standard_key in normalized:
            try:
                # Use += to aggregate if LLM splits mana weirdly
                normalized[standard_key] += int(value) 
            except (ValueError, TypeError):
                continue 
                
    return normalized


def _clean_json_param(param: Any) -> list:
    """Helper to safely parse stringified lists/JSON from LLMs."""
    if param is None:
        return []
    if isinstance(param, list):
        return param
    if isinstance(param, str):
        trimmed = param.strip()
        if not trimmed:
            return []
        try:
            parsed = json.loads(trimmed)
            return parsed if isinstance(parsed, list) else [parsed]
        except json.JSONDecodeError:
            try:
                parsed = ast.literal_eval(trimmed)
                return parsed if isinstance(parsed, list) else [parsed]
            except (ValueError, SyntaxError):
                return [trimmed]
    return []


@tool
def fetch_card(card_name: str) -> dict:
    """
    Fetches the exact Oracle text, type line, and mana cost of a Magic: The Gathering card.
    ALWAYS use this tool before casting a spell to ensure you have the correct data.
    
    Args:
        card_name: The name of the card to look up (e.g., "Grizzly Bears", "Lightning Bolt").
    """
    try:
        print(f"\n[DEBUG] 🔍 Semantic Search triggered for: '{card_name}'")
        table = db.open_table("cards")
        
        query_vector = list(embed_model.embed([card_name]))[0]
        results = table.search(query_vector).limit(1).to_list()
        
        if results:
            card = results[0]
            return {
                "name": card["name"],
                "type_line": card["type_line"],
                "mana_cost": card["mana_cost"],
                "oracle_text": card["oracle_text"]
            }
        
        return {"status": "error", "message": f"Could not find card matching '{card_name}'"}
    
    except Exception as e:
        return {"status": "error", "message": f"Database search failed: {str(e)}"}


@tool
def validate_move(
    card_name: str,
    action_type: str = "CastSpell",
    type_line: str = "[]",
    mana_cost: str = "",
    board_state: str = "[]",
    mana_pool: str = "{}",
    stack: str = "[]",
    lands_played: int = 0,
    targets: str = "[]"
) -> dict:
    """
    Validates a Magic: The Gathering move by checking the board state against the Comprehensive Rules.
    
    Args:
        card_name: The name of the card being played.
        action_type: The type of action ("CastSpell", "PlayLand", "ActivateAbility").
        type_line: A stringified list of card types (e.g., '["Instant"]', '["Creature"]').
        mana_cost: Mana cost string (e.g. "{R}"). Empty string for lands.
        
        # THE FIX: Explicitly require the ID mapping in the docstring
        board_state: A stringified list of JSON objects representing permanents. MUST include 'id' and 'name'. Example: '[{"id": "bear-1", "name": "Grizzly Bears"}]'. The 'id' MUST match the 'id' used in targets.
        
        mana_pool: A stringified dict of available mana. Example: '{"U": 4}' or '{"red": 1}'.
        stack: A stringified list of spells currently waiting to resolve.
        lands_played: Number of lands played this turn (default 0).
        targets: A stringified list of target objects. Example: '[{"type": "Permanent", "id": "bear-1"}]'.
    """
    print(f"\n[DEBUG] 🛠️ The Agent is checking move: {card_name} (Cost: {mana_cost})")
    
    # Run the string inputs through our robust JSON cleaners
    parsed_type_line = _clean_json_param(type_line)
    if not parsed_type_line:
        parsed_type_line = ["Unknown"]
        
    # Auto-fill missing card data if LLM skipped fetch_card
    if not mana_cost or parsed_type_line == ["Unknown"]:
        cached = _lookup_card_direct(card_name)
        if cached:
            if not mana_cost:
                mana_cost = cached["mana_cost"]
            if parsed_type_line == ["Unknown"]:
                parsed_type_line = cached["type_line"]

    clean_mana_pool = _normalize_mana_pool(mana_pool)

    raw_board = _clean_json_param(board_state)
    normalized_board = [_normalize_permanent(item) for item in raw_board]

    raw_stack = _clean_json_param(stack)
    normalized_stack = [_normalize_stack_item(item) for item in raw_stack]

    parsed_targets = _clean_json_param(targets)
    if isinstance(parsed_targets, list):
        for t in parsed_targets:
            if isinstance(t, dict) and "type" not in t:
                t["type"] = "Permanent"

    game_state_payload = {
        "active_player": "Player",
        "is_active_player": True,
        "phase": "Main Phase 1",
        "battlefield": normalized_board,
        "stack": normalized_stack,
        "lands_played": lands_played,
        "mana_pool": clean_mana_pool,
        "pending_action": {
            "type": action_type,
            "payload": {
                "card": {
                    "name": card_name,
                    "type_line": parsed_type_line,
                    "mana_cost": mana_cost,
                    "oracle_text": "" 
                },
                "targets": parsed_targets
            }
        }
    }
    
    try:
        ruling_raw_string = mtg_logic_core.check_board_state(json.dumps(game_state_payload))
        try:
            ruling_parsed = json.loads(ruling_raw_string)
        except json.JSONDecodeError:
            ruling_parsed = ruling_raw_string

        return {
            "status": "success",
            "ruling": ruling_parsed
        }
    except Exception as e:
        return {"status": "error", "message": str(e)}


@tool
def resolve_stack(
    card_name: str,
    board_state: str = "[]", 
    targets: str = "[]",
    # --- DUMMY PARAMS TO ABSORB LLM HALLUCINATIONS ---
    action_type: str = "",
    mana_cost: str = "",
    mana_pool: str = "",
    stack: str = ""
) -> dict:
    """
    Resolves the top spell or ability on the stack and applies State-Based Actions.
    
    Args:
        card_name: The name of the spell resolving (e.g., "Lightning Bolt").
        board_state: A stringified list of permanents on the battlefield.
        targets: A stringified list of target objects for the spell.
        action_type: (Optional) Ignored by engine.
        mana_cost: (Optional) Ignored by engine.
        mana_pool: (Optional) Ignored by engine.
        stack: (Optional) Ignored by engine.
    """
    print("\n[DEBUG] 🛠️ The Agent is resolving the top of the stack.")

    # Normalize data passed in.
    raw_board = _clean_json_param(board_state)
    normalized_board = [_normalize_permanent(item) for item in raw_board]

    parsed_targets = _clean_json_param(targets)
    if isinstance(parsed_targets, list):
        for t in parsed_targets:
            if isinstance(t, dict) and "type" not in t:
                t["type"] = "Permanent"

    # Fetch cached type_line if available
    cached = _lookup_card_direct(card_name)
    type_line = cached["type_line"] if cached else ["Instant"]

    constructed_stack = [{
        "id": f"spell-{hash(card_name) % 10000}",
        "controller": "Player",
        "targets": parsed_targets,
        "card": {
            "name": card_name,
            "type_line": type_line,
            "effects": [{"type": "DealDamage", "amount": 3}] 
        }
    }]

    game_state_payload = {
        "active_player": "Player",
        "is_active_player": True,
        "phase": "Main Phase 1",
        "battlefield": normalized_board,
        "stack": constructed_stack,
        "lands_played": 0,
        "mana_pool": {"w": 0, "u": 0, "b": 0, "r": 0, "g": 0, "c": 0},
        "pending_action": None 
    }
    
    try:
        ruling_raw_str = mtg_logic_core.resolve_stack_top(json.dumps(game_state_payload))
        try:
            ruling_parsed = json.loads(ruling_raw_str)
        except json.JSONDecodeError:
            ruling_parsed = ruling_raw_str
            
        return {
            "status": "success",
            "ruling": ruling_parsed
        }
    except Exception as e:
        return {"status": "error", "message": str(e)}


@tool
def play_card(
    card_name: str,
    action_type: str = "CastSpell",
    type_line: str = "[]",
    mana_cost: str = "",
    board_state: str = "[]",
    mana_pool: str = "{}",
    stack: str = "[]",
    lands_played: int = 0,
    targets: str = "[]"
) -> dict:
    """
    Validates AND resolves a Magic: The Gathering move in one step.
    Use this instead of calling validate_move and resolve_stack separately.
    
    Args:
        card_name: The name of the card being played.
        action_type: The type of action ("CastSpell", "PlayLand", "ActivateAbility").
        type_line: A stringified list of card types (e.g., '["Instant"]').
        mana_cost: Mana cost string (e.g. "{R}"). Empty string for lands.
        board_state: Stringified JSON list of permanents. MUST include 'id' and 'name'.
        mana_pool: Stringified dict of available mana. Example: '{"red": 1}'.
        stack: Stringified list of spells on the stack.
        lands_played: Number of lands played this turn (default 0).
        targets: Stringified list of target objects matching the board_state IDs.
    """
    print(f"\n[DEBUG] 🛠️ Macro Tool executing for: {card_name}")
    
    # Step 1: Validate
    validation_result = validate_move.invoke({
        "card_name": card_name, "action_type": action_type, "type_line": type_line,
        "mana_cost": mana_cost, "board_state": board_state, "mana_pool": mana_pool,
        "stack": stack, "lands_played": lands_played, "targets": targets
    })
    
    # If illegal, stop immediately and return the ruling to the LLM
    if validation_result.get("status") == "error":
        return validation_result
        
    rulings = validation_result.get("ruling", [])
    if any(isinstance(r, dict) and r.get("status") == "illegal" for r in rulings):
        return {"status": "illegal", "details": rulings}
        
    # Step 2: Resolve (since it is legal)
    resolve_result = resolve_stack.invoke({
        "card_name": card_name,
        "board_state": board_state,
        "targets": targets
    })
    
    return {
        "status": "success",
        "validation": "Legal",
        "resolution": resolve_result.get("ruling", "Unknown resolution")
    }