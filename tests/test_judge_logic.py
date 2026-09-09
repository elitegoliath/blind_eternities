import mtg_logic_core
import json

def make_state(phase="Main Phase 1", is_active=True, stack=None, action=None, mana=None):
    if mana is None:
        mana = {} # Defaults to 0 in Rust
        
    return json.dumps({
        "active_player": "Hero",
        "is_active_player": is_active,
        "phase": phase,
        "battlefield": [],
        "stack": stack if stack else [],
        "lands_played": 0,
        "mana_pool": mana,
        "pending_action": action
    })

def run_test(name, state_json, expected_status):
    print(f"Testing: {name:<40}", end=" ")
    response = mtg_logic_core.check_board_state(state_json)
    result = json.loads(response)[0]
    
    if result["status"] == expected_status:
        print("✅ PASS")
    else:
        print(f"❌ FAIL (Expected {expected_status}, got {result})")

print("--- ⚖️  JUDGE MANA TESTS ⚖️  ---\n")

# 1. Cast Sol Ring (Legal)
# Cost: {1}, Have: {R}:1
sol_ring = {
    "type": "CastSpell",
    "payload": { "name": "Sol Ring", "type_line": ["Artifact"], "mana_cost": "{1}" }
}
run_test("Cast Sol Ring (Have {R})", 
    make_state(mana={"red": 1}, action=sol_ring), 
    "legal"
)

# 2. Cast Counterspell (Legal)
# Cost: {U}{U}, Have: {U}:2
counterspell = {
    "type": "CastSpell",
    "payload": { "name": "Counterspell", "type_line": ["Instant"], "mana_cost": "{U}{U}" }
}
# Note: Since it's an Instant, we can test it during "Combat" too
run_test("Cast Counterspell (Exact Mana)", 
    make_state(phase="Combat", mana={"blue": 2}, action=counterspell), 
    "legal"
)

# 3. Cast Counterspell (Illegal - Wrong Color)
# Cost: {U}{U}, Have: {R}:2
run_test("Cast Counterspell (Wrong Color)", 
    make_state(mana={"red": 2}, action=counterspell), 
    "illegal"
)

# 4. Cast Counterspell (Illegal - Not Enough)
# Cost: {U}{U}, Have: {U}:1
run_test("Cast Counterspell (Insufficient)", 
    make_state(mana={"blue": 1}, action=counterspell), 
    "illegal"
)