import mtg_logic_core
import json

# Helper to build the state easily
def make_state(phase="Main Phase 1", is_active=True, stack=None, action=None):
    return json.dumps({
        "active_player": "Hero",
        "is_active_player": is_active,
        "phase": phase,
        "battlefield": [],
        "stack": stack if stack else [],
        "lands_played": 0,
        "pending_action": action
    })

def run_test(name, state_json, expected_status):
    print(f"Testing: {name}...", end=" ")
    response = mtg_logic_core.check_board_state(state_json)
    result = json.loads(response)[0]
    
    if result["status"] == expected_status:
        print("✅ PASS")
    else:
        print(f"❌ FAIL (Expected {expected_status}, got {result})")

print("--- ⚖️  JUDGE TIMING TESTS ⚖️  ---\n")

# 1. Legal Land Drop
# Context: Main Phase, Empty Stack, My Turn
land_action = {
    "type": "PlayLand", 
    "payload": { "name": "Mountain", "type_line": ["Land"], "mana_cost": None }
}
run_test(
    "Legal Land Drop", 
    make_state(phase="Main Phase 1", stack=[], action=land_action), 
    "legal"
)

# 2. Illegal Land Drop (Stack not empty)
# Context: Stack has a spell on it
run_test(
    "Illegal Land Drop (Stack Dirty)", 
    make_state(phase="Main Phase 1", stack=["Lightning Bolt"], action=land_action), 
    "illegal"
)

# 3. Illegal Sorcery (Opponent's Turn)
# Context: Trying to cast a creature during opponent's turn
sorcery_action = {
    "type": "CastSpell",
    "payload": { "name": "Grizzly Bears", "type_line": ["Creature"], "mana_cost": "{1}{G}" }
}
run_test(
    "Illegal Sorcery (Opponent's Turn)",
    make_state(is_active=False, action=sorcery_action), # is_active=False
    "illegal"
)

# 4. Legal Instant (Opponent's Turn)
# Context: Casting an Instant during opponent's turn
instant_action = {
    "type": "CastSpell",
    "payload": { "name": "Giant Growth", "type_line": ["Instant"], "mana_cost": "{G}" }
}
run_test(
    "Legal Instant (Opponent's Turn)",
    make_state(is_active=False, action=instant_action),
    "legal"
)