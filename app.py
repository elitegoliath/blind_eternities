import json
import mtg_logic_core
from fastapi import FastAPI, WebSocket, WebSocketDisconnect
from python_agent.main import BlindEternitiesAgent
from python_agent.state_store import LocalJSONStateStore
from python_agent.interceptors import QwenToolInterceptor
from python_agent.router import classify_query, Intent
from python_agent.pipelines import handle_definition, handle_interaction
from python_agent.translator import translate_to_action

app = FastAPI(title="Blind Eternities Engine API")

agent = BlindEternitiesAgent(
    state_store=LocalJSONStateStore(),
    interceptor=QwenToolInterceptor()
)

class ConnectionManager:
    def __init__(self):
        self.active_connections: dict[str, dict[str, WebSocket]] = {}

    async def connect(self, websocket: WebSocket, session_id: str, player_id: str):
        await websocket.accept()
        if session_id not in self.active_connections:
            self.active_connections[session_id] = {}
        self.active_connections[session_id][player_id] = websocket

    def disconnect(self, session_id: str, player_id: str):
        if session_id in self.active_connections:
            if player_id in self.active_connections[session_id]:
                del self.active_connections[session_id][player_id]
            if not self.active_connections[session_id]:
                del self.active_connections[session_id]

    async def broadcast_state(self, session_id: str):
        if session_id in self.active_connections:
            state = agent.state_store.load_state(session_id)
            state_json = json.dumps(state)
            for player_id, ws in self.active_connections[session_id].items():
                sanitized_raw = mtg_logic_core.sanitize_state(state_json, player_id)
                await ws.send_json({
                    "type": "game_state",
                    "state": json.loads(sanitized_raw)
                })

    async def send_error(self, session_id: str, player_id: str, error: dict):
        if session_id in self.active_connections and player_id in self.active_connections[session_id]:
            ws = self.active_connections[session_id][player_id]
            await ws.send_json({
                "type": "error",
                "error": error
            })

    async def send_message(self, session_id: str, player_id: str, message: str):
        if session_id in self.active_connections and player_id in self.active_connections[session_id]:
            ws = self.active_connections[session_id][player_id]
            await ws.send_json({
                "type": "message",
                "content": message
            })

manager = ConnectionManager()

@app.get("/state/{session_id}/{player_id}")
async def get_sanitized_state(session_id: str, player_id: str):
    state = agent.state_store.load_state(session_id)
    sanitized_raw = mtg_logic_core.sanitize_state(json.dumps(state), player_id)
    return json.loads(sanitized_raw)

@app.websocket("/ws/{session_id}/{player_id}")
async def websocket_endpoint(websocket: WebSocket, session_id: str, player_id: str):
    await manager.connect(websocket, session_id, player_id)
    
    # Send initial state
    await manager.broadcast_state(session_id)
    
    try:
        while True:
            data = await websocket.receive_text()
            try:
                payload = json.loads(data)
            except json.JSONDecodeError:
                await manager.send_error(session_id, player_id, {"type": "ParseError", "details": "Invalid JSON payload."})
                continue
            
            is_action = False
            # Try to parse as Action
            try:
                # We need a proper way to validate the union
                action_type = payload.get("type")
                if action_type in ["CastSpell", "PlayLand", "ActivateAbility", "DeclareAttackers", "DeclareBlockers", "PassPriority"]:
                    is_action = True
            except:
                pass
            
            if is_action:
                state = agent.state_store.load_state(session_id)
                state['pending_action'] = payload
                ruling_raw = mtg_logic_core.apply_action(json.dumps(state))
                ruling = json.loads(ruling_raw)
                
                if ruling.get("status") == "error":
                    await manager.send_error(session_id, player_id, ruling.get("error", {}))
                else:
                    new_state = ruling.get("state", state)
                    # Reset consecutive passes on active actions unless it was PassPriority
                    if payload.get("type") != "PassPriority":
                        new_state["consecutive_passes"] = 0
                    agent.state_store.save_state(session_id, new_state)
                    await manager.broadcast_state(session_id)
            else:
                # Treat as NLP query
                query = payload.get("query", data)
                classification = classify_query(query)
                
                if classification.intent == Intent.DEFINITION:
                    ans = handle_definition(query, classification.entities)
                    await manager.send_message(session_id, player_id, ans)
                elif classification.intent == Intent.INTERACTION:
                    ans = handle_interaction(query, classification.entities)
                    await manager.send_message(session_id, player_id, ans)
                elif classification.intent == Intent.SIMULATION:
                    state = agent.state_store.load_state(session_id)
                    from python_agent.models import GameState
                    gs = GameState(**state)
                    action_model = translate_to_action(query, gs)
                    
                    state['pending_action'] = action_model.model_dump()
                    ruling_raw = mtg_logic_core.apply_action(json.dumps(state))
                    ruling = json.loads(ruling_raw)
                    
                    if ruling.get("status") == "error":
                        await manager.send_error(session_id, player_id, ruling.get("error", {}))
                    else:
                        new_state = ruling.get("state", state)
                        if action_model.type != "PassPriority":
                            new_state["consecutive_passes"] = 0
                        agent.state_store.save_state(session_id, new_state)
                        await manager.broadcast_state(session_id)

    except WebSocketDisconnect:
        manager.disconnect(session_id, player_id)
    except Exception as e:
        await manager.send_error(session_id, player_id, {"type": "InternalError", "details": str(e)})

