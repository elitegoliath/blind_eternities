# python_agent/__init__.py

# For a modern Python application, we use this file to expose a "Clean API"
# to the rest of the system (or tests).
# This allows us to import core logic using from python_agent import agent_loop
# instead of digging into submodules.

# Expose the specific functions meant to be public
# from .tools import validate_move, search_rules
from .tools import validate_move
from .llm_engine import get_llm, SYSTEM_PROMPT

# Define what happens on 'from python_agent import *'
# __all__ = ["validate_move", "search_rules", "get_llm", "SYSTEM_PROMPT"]
__all__ = ["validate_move", "get_llm", "SYSTEM_PROMPT"]