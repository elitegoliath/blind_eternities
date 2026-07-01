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
def validate_move(card_name: str, board_state: Union[str, List[Dict[str, Any]]]) -> dict:
    """
    Validates a Magic: The Gathering move by checking the board state against the Comprehensive Rules.
    
    Args:
        card_name: The name of the card being played or activated (e.g. "Urza, Lord High Artificer").
        board_state: A list of JSON objects representing the cards currently on the battlefield. 
                     Example: [{"name": "Urza...", "is_legendary": true, "controller": "me"}]
    """
    
    print(f"\n[DEBUG] 🛠️  The Agent is calling Rust for: {card_name}")
    

    # 1. Defensively clean up board_state if the LLM passed it as a string
    if isinstance(board_state, str):
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
                    "message": "board_state could not be parsed into a valid list of cards."
                }

    # Double check we have an actual list now
    if not isinstance(board_state, list):
        return {
            "status": "error",
            "message": "board_state validation failed: Input must resolve to a list."
        }

    # 2. Construct the payload for Rust
    # We wrap the separate arguments into the single JSON structure the Rust parser expects
    payload = json.dumps({
        "card_name": card_name,
        "battlefield": board_state
    })
    
    # 3. Call the Rust "Judge"
    try:
        # returns string like "Legal" or "StateBasedAction: Legend Rule"
        ruling = mtg_logic_core.check_board_state(payload) 
        return {"status": "success", "ruling": ruling}
    except Exception as e:
        return {"status": "error", "message": str(e)}

# Note: We don't define search_rules yet, but can be added here later with @tool