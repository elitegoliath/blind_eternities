# python_agent/tools.py
# This file is the "Translation Layer."
# It takes a fuzzy request from the LLM and turns it into a
# strict function call to the compiled Rust binary.

import lancedb
from fastembed import TextEmbedding
import json
import ast
from typing import Union, List, Dict, Any
from langchain_core.tools import tool
import mtg_logic_core  # <--- This is the compiled Rust code!

# Initialize the models outside the function so they stay hot in memory
print("[DEBUG] 📚 Loading FastEmbed Model and LanceDB...")
embed_model = TextEmbedding(model_name="sentence-transformers/all-MiniLM-L6-v2")
db = lancedb.connect("/app/data/lancedb") # Maps to the Docker volume mount

@tool
def fetch_card(card_name: str) -> dict:
    """
    Fetches the exact Oracle text, type line, and mana cost of a Magic: The Gathering card.
    ALWAYS use this tool before casting a spell to ensure you have the correct data.
    
    Args:
        card_name: The name of the card to look up (e.g., "Grizzly Bears").
    """
    try:
        print(f"\n[DEBUG] 🔍 Semantic Search triggered for: '{card_name}'")
        table = db.open_table("cards")
        
        # Embed the query
        query_vector = list(embed_model.embed([card_name]))[0]
        
        # Perform the Vector Search
        results = table.search(query_vector).limit(1).to_list()
        
        if results:
            card = results[0]
            # Strip out the massive vector array to save LLM context window limits
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
    type_line: Union[str, List[str]] = None,
    mana_cost: str = "",
    board_state: Union[str, List[Dict[str, Any]]] = None,
    mana_pool: dict = None,
    stack: Union[str, List[str]] = None,
    lands_played: int = 0
) -> dict:
    """
    Validates a Magic: The Gathering move by checking the board state against the Comprehensive Rules.
    
    Args:
        card_name: The name of the card being played or activated.
        action_type: The type of action being taken. Must be one of: "CastSpell", "PlayLand", "ActivateAbility".
        type_line: A list of card types and supertypes. Examples: ["Creature"], ["Legendary", "Creature"], ["Basic", "Land"]. Valid base types include Artifact, Creature, Enchantment, Instant, Land, Planeswalker, Sorcery.
        mana_cost: The mana cost of the card. MUST be exactly "" (empty string) for Lands.
        board_state: board_state: A list of JSON objects representing cards currently on the battlefield. Each object MUST include a "name", "type_line", and a unique "id" (e.g., "bear-1").
        mana_pool: A dictionary of available mana. ONLY include the exact mana explicitly provided by the user.
        stack: A list of JSON objects representing spells on the stack. Each object MUST include "id", "card" (with name and type_line), and "targets". 
        lands_played: The number of lands the player has already played this turn. Default is 0.
        targets: A list of JSON target objects for the pending action. Examples:
                [{"type": "Permanent", "id": "bear-1"}]
                [{"type": "Player", "id": "Opponent"}]
                [{"type": "StackObject", "id": "spell-1"}]
    """
    
    print(f"\n[DEBUG] 🛠️  The Agent is checking move: {card_name} (Cost: {mana_cost})")
    
    # Clean up optional parameters
    if board_state is None:
        board_state = []
    if mana_pool is None:
        mana_pool = {}
    if stack is None:
        stack = []
    if type_line is None:
        type_line = ["Unknown"]

    # Defensively clean up type_line if passed as a string
    if isinstance(type_line, str):
        try:
            parsed = json.loads(type_line.replace("'", '"'))
            type_line = parsed if isinstance(parsed, list) else [type_line]
        except json.JSONDecodeError:
            type_line = [type_line]

    # Defensively clean up board_state if the LLM passed it as a string
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

    # Double check that board state is a list
    if not isinstance(board_state, list):
        return {
            "status": "error",
            "message": "board_state must resolve to a list."
        }

    # Clean up stack strings
    if isinstance(stack, str):
        if not stack.strip():
            stack = []
        else:
            try:
                stack = json.loads(stack)
            except json.JSONDecodeError:
                try:
                    stack = ast.literal_eval(stack)
                except (ValueError, SyntaxError):
                    stack = [stack] # Treat raw string as a single spell name

    # Double check that board state is a list
    if not isinstance(stack, list):
        return {"status": "error", "message": "stack must resolve to a list."}    

    # Construct the STRICT GameState payload for Rust
    # Build the exact structural scaffolding that models.rs expects
    game_state_payload = {
        "active_player": "Player",
        "is_active_player": True,
        "phase": "Main Phase 1", # Must match the #[serde(rename)] in Rust exactly
        "battlefield": board_state,
        "stack": stack,
        "lands_played": lands_played,
        "mana_pool": mana_pool,
        "pending_action": {
            "type": action_type,
            "payload": {
                "name": card_name,
                "type_line": type_line,
                "mana_cost": mana_cost
            }
        }
    }
    
    # Call the Rust "Judge"
    try:
        # returns string like "Legal" or "StateBasedAction: Legend Rule"
        # Rust returns a serialized JSON string, e.g., '[{"status":"Legal"}]'
        ruling_raw_string = mtg_logic_core.check_board_state(json.dumps(game_state_payload)) 
        
        # Unpack the inner JSON string into a native Python list/dict
        # to eliminate the escaped backslashes seen in the AI response in the CLI.
        
        try:
            ruling_parsed = json.loads(ruling_raw_string)
        except json.JSONDecodeError:
            # Fallback just in case Rust ever returns a plain un-serialized string
            ruling_parsed = ruling_raw_string

        return {
            "status": "success",
            "ruling": ruling_parsed
        }
    except Exception as e:
        return {"status": "error", "message": str(e)}

@tool
def resolve_stack(
    board_state: Union[str, List[Dict[str, Any]]] = None, 
    stack: Union[str, List[Dict[str, Any]]] = None
) -> dict:
    """
    Resolves the top spell or ability on the stack (LIFO order).
    Call this when all players pass priority and the stack is not empty.
    
    IMPORTANT: Format your JSON exactly like this example:
    
    board_state example:
    [
      {
        "id": "bear-1", 
        "name": "Grizzly Bears", 
        "controller": "Opponent", 
        "type_line": ["Creature"], 
        "power": 2, 
        "toughness": 2, 
        "damage_marked": 0, 
        "oracle_text": ""
      }
    ]
    
    stack example:
    [
      {
        "id": "bolt-1", 
        "controller": "Player", 
        "targets": [{"type": "Permanent", "id": "bear-1"}], 
        "card": {
          "name": "Lightning Bolt", 
          "type_line": ["Instant"], 
          "effects": [{"type": "DealDamage", "amount": 3}]
        }
      }
    ]
    """
    
    print("\n[DEBUG] 🛠️  The Agent is resolving the top of the stack.")
    
    # 1. Initialize defaults
    if board_state is None:
        board_state = []
    if stack is None:
        stack = []

    # 2. Defensively clean up board_state
    if isinstance(board_state, str):
        if not board_state.strip():
            board_state = []
        else:
            try:
                board_state = json.loads(board_state)
            except json.JSONDecodeError:
                try:
                    board_state = ast.literal_eval(board_state)
                except (ValueError, SyntaxError):
                    return {"status": "error", "message": "board_state must be a valid JSON array."}

    # 3. Defensively clean up stack
    if isinstance(stack, str):
        if not stack.strip():
            stack = []
        else:
            try:
                stack = json.loads(stack)
            except json.JSONDecodeError:
                try:
                    stack = ast.literal_eval(stack)
                except (ValueError, SyntaxError):
                    return {"status": "error", "message": "stack must be a valid JSON array."}

    if not isinstance(board_state, list):
        return {"status": "error", "message": "board_state must resolve to a list."}
    if not isinstance(stack, list):
        return {"status": "error", "message": "stack must resolve to a list."}
    
    # 4. Construct the GameState payload
    game_state_payload = {
        "active_player": "Player",
        "is_active_player": True,
        "phase": "Main Phase 1", 
        "battlefield": board_state,
        "stack": stack, 
        "lands_played": 0,
        "mana_pool": {},
        "pending_action": None 
    }
    
    # 5. Call the Rust Engine
    try:
        ruling_raw_str = mtg_logic_core.resolve_stack_top(json.dumps(game_state_payload)) 
        
        try:
            ruling_parsed = json.loads(ruling_raw_str)
        except json.JSONDecodeError:
            ruling_parsed = ruling_raw_str
            
        return ruling_parsed
    except Exception as e:
        return {"status": "error", "message": str(e)}

# Note: We don't define search_rules yet, but can be added here later with @tool