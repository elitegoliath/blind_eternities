# python_agent/state_store.py
import json
from pathlib import Path
from .interfaces import StateStoreBackend

class LocalJSONStateStore(StateStoreBackend):
    """Default local JSON-backed state store."""
    def __init__(self, base_dir: str = "."):
        self.base_dir = Path(base_dir)
        self.base_dir.mkdir(parents=True, exist_ok=True)

    def _get_file_path(self, session_id: str) -> Path:
        return self.base_dir / f"session_{session_id}.json"

    def load_state(self, session_id: str) -> dict:
        state_file = self._get_file_path(session_id)
        if state_file.exists():
            try:
                with open(state_file, "r") as f:
                    return json.load(f)
            except json.JSONDecodeError:
                pass
                
        # Default starting state
        return {
            "active_player": "Player",
            "is_active_player": True,
            "phase": "Main Phase 1",
            "battlefield": [],
            "stack": [],
            "lands_played": 0,
            "mana_pool": {"w": 0, "u": 0, "b": 0, "r": 0, "g": 0, "c": 0},
            "consecutive_passes": 0
        }

    def save_state(self, session_id: str, state: dict) -> None:
        state_file = self._get_file_path(session_id)
        with open(state_file, "w") as f:
            json.dump(state, f, indent=2)
