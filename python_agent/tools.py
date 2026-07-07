# python_agent/tools.py
# This file is the "Translation Layer."
# It takes a fuzzy request from the LLM and turns it into a
# strict function call to the compiled Rust binary.

import json
import ast
from typing import Union, List, Dict, Any
from langchain_core.tools import tool
import mtg_logic_core  # <--- This is the compiled Rust code!

@tool
def validate_move(
    card_name: str,
    mana_cost: str = "",
    board_state: Union[str, List[Dict[str, Any]]] = None,
    mana_pool: dict = None
) -> dict:
    """
    Validates a Magic: The Gathering move by checking the board state against the Comprehensive Rules.
    
    Args:
        card_name: The name of the card being played.
        mana_cost: The mana cost of the card, e.g., "{1}{U}{U}".
        board_state: A list of JSON objects representing the cards currently on the battlefield. 
        mana_pool: A dictionary of available mana. Example: {"blue": 3, "red": 0, "colorless": 1}
    """
    
    print(f"\n[DEBUG] 🛠️  The Agent is checking move: {card_name} (Cost: {mana_cost})")
    
    # 1. Clean up optional parameters
    if board_state is None:
        board_state = []
    if mana_pool is None:
        mana_pool = {}

    # 2. Defensively clean up board_state if the LLM passed it as a string
    if isinstance(board_state, str):
        # Handle empty string cases
        if not board_state.strip():
            board_state = []
        else:
            try:
                # json.loads handles proper JSON strings
                board_state = json.loads(board_state)
            except json.JSONDecodeError:
                try:
                    # ast.literal_eval handles strings using Python-style single quotes
                    board_state - ast.literal_eval(board_state)
                except (ValueError, SyntaxError):
                    return {
                        "status": "error",
                        "message": "board_state must be a valid JSON array."
                    }

    # Double check we have an actual list now
    if not isinstance(board_state, list):
        return {
            "status": "error",
            "message": "board_state must resolve to a list."
        }

    # 2. Construct the STRICT GameState payload for Rust
    # We build the exact structural scaffolding that models.rs expects
    game_state_payload = {
        "active_player": "Player",
        "is_active_player": True,
        "phase": "Main Phase 1", # Must match the #[serde(rename)] in Rust exactly
        "battlefield": board_state,
        "stack": [],
        "lands_played": 0,
        "mana_pool": mana_pool,
        "pending_action": {
            "type": "CastSpell",
            "payload": {
                "name": card_name,
                "type_line": ["Unknown"], # Safe fallback for the engine
                "mana_cost": mana_cost
            }
        }
    }
    
    # 3. Call the Rust "Judge"
    try:
        # returns string like "Legal" or "StateBasedAction: Legend Rule"
        ruling = mtg_logic_core.check_board_state(json.dumps(game_state_payload)) 
        return {"status": "success", "ruling": ruling}
    except Exception as e:
        return {"status": "error", "message": str(e)}

# Note: We don't define search_rules yet, but can be added here later with @tool