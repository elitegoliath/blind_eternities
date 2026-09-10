from fastapi import FastAPI, WebSocket, WebSocketDisconnect
from python_agent.main import BlindEternitiesAgent
from python_agent.state_store import LocalJSONStateStore
from python_agent.interceptors import QwenToolInterceptor

app = FastAPI(title="Blind Eternities Engine API")

# Default core setup (open-source implementation)
agent = BlindEternitiesAgent(
    state_store=LocalJSONStateStore(),
    interceptor=QwenToolInterceptor()
)

@app.websocket("/ws/{session_id}")
async def websocket_endpoint(websocket: WebSocket, session_id: str):
    await websocket.accept()
    try:
        while True:
            # Wait for user input from the client (e.g. React Native)
            data = await websocket.receive_text()
            
            # Process via LangGraph and stream structured JSON events back to client immediately
            async for event in agent.stream_events(session_id, data):
                await websocket.send_json(event)
                
    except WebSocketDisconnect:
        print(f"Client {session_id} disconnected.")
    except Exception as e:
        print(f"WebSocket error for {session_id}: {e}")
