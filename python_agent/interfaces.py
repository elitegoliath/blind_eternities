# python_agent/interfaces.py
from typing import Protocol, Any, Dict

class StateStoreBackend(Protocol):
    """Protocol for abstracting game state storage."""
    def load_state(self, session_id: str) -> dict:
        ...
        
    def save_state(self, session_id: str, state: dict) -> None:
        ...

class LLMInterceptor(Protocol):
    """Protocol for abstracting LLM response parsing/intercepting (e.g., extracting buried tool calls)."""
    def __call__(self, state: dict) -> dict:
        ...

class AgentMiddleware(Protocol):
    """Protocol for pre-action and post-action hooks."""
    def __call__(self, state: dict) -> dict:
        ...
