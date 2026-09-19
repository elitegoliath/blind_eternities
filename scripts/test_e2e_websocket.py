import asyncio
import websockets
import json

async def test_websocket():
    uri = "ws://localhost:8000/ws/test_session/player_1"
    
    state_file = "session_test_session.json"
    pass
    with open(state_file, "w") as f:
        json.dump({
            "active_player": "player_1",
            "priority_player": "player_1",
            "turn_number": 1,
            "phase": "Main Phase 1",
            "stack": [],
            "battlefield": [],
            "graveyard": [],
            "exile": [],
            "hand": {"player_1": [], "player_2": [{"name": "Hidden Card", "type_line": [], "oracle_text": "", "effects": [], "mana_cost": ""}]},
            "life_totals": {"player_1": 20, "player_2": 20},
            "mana_pool": {},
            "continuous_effects": [],
            "pending_triggers": [],
            "lands_played": 0,
            "consecutive_passes": 0,
            "rules_config": {
                "legend_rule_enabled": True,
                "legend_max_allowed": 1,
                "legend_scope": "controller",
                "max_lands_per_turn": 1
            }
        }, f)
    
    async with websockets.connect(uri) as websocket:
        response = await websocket.recv()
        data = json.loads(response)
        
        print("Received initial data:", data)
        assert data["type"] == "game_state"
        assert "state" in data
        assert data["state"]["active_player"] == "player_1"
        print("✅ Received initial sanitized game state!")
        
        action_payload = {
            "type": "PassPriority"
        }
        await websocket.send(json.dumps(action_payload))
        
        response = await websocket.recv()
        data = json.loads(response)
        print("Received updated data:", data)
        
        assert data["type"] == "game_state"
        assert "state" in data
        print(f"✅ Received updated game state: priority_player = {data['state']['priority_player']}")

if __name__ == "__main__":
    asyncio.run(test_websocket())
