# python_agent/tools.py
# This file is the "Translation Layer."
# It takes a fuzzy request from the LLM and turns it into a
# strict function call to the compiled Rust binary.

import json
from langchain_core.tools import tool
import mtg_logic_core  # <--- This is the compiled Rust code!

@tool
def validate_move(card_name: str, board_state: list) -> dict:
    """
    Validates a Magic: The Gathering move by checking the board state against the Comprehensive Rules.
    
    Args:
        card_name: The name of the card being played or activated (e.g. "Urza, Lord High Artificer").
        board_state: A list of JSON objects representing the cards currently on the battlefield. 
                     Example: [{"name": "Urza...", "is_legendary": true, "controller": "me"}]
    """
    
    print(f"\n[DEBUG] 🛠️  The Agent is calling Rust for: {card_name}")
    
    # 1. Construct the payload for Rust
    # We wrap the separate arguments into the single JSON structure the Rust parser expects
    payload = json.dumps({
        "card_name": card_name,
        "battlefield": board_state
    })
    
    # 2. Call the Rust "Judge"
    try:
        # returns string like "Legal" or "StateBasedAction: Legend Rule"
        ruling = mtg_logic_core.check_board_state(payload) 
        return {"status": "success", "ruling": ruling}
    except Exception as e:
        return {"status": "error", "message": str(e)}

# Note: We don't define search_rules yet, but you can add it here later with @tool