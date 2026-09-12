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
        "pending_action": action,
        "consecutive_passes": 0
    })

def run_test(state_json, expected_status):
    response = mtg_logic_core.check_board_state(state_json)
    result = json.loads(response)[0]
    assert result["status"] == expected_status

def test_cast_sol_ring_have_r():
    sol_ring = {
        "type": "CastSpell",
        "payload": { "card": { "name": "Sol Ring", "type_line": ["Artifact"], "mana_cost": "{1}", "oracle_text": "", "effects": [] } }
    }
    run_test(make_state(mana={"red": 1}, action=sol_ring), "legal")

def test_cast_counterspell_exact_mana():
    counterspell = {
        "type": "CastSpell",
        "payload": { "card": { "name": "Counterspell", "type_line": ["Instant"], "mana_cost": "{U}{U}", "oracle_text": "", "effects": [] } }
    }
    run_test(make_state(phase="Combat", mana={"blue": 2}, action=counterspell), "legal")

def test_cast_counterspell_wrong_color():
    counterspell = {
        "type": "CastSpell",
        "payload": { "card": { "name": "Counterspell", "type_line": ["Instant"], "mana_cost": "{U}{U}", "oracle_text": "", "effects": [] } }
    }
    run_test(make_state(mana={"red": 2}, action=counterspell), "illegal")

def test_cast_counterspell_insufficient_mana():
    counterspell = {
        "type": "CastSpell",
        "payload": { "card": { "name": "Counterspell", "type_line": ["Instant"], "mana_cost": "{U}{U}", "oracle_text": "", "effects": [] } }
    }
    run_test(make_state(mana={"blue": 1}, action=counterspell), "illegal")

if __name__ == "__main__":
    import pytest
    pytest.main([__file__])
