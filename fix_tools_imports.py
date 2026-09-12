with open("python_agent/tools.py", "r") as f:
    content = f.read()

imports = """import ast
import json
import os
import re
from pathlib import Path
from typing import Any, List, Optional, Union

import lancedb
from fastembed import TextEmbedding
from langchain_core.runnables import RunnableConfig
from langchain_core.tools import tool

import mtg_logic_core  # type: ignore # <--- This is the compiled Rust code!
from .state_store import LocalJSONStateStore

# Initialize the models outside the function so they stay hot in memory
"""

# Replace the original import section
lines = content.split('\n')
start = 0
for i, line in enumerate(lines):
    if line.startswith("import os"):
        start = i
        break
end = 0
for i, line in enumerate(lines):
    if line.startswith("# Initialize the models outside"):
        end = i
        break

new_content = '\n'.join(lines[:start]) + '\n' + imports + '\n'.join(lines[end+1:])

with open("python_agent/tools.py", "w") as f:
    f.write(new_content)
